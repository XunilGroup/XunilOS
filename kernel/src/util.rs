use spin::{Mutex, MutexGuard};

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

pub struct Locked<A> {
    inner: Mutex<A>,
}

impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> MutexGuard<A> {
        self.inner.lock()
    }
}
