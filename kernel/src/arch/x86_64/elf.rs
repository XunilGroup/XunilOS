use alloc::vec::Vec;
use x86_64::{
    VirtAddr,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTableFlags, PhysFrame, Size4KiB,
    },
};

use crate::arch::x86_64::{paging::XunilFrameAllocator, usermode::enter_usermode_x86_64};

pub fn run_elf_x86_64(
    entry_point: *const u8,
    frame_allocator: &mut XunilFrameAllocator,
    mapper: &mut OffsetPageTable,
) {
    let stack_base: u64 = 0x0000_7fff_0000_0000;
    let page_count = 3;
    let page_size = 0x1000u64;

    let mut frames: Vec<PhysFrame<Size4KiB>> = Vec::new();
    for i in 0..page_count {
        let frame = frame_allocator.allocate_frame().unwrap();
        frames.push(frame);

        let virt_addr = VirtAddr::new(stack_base + i as u64 * page_size);
        let page = Page::<Size4KiB>::containing_address(virt_addr);

        unsafe {
            mapper
                .map_to(
                    page,
                    frame,
                    PageTableFlags::PRESENT
                        | PageTableFlags::WRITABLE
                        | PageTableFlags::USER_ACCESSIBLE,
                    frame_allocator,
                )
                .unwrap()
                .flush();
        }
    }

    let stack_top = stack_base + (page_count as u64 * page_size);
    let rsp = (stack_top & !0xF) - 8;

    enter_usermode_x86_64(entry_point as u64, rsp);
}
