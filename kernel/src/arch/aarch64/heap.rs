use crate::{mm::heap::LinkedListAllocator, util::Locked};
#[global_allocator]
pub static ALLOCATOR: Locked<LinkedListAllocator> = Locked::new(LinkedListAllocator::new());
