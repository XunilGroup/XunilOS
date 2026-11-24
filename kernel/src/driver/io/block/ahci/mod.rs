use core::hint::spin_loop;

use crate::{
    driver::io::{
        block::{BlockDevice, BlockError},
        pci::{PCIBar, PCIDevice, enable_memory_and_bus_master},
    },
    println,
    util::wait_until,
};

#[cfg(target_arch = "x86_64")]
use crate::arch::x86_64::paging::{MMIO_VIRT_BASE, map_mmio};
#[cfg(target_arch = "x86_64")]
use x86_64::structures::paging::OffsetPageTable;

#[cfg(target_arch = "aarch64")]
use crate::arch::aarch64::paging::AArchPageTable;

use alloc::vec::Vec;

#[cfg(target_arch = "aarch64")]
type PageTable = AArchPageTable;

#[cfg(target_arch = "aarch64")]
pub fn map_mmio(base: u64, mapper: &mut AArchPageTable, size: u64) {}

#[cfg(target_arch = "x86_64")]
type PageTable<'a> = OffsetPageTable<'a>;

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

#[derive(Clone)]
pub struct AHCIPortRegs {
    pub base: u64,
}

impl AHCIPortRegs {
    pub fn read(&self, offset: u64) -> u32 {
        return unsafe { core::ptr::read_volatile((self.base + offset) as *const u32) };
    }

    pub fn write(&self, offset: u64, data: u32) {
        unsafe { core::ptr::write_volatile((self.base + offset) as *mut u32, data) };
    }

    pub fn is(&self) -> u32 {
        return self.read(0x10);
    }

    pub fn ie(&self) -> u32 {
        return self.read(0x14);
    }

    pub fn cmd(&self) -> u32 {
        return self.read(0x18);
    }

    pub fn sig(&self) -> u32 {
        return self.read(0x24);
    }

    pub fn ssts(&self) -> u32 {
        return self.read(0x28);
    }
}

pub struct AHCIRegs {
    pub base: u64,
}

impl AHCIRegs {
    pub fn read(&self, offset: u64) -> u32 {
        return unsafe { core::ptr::read_volatile((self.base + offset) as *const u32) };
    }

    pub fn write(&self, offset: u64, data: u32) {
        unsafe { core::ptr::write_volatile((self.base + offset) as *mut u32, data) };
    }

    pub fn cap(&self) -> u32 {
        return self.read(0x00);
    }

    pub fn is(&self) -> u32 {
        return self.read(0x08);
    }

    pub fn ghc(&self) -> u32 {
        return self.read(0x04);
    }

    pub fn set_ghc(&self, value: u32) {
        return self.write(0x04, value);
    }

    pub fn pi(&self) -> u32 {
        return self.read(0x0C);
    }

    pub fn vs(&self) -> u32 {
        return self.read(0x10);
    }

    pub fn port(&self, index: u64) -> AHCIPortRegs {
        return AHCIPortRegs {
            base: self.base + 0x100 + index * 0x80,
        };
    }
}

pub struct AHCIController {
    pub regs: AHCIRegs,
    pub abar: PCIBar,
    pub pci_device: PCIDevice,
    pub sata_devices: [Option<SATADevice>; 6],
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
        let sata_device = SATADevice { port: port.clone() };

        self.sata_devices[self.sata_index] = Some(sata_device);
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

    pub fn get_info(&self) -> (u32, u32, u32, u32, u32) {
        return (
            self.regs.cap(),
            self.regs.ghc(),
            self.regs.is(),
            self.regs.pi(),
            self.regs.vs(),
        );
    }
}

pub struct SATADevice {
    port: AHCIPortRegs,
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
        unimplemented!()
    }
    fn capacity_sectors(&self) -> u32 {
        unimplemented!()
    }
}

#[cfg(target_arch = "x86_64")]
pub fn discover_ahci(pci_devices: Vec<PCIDevice>, mapper: &mut PageTable) -> Vec<AHCIController> {
    let mut ahci_controllers = Vec::<AHCIController>::with_capacity(64);
    for device in pci_devices {
        if device.class == 0x01 && device.subclass == 0x06 {
            // SATA/AHCI devices
            if [0x01, 0x85, 0x00].contains(&device.prog_if) {
                println!("{:?}", device);

                if let Some(abar) = device.bars[5] {
                    let mut ahci_controller = AHCIController {
                        enabled: false,
                        sata_index: 0,
                        regs: AHCIRegs {
                            base: MMIO_VIRT_BASE + abar.base,
                        },
                        abar,
                        pci_device: device,
                        sata_devices: [const { None }; 6],
                    };

                    ahci_controller.initialize(mapper);

                    let (cap, ghc, interrupt_status, ports_implemented, version) =
                        ahci_controller.get_info();

                    ahci_controllers.push(ahci_controller);

                    println!(
                        "AHCI controller found at bus {} slot {}: CAP={} GHC={} IS={} PI={} VS={}",
                        device.bus,
                        device.slot,
                        cap,
                        ghc,
                        interrupt_status,
                        ports_implemented,
                        version
                    );
                }
            }
        }
    }

    ahci_controllers
}
