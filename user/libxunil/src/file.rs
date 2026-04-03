use core::ptr::null_mut;

#[unsafe(no_mangle)]
extern "C" fn fopen(path: *const u8, mode: *const u8) -> *mut u8 {
    null_mut()
}

#[unsafe(no_mangle)]
extern "C" fn fclose(file_ptr: *mut u8) -> i32 {
    0
}

#[unsafe(no_mangle)]
extern "C" fn fprintf(file_ptr: *mut u8, s: *const u8) -> i32 {
    0
}

#[unsafe(no_mangle)]
extern "C" fn fread(ptr: *mut u8, size: i32, nmemb: *const u8, fp: *const u8) -> i32 {
    0
}

#[unsafe(no_mangle)]
extern "C" fn fwrite(ptr: *mut u8, size: i32, nmemb: *const u8, fp: *const u8) -> i32 {
    0
}

#[unsafe(no_mangle)]
extern "C" fn fseek(s: *const u8, size: i32, fp: *const u8) -> i32 {
    0
}

#[unsafe(no_mangle)]
extern "C" fn ftell(fp: *const u8) -> i64 {
    0
}

#[unsafe(no_mangle)]
extern "C" fn fflush(file_ptr: *mut u8) -> i32 {
    0
}

#[unsafe(no_mangle)]
extern "C" fn mkdir(path: *const u8, mode: *const u8) -> i32 {
    0
}

#[unsafe(no_mangle)]
extern "C" fn remove(path: *const u8) -> i32 {
    0
}

#[unsafe(no_mangle)]
extern "C" fn rename(path: *const u8, new_path: *const u8) -> i32 {
    0
}

#[unsafe(no_mangle)]
unsafe extern "C" fn vfprintf(stream: *const u8, format: *const u8, args: ...) -> i32 {
    0
}

#[unsafe(no_mangle)]
unsafe extern "C" fn vsnprintf(s: *mut u8, n: i32, format: *const u8, args: ...) -> i32 {
    0
}
