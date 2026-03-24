use core::arch::asm;
use limine::response::{HhdmResponse, MemoryMapResponse};

#[cfg(target_arch = "x86_64")]
use crate::arch::x86_64::{
    init::init_x86_64,
    paging::{XunilFrameAllocator, example_mapping, initialize_paging},
};
#[cfg(target_arch = "x86_64")]
use x86_64::{
    VirtAddr, registers::control::Cr3, structures::paging::OffsetPageTable,
    structures::paging::Page,
};

#[cfg(target_arch = "x86_64")]
pub fn memory_management_init(
    hhdm_response: &HhdmResponse,
    memory_map_response: &MemoryMapResponse,
) -> OffsetPageTable<'static> {
    let physical_offset = VirtAddr::new(hhdm_response.offset());
    let (frame, _) = Cr3::read();
    let mut mapper = unsafe { initialize_paging(physical_offset) };

    let l4_virt = physical_offset + frame.start_address().as_u64() + 0xb8000;
    let mut frame_allocator = XunilFrameAllocator::new(memory_map_response.entries());
    let page = Page::containing_address(l4_virt);
    unsafe {
        example_mapping(page, &mut mapper, &mut frame_allocator);
    }
    mapper
}

#[cfg(target_arch = "x86_64")]
pub fn init(
    hhdm_response: &HhdmResponse,
    memory_map_response: &MemoryMapResponse,
) -> OffsetPageTable<'static> {
    init_x86_64();

    return memory_management_init(hhdm_response, memory_map_response);
}

pub fn idle() -> ! {
    loop {
        unsafe {
            #[cfg(target_arch = "x86_64")]
            asm!("hlt");
            #[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
            asm!("wfi");
            #[cfg(target_arch = "loongarch64")]
            asm!("idle 0");
        }
    }
}
