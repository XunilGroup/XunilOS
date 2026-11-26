use alloc::sync::Arc;

use crate::{
    arch::arch::PageTable,
    driver::io::{
        block::ahci::{
            device::{SATACommandHeader, SATADevice},
            regs::{AHCICap, AHCIPortRegs, AHCIRegs},
        },
        pci::{PCIBar, PCIDevice, enable_memory_and_bus_master},
    },
    println,
    util::wait_until,
};

// TODO: make this work on aarch64
#[cfg(target_arch = "aarch64")]
pub fn map_mmio(base: u64, mapper: &mut PageTable, size: u64) {}

// TODO: make this work on aarch64
#[cfg(target_arch = "aarch64")]
pub fn alloc_dma_frame() -> Option<(u64, *mut u8)> {
    None
}

#[cfg(target_arch = "x86_64")]
use crate::arch::x86_64::paging::{alloc_dma_frame, map_mmio};

const SATA_SIG_ATAPI: u32 = 0xEB140101; // SATAPI drive
const SATA_SIG_SEMB: u32 = 0xC33C0101; // Enclosure management bridge
const SATA_SIG_PM: u32 = 0x96690101; // Port multiplier

const AHCI_DEV_NULL: u32 = 0;
const AHCI_DEV_SATA: u32 = 1;
const AHCI_DEV_SEMB: u32 = 2;
const AHCI_DEV_PM: u32 = 3;
const AHCI_DEV_SATAPI: u32 = 4;

const HBA_PORT_IPM_ACTIVE: u32 = 1;
const HBA_PORT_DET_PRESENT: u32 = 3;

pub struct AHCIController {
    pub regs: AHCIRegs,
    pub abar: PCIBar,
    pub pci_device: PCIDevice,
    pub sata_devices: [Option<SATADevice>; 32],
    pub sata_index: usize,
    pub enabled: bool,
}

impl AHCIController {
    pub fn initialize(&mut self, mapper: &mut PageTable) {
        enable_memory_and_bus_master(
            self.pci_device.bus,
            self.pci_device.slot,
            self.pci_device.function,
        );
        map_mmio(self.abar.base, mapper, 0x2000); // AHCI size is always 0x2000

        // set AHCI enable at most 5 times because some controllers need it?

        let mut ahci_enabled = false;
        for _ in 0..5 {
            if self.regs.ghc() & (1 << 31) != 0 {
                ahci_enabled = true;
                break;
            }
            self.regs.set_ghc(self.regs.ghc() | (1 << 31));
            if wait_until(|| self.regs.ghc() & (1 << 31) != 0, 1_000) {
                ahci_enabled = true;
                break;
            }
        }

        if !ahci_enabled {
            println!(
                "Failed to initialize AHCI Controller for PCI Device: {:?}",
                self.pci_device
            );
            return;
        }

        self.regs.set_ghc(self.regs.ghc() | 1); // HBA Reset

        if !wait_until(|| self.regs.ghc() & 1 == 0, 100_000) {
            println!(
                "Failed to initialize AHCI Controller for PCI Device: {:?}",
                self.pci_device
            );
            return;
        }

        self.regs.set_ghc(self.regs.ghc() | (1 << 31)); // set AHCI enable again (some controllers reset its value)

        if !wait_until(|| (self.regs.ghc() >> 31) & 1 != 0, 100_000) {
            println!(
                "Failed to initialize AHCI Controller for PCI Device: {:?}",
                self.pci_device
            );
            return;
        }

        self.enabled = true;

        let mut pi = self.regs.pi();

        for i in 0..32 {
            if (pi & 1) == 1 {
                match self.check_port_type(&self.regs.port(i)) {
                    AHCI_DEV_SATA => {
                        println!("SATA drive found at port {}", i);

                        self.initialize_sata_port(&self.regs.port(i));
                    }
                    AHCI_DEV_SATAPI => {
                        println!("Unsupported SATAPI drive found at port {}", i);
                    }
                    AHCI_DEV_SEMB => {
                        println!("Unsupported SEMB drive found at port {}", i);
                    }
                    AHCI_DEV_PM => {
                        println!("Unsupported PM drive found at port {}", i);
                    }
                    _ => {}
                }
            }

            pi >>= 1
        }
    }

    pub fn initialize_sata_port(&mut self, port: &AHCIPortRegs) {
        let (clb_base_phys, clb_base_virt) = alloc_dma_frame().unwrap_or((0, 0 as *mut u8));
        let (fis_base_phys, fis_base_virt) = alloc_dma_frame().unwrap_or((0, 0 as *mut u8));

        if clb_base_phys == 0 || fis_base_phys == 0 {
            println!("DMA alloc failed for port");
            return;
        }

        let mut sata_device = SATADevice {
            port: Arc::new(port.clone()),
            clb_base_virt: clb_base_virt as u64,
            fis_base_virt: fis_base_virt as u64,
            sector_size: 0,
            capacity_sectors: 0,
            ctba_base_virt: [0; 32],
        };

        sata_device.stop_cmd_engine();

        port.set_clb(clb_base_phys as u32);
        port.set_clbu((clb_base_phys >> 32) as u32);

        port.set_fb(fis_base_phys as u32);
        port.set_fbu((fis_base_phys >> 32) as u32);

        // clear sata error and interrupt status before starting cmd engine
        port.set_serr(0xFFFF_FFFF);
        port.set_is(0xFFFF_FFFF);

        for i in 0..32 {
            let (ctb_base_phys, ctb_base_virt) = alloc_dma_frame().unwrap_or((0, 0 as *mut u8));

            if ctb_base_phys == 0 {
                return;
            }

            let mut cmd_header = SATACommandHeader::default();
            cmd_header.ctba = ctb_base_phys as u32;
            cmd_header.ctbau = (ctb_base_phys >> 32) as u32;
            unsafe { *((clb_base_virt as *mut SATACommandHeader).add(i as usize)) = cmd_header }

            sata_device.ctba_base_virt[i] = ctb_base_virt as u64;
        }

        sata_device.start_cmd_engine();

        sata_device.identify();

        self.sata_devices[self.sata_index] = Some(sata_device);
        self.sata_index += 1;
    }

    pub fn check_port_type(&self, port: &AHCIPortRegs) -> u32 {
        let ssts = port.ssts();

        let ipm = (ssts >> 8) & 0x0F;
        let det = ssts & 0x0F;

        if det != HBA_PORT_DET_PRESENT {
            return AHCI_DEV_NULL;
        }

        if ipm != HBA_PORT_IPM_ACTIVE {
            return AHCI_DEV_NULL;
        }

        match port.sig() {
            SATA_SIG_ATAPI => AHCI_DEV_SATAPI,
            SATA_SIG_SEMB => AHCI_DEV_SEMB,
            SATA_SIG_PM => AHCI_DEV_PM,
            _ => AHCI_DEV_SATA,
        }
    }

    pub fn get_info(&self) -> (AHCICap, u32, u32, u32, u32) {
        return (
            self.regs.cap(),
            self.regs.ghc(),
            self.regs.is(),
            self.regs.pi(),
            self.regs.vs(),
        );
    }
}
