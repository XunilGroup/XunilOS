pub mod controller;
pub mod device;
pub mod fis;
pub mod regs;

use crate::{
    arch::arch::PageTable,
    driver::io::{
        block::ahci::{controller::AHCIController, regs::AHCIRegs},
        pci::PCIDevice,
    },
    println,
};

#[cfg(target_arch = "x86_64")]
use crate::arch::x86_64::paging::MMIO_VIRT_BASE;

use alloc::vec::Vec;

pub fn discover_ahci(pci_devices: Vec<PCIDevice>, mapper: &mut PageTable) -> Vec<AHCIController> {
    let mut ahci_controllers = Vec::<AHCIController>::with_capacity(64);
    for device in pci_devices {
        if device.class == 0x01 && device.subclass == 0x06 {
            // SATA/AHCI devices
            if [0x01, 0x85, 0x00].contains(&device.prog_if) {
                if let Some(abar) = device.bars[5] {
                    #[cfg(target_arch = "x86_64")]
                    let mmio_virt_base = MMIO_VIRT_BASE;

                    // TODO: make this work on aarch64
                    #[cfg(target_arch = "aarch64")]
                    let mmio_virt_base = 0;

                    let mut ahci_controller = AHCIController {
                        enabled: false,
                        sata_index: 0,
                        regs: AHCIRegs {
                            base: mmio_virt_base + abar.base,
                        },
                        abar,
                        pci_device: device,
                        sata_devices: [const { None }; 32],
                    };

                    ahci_controller.initialize(mapper);

                    let (cap, ghc, interrupt_status, ports_implemented, version) =
                        ahci_controller.get_info();

                    ahci_controllers.push(ahci_controller);

                    println!(
                        "AHCI controller found at bus {} slot {}: CAP={:?} GHC={} IS={} PI={} VS={}",
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
