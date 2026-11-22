use crate::{
    driver::io::{
        block::{BlockDevice, BlockError},
        pci::{PCIDevice, enable_memory_and_bus_master},
    },
    println,
};

#[cfg(target_arch = "x86_64")]
use crate::arch::x86_64::paging::{MMIO_VIRT_BASE, map_mmio};
#[cfg(target_arch = "x86_64")]
use x86_64::structures::paging::OffsetPageTable;

use alloc::vec::Vec;

pub struct AHCIController {
    pub pci_device: PCIDevice,
}

impl AHCIController {}

pub struct SATADevice {}

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
pub fn discover_ahci(
    pci_devices: Vec<PCIDevice>,
    mapper: &mut OffsetPageTable,
) -> Vec<AHCIController> {
    let mut ahci_controllers = Vec::<AHCIController>::with_capacity(64);
    for device in pci_devices {
        if device.class == 0x01 && device.subclass == 0x06 {
            // SATA/AHCI devices
            if [0x01, 0x85, 0x00].contains(&device.prog_if) {
                println!("{:?}", device);

                if let Some(abar) = device.bars[5] {
                    enable_memory_and_bus_master(device.bus, device.slot, device.function);
                    map_mmio(abar.base, mapper, 0x2000); // AHCI size is always 0x2000

                    let cap =
                        unsafe { core::ptr::read_volatile((MMIO_VIRT_BASE + 0x00) as *const u32) };
                    let ghc =
                        unsafe { core::ptr::read_volatile((MMIO_VIRT_BASE + 0x04) as *const u32) };
                    let interrupt_status =
                        unsafe { core::ptr::read_volatile((MMIO_VIRT_BASE + 0x08) as *const u32) };
                    let ports_implemented =
                        unsafe { core::ptr::read_volatile((MMIO_VIRT_BASE + 0x0C) as *const u32) };
                    let version =
                        unsafe { core::ptr::read_volatile((MMIO_VIRT_BASE + 0x10) as *const u32) };

                    ahci_controllers.push(AHCIController { pci_device: device });
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
