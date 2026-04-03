#![no_std]
#![feature(c_variadic)]

use core::ptr::null;

use crate::syscall::syscall3;

pub mod file;
pub mod mem;
pub mod syscall;

const SYS_EXIT: usize = 1;
const SYS_WRITE: usize = 60;

#[unsafe(no_mangle)]
extern "C" fn write(fd: i64, buf: *const u8, count: usize) -> isize {
    unsafe { syscall3(SYS_WRITE, fd as usize, buf as usize, count) }
}

#[unsafe(no_mangle)]
extern "C" fn exit(code: i64) -> ! {
    unsafe { syscall3(SYS_EXIT, code as usize, 0, 0) };
    loop {}
}

#[unsafe(no_mangle)]
extern "C" fn strlen(s: *const u8) -> usize {
    let mut len = 0;
    while unsafe { *s.add(len) } != 0 {
        len += 1;
    }
    len
}

#[unsafe(no_mangle)]
extern "C" fn puts(s: *const u8) -> isize {
    write(1, s, strlen(s));

    0
}

#[unsafe(no_mangle)]
extern "C" fn putchar(s: i32) -> isize {
    write(1, (s as u8 + b'0') as *const u8, 1);

    0
}

#[unsafe(no_mangle)]
extern "C" fn abs(n: i64) -> i64 {
    n.abs()
}

#[unsafe(no_mangle)]
unsafe extern "C" fn printf(format: *const u8, args: ...) {
    unsafe { syscall3(SYS_WRITE, 1, format as usize, strlen(format)) };
}

#[unsafe(no_mangle)]
extern "C" fn atoi(mut c: *const u8) -> i64 {
    let mut value: i64 = 0;
    let mut sign: i64 = 1;
    unsafe {
        if (*c) == b'+' || (*c) == b'-' {
            if *c == b'-' {
                sign = -1;
            }
            c = c.add(1);
        }
        while (*c).is_ascii_digit() {
            value *= 10;
            value += ((*c) - b'0') as i64;
            c = c.add(1);
        }
    }

    value * sign
}

#[unsafe(no_mangle)]
extern "C" fn atof(mut c: *const u8) -> f64 {
    0.0
}

pub fn compare_str(str_1: *const u8, str_2: *const u8, case: bool, n: usize) -> i32 {
    let mut len = 0;

    while len < n {
        let mut c_1 = unsafe { *str_1.add(len) };
        let mut c_2 = unsafe { *str_2.add(len) };

        if case {
            c_1 = c_1.to_ascii_lowercase();
            c_2 = c_2.to_ascii_lowercase();
        }

        if c_1 != c_2 {
            return (c_1 - c_2) as i32;
        }

        if c_1 == 0 {
            return 0;
        }

        len += 1;
    }

    0
}

#[unsafe(no_mangle)]
unsafe extern "C" fn strcasecmp(str_1: *const u8, str_2: *const u8) -> i32 {
    compare_str(str_1, str_2, true, 999999999999999)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn strcmp(str_1: *const u8, str_2: *const u8) -> i32 {
    compare_str(str_1, str_2, false, 999999999999999)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn strncasecmp(str_1: *const u8, str_2: *const u8, n: i32) -> i32 {
    compare_str(str_1, str_2, true, n as usize)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn strncmp(str_1: *const u8, str_2: *const u8, n: i32) -> i32 {
    compare_str(str_1, str_2, false, n as usize)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn strncopy(s: *const u8, n: i32) -> *const u8 {
    null()
}

#[unsafe(no_mangle)]
unsafe extern "C" fn strdup(s: *const u8) -> *const u8 {
    null()
}

#[unsafe(no_mangle)]
unsafe extern "C" fn strstr(s: *const u8) -> *const u8 {
    null()
}

#[unsafe(no_mangle)]
unsafe extern "C" fn strchr(s: *const u8, ch: u8) -> *const u8 {
    null()
}

#[unsafe(no_mangle)]
unsafe extern "C" fn strrchr(s: *const u8, ch: u8) -> *const u8 {
    let mut n = 0;
    let mut last: *const u8 = null();

    if ch == 0 {
        while unsafe { *s.add(n) } != 0 {
            n += 1;
        }
        unsafe { s.add(n + 1) }
    } else {
        while unsafe { *s.add(n) } != 0 {
            let cur_ch = unsafe { s.add(n) };
            if unsafe { *cur_ch == ch } {
                last = cur_ch;
            }
            n += 1;
        }

        last
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn toupper(char: u8) -> u8 {
    char.to_ascii_uppercase()
}

#[unsafe(no_mangle)]
unsafe extern "C" fn tolower(char: u8) -> u8 {
    char.to_ascii_lowercase()
}

#[unsafe(no_mangle)]
extern "C" fn system(cmd: *const u8) -> i32 {
    0
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
