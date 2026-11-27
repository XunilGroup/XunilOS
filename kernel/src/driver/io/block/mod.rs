use alloc::{string::String, vec::Vec};

pub mod ahci;
pub mod gpt;
pub mod nvme;

pub trait PartitionTable {
    fn partitions(&self) -> Result<Vec<Partition>, BlockError>;
}

pub struct Partition {
    pub name: String,
    pub start: u64,
    pub end: u64,
    pub partition_type_guid: u128,
    pub unique_partition_guid: u128,
}

#[derive(Debug)]
pub enum BlockError {
    IOTimeout,
    ReadError,
    WriteError,
    InvalidSector,
    SystemError,
}

pub trait BlockDevice: Send + Sync {
    fn read_sectors(&self, sector: u64, count: u32, buffer: &mut [u8]) -> Result<(), BlockError>;
    fn write_sectors(&self, sector: u64, count: u32, buffer: &[u8]) -> Result<(), BlockError>;
    fn sector_size(&self) -> u32;
    fn capacity_sectors(&self) -> u64;
}
