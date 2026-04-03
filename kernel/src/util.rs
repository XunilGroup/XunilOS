use crate::{driver::timer::TIMER, println};
use spin::{Mutex, MutexGuard};

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

pub fn test_performance<F: FnOnce()>(function: F) {
    let start = TIMER.now();
    let ret = function();
    println!("took {} ms", (TIMER.now() - start).elapsed());
    ret
}

pub fn get_bit(value: u8, position: u8) -> u8 {
    (value >> position) & 1
}

#[inline]
pub const fn align_down(addr: u64, align: u64) -> u64 {
    assert!(align.is_power_of_two(), "`align` must be a power of two");
    addr & !(align - 1)
}

#[inline]
pub const fn align_up(addr: u64, align: u64) -> u64 {
    assert!(align.is_power_of_two(), "`align` must be a power of two");
    let align_mask = align - 1;
    if addr & align_mask == 0 {
        addr
    } else {
        if let Some(aligned) = (addr | align_mask).checked_add(1) {
            aligned
        } else {
            panic!("attempt to add with overflow")
        }
    }
}
