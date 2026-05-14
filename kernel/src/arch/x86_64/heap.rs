use crate::util::Locked;
use crate::{arch::x86_64::paging::FRAME_ALLOCATOR_X86_64, mm::heap::LinkedListAllocator};
use x86_64::{
    VirtAddr,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTableFlags as Flags, Size4KiB,
        mapper::MapToError,
    },
};

#[global_allocator]
pub static ALLOCATOR: Locked<LinkedListAllocator> = Locked::new(LinkedListAllocator::new());

pub const HEAP_START: usize = 0xffffffff90000000;
pub const HEAP_SIZE: usize = 64 * 1024 * 1024; // 64 MiB

pub fn init_heap(mapper: &mut OffsetPageTable) -> Result<(), MapToError<Size4KiB>> {
    let page_range = {
        let page_start = VirtAddr::new(HEAP_START as u64);
        let page_end = page_start + HEAP_SIZE as u64 - 1u64;
        let heap_start_page: Page<Size4KiB> = Page::containing_address(page_start);
        let heap_end_page: Page<Size4KiB> = Page::containing_address(page_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    let mut frame_allocator = FRAME_ALLOCATOR_X86_64.lock();

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::<Size4KiB>::FrameAllocationFailed)?;
        let flags = Flags::PRESENT | Flags::WRITABLE;
        unsafe {
            mapper
                .map_to(page, frame, flags, &mut *frame_allocator)
                .map_err(|e| e)?
                .flush();
        }
    }

    drop(frame_allocator);

    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }

    Ok(())
}
