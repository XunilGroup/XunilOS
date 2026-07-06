use crate::driver::io::{
    block::BlockDevice,
    fs::{
        FSError, FileSystem,
        ext2::{
            BlockGroupDescriptor, Ext2Inode, Ext2Superblock, read_raw_bytes_from_disk,
            read_u16_from_offset, read_u32_from_offset,
        },
    },
};
use alloc::{string::String, sync::Arc, vec::Vec};
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

        let mut raw_block_group_opt = read_raw_bytes_from_disk(
            self.device.clone(),
            self.block_offset
                + 1024
                + 1024
                + (block_group_n * (size_of::<BlockGroupDescriptor>() as u64)),
            size_of::<BlockGroupDescriptor>() as u32,
        );

        match raw_block_group_opt {
            Some(mut raw_block_group) => Some(BlockGroupDescriptor::from_byte_array(
                raw_block_group.as_mut_slice(),
            )),
            _ => None,
        }
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
}

impl FileSystem for Ext2FileSystem {
    fn read_file(&self, path: &str) -> Result<Vec<u8>, FSError> {
        // TODO: traverse path to get inode count, for testing, inode n is 0
        let inode_n = 0;
        let inode = match self.get_inode(inode_n) {
            Some(inode) => inode,
            None => return Err(FSError::IOError),
        };

        let mut data = Vec::<u8>::new();

        for &block in &inode.i_block {
            if block == 0 {
                break;
            }

            let block_data = match self.get_block(block) {
                Some(block_data) => block_data,
                None => return Err(FSError::IOError),
            };

            data.extend_from_slice(&block_data);
        }

        Ok(data)
    }

    fn write_file(&self, path: &str, data: &[u8]) -> Result<usize, FSError> {
        unimplemented!()
    }

    fn list_directory(&self, path: &str) -> Result<Vec<String>, FSError> {
        unimplemented!()
    }

    fn exists(&self, path: &str) -> bool {
        unimplemented!()
    }
}
