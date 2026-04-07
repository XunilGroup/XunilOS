use alloc::vec::Vec;
use x86_64::{
    VirtAddr,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTableFlags, PhysFrame, Size4KiB,
    },
};

use crate::{
    arch::{
        arch::FRAME_ALLOCATOR,
        x86_64::{paging::XunilFrameAllocator, usermode::enter_usermode_x86_64},
    },
    task::{process::Process, scheduler::SCHEDULER},
};

pub fn run_elf_x86_64(entry_point: *const u8, heap_base: u64) {
    let stack_base: u64 = 0x0000_7fff_0000_0000;
    let page_count = 4096; // 16 mib
    let page_size = 0x1000u64;
    let stack_top = stack_base + (page_count as u64 * page_size);

    let process_pid = SCHEDULER
        .spawn_process(entry_point as u64, stack_top, heap_base)
        .unwrap();

    SCHEDULER.with_process(process_pid, |process| {
        process.address_space.use_address_space()
    });

    let mut frames: Vec<PhysFrame<Size4KiB>> = Vec::new();
    let mut frame_allocator = FRAME_ALLOCATOR.lock();

    for i in 0..page_count {
        let frame = frame_allocator.allocate_frame().unwrap();
        frames.push(frame);

        let virt_addr = VirtAddr::new(stack_base + i as u64 * page_size);
        let page = Page::<Size4KiB>::containing_address(virt_addr);

        unsafe {
            SCHEDULER.with_process(process_pid, |process| {
                process
                    .address_space
                    .mapper
                    .map_to(
                        page,
                        frame,
                        PageTableFlags::PRESENT
                            | PageTableFlags::WRITABLE
                            | PageTableFlags::USER_ACCESSIBLE,
                        &mut *frame_allocator,
                    )
                    .unwrap()
                    .flush();
            });
        }
    }
    drop(frame_allocator);

    SCHEDULER.run_process(process_pid, entry_point);
}
