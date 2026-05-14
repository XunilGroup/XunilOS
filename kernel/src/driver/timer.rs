use core::{
    ops::{Add, Sub},
    sync::atomic::{AtomicU64, Ordering},
};

pub static TIMER: Timer = Timer::new();

pub struct Timer {
    pub interrupt_count: AtomicU64,
    pub date_at_boot: AtomicU64,
}

impl Timer {
    pub const fn new() -> Self {
        Self {
            interrupt_count: AtomicU64::new(0),
            date_at_boot: AtomicU64::new(0),
        }
    }

    pub fn set_date_at_boot(&self, date_at_boot: u64) {
        self.date_at_boot.store(date_at_boot, Ordering::Relaxed);
    }

    pub fn get_date_at_boot(&self) -> u64 {
        self.date_at_boot.load(Ordering::Relaxed)
    }

    pub fn interrupt(&self) {
        self.interrupt_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn now(&self) -> Time {
        Time {
            interrupt_count: self.interrupt_count.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Time {
    pub interrupt_count: u64,
}

impl Time {
    pub fn new(interrupt_count: u64) -> Self {
        Self { interrupt_count }
    }

    pub fn elapsed(&self) -> u64 {
        self.interrupt_count
    }

    pub fn ticks_since(&self) -> u64 {
        let now = TIMER.interrupt_count.load(Ordering::Relaxed);
        now.saturating_sub(self.interrupt_count)
    }
}

impl Add for Time {
    type Output = Time;
    fn add(self, other: Time) -> Time {
        Time::new(self.interrupt_count + other.interrupt_count)
    }
}

impl Sub for Time {
    type Output = Time;
    fn sub(self, other: Time) -> Time {
        Time::new(self.interrupt_count - other.interrupt_count)
    }
}
