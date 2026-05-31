use alloc::{
    collections::btree_map::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use lazy_static::lazy_static;

use crate::driver::io::fs::assets::*;

lazy_static! {
    static ref FILE_CONTENT: BTreeMap<&'static str, &'static [u8]> = {
        let mut map = BTreeMap::new();
        map.insert("testfile", &b"Hello, World!"[..]);
        map.insert("helloworld.elf", HELLOWORLD_ELF);
        map.insert("badapple", BADAPPLE_ELF);
        map.insert("doomgeneric", DOOM_ELF);
        map.insert("shell", SHELL_ELF);
        map.insert("doom1.wad", DOOM_WAD);
        map.insert("doom.cfg", &b""[..]);
        map.insert("default.cfg", &b""[..]);
        map
    };
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct FILE {
    pub name: String,
    pub size: usize,
    pub data: Vec<u8>,
    pub cursor: usize,
    pub writable: bool,
    pub fd: i64,
}

impl FILE {
    pub fn new(name: String, data: Vec<u8>, writable: bool) -> FILE {
        FILE {
            name,
            data: data.clone(),
            cursor: 0,
            writable,
            fd: -1,
            size: data.len(),
        }
    }
}

pub struct VFS {
    files: Vec<FILE>,
    next_fd: i64,
}

impl VFS {
    pub fn new() -> VFS {
        VFS {
            files: Vec::new(),
            next_fd: 0,
        }
    }

    pub fn open(&mut self, name: &str, mode: &str) -> i64 {
        let is_write = mode.contains("w") || mode.contains("a");

        if let Some(file) = self
            .files
            .iter_mut()
            .find(|file| file.name.as_str() == name)
        {
            file.cursor = 0;
            file.writable = is_write;

            return file.fd;
        }

        let fd = self.next_fd;
        self.next_fd += 1;

        let empty_data = &"".as_bytes();

        let data = FILE_CONTENT.get(name).unwrap_or(empty_data);

        let file = FILE {
            name: name.to_string(),
            data: data.to_vec(),
            cursor: 0,
            writable: is_write,
            fd,
            size: data.len(),
        };

        self.files.push(file);

        fd
    }

    pub fn close(&mut self, fd: i64) -> i32 {
        if let Some(file_pos) = self.files.iter().position(|file| file.fd == fd) {
            self.files.remove(file_pos);
            0
        } else {
            -1
        }
    }

    pub fn lseek(&mut self, fd: i64, offset: i64, whence: i32) -> i64 {
        let f = match self.files.iter_mut().find(|file| file.fd == fd) {
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

    pub fn read(&mut self, fd: i64, len: usize) -> Option<(*const u8, usize)> {
        if let Some(f) = self.files.iter_mut().find(|file| file.fd == fd) {
            if f.cursor > f.size {
                return Some((f.data.as_ptr(), 0));
            }

            let available = f.size - f.cursor;
            let to_read = len.min(available);

            let src = unsafe { f.data.as_ptr().add(f.cursor) };
            f.cursor = f.cursor.saturating_add(to_read);

            Some((src, to_read))
        } else {
            None
        }
    }

    pub fn write(&mut self, fd: i64, data: &[u8]) -> Option<usize> {
        let file = self.files.iter_mut().find(|f| f.fd == fd)?;

        if !file.writable {
            return None;
        }

        file.data.extend_from_slice(data);
        file.size = file.data.len();
        Some(data.len())
    }
}

pub static mut VFS_INSTANCE: Option<VFS> = None;

pub fn init_vfs() {
    unsafe {
        VFS_INSTANCE = Some(VFS {
            files: Vec::new(),
            next_fd: 3,
        })
    }
}

#[unsafe(no_mangle)]
pub fn vfs_open(name: &str, mode: &str) -> i64 {
    #[allow(static_mut_refs)]
    unsafe {
        VFS_INSTANCE.as_mut().unwrap().open(name, mode)
    }
}

#[unsafe(no_mangle)]
pub fn vfs_close(fd: i64) -> i32 {
    #[allow(static_mut_refs)]
    unsafe {
        VFS_INSTANCE.as_mut().unwrap().close(fd)
    }
}

#[unsafe(no_mangle)]
pub fn vfs_write(ptr: *const u8, size: usize, count: usize, fd: i64) -> Option<usize> {
    if ptr.is_null() {
        return None;
    }

    let len = size.checked_mul(count)?;

    unsafe {
        let slice = core::slice::from_raw_parts(ptr, len);
        #[allow(static_mut_refs)]
        VFS_INSTANCE.as_mut().unwrap().write(fd, slice)
    }
}

#[unsafe(no_mangle)]
pub fn vfs_read(fd: i64, len: usize) -> Option<(*const u8, usize)> {
    #[allow(static_mut_refs)]
    unsafe {
        VFS_INSTANCE.as_mut().unwrap().read(fd, len)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vfs_lseek(fd: i64, offset: i64, whence: i32) -> i64 {
    #[allow(static_mut_refs)]
    unsafe {
        VFS_INSTANCE.as_mut().unwrap().lseek(fd, offset, whence)
    }
}
