use crate::{driver::timer::TIMER, println};
use spin::{Mutex, MutexGuard};

pub struct U64Buf {
    buf: [u8; 20],
    start: usize,
}

impl U64Buf {
    pub fn new(n: u64) -> Self {
        let mut buf = [0u8; 20];
        let mut pos = 20;
        let mut val = n;

        if val == 0 {
            buf[19] = b'0';
            return Self { buf, start: 19 };
        }

        while val > 0 {
            pos -= 1;
            buf[pos] = b'0' + (val % 10) as u8;
            val /= 10;
        }

        Self { buf, start: pos }
    }

    pub fn as_str(&self) -> &str {
        unsafe { core::str::from_utf8_unchecked(&self.buf[self.start..]) }
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

    #[allow(mismatched_lifetime_syntaxes)]
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
