#![no_std]
#![feature(c_variadic)]
use core::{
    ffi::VaList,
    fmt::{Error, Result, Write},
    ptr::{addr_of_mut, null, null_mut},
    usize,
};

use crate::{
    mem::{malloc, memcpy},
    syscall::{DRAW_BUFFER, DRAW_PIXEL, EXIT, FRAMEBUFFER_SWAP, WRITE, syscall0, syscall3},
};

pub mod file;
pub mod heap;
pub mod mem;
pub mod syscall;
pub mod time;
pub mod util;

static mut ERRNO: core::ffi::c_int = 0;

static TOUPPER_TABLE: [i32; 384] = {
    let mut table = [0i32; 384];
    let mut i = 0usize;
    while i < 384 {
        let c = i.wrapping_sub(128) as u8;
        table[i] = if c.is_ascii_lowercase() {
            (c - 0x20) as i32
        } else {
            c as i32
        };
        i += 1;
    }
    table
};

#[unsafe(no_mangle)]
extern "C" fn write(fd: i32, buf: *const u8, count: usize) -> isize {
    unsafe { syscall3(WRITE, fd as isize, buf as isize, count as isize) }
}

#[unsafe(no_mangle)]
extern "C" fn exit(code: i32) -> ! {
    unsafe { syscall3(EXIT, code as isize, 0, 0) };
    loop {
        unsafe { core::arch::asm!("nop") };
    }
}

#[unsafe(no_mangle)]
extern "C" fn strlen(s: *const u8) -> usize {
    let mut len = 0usize;
    while unsafe { *s.add(len) } != 0 {
        len += 1;
    }
    len
}

#[unsafe(no_mangle)]
extern "C" fn puts(s: *const u8) -> i32 {
    write(1, s, strlen(s));
    write(1, b"\n\0".as_ptr(), 1);

    0
}

#[unsafe(no_mangle)]
extern "C" fn putchar(c: i32) -> i32 {
    let b = c as u8;
    write(1, core::ptr::addr_of!(b), 1);

    0
}

#[unsafe(no_mangle)]
extern "C" fn abs(n: i32) -> i32 {
    n.abs()
}

struct BufWriter {
    buf: *mut u8,
    max: usize,
    pos: usize,
}

impl Write for BufWriter {
    fn write_str(&mut self, s: &str) -> Result {
        for byte in s.bytes() {
            if self.pos >= self.max {
                return Err(Error);
            }
            unsafe {
                *self.buf.add(self.pos) = byte;
            }
            self.pos += 1;
        }
        Ok(())
    }
}

struct StdoutWriter {
    size: usize,
}

impl Write for StdoutWriter {
    fn write_str(&mut self, s: &str) -> Result {
        self.size += s.len();
        write(1, s.as_ptr(), s.len());
        Ok(())
    }
}

pub unsafe fn write_c_formatted(fmt: *const u8, args: &mut VaList, writer: &mut impl Write) {
    let mut fi = 0usize;

    loop {
        let ch = unsafe { *fmt.add(fi) };
        fi += 1;
        if ch == 0 {
            break;
        }

        if ch != b'%' {
            let _ = writer.write_char(ch as char);
            continue;
        }

        let mut precision: Option<usize> = None;
        let mut next_byte = unsafe { *fmt.add(fi) };

        if next_byte == b'.' {
            fi += 1;
            let mut p_val = 0usize;
            loop {
                let digit = unsafe { *fmt.add(fi) };
                if digit >= b'0' && digit <= b'9' {
                    p_val = p_val * 10 + (digit - b'0') as usize;
                    fi += 1;
                } else {
                    break;
                }
            }
            precision = Some(p_val);
            next_byte = unsafe { *fmt.add(fi) };
        }

        let spec = next_byte;
        fi += 1;

        unsafe {
            match spec {
                b'd' | b'i' => {
                    let v: i32 = args.arg();
                    if let Some(p) = precision {
                        let _ = write!(writer, "{:01$}", v, p);
                    } else {
                        let _ = write!(writer, "{}", v);
                    }
                }
                b'u' => {
                    let v: u32 = args.arg();
                    if let Some(p) = precision {
                        let _ = write!(writer, "{:01$}", v, p);
                    } else {
                        let _ = write!(writer, "{}", v);
                    }
                }
                b'x' => {
                    let v: u32 = args.arg();
                    if let Some(p) = precision {
                        let _ = write!(writer, "{:01$x}", v, p);
                    } else {
                        let _ = write!(writer, "{:x}", v);
                    }
                }
                b'X' => {
                    let v: u32 = args.arg();
                    if let Some(p) = precision {
                        let _ = write!(writer, "{:01$X}", v, p);
                    } else {
                        let _ = write!(writer, "{:X}", v);
                    }
                }
                b'o' => {
                    let v: u32 = args.arg();
                    if let Some(p) = precision {
                        let _ = write!(writer, "{:01$o}", v, p);
                    } else {
                        let _ = write!(writer, "{:o}", v);
                    }
                }
                b'p' => {
                    let v: *const u8 = args.arg();
                    if v.is_null() {
                        let _ = writer.write_str("(null)");
                    } else {
                        let _ = write!(writer, "0x{:x}", v as usize);
                    }
                }
                b'c' => {
                    let v: i32 = args.arg();
                    let _ = writer.write_char((v as u8) as char);
                }
                b's' => {
                    let ptr: *const u8 = args.arg();
                    if ptr.is_null() {
                        let _ = writer.write_str("(null)");
                    } else {
                        let mut si = 0usize;
                        loop {
                            let c = *ptr.add(si);
                            if c == 0 {
                                break;
                            }
                            if let Some(p) = precision {
                                if si >= p {
                                    break;
                                }
                            }
                            let _ = writer.write_char(c as char);
                            si += 1;
                        }
                    }
                }
                b'f' | b'F' | b'g' | b'G' => {
                    let v: f64 = args.arg();
                    if let Some(p) = precision {
                        let _ = write!(writer, "{:.*}", p, v);
                    } else {
                        let _ = write!(writer, "{}", v);
                    }
                }
                b'l' => {
                    let next_spec = *fmt.add(fi);
                    fi += 1;
                    match next_spec {
                        b'd' | b'i' => {
                            let v: i64 = args.arg();
                            if let Some(p) = precision {
                                let _ = write!(writer, "{:01$}", v, p);
                            } else {
                                let _ = write!(writer, "{}", v);
                            }
                        }
                        b'u' => {
                            let v: u64 = args.arg();
                            if let Some(p) = precision {
                                let _ = write!(writer, "{:01$}", v, p);
                            } else {
                                let _ = write!(writer, "{}", v);
                            }
                        }
                        _ => {
                            let _ = writer.write_char('%');
                            let _ = writer.write_char('l');
                            let _ = writer.write_char(next_spec as char);
                        }
                    }
                }
                b'%' => {
                    let _ = writer.write_char('%');
                }
                _ => {
                    let _ = writer.write_char('%');
                    let _ = writer.write_char(spec as char);
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printf(fmt: *const u8, mut args: ...) -> i32 {
    let mut writer = StdoutWriter { size: 0 };

    unsafe { write_c_formatted(fmt, &mut args, &mut writer) };

    writer.size as i32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vsnprintf(
    buf: *mut u8,
    size: usize,
    fmt: *const u8,
    mut args: VaList,
) -> i32 {
    if buf.is_null() || size == 0 || fmt.is_null() {
        return -1;
    }

    let max = size - 1;
    let mut writer = BufWriter { buf, max, pos: 0 };

    unsafe { write_c_formatted(fmt, &mut args, &mut writer) };

    unsafe {
        *buf.add(writer.pos) = 0;
    }

    writer.pos as i32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn snprintf(buf: *mut u8, size: usize, fmt: *const u8, mut args: ...) -> i32 {
    if buf.is_null() || size == 0 || fmt.is_null() {
        return -1;
    }

    let max = size - 1;
    let mut writer = BufWriter { buf, max, pos: 0 };

    unsafe { write_c_formatted(fmt, &mut args, &mut writer) };

    unsafe {
        *buf.add(writer.pos) = 0;
    }

    writer.pos as i32
}

unsafe extern "C" {
    fn main(argc: i32, argv: *const *const u8) -> i32;
}

#[unsafe(no_mangle)]
pub extern "C" fn __stack_chk_fail() -> ! {
    exit(127)
}

#[unsafe(no_mangle)]
pub extern "C" fn __stack_chk_fail_local() -> ! {
    __stack_chk_fail()
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let code = unsafe { main(0, null()) };

    exit(code as i32);
}
#[unsafe(no_mangle)]
extern "C" fn atoi(mut c: *const u8) -> i32 {
    let mut value: i32 = 0;
    let mut sign: i32 = 1;
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
            value += ((*c) - b'0') as i32;
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

pub fn compare_str(str_1: *const u8, str_2: *const u8, ignore_case: bool, n: usize) -> i32 {
    let mut len = 0;
    while len < n {
        let mut c_1 = unsafe { *str_1.add(len) };
        let mut c_2 = unsafe { *str_2.add(len) };
        if ignore_case {
            c_1 = c_1.to_ascii_lowercase();
            c_2 = c_2.to_ascii_lowercase();
        }
        if c_1 != c_2 {
            return (c_1 as i32) - (c_2 as i32);
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
    compare_str(str_1, str_2, true, usize::MAX)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn strcmp(str_1: *const u8, str_2: *const u8) -> i32 {
    compare_str(str_1, str_2, false, usize::MAX)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn strncasecmp(str_1: *const u8, str_2: *const u8, n: usize) -> i32 {
    compare_str(str_1, str_2, true, n)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn strncmp(str_1: *const u8, str_2: *const u8, n: usize) -> i32 {
    compare_str(str_1, str_2, false, n)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn draw_pixel(x: u32, y: u32, color: u32) -> i32 {
    unsafe {
        return syscall3(DRAW_PIXEL, x as isize, y as isize, color as isize) as i32;
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn draw_buffer(buffer: *const u32, width: u32, height: u32) -> i32 {
    unsafe {
        return syscall3(
            DRAW_BUFFER,
            buffer as isize,
            width as isize,
            height as isize,
        ) as i32;
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn framebuffer_swap() -> i32 {
    unsafe {
        return syscall0(FRAMEBUFFER_SWAP) as i32;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strncpy(dest: *mut u8, source: *const u8, n: usize) -> *mut u8 {
    let mut i = 0usize;
    while i < n {
        let b = unsafe { *source.add(i) };
        unsafe { *dest.add(i) = b };

        if b == 0 {
            let mut j = i + 1;
            while j < n {
                unsafe { *dest.add(j) = 0 };
                j += 1;
            }
            break;
        }
        i += 1;
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
    unsafe { memcpy(memory, s, len + 1) };
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
unsafe extern "C" fn strchr(s: *const u8, ch: i32) -> *const u8 {
    if s.is_null() {
        return null();
    }

    let mut i = 0usize;

    unsafe {
        while *s.add(i) != 0 {
            if *s.add(i) == ch as u8 {
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
unsafe extern "C" fn toupper(char: i32) -> i32 {
    (char as u8).to_ascii_uppercase() as i32
}

#[unsafe(no_mangle)]
unsafe extern "C" fn tolower(char: i32) -> i32 {
    (char as u8).to_ascii_lowercase() as i32
}

#[unsafe(no_mangle)]
extern "C" fn system(cmd: *const u8) -> i32 {
    0
}
#[unsafe(no_mangle)]
extern "C" fn __ctype_toupper_loc() -> *const u8 {
    TOUPPER_TABLE.as_ptr() as *const u8
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __isoc23_sscanf() -> ! {
    panic!("sscanf not implemented");
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sscanf(str: *mut u8, fmt: *const u8, args: ...) -> i32 {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __errno_location() -> *mut core::ffi::c_int {
    addr_of_mut!(ERRNO)
}

#[panic_handler]
fn panic(error: &core::panic::PanicInfo) -> ! {
    exit(-1)
}
