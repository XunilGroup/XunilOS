use crate::{
    arch::x86_64::{
        gdt::load_gdt_x86_64,
        interrupts::{PICS, init_idt_x86_64},
        mouse::setup_mouse,
    },
    driver::mouse::MOUSE,
};
use limine::response::{HhdmResponse, MemoryMapResponse};
use x86_64::instructions::interrupts::without_interrupts;
use x86_64::instructions::{interrupts, port::Port};

const TIMER_PRECISION_HZ: u32 = 1000;
const PIT_DIVISOR: u16 = (1_193_182_u32 / TIMER_PRECISION_HZ) as u16;

#[cfg(target_arch = "x86_64")]
use crate::arch::x86_64::{
    heap::init_heap,
    paging::{XunilFrameAllocator, initialize_paging},
};
#[cfg(target_arch = "x86_64")]
use x86_64::{VirtAddr, structures::paging::OffsetPageTable};

#[cfg(target_arch = "x86_64")]
pub fn memory_management_init<'a>(
    hhdm_response: &HhdmResponse,
    memory_map_response: &'a MemoryMapResponse,
) -> (OffsetPageTable<'static>, XunilFrameAllocator<'a>) {
    let physical_offset = VirtAddr::new(hhdm_response.offset());
    let mapper = unsafe { initialize_paging(physical_offset) };
    let frame_allocator =
        XunilFrameAllocator::new(hhdm_response.offset(), memory_map_response.entries());
    (mapper, frame_allocator)
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
    hhdm_response: &HhdmResponse,
    memory_map_response: &'a MemoryMapResponse,
) -> (OffsetPageTable<'static>, XunilFrameAllocator<'a>) {
    load_gdt_x86_64();
    init_idt_x86_64();

    unsafe {
        let mut pics = PICS.lock();
        pics.initialize();
        let master_mask = 0xF8; // unmask cascade to slave
        let slave_mask = 0xEF; // unmask mouse interrupt (clear bit 4)
        pics.write_masks(master_mask, slave_mask);
    }

    let mouse_status = setup_mouse();
    set_pit_interval();

    interrupts::enable();

    let (mut mapper, mut frame_allocator) =
        memory_management_init(hhdm_response, memory_map_response);

    init_heap(&mut mapper, &mut frame_allocator)
        .ok()
        .expect("Failed to initalize heap");

    MOUSE.set_status(mouse_status);

    return (mapper, frame_allocator);
}
