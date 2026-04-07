use crate::arch::x86_64::paging::{FRAME_ALLOCATOR_X86_64, XunilFrameAllocator};
use crate::util::{Locked, serial_print};
use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::null_mut,
};
use x86_64::{
    VirtAddr,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTableFlags as Flags, Size4KiB,
        mapper::MapToError,
    },
};

fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

#[global_allocator]
pub static ALLOCATOR: Locked<LinkedListAllocator> = Locked::new(LinkedListAllocator::new());

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 64 * 1024 * 1024; // 64 MiB

pub struct LinkedNode {
    pub size: usize,
    pub next: Option<&'static mut LinkedNode>,
}

impl LinkedNode {
    pub const fn new(size: usize) -> LinkedNode {
        LinkedNode { size, next: None }
    }

    pub fn start_addr(&self) -> usize {
        self as *const Self as usize
    }

    pub fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}

pub struct LinkedListAllocator {
    head: LinkedNode,
}

impl LinkedListAllocator {
    pub const fn new() -> LinkedListAllocator {
        Self {
            head: LinkedNode::new(0),
        }
    }

    fn size_align(layout: Layout) -> (usize, usize) {
        let layout = layout
            .align_to(16)
            .expect("Align to LinkedNode failed")
            .pad_to_align();

        let size = layout.size().max(core::mem::size_of::<LinkedNode>()); // either take layout's size or atleast the size of a single linked node.

        (size, layout.align())
    }

    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        unsafe {
            self.add_free_memory_region(heap_start, heap_size);
        }
    }

    unsafe fn add_free_memory_region(&mut self, start: usize, size: usize) {
        assert_eq!(align_up(start, 16), start); // Check if we are up at least 1 LinkedNode size
        assert!(size >= core::mem::size_of::<LinkedNode>()); // check if we have enough space for a LinkedNode

        let mut linked_node = LinkedNode::new(size);
        linked_node.next = self.head.next.take();

        let linked_node_ptr = start as *mut LinkedNode; // Treat the start memory region as a LinkedNode type
        unsafe {
            linked_node_ptr.write(linked_node); // write the data, very risky
            self.head.next = Some(&mut *linked_node_ptr);
        }
    }

    fn find_region(
        &mut self,
        size: usize,
        align: usize,
    ) -> Option<(&'static mut LinkedNode, usize)> {
        let mut current = &mut self.head;

        while let Some(ref mut region) = current.next {
            if let Ok(alloc_start) = Self::alloc_from_region(&region, size, align) {
                let next = region.next.take();
                let ret = Some((current.next.take().unwrap(), alloc_start));
                current.next = next;

                return ret;
            } else {
                current = current.next.as_mut().unwrap();
            }
        }

        None
    }

    fn alloc_from_region(region: &LinkedNode, size: usize, align: usize) -> Result<usize, ()> {
        let alloc_start = align_up(region.start_addr(), align);
        let alloc_end = alloc_start.checked_add(size).ok_or(())?; // check for overflows

        if alloc_end > region.end_addr() {
            return Err(());
        }

        let excess_size = region.end_addr() - alloc_end;
        if excess_size > 0 && excess_size < core::mem::size_of::<LinkedNode>() {
            // if the remaining space is not enough for another LinkedNode, skip this region.
            return Err(());
        }

        Ok(alloc_start)
    }
}

unsafe impl GlobalAlloc for Locked<LinkedListAllocator> {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        let (size, align) = LinkedListAllocator::size_align(_layout);
        let mut allocator = self.lock();

        if let Some((region, alloc_start)) = allocator.find_region(size, align) {
            let alloc_end = alloc_start.checked_add(size).expect("overflow");

            let excess_size = region.end_addr() - alloc_end;

            if excess_size > 0 {
                unsafe {
                    allocator.add_free_memory_region(alloc_end, excess_size);
                }
            }

            drop(allocator);

            alloc_start as *mut u8
        } else {
            null_mut()
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        let (size, _) = LinkedListAllocator::size_align(_layout);

        unsafe {
            self.lock().add_free_memory_region(_ptr as usize, size);
        }
    }
}

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
