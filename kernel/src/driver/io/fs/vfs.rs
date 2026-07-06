use crate::driver::io::fs::{FILE, FileSystem, fakefs::FAKE_FS};
use alloc::{string::ToString, sync::Arc, vec::Vec};

pub struct VFS {
    files: Vec<FILE>,
    next_fd: i64,
    filesystems: Vec<Arc<dyn FileSystem>>,
}

impl VFS {
    pub fn new() -> VFS {
        let mut filesystems = Vec::new();
        filesystems.push(Arc::new(FAKE_FS.clone()) as Arc<dyn FileSystem>);
        VFS {
            files: Vec::new(),
            next_fd: 0,
            filesystems,
        }
    }

    pub fn add_filesystem(&mut self, fs: Arc<dyn FileSystem>) {
        self.filesystems.push(fs);
    }

    pub fn get_filesystem(&self, path: &str) -> Option<Arc<dyn FileSystem>> {
        for filesystem in &self.filesystems {
            if filesystem.exists(path) {
                return Some(filesystem.clone()); // only arc cloning, this goes faassst
            }
        }

        return None;
    }

    fn open(&mut self, path: &str, mode: &str) -> i64 {
        let fs = match self.get_filesystem(path) {
            Some(fs) => fs,
            None => return -1,
        };

        let is_write = mode.contains("w") || mode.contains("a");

        if let Some(file) = self
            .files
            .iter_mut()
            .find(|file| file.path.as_str() == path)
        {
            file.cursor = 0;
            file.writable = is_write;

            return file.fd;
        }

        let fd = self.next_fd;
        self.next_fd += 1;

        let data = match fs.read_file(path) {
            Ok(data) => data,
            Err(_) => return -1,
        };

        let file = FILE {
            path: path.to_string(),
            data: data.to_vec(),
            cursor: 0,
            writable: is_write,
            fd,
            size: data.len(),
        };

        self.files.push(file);

        fd
    }

    fn close(&mut self, fd: i64) -> i32 {
        if let Some(file_pos) = self.files.iter().position(|file| file.fd == fd) {
            self.files.remove(file_pos);
            0
        } else {
            -1
        }
    }

    fn lseek(&mut self, fd: i64, offset: i64, whence: i32) -> i64 {
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

    fn read(&mut self, fd: i64, len: usize) -> Option<(*const u8, usize)> {
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

    fn write(&mut self, fd: i64, data: &[u8]) -> Option<usize> {
        let path = {
            let file = self.files.iter_mut().find(|f| f.fd == fd)?;
            if !file.writable {
                return None;
            }
            file.path.clone()
        };

        let fs = self.get_filesystem(&path)?;

        let file = self.files.iter_mut().find(|f| f.fd == fd)?;
        file.data.extend_from_slice(data);
        file.size = file.data.len();
        fs.write_file(&file.path, &file.data);

        Some(data.len())
    }
}

pub static mut VFS_INSTANCE: Option<VFS> = None;

pub fn init_vfs() {
    unsafe { VFS_INSTANCE = Some(VFS::new()) }
}

#[unsafe(no_mangle)]
pub fn vfs_open(path: &str, mode: &str) -> i64 {
    #[allow(static_mut_refs)]
    unsafe {
        VFS_INSTANCE.as_mut().unwrap().open(path, mode)
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
