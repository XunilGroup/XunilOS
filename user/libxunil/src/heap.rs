use spin::mutex::Mutex;

use crate::util::align_up;

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 256 * 1024 * 1024; // 256 MiB

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

    pub unsafe fn add_free_memory_region(&mut self, start: usize, size: usize) {
        assert_eq!(align_up(start, core::mem::align_of::<LinkedNode>()), start); // Check if we are up at least 1 LinkedNode size
        assert!(size >= core::mem::size_of::<LinkedNode>()); // check if we have enough space for a LinkedNode

        let mut linked_node = LinkedNode::new(size);
        linked_node.next = self.head.next.take();

        let linked_node_ptr = start as *mut LinkedNode; // Treat the start memory region as a LinkedNode type
        unsafe {
            linked_node_ptr.write(linked_node); // write the data, very risky
            self.head.next = Some(&mut *linked_node_ptr);
        }
    }

    pub fn find_region(
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
        let alloc_start = align_up(region.start_addr() + core::mem::size_of::<usize>(), align);
        let alloc_end = (alloc_start - core::mem::size_of::<usize>())
            .checked_add(size)
            .ok_or(())?; // check for overflows

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

pub static ALLOCATOR: Mutex<LinkedListAllocator> = Mutex::new(LinkedListAllocator::new());
