use crate::arch::x86_64::gdt::load_gdt_x86_64;
use crate::arch::x86_64::interrupts::{PICS, init_idt_x86_64};
use limine::response::{HhdmResponse, MemoryMapResponse};
use x86_64::instructions::interrupts;

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
    let frame_allocator = XunilFrameAllocator::new(memory_map_response.entries());
    (mapper, frame_allocator)
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
        pics.write_masks(0xFC, 0xFF);
    }

    interrupts::enable();

    let (mut mapper, mut frame_allocator) =
        memory_management_init(hhdm_response, memory_map_response);

    init_heap(&mut mapper, &mut frame_allocator)
        .ok()
        .expect("Failed to initalize heap");

    return (mapper, frame_allocator);
}
