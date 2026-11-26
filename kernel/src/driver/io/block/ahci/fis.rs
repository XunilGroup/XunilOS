//! SATA FIS structures (AHCI 1.3.1 §3.4 / SATA 3.x).

pub const FIS_TYPE_REG_H2D: u8 = 0x27;
pub const FIS_TYPE_REG_D2H: u8 = 0x34;
pub const FIS_TYPE_DMA_ACT: u8 = 0x39;
pub const FIS_TYPE_DMA_SETUP: u8 = 0x41;
pub const FIS_TYPE_DATA: u8 = 0x46;
pub const FIS_TYPE_BIST: u8 = 0x58;
pub const FIS_TYPE_PIO_SETUP: u8 = 0x5F;
pub const FIS_TYPE_DEV_BITS: u8 = 0xA1;

#[repr(C)]
#[derive(Debug, Default)]
pub struct FisRegH2d {
    pub fis_type: u8, // FIS_TYPE_REG_H2D
    pub flags: u8,    // pmport:4, rsv0:3, c:1
    pub command: u8,
    pub featurel: u8,
    pub lba0: u8,
    pub lba1: u8,
    pub lba2: u8,
    pub device: u8,
    pub lba3: u8,
    pub lba4: u8,
    pub lba5: u8,
    pub featureh: u8,
    pub countl: u8,
    pub counth: u8,
    pub icc: u8,
    pub control: u8,
    pub rsv: [u8; 4],
}

impl FisRegH2d {
    #[inline]
    pub fn get(&self, mask: u8, shift: u32) -> u8 {
        (self.flags >> shift) & mask
    }

    #[inline]
    pub fn set(&mut self, mask: u8, shift: u32, v: u8) {
        self.flags = (self.flags & !(mask << shift)) | ((v & mask) << shift);
    }

    pub fn pmport(&self) -> u8 {
        self.get(0xF, 0)
    }

    pub fn set_pmport(&mut self, v: u8) {
        self.set(0xF, 0, v)
    }

    pub fn c(&self) -> u8 {
        self.get(1, 7)
    }

    pub fn set_c(&mut self, v: u8) {
        self.set(1, 7, v)
    }
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct FisRegD2h {
    pub fis_type: u8, // FIS_TYPE_REG_D2H
    pub flags: u8,    // pmport:4, rsv0:2, i:1, rsv1:1
    pub status: u8,
    pub error: u8,
    pub lba0: u8,
    pub lba1: u8,
    pub lba2: u8,
    pub device: u8,
    pub lba3: u8,
    pub lba4: u8,
    pub lba5: u8,
    pub rsv2: u8,
    pub countl: u8,
    pub counth: u8,
    pub rsv3: [u8; 2],
    pub rsv4: [u8; 4],
}

impl FisRegD2h {
    #[inline]
    pub fn get(&self, mask: u8, shift: u32) -> u8 {
        (self.flags >> shift) & mask
    }

    #[inline]
    pub fn set(&mut self, mask: u8, shift: u32, v: u8) {
        self.flags = (self.flags & !(mask << shift)) | ((v & mask) << shift);
    }

    pub fn pmport(&self) -> u8 {
        self.get(0xF, 0)
    }

    pub fn set_pmport(&mut self, v: u8) {
        self.set(0xF, 0, v)
    }

    pub fn interrupt(&self) -> u8 {
        self.get(1, 6)
    }

    pub fn set_interrupt(&mut self, v: u8) {
        self.set(1, 6, v)
    }
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct FisData {
    pub fis_type: u8, // FIS_TYPE_DATA
    pub flags: u8,    // pmport:4, rsv0:4
    pub rsv1: [u8; 2],
    pub data: [u32; 1],
}

impl FisData {
    #[inline]
    pub fn get(&self, mask: u8, shift: u32) -> u8 {
        (self.flags >> shift) & mask
    }

    #[inline]
    pub fn set(&mut self, mask: u8, shift: u32, v: u8) {
        self.flags = (self.flags & !(mask << shift)) | ((v & mask) << shift);
    }

    pub fn pmport(&self) -> u8 {
        self.get(0xF, 0)
    }

    pub fn set_pmport(&mut self, v: u8) {
        self.set(0xF, 0, v)
    }
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct FisPioSetup {
    pub fis_type: u8, // FIS_TYPE_PIO_SETUP
    pub flags: u8,    // pmport:4, rsv0:1, d:1, i:1, rsv1:1
    pub status: u8,
    pub error: u8,
    pub lba0: u8,
    pub lba1: u8,
    pub lba2: u8,
    pub device: u8,
    pub lba3: u8,
    pub lba4: u8,
    pub lba5: u8,
    pub rsv2: u8,
    pub countl: u8,
    pub counth: u8,
    pub rsv3: u8,
    pub e_status: u8,
    pub tc: u16,
    pub rsv4: [u8; 2],
}

impl FisPioSetup {
    #[inline]
    pub fn get(&self, mask: u8, shift: u32) -> u8 {
        (self.flags >> shift) & mask
    }

    #[inline]
    pub fn set(&mut self, mask: u8, shift: u32, v: u8) {
        self.flags = (self.flags & !(mask << shift)) | ((v & mask) << shift);
    }

    pub fn pmport(&self) -> u8 {
        self.get(0xF, 0)
    }

    pub fn set_pmport(&mut self, v: u8) {
        self.set(0xF, 0, v)
    }

    pub fn direction(&self) -> u8 {
        self.get(1, 5)
    }

    pub fn set_direction(&mut self, v: u8) {
        self.set(1, 5, v)
    }

    pub fn interrupt(&self) -> u8 {
        self.get(1, 6)
    }

    pub fn set_interrupt(&mut self, v: u8) {
        self.set(1, 6, v)
    }
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct FisDmaSetup {
    pub fis_type: u8, // FIS_TYPE_DMA_SETUP
    pub flags: u8,    // pmport:4, rsv0:1, d:1, i:1, a:1
    pub rsved: [u8; 2],
    pub dma_buffer_id_lo: u32,
    pub dma_buffer_id_hi: u32,
    pub rsvd: u32,
    pub dma_buf_offset: u32,
    pub transfer_count: u32,
    pub resvd: u32,
}

impl FisDmaSetup {
    #[inline]
    pub fn get(&self, mask: u8, shift: u32) -> u8 {
        (self.flags >> shift) & mask
    }

    #[inline]
    pub fn set(&mut self, mask: u8, shift: u32, v: u8) {
        self.flags = (self.flags & !(mask << shift)) | ((v & mask) << shift);
    }

    pub fn pmport(&self) -> u8 {
        self.get(0xF, 0)
    }

    pub fn set_pmport(&mut self, v: u8) {
        self.set(0xF, 0, v)
    }

    pub fn direction(&self) -> u8 {
        self.get(1, 5)
    }

    pub fn set_direction(&mut self, v: u8) {
        self.set(1, 5, v)
    }

    pub fn interrupt(&self) -> u8 {
        self.get(1, 6)
    }

    pub fn set_interrupt(&mut self, v: u8) {
        self.set(1, 6, v)
    }

    pub fn auto_activate(&self) -> u8 {
        self.get(1, 7)
    }

    pub fn set_auto_activate(&mut self, v: u8) {
        self.set(1, 7, v)
    }
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct FisDevBits {
    pub fis_type: u8, // FIS_TYPE_DEV_BITS
    pub flags: u8,    // pmport:4, rsv0:2, i:1, n:1
    pub status: u8,
    pub error: u8,
    pub s_active: u32,
}

impl FisDevBits {
    #[inline]
    pub fn get(&self, mask: u8, shift: u32) -> u8 {
        (self.flags >> shift) & mask
    }

    #[inline]
    pub fn set(&mut self, mask: u8, shift: u32, v: u8) {
        self.flags = (self.flags & !(mask << shift)) | ((v & mask) << shift);
    }

    pub fn pmport(&self) -> u8 {
        self.get(0xF, 0)
    }

    pub fn set_pmport(&mut self, v: u8) {
        self.set(0xF, 0, v)
    }

    pub fn interrupt(&self) -> u8 {
        self.get(1, 6)
    }

    pub fn set_interrupt(&mut self, v: u8) {
        self.set(1, 6, v)
    }

    pub fn notification(&self) -> u8 {
        self.get(1, 7)
    }

    pub fn set_notification(&mut self, v: u8) {
        self.set(1, 7, v)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct HbaFis {
    pub dsfis: FisDmaSetup,      // 0x00
    pub pad0: [u8; 4],           // 0x1C
    pub psfis: FisPioSetup,      // 0x20
    pub pad1: [u8; 12],          // 0x34
    pub rfis: FisRegD2h,         // 0x40
    pub pad2: [u8; 4],           // 0x54
    pub sdbfis: FisDevBits,      // 0x58
    pub ufis: [u8; 64],          // 0x60
    pub rsv: [u8; 0x100 - 0xA0], // 0xA0
}

impl Default for HbaFis {
    fn default() -> Self {
        Self {
            dsfis: FisDmaSetup::default(),
            pad0: [0; 4],
            psfis: FisPioSetup::default(),
            pad1: [0; 12],
            rfis: FisRegD2h::default(),
            pad2: [0; 4],
            sdbfis: FisDevBits::default(),
            ufis: [0; 64],
            rsv: [0; 0x100 - 0xA0],
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct HbaPrdtEntry {
    pub dba: u32,  // Data base address
    pub dbau: u32, // Data base address upper 32 bits
    pub rsv: u32,  // Reserved
    pub dbc: u32,  // bits 21:0 byte count (even), bit 31 interrupt
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct HbaCmdTbl {
    pub cfis: [u8; 64],                // 0x00 Command FIS
    pub acmd: [u8; 16],                // 0x40 ATAPI command, 12 or 16 bytes
    pub rsv: [u8; 48],                 // 0x50 Reserved
    pub prdt_entry: [HbaPrdtEntry; 1], // 0x80 Physical region descriptor table entries
}

impl Default for HbaCmdTbl {
    fn default() -> Self {
        Self {
            cfis: [0; 64],
            acmd: [0; 16],
            rsv: [0; 48],
            prdt_entry: [HbaPrdtEntry::default()],
        }
    }
}
