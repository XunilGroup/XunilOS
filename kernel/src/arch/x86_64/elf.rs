use alloc::vec::Vec;
use x86_64::{
    VirtAddr,
    structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags, PhysFrame, Size4KiB},
};

use crate::{
    arch::arch::FRAME_ALLOCATOR, driver::elf::loader::load_file, mm::address_space::AddressSpace,
    println, task::scheduler::SCHEDULER,
};

pub fn run_elf_x86_64(file_bytes: &[u8], should_swapgs: bool) {
    let stack_base: u64 = 0x0000_7fff_0000_0000;
    let page_count = 4096; // 16 mib
    let page_size = 0x1000u64;
    let stack_top = stack_base + (page_count as u64 * page_size);

    if let Some(mut address_space) = AddressSpace::new() {
        address_space.use_address_space();

        let (entry_point, heap_base) = load_file(&mut address_space.mapper, file_bytes);

        println!("Entry point: {:?}", entry_point);

        let process_pid = SCHEDULER
            .spawn_process(entry_point as u64, stack_top, heap_base)
            .unwrap();

        let mut frames: Vec<PhysFrame<Size4KiB>> = Vec::new();
        let mut frame_allocator = FRAME_ALLOCATOR.lock();

        for i in 0..page_count {
            let frame = frame_allocator.allocate_frame().unwrap();
            frames.push(frame);

            let virt_addr = VirtAddr::new(stack_base + i as u64 * page_size);
            let page = Page::<Size4KiB>::containing_address(virt_addr);

            unsafe {
                address_space
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
            }
        }
        drop(frame_allocator);

        SCHEDULER.with_process(process_pid, |process| {
            process.address_space = Some(address_space)
        });

        SCHEDULER.switch_to(process_pid, should_swapgs);
    } else {
        return;
    };
}
