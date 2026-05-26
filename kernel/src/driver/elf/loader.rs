use core::{
    ptr::null,
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(target_arch = "x86_64")]
use crate::arch::x86_64::{interrupts, paging::create_and_map_multiple_pages};
#[allow(unused_imports)]
use crate::driver::elf::{
    header::{
        EI_CLASS, EI_DATA, EI_VERSION, ELF_MAGIC, EM_AARCH64, EM_X86_64, ET_DYN, ET_EXEC, ET_REL,
        Elf64Ehdr,
    },
    program::load_program,
};

#[cfg(target_arch = "aarch64")]
use crate::arch::aarch64::paging::{
    AArchPageTable, create_and_map_multiple_pages, kernel_data_flags, user_data_flags,
};
use crate::{
    arch::arch::KERNEL_MAPPER,
    mm::address_space::AddressSpace,
    println,
    task::{context::UserContext, scheduler::SCHEDULER},
};
#[cfg(target_arch = "x86_64")]
use x86_64::{
    registers::control::{Cr3, Cr3Flags},
    structures::paging::{OffsetPageTable, PageTableFlags, PhysFrame, Size4KiB},
};

#[cfg(target_arch = "aarch64")]
type PageTable = AArchPageTable;

#[cfg(target_arch = "x86_64")]
type PageTable<'a> = OffsetPageTable<'a>;

static mut NEXT_KERNEL_STACK_ADDR: AtomicU64 = AtomicU64::new(0xFFFF_FFFF_7000_0000);

pub fn next_kstack(mapper: &mut PageTable) -> u64 {
    const PAGES: u64 = 2048;
    #[allow(static_mut_refs)]
    let addr = unsafe { NEXT_KERNEL_STACK_ADDR.fetch_add(PAGES * 4096, Ordering::Relaxed) };

    #[cfg(target_arch = "aarch64")]
    create_and_map_multiple_pages(mapper, PAGES, addr, kernel_data_flags());

    #[cfg(target_arch = "x86_64")]
    create_and_map_multiple_pages(
        mapper,
        PAGES,
        addr,
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
    );

    assert!(addr & 0xF == 0, "kernel stack isn't 16 byte aligned");

    addr + PAGES * 4096
}

pub fn validate_elf(elf_header: &Elf64Ehdr, elf_len: usize) -> bool {
    #[cfg(target_arch = "x86_64")]
    let required_machine = EM_X86_64;

    #[cfg(target_arch = "aarch64")]
    let required_machine = EM_AARCH64;

    elf_header.e_ident[0..4] == ELF_MAGIC
    // 64 bit
    && elf_header.e_ident[EI_CLASS] == 2
    // little-endian
    && elf_header.e_ident[EI_DATA] == 1
    && elf_header.e_ident[EI_VERSION] == 1
    && elf_header.e_version == 1
    // check architecture
    && elf_header.e_machine == required_machine
    // disallow object files
    && [ET_DYN, ET_EXEC].contains(&elf_header.e_type)
    // standard elf64
    && elf_header.e_phentsize == 56
    && elf_header.e_phnum != 0 // zero program headers
    && (elf_header.e_phoff + (elf_header.e_phnum*elf_header.e_phentsize) as u64) <= elf_len as u64
}

pub fn load_file(mapper: &mut PageTable, elf_bytes: &[u8]) -> (*const u8, u64) {
    // elf header size
    if elf_bytes.len() < 64 {
        return (null(), 0);
    }

    let elf_header: Elf64Ehdr =
        unsafe { core::ptr::read_unaligned(elf_bytes.as_ptr() as *const Elf64Ehdr) };

    if !validate_elf(&elf_header, elf_bytes.len()) {
        panic!("Invalid ELF");
    }

    let elf_header_ptr = elf_bytes.as_ptr() as *const Elf64Ehdr;

    return match elf_header.e_type {
        ET_EXEC => unsafe { load_program(mapper, elf_header_ptr, elf_bytes, false) },
        ET_DYN => unsafe { load_program(mapper, elf_header_ptr, elf_bytes, true) },
        ET_REL => return (null(), 0),
        _ => return (null(), 0),
    };
}

pub fn run_elf(file_bytes: &[u8], should_swapgs: bool, switch_to: bool) {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!("msr daifset, #2")
    };

    #[cfg(target_arch = "x86_64")]
    {
        use x86_64::instructions::interrupts;
        interrupts::disable();
    }

    let stack_base: u64 = 0x0000_7fff_0000_0000;
    let page_count = 4096; // 16 mib
    let page_size = 0x1000u64;
    let stack_top = stack_base + (page_count as u64 * page_size);

    if let Some(mut address_space) = AddressSpace::new() {
        #[cfg(target_arch = "aarch64")]
        let previous_pt_ptr: u64;
        #[cfg(target_arch = "aarch64")]
        unsafe {
            core::arch::asm!("mrs {}, ttbr0_el1", out(reg) previous_pt_ptr)
        };

        #[cfg(target_arch = "x86_64")]
        let previous_pt: (PhysFrame<Size4KiB>, Cr3Flags);

        #[cfg(target_arch = "x86_64")]
        {
            use x86_64::registers::control::Cr3;

            previous_pt = Cr3::read();
        }

        address_space.use_address_space();

        let (entry_point, heap_base) = load_file(&mut address_space.mapper, file_bytes);

        println!("Entry point: {:?}", entry_point);

        #[allow(static_mut_refs)]
        let process_pid = SCHEDULER
            .spawn_process(
                entry_point as u64,
                stack_top,
                next_kstack(unsafe { KERNEL_MAPPER.get_mut().as_mut().unwrap() }),
                heap_base,
            )
            .unwrap();

        #[cfg(target_arch = "aarch64")]
        create_and_map_multiple_pages(
            &mut address_space.mapper,
            page_count,
            stack_base,
            user_data_flags(),
        );

        #[cfg(target_arch = "x86_64")]
        create_and_map_multiple_pages(
            &mut address_space.mapper,
            page_count,
            stack_base,
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE,
        );

        SCHEDULER.with_process(process_pid, |process| {
            process.address_space = Some(address_space)
        });

        #[cfg(target_arch = "aarch64")]
        SCHEDULER.with_process(process_pid, |process| {
            process.saved_ctx = Some(UserContext {
                x0: 0,
                x1: 0,
                x2: 0,
                x3: 0,
                x4: 0,
                x5: 0,
                x6: 0,
                x7: 0,
                x8: 0,
                x9: 0,
                x10: 0,
                x11: 0,
                x12: 0,
                x13: 0,
                x14: 0,
                x15: 0,
                x16: 0,
                x17: 0,
                x18: 0,
                x19: 0,
                x20: 0,
                x21: 0,
                x22: 0,
                x23: 0,
                x24: 0,
                x25: 0,
                x26: 0,
                x27: 0,
                x28: 0,
                x29: 0,
                x30: 0,
                elr_el1: entry_point as u64,
                spsr_el1: 0,
                esr_el1: 0,
                far_el1: 0,
                sp_el0: process.stack_top,
                sp_el1: process.kernel_stack_top,
                _pad1: 0,
            });
        });

        if switch_to {
            #[cfg(target_arch = "aarch64")]
            unsafe {
                core::arch::asm!("msr daifset, #2")
            };
            SCHEDULER.switch_to(process_pid, should_swapgs);
        } else {
            #[cfg(target_arch = "aarch64")]
            unsafe {
                core::arch::asm!(
                    "msr ttbr0_el1, {root}",
                    "dsb ishst",
                    "tlbi vmalle1is",
                    "dsb ish",
                    "isb",
                    root = in(reg) previous_pt_ptr
                );
            }

            #[cfg(target_arch = "x86_64")]
            unsafe {
                use x86_64::instructions::interrupts;

                Cr3::write(previous_pt.0, previous_pt.1);
                interrupts::enable();
            };
        }
    } else {
        return;
    };
}
