use core::ptr::null_mut;

#[unsafe(no_mangle)]
extern "C" fn calloc(nitems: u64, size: u64) -> *mut u8 {
    null_mut()
}

#[unsafe(no_mangle)]
extern "C" fn free(ptr: *mut u8) {}

#[unsafe(no_mangle)]
extern "C" fn malloc(size: u64) -> *mut u8 {
    null_mut()
}

#[unsafe(no_mangle)]
extern "C" fn memcpy(dest_str: *mut u8, src_str: *const u8, n: u64) {}

#[unsafe(no_mangle)]
extern "C" fn memset(str: *mut u8, c: i64, n: u64) -> *mut u8 {
    null_mut()
}

#[unsafe(no_mangle)]
unsafe extern "C" fn realloc(ptr: *mut u8, size: u64) -> *mut u8 {
    null_mut()
}
