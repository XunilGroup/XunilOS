use core::slice;

use crate::driver::io::{block::BlockDevice, fs::FSError};
use alloc::vec;
use alloc::{sync::Arc, vec::Vec};

pub mod fs;

fn read_u32_from_offset(buf: &[u8], off: &mut usize) -> u32 {
    let value = u32::from_le_bytes(buf[*off..*off + 4].try_into().unwrap());
    *off += 4;
    return value;
}

fn read_u16_from_offset(buf: &[u8], off: &mut usize) -> u16 {
    let value = u16::from_le_bytes(buf[*off..*off + 2].try_into().unwrap());
    *off += 2;
    return value;
}
pub fn read_raw_bytes_from_disk(
    device: Arc<dyn BlockDevice>,
    offset: u64,
    size: u32,
) -> Option<Vec<u8>> {
    let mut buffer_vec = vec![0u8; size as usize];
    let mut buffer_slice = buffer_vec.as_mut_slice();

    match device.read_sectors(
        offset + (size as u64), // block groups come after
        size,
        buffer_slice,
    ) {
        Err(_) => return None,
        _ => Some(buffer_vec),
    }
}

#[repr(C)]
struct Ext2Inode {
    i_mode: u16,
    i_uid: u16,
    i_size: u32,
    i_atime: u32,
    i_ctime: u32,
    i_mtime: u32,
    i_dtime: u32,
    i_gid: u16,
    i_links_count: u16,
    i_blocks: u32,
    i_flags: u32,
    i_osd1: u32,
    i_block: [u32; 15],
    i_generation: u32,
    i_file_acl: u32,
    i_dir_acl: u32,
    i_faddr: u32,
    i_osd2: [u8; 12],
}

impl Ext2Inode {
    fn from_byte_array(byte_array: &mut [u8]) -> Self {
        let mut off = 0;
        Self {
            i_mode: read_u16_from_offset(byte_array, &mut off),
            i_uid: read_u16_from_offset(byte_array, &mut off),
            i_size: read_u32_from_offset(byte_array, &mut off),
            i_atime: read_u32_from_offset(byte_array, &mut off),
            i_ctime: read_u32_from_offset(byte_array, &mut off),
            i_mtime: read_u32_from_offset(byte_array, &mut off),
            i_dtime: read_u32_from_offset(byte_array, &mut off),
            i_gid: read_u16_from_offset(byte_array, &mut off),
            i_links_count: read_u16_from_offset(byte_array, &mut off),
            i_blocks: read_u32_from_offset(byte_array, &mut off),
            i_flags: read_u32_from_offset(byte_array, &mut off),
            i_osd1: read_u32_from_offset(byte_array, &mut off),
            i_block: {
                let mut block_list: [u32; 15] = [0u32; 15];

                for n in 0..15 {
                    block_list[n] = read_u32_from_offset(byte_array, &mut off)
                }

                block_list
            },
            i_generation: read_u32_from_offset(byte_array, &mut off),
            i_file_acl: read_u32_from_offset(byte_array, &mut off),
            i_dir_acl: read_u32_from_offset(byte_array, &mut off),
            i_faddr: read_u32_from_offset(byte_array, &mut off),
            i_osd2: {
                let osd2 = byte_array[off..off + 12].try_into().unwrap();
                off += 12;
                osd2
            },
        }
    }
}

#[repr(C)]
struct BlockGroupDescriptor {
    bg_block_bitmap: u32,
    bg_inode_bitmap: u32,
    bg_inode_table: u32,
    bg_free_blocks_count: u16,
    bg_free_inodes_count: u16,
    bg_used_dirs_count: u16,
    bg_pad: u16,
    bg_reserved: [u8; 12],
}

impl BlockGroupDescriptor {
    fn from_byte_array(byte_array: &mut [u8]) -> Self {
        let mut off = 0;
        Self {
            bg_block_bitmap: read_u32_from_offset(byte_array, &mut off),
            bg_inode_bitmap: read_u32_from_offset(byte_array, &mut off),
            bg_inode_table: read_u32_from_offset(byte_array, &mut off),
            bg_free_blocks_count: read_u16_from_offset(byte_array, &mut off),
            bg_free_inodes_count: read_u16_from_offset(byte_array, &mut off),
            bg_used_dirs_count: read_u16_from_offset(byte_array, &mut off),
            bg_pad: read_u16_from_offset(byte_array, &mut off),
            bg_reserved: {
                let reserved = byte_array[off..off + 12].try_into().unwrap();
                off += 12;
                reserved
            },
        }
    }
}

#[repr(C)]
struct Ext2Superblock {
    s_inodes_count: u32,
    s_blocks_count: u32,
    s_r_blocks_count: u32,
    s_free_blocks_count: u32,
    s_free_inodes_count: u32,
    s_first_data_block: u32,
    s_log_block_size: u32,
    s_log_frag_size: u32,
    s_blocks_per_group: u32,
    s_frags_per_group: u32,
    s_inodes_per_group: u32,
    s_mtime: u32,
    s_wtime: u32,
    s_mnt_count: u16,
    s_max_mnt_count: u16,
    s_magic: u16,
    s_state: u16,
    s_errors: u16,
    s_minor_rev_level: u16,
    s_lastcheck: u32,
    s_checkinterval: u32,
    s_creator_os: u32,
    s_rev_level: u32,
    s_def_resuid: u16,
    s_def_resgid: u16,
}

impl Ext2Superblock {
    fn from_byte_array(byte_array: &mut [u8]) -> Self {
        let mut off = 0;
        Self {
            s_inodes_count: read_u32_from_offset(byte_array, &mut off),
            s_blocks_count: read_u32_from_offset(byte_array, &mut off),
            s_r_blocks_count: read_u32_from_offset(byte_array, &mut off),
            s_free_blocks_count: read_u32_from_offset(byte_array, &mut off),
            s_free_inodes_count: read_u32_from_offset(byte_array, &mut off),
            s_first_data_block: read_u32_from_offset(byte_array, &mut off),
            s_log_block_size: read_u32_from_offset(byte_array, &mut off),
            s_log_frag_size: read_u32_from_offset(byte_array, &mut off),
            s_blocks_per_group: read_u32_from_offset(byte_array, &mut off),
            s_frags_per_group: read_u32_from_offset(byte_array, &mut off),
            s_inodes_per_group: read_u32_from_offset(byte_array, &mut off),
            s_mtime: read_u32_from_offset(byte_array, &mut off),
            s_wtime: read_u32_from_offset(byte_array, &mut off),
            s_mnt_count: read_u16_from_offset(byte_array, &mut off),
            s_max_mnt_count: read_u16_from_offset(byte_array, &mut off),
            s_magic: read_u16_from_offset(byte_array, &mut off),
            s_state: read_u16_from_offset(byte_array, &mut off),
            s_errors: read_u16_from_offset(byte_array, &mut off),
            s_minor_rev_level: read_u16_from_offset(byte_array, &mut off),
            s_lastcheck: read_u32_from_offset(byte_array, &mut off),
            s_checkinterval: read_u32_from_offset(byte_array, &mut off),
            s_creator_os: read_u32_from_offset(byte_array, &mut off),
            s_rev_level: read_u32_from_offset(byte_array, &mut off),
            s_def_resuid: read_u16_from_offset(byte_array, &mut off),
            s_def_resgid: read_u16_from_offset(byte_array, &mut off),
        }
    }
}
