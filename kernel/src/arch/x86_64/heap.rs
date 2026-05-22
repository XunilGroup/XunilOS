use crate::arch::x86_64::paging::create_and_map_multiple_pages;
use crate::mm::heap::LinkedListAllocator;
use crate::util::Locked;
use x86_64::structures::paging::{
    OffsetPageTable, PageTableFlags as Flags, Size4KiB, mapper::MapToError,
};

#[global_allocator]
pub static ALLOCATOR: Locked<LinkedListAllocator> = Locked::new(LinkedListAllocator::new());

pub const HEAP_START: usize = 0xffffffff90000000;
pub const HEAP_SIZE: usize = 4 * 1024 * 1024; // 64 MiB

pub fn init_heap(mapper: &mut OffsetPageTable) -> Result<(), MapToError<Size4KiB>> {
    let page_count = HEAP_SIZE / 4096;

    create_and_map_multiple_pages(
        mapper,
        page_count as u64,
        HEAP_START as u64,
        Flags::PRESENT | Flags::WRITABLE,
    );

    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }

    Ok(())
}
