use crate::{
    arch::x86_64::{
        gdt::load_gdt_x86_64,
        heap::init_heap,
        interrupts::{PICS, init_idt_x86_64},
        kmi::{LAPIC_BASE, setup_kmi},
        paging::{FRAME_ALLOCATOR_X86_64, MMIO_VIRT_BASE, initialize_paging_x86_64, map_mmio},
        syscall::init_syscalls,
    },
    config::TIMER_FREQUENCY_HZ,
    driver::{
        io::{
            block::{
                ahci::{device::SATADevice, discover_ahci},
                gpt::find_gpt,
            },
            fs::{
                FileSystem,
                ext2::fs::Ext2FileSystem,
                vfs::{VFS_INSTANCE, init_vfs},
            },
            pci::discover_devices,
        },
        ipc::init_ipc,
    },
    mm::shm::init_shm,
    println,
};

use alloc::{sync::Arc, vec::Vec};
use x86_64::{
    VirtAddr,
    instructions::{interrupts, interrupts::without_interrupts, port::Port},
    registers::control::{Cr0, Cr0Flags, Cr4, Cr4Flags},
    structures::paging::OffsetPageTable,
};

use limine::request::{HhdmRepsonse, MemmapResponse};

const PIT_DIVISOR: u16 = (1_193_182_u32 / TIMER_FREQUENCY_HZ as u32) as u16;

pub fn memory_management_init(
    hhdm_response: &HhdmRepsonse,
    memory_map_response: &MemmapResponse,
) -> OffsetPageTable<'static> {
    let physical_offset = VirtAddr::new(hhdm_response.offset);
    let mapper = unsafe { initialize_paging_x86_64(physical_offset) };
    let mut frame_allocator = FRAME_ALLOCATOR_X86_64.lock();
    frame_allocator.initialize(memory_map_response.entries());
    drop(frame_allocator);
    mapper
}

pub fn set_pit_interval() {
    without_interrupts(|| {
        let mut command_port: Port<u8> = Port::new(0x43);
        let mut data_port: Port<u8> = Port::new(0x40);

        unsafe {
            command_port.write(0b00_11_011_0);
            data_port.write((PIT_DIVISOR & 0xFF) as u8); // low byte
            data_port.write(((PIT_DIVISOR >> 8) & 0xFF) as u8); // high byte
        }
    });
}

pub fn init_x86_64<'a>(
    hhdm_response: &HhdmRepsonse,
    memory_map_response: &'a MemmapResponse,
) -> OffsetPageTable<'a> {
    load_gdt_x86_64();

    unsafe {
        // setup SSE (SIMD = Single Instruction, Multiple Data)
        let mut cr0 = Cr0::read();
        cr0.remove(Cr0Flags::EMULATE_COPROCESSOR);
        cr0.insert(Cr0Flags::MONITOR_COPROCESSOR);
        Cr0::write(cr0);

        let mut cr4 = Cr4::read();
        cr4.insert(Cr4Flags::OSFXSR);
        cr4.insert(Cr4Flags::OSXMMEXCPT_ENABLE);
        Cr4::write(cr4);
    }

    init_idt_x86_64();

    init_syscalls();

    unsafe {
        let mut pics = PICS.lock();
        pics.initialize();
    }

    setup_kmi();

    set_pit_interval();

    let mut mapper = memory_management_init(hhdm_response, memory_map_response);

    init_heap(&mut mapper)
        .ok()
        .expect("Failed to initalize heap");

    unsafe {
        use x86_64::registers::model_specific::Msr;
        let mut apic_base = Msr::new(0x1B); // IA32_APIC_BASE
        let val = apic_base.read();
        apic_base.write(val | (1 << 11)); // enable APIC with bit 11

        let apic_phys = val & 0x000F_FFFF_FFFF_F000;
        map_mmio(apic_phys, &mut mapper, 0x1000);
        let apic = (MMIO_VIRT_BASE + apic_phys) as *mut u32;

        core::ptr::write_volatile(apic.add(0xF0 / 4), 0x1FF); // enable SVR
        core::ptr::write_volatile(apic.add(0x80 / 4), 0); // enable TPR
        core::ptr::write_volatile(apic.add(0x350 / 4), 0x10700); // mask LINT0

        map_mmio(0xFEC0_0000, &mut mapper, 0x1000);
        let io = (MMIO_VIRT_BASE + 0xFEC0_0000) as *mut u32;

        let route = |pin: u32, vector: u32| {
            core::ptr::write_volatile(io, 0x11 + pin * 2);
            core::ptr::write_volatile(io.add(4), 0);
            core::ptr::write_volatile(io, 0x10 + pin * 2);
            core::ptr::write_volatile(io.add(4), vector);
        };

        route(2, 32); // PIT
        route(1, 33); // keyboard
        route(12, 44); // mouse

        LAPIC_BASE = MMIO_VIRT_BASE + apic_phys;

        PICS.lock().write_masks(0xFF, 0xFF);
    }

    interrupts::enable();

    init_ipc();
    init_shm();
    init_vfs();

    let pci_devices = discover_devices();
    let controllers = discover_ahci(pci_devices, &mut mapper);
    let mut sata_devices = Vec::<SATADevice>::new();

    for controller in controllers {
        for sata_device_opt in controller.sata_devices {
            if let Some(sata_device) = sata_device_opt {
                println!(
                    "SATA Device: sector_size={} capacity={}",
                    sata_device.sector_size, sata_device.capacity_sectors
                );
                if let Some(partition_table) = find_gpt(&sata_device) {
                    println!(
                        "GPT Partition Table: rev={}, guid={} partition_count={}",
                        partition_table.revision,
                        partition_table.guid,
                        partition_table.partitions.len()
                    );
                    for partition in partition_table.partitions {
                        println!(
                            "Partition: name={} start={}, end={} type_guid={} unique_guid={}",
                            partition.name,
                            partition.start,
                            partition.end,
                            partition.partition_type_guid,
                            partition.unique_partition_guid
                        );
                        if let Ok(filesystem) = Ext2FileSystem::new(
                            partition.start * (sata_device.sector_size as u64),
                            Arc::new(sata_device.clone()),
                        ) {
                            println!("Ext2 Filesystem found");
                            match filesystem.list_directory("/") {
                                Ok(contents) => {
                                    println!("Contents of /: {:?}", contents)
                                }
                                Err(err) => {
                                    println!("Could not get contents of /: {:?}", err)
                                }
                            }
                            #[allow(static_mut_refs)]
                            unsafe {
                                VFS_INSTANCE
                                    .as_mut()
                                    .unwrap()
                                    .add_filesystem(Arc::new(filesystem));
                            }
                        }
                    }
                }

                sata_devices.push(sata_device);
            }
        }
    }

    return mapper;
}
