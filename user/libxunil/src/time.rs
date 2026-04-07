use crate::syscall::{CLOCK_GETTIME, SLEEP, syscall0, syscall1};

#[repr(C)]
pub struct Timeval {
    pub tv_sec: i64,
    pub tv_usec: i64,
}

#[repr(C)]
pub struct Timezone {
    pub tz_minuteswest: i32,
    pub tz_dsttime: i32,
}

pub const TICKS_PER_SECOND: u64 = 1000;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sleep_ms(ms: i32) -> i32 {
    unsafe { syscall1(SLEEP, ms as isize) };
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gettimeofday(tv: *mut Timeval, _tz: *mut Timezone) -> i32 {
    unsafe {
        if !tv.is_null() {
            let ticks = syscall0(CLOCK_GETTIME) as u64;

            let seconds = ticks / TICKS_PER_SECOND;
            let microseconds = (ticks % TICKS_PER_SECOND) * (1_000_000 / TICKS_PER_SECOND);

            (*tv).tv_sec = seconds as i64;
            (*tv).tv_usec = microseconds as i64;
        }
    }

    0
}
