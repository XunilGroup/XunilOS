pub mod ahci;

pub enum BlockError {
    IOTimeout,
    ReadError,
    WriteError,
    InvalidSector,
}

pub trait BlockDevice: Send + Sync {
    fn read_sectors(&self, sector: u64, count: u32, buffer: &mut [u8]) -> Result<(), BlockError>;
    fn write_sectors(&self, sector: u64, count: u32, buffer: &[u8]) -> Result<(), BlockError>;
    fn sector_size(&self) -> u32;
    fn capacity_sectors(&self) -> u32;
}

pub struct MockBlockDevice {}

impl BlockDevice for MockBlockDevice {
    fn read_sectors(&self, sector: u64, count: u32, buffer: &mut [u8]) -> Result<(), BlockError> {
        unimplemented!()
    }
    fn write_sectors(&self, sector: u64, count: u32, buffer: &[u8]) -> Result<(), BlockError> {
        unimplemented!()
    }
    fn sector_size(&self) -> u32 {
        unimplemented!()
    }
    fn capacity_sectors(&self) -> u32 {
        unimplemented!()
    }
}
