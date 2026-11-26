use alloc::sync::Arc;

use crate::{
    driver::io::block::{
        BlockDevice, BlockError,
        ahci::{
            fis::{FIS_TYPE_REG_H2D, FisRegH2d, HbaCmdTbl},
            regs::AHCIPortRegs,
        },
    },
    println,
    util::wait_until,
};

// TODO: make this work on aarch64
#[cfg(target_arch = "aarch64")]
pub fn alloc_dma_frame() -> Option<(u64, *mut u8)> {
    None
}

#[cfg(target_arch = "x86_64")]
use crate::arch::x86_64::paging::alloc_dma_frame;

const ATA_CMD_IDENTIFY: u8 = 0xEC;
const HBA_PX_IS_TFES: u32 = 1 << 30;

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct SATACommandHeader {
    flags: u16,
    pub prdtl: u16,
    prdbc: u32,
    pub ctba: u32,
    pub ctbau: u32,
    pub rsv1: [u32; 4],
}

impl SATACommandHeader {
    #[inline]
    pub fn get(&self, mask: u16, shift: u32) -> u16 {
        (self.flags >> shift) & mask
    }
    #[inline]
    pub fn set(&mut self, mask: u16, shift: u32, v: u16) {
        self.flags = (self.flags & !(mask << shift)) | ((v & mask) << shift);
    }

    pub fn cfl(&self) -> u8 {
        self.get(0x1F, 0) as u8
    }

    pub fn set_cfl(&mut self, v: u8) {
        self.set(0x1F, 0, v as u16)
    }

    pub fn atapi(&self) -> u8 {
        self.get(1, 5) as u8
    }

    pub fn set_atapi(&mut self, v: u8) {
        self.set(1, 5, v as u16)
    }

    pub fn write(&self) -> u8 {
        self.get(1, 6) as u8
    }

    pub fn set_write(&mut self, v: u8) {
        self.set(1, 6, v as u16)
    }

    pub fn prefetchable(&self) -> u8 {
        self.get(1, 7) as u8
    }

    pub fn set_prefetchable(&mut self, v: u8) {
        self.set(1, 7, v as u16)
    }

    pub fn reset(&self) -> u8 {
        self.get(1, 8) as u8
    }

    pub fn set_reset(&mut self, v: u8) {
        self.set(1, 8, v as u16)
    }

    pub fn bist(&self) -> u8 {
        self.get(1, 9) as u8
    }

    pub fn set_bist(&mut self, v: u8) {
        self.set(1, 9, v as u16)
    }

    pub fn clear(&self) -> u8 {
        self.get(1, 10) as u8
    }

    pub fn set_clear(&mut self, v: u8) {
        self.set(1, 10, v as u16)
    }

    pub fn pmp(&self) -> u8 {
        self.get(0xF, 16) as u8
    }

    pub fn set_pmp(&mut self, v: u8) {
        self.set(0xF, 16, v as u16)
    }
}
pub struct SATADevice {
    pub port: Arc<AHCIPortRegs>,
    pub clb_base_virt: u64,
    pub fis_base_virt: u64,
    pub ctba_base_virt: [u64; 32],
    pub sector_size: u32,
    pub capacity_sectors: u64,
}

impl SATADevice {
    pub fn identify(&mut self) {
        while self.port.ci() & 1 == 1 {} // wait until CI bit 0 clears

        let slot = self.find_cmdslot();

        if slot == 255 {
            println!("Could not find slot for SATA command");
            return;
        }

        let mut fis = FisRegH2d::default();

        fis.fis_type = FIS_TYPE_REG_H2D;
        fis.command = ATA_CMD_IDENTIFY;
        fis.device = 0;
        fis.set_c(1);

        let (buf_phys, buf_virt) = alloc_dma_frame().unwrap_or((0, 0 as *mut u8));

        if buf_phys == 0 {
            println!("Could not allocate DMA buffer for IDENTIFY ATA command");
            return;
        }

        unsafe {
            let mut cmd_header =
                *((self.clb_base_virt as *mut SATACommandHeader).add(slot as usize));
            cmd_header.set_cfl(5);
            cmd_header.set_write(0);
            cmd_header.prdtl = 1;
            cmd_header.prdbc = 0; // hardware writes this back
            *((self.clb_base_virt as *mut SATACommandHeader).add(slot as usize)) = cmd_header;

            let cmdt = self.ctba_base_virt[slot as usize] as *mut HbaCmdTbl;
            core::ptr::write(cmdt as *mut FisRegH2d, fis);

            let prdt = &mut (*cmdt).prdt_entry[0];
            prdt.dba = buf_phys as u32;
            prdt.dbau = (buf_phys >> 32) as u32;
            prdt.rsv = 0;
            prdt.dbc = (512u32 - 1) | (1 << 31); // byte count-1, interrupt on completion
        }

        self.port.set_ci(1 << slot);

        let mut failed = false;

        let done = wait_until(
            || -> bool {
                if self.port.ci() & (1 << slot) == 0 {
                    return true;
                }

                if self.port.is() & HBA_PX_IS_TFES != 0 {
                    failed = true;
                    return true;
                }

                return false;
            },
            100_000,
        );

        if failed {
            println!("Task error returned for IDENTIFY ATA command");
            return;
        } else if !done {
            println!("Device failed to respond to IDENTIFY ATA command in time");
            return;
        }

        let words = unsafe { core::slice::from_raw_parts(buf_virt as *const u16, 256) };

        self.sector_size = if words[106] & (1 << 12) == 0 {
            512
        } else {
            (words[117] as u32 | ((words[118] as u32) << 16)) * 2
        };

        let lba48 = words[83] & (1 << 10) != 0;
        self.capacity_sectors = if lba48 {
            (words[100] as u64)
                | ((words[101] as u64) << 16)
                | ((words[102] as u64) << 32)
                | ((words[103] as u64) << 48)
        } else {
            (words[60] as u64) | ((words[61] as u64) << 16)
        };

        println!(
            "SATA port IDENTIFY successful sector_size={} capacity={}",
            self.sector_size, self.capacity_sectors
        );
    }

    pub fn find_cmdslot(&self) -> u8 {
        let mut slots = self.port.sact() | self.port.ci();

        for i in 0..32 {
            if slots & 1 == 0 {
                return i;
            }

            slots >>= 1
        }

        return 255;
    }

    pub fn stop_cmd_engine(&self) {
        self.port.set_cmd(self.port.cmd() & !0x0001); // clear ST
        self.port.set_cmd(self.port.cmd() & !0x0010); // clear FRE

        loop {
            let cmd = self.port.cmd();
            // wait until FR and CR clears
            if (cmd >> 14) & 1 == 1 || (cmd >> 15) & 1 == 1 {
                continue;
            }

            break;
        }
    }

    pub fn start_cmd_engine(&self) {
        while (self.port.cmd() >> 15) & 1 == 1 {} // wait until CR clears

        self.port.set_cmd(self.port.cmd() | 0x0010); // set FRE
        self.port.set_cmd(self.port.cmd() | 0x0001); // set ST
    }
}

impl BlockDevice for SATADevice {
    fn read_sectors(
        &self,
        _sector: u64,
        _count: u32,
        _buffer: &mut [u8],
    ) -> Result<(), BlockError> {
        unimplemented!()
    }
    fn write_sectors(&self, _sector: u64, _count: u32, _buffer: &[u8]) -> Result<(), BlockError> {
        unimplemented!()
    }
    fn sector_size(&self) -> u32 {
        self.sector_size
    }
    fn capacity_sectors(&self) -> u64 {
        self.capacity_sectors
    }
}
