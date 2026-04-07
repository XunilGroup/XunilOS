use core::{ffi::c_int, mem, ptr::null_mut, usize};

use crate::{
    heap::{ALLOCATOR, LinkedNode},
    syscall::{BRK, syscall1},
    util::align_up,
};

pub fn sbrk(increment: i64) -> isize {
    unsafe { syscall1(BRK, increment as isize) as isize }
}

const MAX_SIZE: u64 = 18446744073709551615;

const HEADER_MAGIC_ALLOC: u64 = 0xBADC0FFEE0DDF00D;
const HEADER_MAGIC_FREE: u64 = 0xFEE1DEADCAFEBABE;

#[repr(C, align(16))]
struct Header {
    magic: u64,
    _pad: u64,
    alloc_size: usize,
    region_size: usize,
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

    unsafe { memset(ptr, 0, total as usize) };
    ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn free(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }

    unsafe {
        let header_ptr = (ptr as usize - mem::size_of::<Header>()) as *mut Header;
        if (header_ptr as usize) & 0xF != 0 {
            core::hint::unreachable_unchecked();
        }

        if (*header_ptr).magic != HEADER_MAGIC_ALLOC {
            return;
        }

        let region_size = (*header_ptr).region_size;
        (*header_ptr).magic = HEADER_MAGIC_FREE;
        let mut allocator = ALLOCATOR.lock();
        allocator.add_free_memory_region(header_ptr as usize, region_size);
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
                magic: HEADER_MAGIC_ALLOC,
                _pad: 0,
                alloc_size: (region.end - region.start).saturating_sub(hdr),
                region_size: region.end - region.start,
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

        if (*header_ptr).magic != HEADER_MAGIC_ALLOC {
            return null_mut();
        }

        let old_alloc_size = (*header_ptr).alloc_size;

        if size <= old_alloc_size {
            return ptr;
        }

        let new_ptr = malloc(size as u64);
        if new_ptr.is_null() {
            return null_mut();
        }

        core::ptr::copy_nonoverlapping(ptr, new_ptr, old_alloc_size);
        free(ptr);
        new_ptr
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memset(dest: *mut u8, c: c_int, n: usize) -> *mut u8 {
    if n == 0 {
        return dest;
    }
    if dest.is_null() {
        return null_mut();
    }

    let byte = c as u8;
    let mut i = 0usize;
    while i < n {
        unsafe { core::ptr::write_volatile(dest.add(i), byte) };
        i += 1;
    }

    dest
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if n == 0 {
        return dest;
    }
    if dest.is_null() || src.is_null() {
        return null_mut();
    }

    let mut i = 0usize;
    while i < n {
        let v = unsafe { core::ptr::read_volatile(src.add(i)) };
        unsafe { core::ptr::write_volatile(dest.add(i), v) };
        i += 1;
    }

    dest
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if n == 0 {
        return dest;
    }
    if dest.is_null() || src.is_null() {
        return null_mut();
    }

    let dest_addr = dest as usize;
    let src_addr = src as usize;

    if dest_addr == src_addr {
        return dest;
    }

    if dest_addr < src_addr || dest_addr >= src_addr.saturating_add(n) {
        let mut i = 0usize;
        while i < n {
            let v = unsafe { core::ptr::read_volatile(src.add(i)) };
            unsafe { core::ptr::write_volatile(dest.add(i), v) };
            i += 1;
        }
    } else {
        let mut i = n;
        while i != 0 {
            i -= 1;
            let v = unsafe { core::ptr::read_volatile(src.add(i)) };
            unsafe { core::ptr::write_volatile(dest.add(i), v) };
        }
    }

    dest
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    if n == 0 {
        return 0;
    }
    if a.is_null() || b.is_null() {
        return 0;
    }

    let mut i = 0usize;
    while i < n {
        let av = unsafe { core::ptr::read_volatile(a.add(i)) };
        let bv = unsafe { core::ptr::read_volatile(b.add(i)) };
        if av != bv {
            return av as i32 - bv as i32;
        }
        i += 1;
    }
    0
}
