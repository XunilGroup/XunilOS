use crate::driver::io::{
    block::BlockDevice,
    fs::{
        FSError, FileSystem,
        ext2::{
            BlockGroupDescriptor, Ext2Inode, Ext2InodeType, Ext2Superblock,
            read_raw_bytes_from_disk, read_u8_from_offset, read_u16_from_offset,
            read_u32_from_offset,
        },
    },
};
use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::f64::math::ceil;

struct Ext2FileSystem {
    superblock: Ext2Superblock,
    block_offset: u64,
    block_group_count: u64,
    block_size: u32,
    inode_size: u32,
    device: Arc<dyn BlockDevice>,
}

impl Ext2FileSystem {
    pub fn new(block_offset: u64, block_device: Arc<dyn BlockDevice>) -> Result<Self, FSError> {
        let mut superblock_raw_opt = read_raw_bytes_from_disk(
            block_device.clone(),
            block_offset + 1024,
            size_of::<Ext2Superblock>() as u32,
        );

        let superblock = match superblock_raw_opt {
            Some(mut superblock_raw) => {
                Ext2Superblock::from_byte_array(superblock_raw.as_mut_slice())
            }
            _ => return Err(FSError::IOError),
        };

        if superblock.s_magic != 0xEF53 {
            return Err(FSError::IOError);
        }

        let block_size = 1024 << superblock.s_log_block_size;
        let block_group_count =
            ceil((superblock.s_blocks_count as f64) / (superblock.s_blocks_per_group as f64))
                as u64;

        Ok(Ext2FileSystem {
            superblock,
            block_size,
            block_group_count,
            inode_size: 128,
            block_offset,
            device: block_device.clone(),
        })
    }

    pub fn get_block_group(&self, block_group_n: u64) -> Option<BlockGroupDescriptor> {
        if block_group_n >= self.block_group_count {
            return None;
        }

        let bgdt_block = if self.block_size == 1024 { 2 } else { 1 };
        let bgdt_offset = self.block_offset + bgdt_block * self.block_size as u64;

        let descriptor_offset =
            bgdt_offset + block_group_n * size_of::<BlockGroupDescriptor>() as u64;

        let mut raw = read_raw_bytes_from_disk(
            self.device.clone(),
            descriptor_offset,
            size_of::<BlockGroupDescriptor>() as u32,
        )?;

        Some(BlockGroupDescriptor::from_byte_array(raw.as_mut_slice()))
    }

    pub fn get_block(&self, fs_block: u32) -> Option<Vec<u8>> {
        let byte_offset = self.block_offset + fs_block as u64 * self.block_size as u64;

        read_raw_bytes_from_disk(self.device.clone(), byte_offset, self.block_size)
    }

    pub fn get_inode(&self, inode_n: u64) -> Option<Ext2Inode> {
        let inode = inode_n - 1;

        let group = inode / self.superblock.s_inodes_per_group as u64;
        let index = inode % self.superblock.s_inodes_per_group as u64;

        let block_group = self.get_block_group(group)?;

        let inode_offset = index * self.inode_size as u64;

        let block = inode_offset / self.block_size as u64;
        let offset = inode_offset % self.block_size as u64;

        let fs_block = block_group.bg_inode_table as u64 + block;

        let byte_offset = self.block_offset + fs_block * self.block_size as u64;

        let mut block_raw =
            read_raw_bytes_from_disk(self.device.clone(), byte_offset, self.block_size)?;

        let inode_bytes =
            &mut block_raw[offset as usize..offset as usize + self.inode_size as usize];

        Some(Ext2Inode::from_byte_array(inode_bytes))
    }

    fn traverse_filesystem_tree(&self, path: &str) -> Result<Ext2Inode, FSError> {
        let path_parts: Vec<&str> = path
            .split("/")
            .filter(|s| !s.is_empty())
            .collect::<Vec<&str>>();

        if path_parts.is_empty() {
            return Err(FSError::InvalidPath);
        }

        let mut inode;
        let mut inode_n = 2;
        for path_n in 0..path_parts.len() {
            inode = match self.get_inode(inode_n) {
                Some(inode) => inode,
                None => return Err(FSError::IOError),
            };

            if path_n != path_parts.len() - 1 {
                match inode.inode_type().unwrap_or(Ext2InodeType::CharDevice) {
                    Ext2InodeType::Directory => {
                        let mut dir_found = false;
                        for &block in &inode.i_block[..12] {
                            if block == 0 {
                                break;
                            }

                            let block_data = match self.get_block(block) {
                                Some(block_data) => block_data,
                                None => return Err(FSError::IOError),
                            };

                            let mut offset = 0;

                            while offset < block_data.len() {
                                let entry_start = offset;

                                let inode =
                                    read_u32_from_offset(block_data.as_slice(), &mut offset);
                                let rec_len =
                                    read_u16_from_offset(block_data.as_slice(), &mut offset);
                                let name_len =
                                    read_u8_from_offset(block_data.as_slice(), &mut offset);
                                let file_type =
                                    read_u8_from_offset(block_data.as_slice(), &mut offset);

                                if inode != 0 {
                                    let name_bytes =
                                        &block_data[offset..offset + name_len as usize];
                                    let name = core::str::from_utf8(name_bytes).unwrap_or("");

                                    if name == path_parts[path_n] {
                                        inode_n = inode as u64;
                                        dir_found = true;
                                        break;
                                    }
                                }

                                if rec_len == 0 {
                                    break;
                                }

                                offset = entry_start + rec_len as usize;
                            }

                            if dir_found {
                                break;
                            }
                        }

                        if !dir_found {
                            return Err(FSError::NotFound);
                        }
                    }
                    _ => return Err(FSError::InvalidPath),
                };
            } else {
                return Ok(inode);
            }
        }

        return Err(FSError::NotFound);
    }
}

impl FileSystem for Ext2FileSystem {
    fn read_file(&self, path: &str) -> Result<Vec<u8>, FSError> {
        let inode = self.traverse_filesystem_tree(path)?;

        if inode.inode_type() != Some(Ext2InodeType::File) {
            return Err(FSError::PermissionDenied);
        }

        let mut data = Vec::<u8>::new();

        for &block in &inode.i_block[..12] {
            if block == 0 {
                break;
            }

            let block_data = match self.get_block(block) {
                Some(block_data) => block_data,
                None => return Err(FSError::IOError),
            };

            data.extend_from_slice(&block_data);
        }

        data.truncate(inode.i_size as usize);

        Ok(data)
    }

    fn write_file(&self, path: &str, data: &[u8]) -> Result<usize, FSError> {
        unimplemented!()
    }

    fn list_directory(&self, path: &str) -> Result<Vec<String>, FSError> {
        let final_directory_inode = self.traverse_filesystem_tree(path)?;

        if final_directory_inode.inode_type() != Some(Ext2InodeType::Directory) {
            return Err(FSError::PermissionDenied);
        }

        let mut entries = Vec::new();

        for &block in &final_directory_inode.i_block[..12] {
            if block == 0 {
                break;
            }

            let block_data = match self.get_block(block) {
                Some(block_data) => block_data,
                None => return Err(FSError::IOError),
            };

            let mut offset = 0;

            while offset < block_data.len() {
                let entry_start = offset;

                let inode = read_u32_from_offset(block_data.as_slice(), &mut offset);
                let rec_len = read_u16_from_offset(block_data.as_slice(), &mut offset);
                let name_len = read_u8_from_offset(block_data.as_slice(), &mut offset);
                let file_type = read_u8_from_offset(block_data.as_slice(), &mut offset);

                if inode != 0 {
                    let name_bytes = &block_data[offset..offset + name_len as usize];
                    let name = core::str::from_utf8(name_bytes).unwrap_or("");

                    entries.push(name.to_string());
                }

                if rec_len == 0 {
                    break;
                }

                offset = entry_start + rec_len as usize;
            }
        }

        Ok(entries)
    }

    fn exists(&self, path: &str) -> bool {
        match self.traverse_filesystem_tree(path) {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}
