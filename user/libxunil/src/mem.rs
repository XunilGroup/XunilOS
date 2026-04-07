use core::{mem, ptr::null_mut, usize};

use crate::{
    heap::{ALLOCATOR, LinkedNode},
    syscall::{BRK, syscall1},
    util::align_up,
};

pub fn sbrk(increment: i64) -> isize {
    unsafe { syscall1(BRK, increment as isize) as isize }
}

const MAX_SIZE: u64 = 18446744073709551615;

#[repr(C, align(16))]
struct Header {
    size: usize,
    _pad: usize,
}

#[unsafe(no_mangle)]
pub extern "C" fn calloc(count: u64, size: u64) -> *mut u8 {
    if count != 0 && size > MAX_SIZE / count {
        return null_mut();
    }

    let mut total = count * size;
    if total == 0 {
        total = 1;
    }

    let ptr = malloc(total);
    if ptr.is_null() {
        return null_mut();
    }

    memset(ptr, 0, total as usize);
    ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn free(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }

    unsafe {
        let header_ptr = (ptr as usize - mem::size_of::<Header>()) as *mut Header;
        let total = (*header_ptr).size;
        let mut allocator = ALLOCATOR.lock();
        allocator.add_free_memory_region(header_ptr as usize, total);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn malloc(size: u64) -> *mut u8 {
    let req = size as usize;
    let req = if req == 0 { 1 } else { req };

    let hdr = mem::size_of::<Header>();
    let needed_unaligned = match hdr.checked_add(req) {
        Some(v) => v,
        None => return null_mut(),
    };
    let align_req = 16;
    let needed = align_up(needed_unaligned, align_req);

    let mut allocator = ALLOCATOR.lock();

    if let Some(region) = allocator.find_region(needed) {
        unsafe {
            let header_ptr = region.start as *mut Header;
            header_ptr.write(Header {
                size: region.end - region.start,
                _pad: 0,
            });
            return (region.start + hdr) as *mut u8;
        }
    }

    let min_region = mem::size_of::<LinkedNode>();
    let req_region = core::cmp::max(needed, min_region);

    let align = align_req;
    let over = match req_region.checked_add(align) {
        Some(v) => v,
        None => return null_mut(),
    };
    if over > i64::MAX as usize {
        return null_mut();
    }

    let raw_start = sbrk(over as i64);
    if raw_start == -1 {
        return null_mut();
    }

    let raw_start = raw_start as usize;
    let aligned_start = align_up(raw_start, align);
    let usable = over - (aligned_start - raw_start);

    if usable < min_region {
        return null_mut();
    }

    unsafe {
        allocator.add_free_memory_region(aligned_start, usable);
    }

    drop(allocator);
    malloc(size)
}

#[unsafe(no_mangle)]
pub extern "C" fn memcpy(dest_str: *mut u8, src_str: *const u8, n: usize) -> *mut u8 {
    unsafe { core::ptr::copy(src_str, dest_str, n) };
    dest_str
}

#[unsafe(no_mangle)]
pub extern "C" fn memset(dst: *mut u8, c: i64, n: usize) -> *mut u8 {
    if dst.is_null() || n == 0 {
        return dst;
    }

    unsafe {
        let val = c as u8;
        let mut i = 0usize;
        while i < n {
            *dst.add(i) = val;
            i += 1;
        }
    }

    dst
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn realloc(ptr: *mut u8, size: usize) -> *mut u8 {
    if size == 0 {
        free(ptr);
        return null_mut();
    }

    if ptr.is_null() {
        return malloc(size as u64);
    }

    unsafe {
        let hdr = mem::size_of::<Header>();
        let header_ptr = (ptr as usize - hdr) as *mut Header;
        let total = (*header_ptr).size;
        let old_payload = total.saturating_sub(hdr);

        if size <= old_payload {
            return ptr;
        }

        let new_ptr = malloc(size as u64);
        if new_ptr.is_null() {
            return null_mut();
        }

        core::ptr::copy_nonoverlapping(ptr, new_ptr, old_payload);
        free(ptr);
        new_ptr
    }
}
