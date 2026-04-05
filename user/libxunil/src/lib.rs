#![no_std]
#![feature(c_variadic)]

use core::ptr::{null, null_mut};

use crate::{
    mem::{malloc, memcpy, memset},
    syscall::{EXIT, WRITE, syscall3},
};

pub mod file;
pub mod heap;
pub mod mem;
pub mod syscall;
pub mod util;

#[unsafe(no_mangle)]
extern "C" fn write(fd: i64, buf: *const u8, count: usize) -> isize {
    unsafe { syscall3(WRITE, fd as usize, buf as usize, count) }
}

#[unsafe(no_mangle)]
extern "C" fn exit(code: i64) -> ! {
    unsafe { syscall3(EXIT, code as usize, 0, 0) };
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
    unsafe { syscall3(WRITE, 1, format as usize, strlen(format)) };
}

#[unsafe(no_mangle)]
extern "C" fn atoi(mut c: *const u8) -> i64 {
    let mut value: i64 = 0;
    let mut sign: i64 = 1;
    unsafe {
        while (*c).is_ascii_whitespace() {
            c = c.add(1);
        }

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

#[inline]
fn pow10_i32(exp: i32) -> f64 {
    let mut e = exp;
    let mut scale: f64 = 1.0;

    if e > 0 {
        while e > 0 {
            scale *= 10.0;
            e -= 1;
        }
    } else if e < 0 {
        while e < 0 {
            scale *= 0.1;
            e += 1;
        }
    }

    scale
}

#[unsafe(no_mangle)]
extern "C" fn atof(mut c: *const u8) -> f64 {
    let mut sign: f64 = 1.0;
    unsafe {
        while (*c).is_ascii_whitespace() {
            c = c.add(1);
        }

        if (*c) == b'+' || (*c) == b'-' {
            if *c == b'-' {
                sign = -1.0;
            }
            c = c.add(1);
        }

        let mut int_part: i64 = 0;
        while (*c).is_ascii_digit() {
            int_part = int_part * 10 + ((*c) - b'0') as i64;
            c = c.add(1);
        }

        let mut result: f64 = int_part as f64;

        if *c == b'.' {
            c = c.add(1);
            let mut factor = 0.1;
            while (*c).is_ascii_digit() {
                result += ((*c) - b'0') as f64 * factor;
                factor *= 0.1;
                c = c.add(1);
            }
        }

        if *c == b'e' || *c == b'E' {
            c = c.add(1);

            let mut exp_sign = 1;
            let mut exp_value = 0;

            if (*c) == b'+' || (*c) == b'-' {
                if *c == b'-' {
                    exp_sign = -1;
                }
                c = c.add(1);
            }

            while (*c).is_ascii_digit() {
                exp_value *= 10;
                exp_value += ((*c) - b'0') as i64;
                c = c.add(1);
            }

            result *= pow10_i32((exp_sign * exp_value) as i32);
        }

        sign * result
    }
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
unsafe extern "C" fn strncpy(dest: *mut u8, source: *const u8, n: usize) -> *mut u8 {
    let mut i = 0usize;
    unsafe {
        while i < n {
            let b = *source.add(i);
            *dest.add(i) = b;
            i += 1;

            if b == 0 {
                while i < n {
                    *dest.add(i) = 0;
                    i += 1;
                }
                break;
            }
        }
    }

    dest
}

#[unsafe(no_mangle)]
unsafe extern "C" fn strdup(s: *const u8) -> *mut u8 {
    let len = strlen(s);
    let memory = malloc((len + 1) as u64);
    if memory.is_null() {
        return null_mut();
    }
    memcpy(memory, s, len + 1);
    memory
}

#[unsafe(no_mangle)]
unsafe extern "C" fn strstr(haystack: *const u8, needle: *const u8) -> *const u8 {
    if haystack.is_null() || needle.is_null() {
        return null();
    }

    let mut h = haystack;

    unsafe {
        if *needle == 0 {
            return haystack;
        }

        while *h != 0 {
            if *h == *needle {
                let mut h2 = h;
                let mut n2 = needle;

                while *n2 != 0 && *h2 != 0 && *h2 == *n2 {
                    h2 = h2.add(1);
                    n2 = n2.add(1);
                }

                if *n2 == 0 {
                    return h;
                }
            }

            h = h.add(1);
        }
    }

    null()
}

#[unsafe(no_mangle)]
unsafe extern "C" fn strchr(s: *const u8, ch: u8) -> *const u8 {
    if s.is_null() {
        return null();
    }

    let mut i = 0usize;

    unsafe {
        while *s.add(i) != 0 {
            if *s.add(i) == ch {
                return s.add(i);
            }

            i += 1;
        }
    }

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
        return unsafe { s.add(n + 1) };
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
