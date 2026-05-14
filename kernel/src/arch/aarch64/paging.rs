use crate::arch::arch::XunilFrameAllocator;
use spin::Mutex;

pub static FRAME_ALLOCATOR_AARCH64: Mutex<XunilFrameAllocator> =
    Mutex::new(XunilFrameAllocator::new());
