#![no_std]
#![no_main]

use crate::syscall::syscall3;

pub mod syscall;

const SYS_EXIT: usize = 1;
const SYS_WRITE: usize = 60;

#[unsafe(no_mangle)]
extern "C" fn write(fd: i32, buf: *const u8, count: usize) -> isize {
    unsafe { syscall3(SYS_WRITE, fd as usize, buf as usize, count) }
}

#[unsafe(no_mangle)]
extern "C" fn exit(code: i32) -> ! {
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

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
