use core::{
    ffi::CStr,
    ptr::{null, null_mut},
};

use crate::{mem::free, printf, puts};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FILE {
    pub data: *const u8,    // pointer to the file's data
    pub size: usize,        // total size
    pub cursor: usize,      // current position
    pub writable: bool,     // is this a write buffer?
    pub write_buf: *mut u8, // for writable fake files
    pub write_cap: usize,
    pub fd: i64,
}

impl FILE {
    pub const fn zeroed() -> FILE {
        FILE {
            data: null(),
            size: 0,
            cursor: 0,
            writable: false,
            write_buf: null_mut(),
            write_cap: 0,
            fd: -1,
        }
    }
}

struct FakeFileEntry {
    name: &'static str,
    data: &'static [u8],
}

#[repr(C, align(8))]
struct AlignedWAD([u8; include_bytes!("../../../assets/doom1.wad").len()]);
static TEST_WAD: AlignedWAD = AlignedWAD(*include_bytes!("../../../assets/doom1.wad"));
static TEST_WAD_BYTES: &[u8] = &TEST_WAD.0;

static FILES: &[FakeFileEntry] = &[
    FakeFileEntry {
        name: "doom1.wad",
        data: TEST_WAD_BYTES,
    },
    FakeFileEntry {
        name: "default.cfg",
        data: b"",
    },
    FakeFileEntry {
        name: "doom.cfg",
        data: b"",
    },
];

static mut FILE_POOL: [FILE; 16] = [FILE::zeroed(); 16];
static mut FILE_POOL_USED: [bool; 16] = [false; 16];
static mut STDERR_FILE: FILE = FILE::zeroed();
static mut STDOUT_FILE: FILE = FILE::zeroed();
static mut STDIN_FILE: FILE = FILE::zeroed();

#[unsafe(no_mangle)]
pub static mut stderr: *mut FILE = unsafe { &raw mut STDERR_FILE };
#[unsafe(no_mangle)]
pub static mut stdin: *mut FILE = unsafe { &raw mut STDIN_FILE };
#[unsafe(no_mangle)]
pub static mut stdout: *mut FILE = unsafe { &raw mut STDOUT_FILE };

pub unsafe fn get_file_pool_slot() -> (*mut FILE, i64) {
    unsafe {
        for i in 0..16 {
            if !FILE_POOL_USED[i] {
                FILE_POOL_USED[i] = true;
                return (&mut FILE_POOL[i], i as i64);
            }
        }

        (null_mut(), -1)
    }
}

#[unsafe(no_mangle)]
extern "C" fn fopen(path: *const i8, mode: *const i8) -> *mut FILE {
    if path.is_null() || mode.is_null() {
        return null_mut();
    }

    unsafe {
        let name = CStr::from_ptr(path).to_str().unwrap_or("");

        for entry in FILES {
            if name.contains(entry.name) {
                let (slot, fd) = get_file_pool_slot();

                if slot.is_null() {
                    return null_mut();
                }

                (*slot).data = entry.data.as_ptr();
                (*slot).size = entry.data.len();
                (*slot).cursor = 0;
                (*slot).writable = false;
                (*slot).write_buf = null_mut();
                (*slot).write_cap = 0;
                (*slot).fd = fd;
                return slot;
            }
        }
    }

    null_mut()
}

#[unsafe(no_mangle)]
extern "C" fn fclose(file_ptr: *mut FILE) -> i32 {
    if file_ptr.is_null() || unsafe { (*file_ptr).fd < 0 || (*file_ptr).fd >= 16 } {
        return -1;
    }

    unsafe { FILE_POOL_USED[(*file_ptr).fd as usize] = false };
    unsafe { *file_ptr = FILE::zeroed() };

    0
}

#[unsafe(no_mangle)]
unsafe extern "C" fn fprintf(file_ptr: *mut FILE, fmt: *const u8, args: ...) -> i32 {
    if fmt.is_null() || file_ptr.is_null() || unsafe { (*file_ptr).fd < 0 || (*file_ptr).fd >= 16 }
    {
        return -1;
    }

    0
}

#[unsafe(no_mangle)]
extern "C" fn fread(ptr: *mut u8, size: usize, nmemb: usize, fp: *mut FILE) -> usize {
    if size == 0
        || nmemb == 0
        || ptr.is_null()
        || fp.is_null()
        || unsafe { (*fp).fd < 0 || (*fp).fd >= 16 }
    {
        return 0;
    }

    let total = match size.checked_mul(nmemb) {
        Some(t) => t,
        None => return 0,
    };

    unsafe {
        let f = &mut *fp;
        if f.cursor > f.size {
            puts(b"failed to read\0".as_ptr());
            return 0;
        }

        let available = f.size - f.cursor;
        let to_read = total.min(available);

        if to_read > 0 {
            core::ptr::copy_nonoverlapping(f.data.add(f.cursor), ptr, to_read);
            f.cursor = f.cursor.saturating_add(to_read);
        }

        to_read / size
    }
}

#[unsafe(no_mangle)]
extern "C" fn fseek(stream: *mut FILE, offset: i64, whence: i32) -> i32 {
    if stream.is_null() || unsafe { (*stream).fd } == -1 {
        return -1;
    }

    let f = unsafe { &mut *stream };

    let new_pos = match whence {
        0 => {
            if offset < 0 {
                return -1;
            }
            offset as usize
        }
        1 => {
            let cur = f.cursor as i64;
            let pos = cur.saturating_add(offset);
            if pos < 0 {
                return -1;
            }
            pos as usize
        }
        2 => {
            let end = f.size as i64;
            let pos = end.saturating_add(offset);
            if pos < 0 {
                return -1;
            }
            pos as usize
        }
        _ => return -1,
    };

    if new_pos > f.size {
        return -1;
    }

    f.cursor = new_pos;
    0
}

#[unsafe(no_mangle)]
extern "C" fn fwrite(ptr: *mut u8, size: usize, count: usize, fp: *mut FILE) -> usize {
    if ptr.is_null() || fp.is_null() || unsafe { (*fp).fd < 0 || (*fp).fd >= 16 } {
        return 0;
    }
    count
}

#[unsafe(no_mangle)]
extern "C" fn ftell(stream: *mut FILE) -> i64 {
    if stream.is_null() || unsafe { (*stream).fd < 0 || (*stream).fd >= 16 } {
        return -1;
    }
    unsafe { (*stream).cursor as i64 }
}

#[unsafe(no_mangle)]
extern "C" fn fflush(file_ptr: *mut FILE) -> i32 {
    0
}

#[unsafe(no_mangle)]
extern "C" fn mkdir(path: *const u8, mode: *const u8) -> i32 {
    0
}

#[unsafe(no_mangle)]
extern "C" fn remove(path: *const i8) -> i32 {
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
