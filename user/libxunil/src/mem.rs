use core::ptr::null_mut;

use crate::{
    heap::ALLOCATOR,
    syscall::{BRK, syscall1},
};

#[unsafe(no_mangle)]
pub extern "C" fn sbrk(increment: i64) -> i64 {
    unsafe { syscall1(BRK, increment as usize) as i64 }
}

const MAX_SIZE: u64 = 18446744073709551615;

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
        let size = *(((ptr as usize) - core::mem::size_of::<usize>()) as *const usize);

        let mut allocator = ALLOCATOR.lock();
        allocator
            .add_free_memory_region(ptr as usize - core::mem::size_of::<usize>(), size as usize);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn malloc(size: u64) -> *mut u8 {
    let mut allocator = ALLOCATOR.lock();

    if let Some(region) = allocator.find_region(size as usize, 16) {
        return region.1 as *mut u8;
    } else {
        let start_addr: i64 = sbrk(size as i64);
        if start_addr == -1 {
            return null_mut();
        }

        unsafe { allocator.add_free_memory_region(start_addr as usize, size as usize) };
        drop(allocator);
        malloc(size)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn memcpy(dest_str: *mut u8, src_str: *const u8, n: usize) -> *mut u8 {
    unsafe { core::ptr::copy(src_str, dest_str, n) };

    dest_str
}

#[unsafe(no_mangle)]
pub extern "C" fn memset(str: *mut u8, c: i64, n: usize) -> *mut u8 {
    unsafe {
        core::ptr::write_bytes(str, c as u8, n);
    }

    str
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn realloc(ptr: *mut u8, size: usize) -> *mut u8 {
    unsafe {
        if size == 0 {
            free(ptr);
            return null_mut();
        }

        let header = ((ptr as usize) - core::mem::size_of::<usize>()) as *mut usize;
        let old_size = *header;

        if old_size == size {
            return ptr;
        } else if size < old_size {
            let mut allocator = ALLOCATOR.lock();
            let difference = old_size.abs_diff(size);
            let start = (ptr as usize) + size;
            *header = size;
            allocator.add_free_memory_region(start as usize, difference as usize);
            return ptr;
        } else {
            let new_ptr = malloc(size as u64);
            if new_ptr.is_null() {
                return null_mut();
            }

            core::ptr::copy_nonoverlapping(ptr, new_ptr, old_size);
            free(ptr);
            return new_ptr;
        }
    }
}
