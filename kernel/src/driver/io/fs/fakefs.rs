#![allow(warnings)]

use crate::driver::io::fs::FSError;
use alloc::{collections::btree_map::BTreeMap, string::String, vec::Vec};
use lazy_static::lazy_static;

use crate::driver::io::fs::{FileSystem, assets::*};

#[derive(Clone)]
pub struct FakeFileSystem {
    file_content: BTreeMap<&'static str, &'static [u8]>,
}
impl FakeFileSystem {
    fn new() -> Self {
        let mut map = BTreeMap::new();
        map.insert("testfile", &b"Hello, World!"[..]);
        map.insert("helloworld.elf", HELLOWORLD_ELF);
        map.insert("badapple", BADAPPLE_ELF);
        map.insert("doomgeneric", DOOM_ELF);
        map.insert("shell", SHELL_ELF);
        map.insert("doom1.wad", DOOM_WAD);
        map.insert("doom.cfg", &b""[..]);
        map.insert("default.cfg", &b""[..]);
        FakeFileSystem { file_content: map }
    }
}

impl FileSystem for FakeFileSystem {
    fn read_file(&self, path: &str) -> Result<Vec<u8>, FSError> {
        for (filename, data) in &self.file_content {
            if path.contains(filename) {
                return Ok(data.to_vec());
            }
        }

        Err(FSError::NotFound)
    }
    fn write_file(&self, path: &str, data: &[u8]) -> Result<usize, FSError> {
        unimplemented!()
    }
    fn list_directory(&self, path: &str) -> Result<Vec<String>, FSError> {
        unimplemented!()
    }
    fn exists(&self, path: &str) -> bool {
        for (filename, data) in &self.file_content {
            if path.contains(filename) {
                return true;
            }
        }

        return false;
    }
}

lazy_static! {
    pub static ref FAKE_FS: FakeFileSystem = {
        let fake_fs = FakeFileSystem::new();
        fake_fs
    };
}
