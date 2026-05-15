use crate::{
    arch::aarch64::paging::{AArchPageTable, alloc_frame, kernel_data_flags},
    mm::heap::LinkedListAllocator,
    util::Locked,
};
#[global_allocator]
pub static ALLOCATOR: Locked<LinkedListAllocator> = Locked::new(LinkedListAllocator::new());

pub const HEAP_START: usize = 0xffffffff90000000;
pub const HEAP_SIZE: usize = 64 * 1024 * 1024; // 64 MiB

pub fn init_heap(mapper: &mut AArchPageTable) {
    let pages = HEAP_SIZE / 4096;

    for i in 0..pages {
        let phys = alloc_frame().expect("frame allocator out of frames");
        let virt = HEAP_START as u64 + i as u64 * 4096;
        mapper.map_page(virt, phys, kernel_data_flags());
    }

    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }
}
