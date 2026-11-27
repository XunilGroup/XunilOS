use alloc::{sync::Arc, vec::Vec};

use crate::{
    driver::io::block::{
        BlockDevice, BlockError,
        ahci::{
            fis::{FIS_TYPE_REG_H2D, FisRegH2d, HBA_CMD_TBL_PRDT_MAX, HbaCmdTbl},
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

// TODO: make this work on aarch64
#[cfg(target_arch = "aarch64")]
pub fn dealloc_dma_frame(phys: u64) {}

#[cfg(target_arch = "x86_64")]
use crate::arch::x86_64::paging::{alloc_dma_frame, dealloc_dma_frame};

const ATA_CMD_IDENTIFY: u8 = 0xEC;
const ATA_CMD_READ_DMA_EX: u8 = 0x25;
const ATA_CMD_WRITE_DMA_EX: u8 = 0x35;

const HBA_PX_IS_TFES: u32 = 1 << 30;
const ATA_DEV_BUSY: u32 = 0x80;
const ATA_DEV_DRQ: u32 = 0x08;

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

#[derive(Clone)]
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
    fn read_sectors(&self, sector: u64, count: u32, buffer: &mut [u8]) -> Result<(), BlockError> {
        let ss = self.sector_size as usize;
        let total = count as usize * ss;
        if buffer.len() < total {
            println!(
                "buffer is less than total needed len={} total={}",
                buffer.len(),
                total
            );
            return Err(BlockError::InvalidSector);
        }

        let frame_count = (total + 4095) / 4096;
        let cmd_count = (frame_count + (HBA_CMD_TBL_PRDT_MAX - 1)) / HBA_CMD_TBL_PRDT_MAX;

        let mut frames = Vec::with_capacity(frame_count);
        for _ in 0..frame_count {
            let (phys, virt) = alloc_dma_frame().ok_or(BlockError::SystemError)?;
            frames.push((phys, virt));
        }

        for cmd_n in 0..cmd_count {
            while self.port.ci() & 1 == 1 {} // wait until CI bit 0 clears

            let slot = self.find_cmdslot();

            if slot == 255 {
                println!("Could not find slot for SATA command");
                return Err(BlockError::IOTimeout);
            }

            let startl = sector as u32;
            let starth = (sector >> 32) as u32;

            let mut fis = FisRegH2d::default();

            fis.fis_type = FIS_TYPE_REG_H2D;
            fis.command = ATA_CMD_READ_DMA_EX;
            fis.set_c(1);
            fis.device = 1 << 6; // LBA mode

            fis.lba0 = startl as u8;
            fis.lba1 = (startl >> 8) as u8;
            fis.lba2 = (startl >> 16) as u8;

            fis.lba3 = starth as u8;
            fis.lba4 = (starth >> 8) as u8;
            fis.lba5 = (starth >> 16) as u8;

            fis.countl = (count & 0xFF) as u8;
            fis.counth = ((count >> 8) & 0xFF) as u8;

            unsafe {
                let mut cmd_header =
                    *((self.clb_base_virt as *mut SATACommandHeader).add(slot as usize));
                cmd_header.set_cfl((size_of::<FisRegH2d>() / size_of::<u32>()) as u8);
                cmd_header.set_write(0);
                cmd_header.prdtl = core::cmp::min(HBA_CMD_TBL_PRDT_MAX as u16, frame_count as u16);

                *((self.clb_base_virt as *mut SATACommandHeader).add(slot as usize)) = cmd_header;

                let cmdt = self.ctba_base_virt[slot as usize] as *mut HbaCmdTbl;
                core::ptr::write(cmdt as *mut FisRegH2d, fis);

                let command_prdtl_base = cmd_n
                    * (core::cmp::min(HBA_CMD_TBL_PRDT_MAX as u16, frame_count as u16) as usize);

                for i in command_prdtl_base..command_prdtl_base + (cmd_header.prdtl as usize) {
                    let (buf_phys, _) = frames.get_unchecked(i);

                    let prdt = &mut (*cmdt).prdt_entry[i - command_prdtl_base];
                    prdt.dba = buf_phys.clone() as u32;
                    prdt.dbau = (buf_phys.clone() >> 32) as u32;
                    prdt.dbc = 4096 - 1;
                }
            }

            if !wait_until(
                || self.port.tfd() & (ATA_DEV_BUSY | ATA_DEV_DRQ) == 0,
                100_000,
            ) {
                return Err(BlockError::IOTimeout);
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
                return Err(BlockError::ReadError);
            } else if !done {
                println!("Device failed to respond to IDENTIFY ATA command in time");
                return Err(BlockError::ReadError);
            };

            // check again
            if self.port.is() & HBA_PX_IS_TFES != 0 {
                println!("Task error returned for IDENTIFY ATA command");
                return Err(BlockError::ReadError);
            }
        }

        let mut copied = 0;

        for (phys, virt) in frames {
            let n = core::cmp::min(4096, total - copied);
            unsafe {
                core::ptr::copy_nonoverlapping(virt, buffer.as_mut_ptr().add(copied), n);
            }
            copied += n;

            dealloc_dma_frame(phys); // do not the memory leak
        }

        return Ok(());
    }

    fn write_sectors(&self, sector: u64, count: u32, buffer: &[u8]) -> Result<(), BlockError> {
        let ss = self.sector_size as usize;
        let total = count as usize * ss;
        if buffer.len() < total {
            return Err(BlockError::InvalidSector);
        }

        let frame_count = (total + 4095) / 4096;
        let cmd_count = (frame_count + (HBA_CMD_TBL_PRDT_MAX - 1)) / HBA_CMD_TBL_PRDT_MAX;

        let mut frames = Vec::with_capacity(frame_count);
        for _ in 0..frame_count {
            let (phys, virt) = alloc_dma_frame().ok_or(BlockError::SystemError)?;
            frames.push((phys, virt));
        }

        let mut copied = 0;

        for n in 0..frame_count {
            unsafe {
                let frame = frames.get_unchecked_mut(n);
                core::ptr::copy_nonoverlapping(buffer.as_ptr().add(copied), frame.1, 4096);
            }
            copied += 4096;
        }

        for cmd_n in 0..cmd_count {
            while self.port.ci() & 1 == 1 {} // wait until CI bit 0 clears

            let slot = self.find_cmdslot();

            if slot == 255 {
                println!("Could not find slot for SATA command");
                return Err(BlockError::IOTimeout);
            }

            let startl = sector as u32;
            let starth = (sector >> 32) as u32;

            let mut fis = FisRegH2d::default();

            fis.fis_type = FIS_TYPE_REG_H2D;
            fis.command = ATA_CMD_WRITE_DMA_EX;
            fis.set_c(1);
            fis.device = 1 << 6; // LBA mode

            fis.lba0 = startl as u8;
            fis.lba1 = (startl >> 8) as u8;
            fis.lba2 = (startl >> 16) as u8;

            fis.lba3 = starth as u8;
            fis.lba4 = (starth >> 8) as u8;
            fis.lba5 = (starth >> 16) as u8;

            fis.countl = (count & 0xFF) as u8;
            fis.countl = ((count >> 8) & 0xFF) as u8;

            unsafe {
                let mut cmd_header =
                    *((self.clb_base_virt as *mut SATACommandHeader).add(slot as usize));
                cmd_header.set_cfl((size_of::<FisRegH2d>() / size_of::<u32>()) as u8);
                cmd_header.set_write(1);
                cmd_header.prdtl = core::cmp::min(HBA_CMD_TBL_PRDT_MAX as u16, frame_count as u16);

                *((self.clb_base_virt as *mut SATACommandHeader).add(slot as usize)) = cmd_header;

                let cmdt = self.ctba_base_virt[slot as usize] as *mut HbaCmdTbl;
                core::ptr::write(cmdt as *mut FisRegH2d, fis);

                let command_prdtl_base = cmd_n
                    * (core::cmp::min(HBA_CMD_TBL_PRDT_MAX as u16, frame_count as u16) as usize);

                for i in command_prdtl_base..command_prdtl_base + (cmd_header.prdtl as usize) {
                    let (buf_phys, _) = frames.get_unchecked(i);

                    let prdt = &mut (*cmdt).prdt_entry[i - command_prdtl_base];
                    prdt.dba = buf_phys.clone() as u32;
                    prdt.dbau = (buf_phys.clone() >> 32) as u32;
                    prdt.dbc = 4096 - 1;
                }
            }

            if !wait_until(
                || self.port.tfd() & (ATA_DEV_BUSY | ATA_DEV_DRQ) == 0,
                100_000,
            ) {
                return Err(BlockError::IOTimeout);
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
                return Err(BlockError::ReadError);
            } else if !done {
                println!("Device failed to respond to IDENTIFY ATA command in time");
                return Err(BlockError::ReadError);
            };

            // check again
            if self.port.is() & HBA_PX_IS_TFES != 0 {
                println!("Task error returned for IDENTIFY ATA command");
                return Err(BlockError::ReadError);
            }
        }

        return Ok(());
    }
    fn sector_size(&self) -> u32 {
        self.sector_size
    }
    fn capacity_sectors(&self) -> u64 {
        self.capacity_sectors
    }
}
