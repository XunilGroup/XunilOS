use alloc::{string::String, vec, vec::Vec};

use crate::{
    driver::io::block::{BlockDevice, Partition},
    println,
};

const GPT_MAGIC: [u8; 8] = *b"EFI PART";

#[repr(C)]
pub struct GPTPartitionTable {
    pub revision: u32,
    pub guid: u128,
    pub partitions: Vec<Partition>,
}

fn le_u32(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(b[off..off + 4].try_into().unwrap())
}
fn le_u64(b: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(b[off..off + 8].try_into().unwrap())
}
fn le_u128(b: &[u8], off: usize) -> u128 {
    u128::from_le_bytes(b[off..off + 16].try_into().unwrap())
}

fn read_utf16le(b: &[u8], off: usize) -> String {
    let mut units = Vec::new();
    for chunk in b[off..off + 72].chunks_exact(2) {
        let u = u16::from_le_bytes([chunk[0], chunk[1]]);
        if u == 0 {
            break;
        }
        units.push(u);
    }
    char::decode_utf16(units)
        .map(|c| c.unwrap_or('\u{FFFD}'))
        .collect()
}

pub fn find_gpt(block_device: &impl BlockDevice) -> Option<GPTPartitionTable> {
    let sector_size = block_device.sector_size() as usize;
    if sector_size < 92 {
        return None;
    }

    let mut buffer = vec![0u8; sector_size];
    let result = block_device.read_sectors(1, 1, buffer.as_mut_slice());

    if !result.is_ok() {
        println!("find_gpt: read of first sector failed");
        return None;
    }

    if buffer[0..8] != GPT_MAGIC {
        return None;
    }

    let revision = le_u32(buffer.as_slice(), 0x8);
    let guid = le_u128(buffer.as_slice(), 0x38);
    let partition_entry_lba_start = le_u64(buffer.as_slice(), 0x48);
    let partition_entry_count = le_u32(buffer.as_slice(), 0x50);
    let partition_entry_size = le_u32(buffer.as_slice(), 0x54);

    if partition_entry_count == 0 {
        println!("Partition entry count is 0");
        return None;
    }

    let total_bytes = partition_entry_count as usize * partition_entry_size as usize;
    let sectors = total_bytes.div_ceil(sector_size);
    let mut entries_buf = vec![0u8; sectors * sector_size];
    block_device
        .read_sectors(partition_entry_lba_start, sectors as u32, &mut entries_buf)
        .ok()?;

    let mut partitions = Vec::<Partition>::with_capacity(partition_entry_count as usize);

    for n in 0..partition_entry_count {
        let partition_entry_base = (n * partition_entry_size) as usize;
        let unique_partition_guid = le_u128(buffer.as_slice(), 0x10);
        let start = le_u64(entries_buf.as_slice(), partition_entry_base + 0x20);
        let end = le_u64(entries_buf.as_slice(), partition_entry_base + 0x28);

        if unique_partition_guid == 0 || start == 0 || end == 0 {
            continue;
        }

        partitions.push(Partition {
            partition_type_guid: le_u128(entries_buf.as_slice(), partition_entry_base + 0x0),
            unique_partition_guid: le_u128(entries_buf.as_slice(), partition_entry_base + 0x10),
            start,
            end,
            name: read_utf16le(&entries_buf.as_slice(), partition_entry_base + 0x38),
        });
    }

    return Some(GPTPartitionTable {
        revision,
        guid,
        partitions,
    });
}
