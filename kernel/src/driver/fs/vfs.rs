use core::ptr::{null, null_mut};

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

pub type Fd = i64;
const MAX_FD: usize = 16;

fn fd_ok(fd: Fd) -> bool {
    fd >= 0 && (fd as usize) < MAX_FD
}

static DOOM_WAD: &[u8] = include_bytes!("../../../../assets/doom1.wad");
static DOOM_ELF: &[u8] = include_bytes!("../../../../assets/doomgeneric");
static HELLOWORLD_ELF: &[u8] = include_bytes!("../../../../assets/helloworld.elf");

static FILES: &[FakeFileEntry] = &[
    FakeFileEntry {
        name: "testfile",
        data: b"Hello, World!",
    },
    FakeFileEntry {
        name: "helloworld.elf",
        data: HELLOWORLD_ELF,
    },
    FakeFileEntry {
        name: "doomgeneric",
        data: DOOM_ELF,
    },
    FakeFileEntry {
        name: "doom1.wad",
        data: DOOM_WAD,
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

unsafe fn file_mut(fd: Fd) -> Option<&'static mut FILE> {
    if !fd_ok(fd) {
        return None;
    }
    let idx = fd as usize;

    if unsafe { !FILE_POOL_USED[idx] } {
        return None;
    }

    return unsafe { Some(&mut FILE_POOL[idx]) };
}

#[unsafe(no_mangle)]
pub fn vfs_open(name: &str, _mode: &str) -> Fd {
    for entry in FILES {
        if entry.name.contains(name) {
            let (slot, fd) = unsafe { get_file_pool_slot() };
            if slot.is_null() {
                return -1;
            }

            unsafe {
                (*slot).data = entry.data.as_ptr();
                (*slot).size = entry.data.len();
                (*slot).cursor = 0;
                (*slot).writable = false;
                (*slot).write_buf = null_mut();
                (*slot).write_cap = 0;
                (*slot).fd = fd;
            }
            return fd;
        }
    }
    -1
}

#[unsafe(no_mangle)]
pub fn vfs_close(fd: Fd) -> i32 {
    if !fd_ok(fd) {
        return -1;
    }

    unsafe {
        let idx = fd as usize;
        if !FILE_POOL_USED[idx] {
            return -1;
        }
        FILE_POOL_USED[idx] = false;
        FILE_POOL[idx] = FILE::zeroed();
    }

    0
}

#[unsafe(no_mangle)]
pub fn vfs_write(ptr: *mut u8, size: usize, count: usize, fp: *mut FILE) -> usize {
    if ptr.is_null() || fp.is_null() || unsafe { (*fp).fd < 0 || (*fp).fd >= 16 } {
        return 0;
    }
    count
}

#[unsafe(no_mangle)]
pub fn vfs_read(fd: Fd, len: usize) -> Option<(*const u8, usize)> {
    unsafe {
        let f = file_mut(fd)?;
        if f.cursor > f.size {
            return Some((f.data, 0));
        }

        let available = f.size - f.cursor;
        let to_read = len.min(available);

        let src = f.data.add(f.cursor);
        f.cursor = f.cursor.saturating_add(to_read);

        Some((src, to_read))
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vfs_lseek(fd: Fd, offset: i64, whence: i32) -> i64 {
    let f = match unsafe { file_mut(fd) } {
        Some(f) => f,
        None => return -1,
    };

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
    f.cursor as i64
}
