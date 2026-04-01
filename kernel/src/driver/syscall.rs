use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::null_mut,
};

use crate::{arch::arch::get_allocator, driver::graphics::framebuffer::with_framebuffer, println};

const SYS_EXIT: usize = 1;
const SYS_WRITE: usize = 60;

pub unsafe fn malloc(size: usize, align: usize) -> *mut u8 {
    let align = if align < 1 {
        1
    } else {
        align.next_power_of_two()
    };
    let layout = match Layout::from_size_align(size, align) {
        Ok(l) => l,
        Err(_) => return null_mut(),
    };

    unsafe { GlobalAlloc::alloc(get_allocator(), layout) }
}

pub unsafe fn free(ptr: *mut u8, size: usize, align: usize) {
    if ptr.is_null() {
        // very important, do not double free
        return;
    }

    let align = if align < 1 {
        1
    } else {
        align.next_power_of_two()
    };

    if let Ok(layout) = Layout::from_size_align(size.max(1), align.max(1)) {
        unsafe { GlobalAlloc::dealloc(get_allocator(), ptr, layout) }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn syscall_dispatch(
    num: usize,
    arg0: usize,
    arg1: usize,
    arg2: usize,
) -> isize {
    match num {
        SYS_WRITE => {
            let buf_ptr = arg1 as *const u8;
            let len = arg2 as usize;
            let bytes: &[u8] = unsafe { core::slice::from_raw_parts(buf_ptr, len) };
            let s = core::str::from_utf8(bytes).unwrap_or("<non-utf8>");
            println!("SYS_WRITE called: {:?} {:?}", s, len);
            with_framebuffer(|fb| fb.swap());
            0
        }
        SYS_EXIT => {
            println!("Program exit.");
            with_framebuffer(|fb| fb.swap());
            0
        }
        _ => -38, // syscall not found
    }
}

pub unsafe fn memset(ptr: *mut u8, val: u8, count: usize) {
    unsafe { core::ptr::write_bytes(ptr, val, count) };
}

pub type Fd = i32;
pub type Off = i64;
