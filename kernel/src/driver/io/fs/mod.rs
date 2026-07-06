#![allow(warnings)]
use alloc::{string::String, vec::Vec};

pub mod assets;
pub mod ext2;
pub mod fakefs;
pub mod vfs;

#[repr(C)]
#[derive(Clone, Debug)]
pub struct FILE {
    pub path: String,
    pub size: usize,
    pub data: Vec<u8>,
    pub cursor: usize,
    pub writable: bool,
    pub fd: i64,
}

impl FILE {
    pub fn new(path: String, data: Vec<u8>, writable: bool) -> FILE {
        FILE {
            path,
            data: data.clone(),
            cursor: 0,
            writable,
            fd: -1,
            size: data.len(),
        }
    }
}

pub struct Entry {
    is_dir: bool,
    name: String,
    pub size: u64,
}

pub enum FSError {
    NotFound,
    PermissionDenied,
    IOError,
    InvalidPath,
}

pub trait FileSystem {
    fn read_file(&self, path: &str) -> Result<Vec<u8>, FSError>;
    fn write_file(&self, path: &str, data: &[u8]) -> Result<usize, FSError>;
    fn list_directory(&self, path: &str) -> Result<Vec<String>, FSError>;
    fn exists(&self, path: &str) -> bool;
}
