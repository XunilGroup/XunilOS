use crate::driver::io::pci::PCIDevice;
use crate::println;
use alloc::vec::Vec;

pub struct AHCIController {
    pub pci_device: PCIDevice,
}

#[cfg(target_arch = "x86_64")]
pub fn discover_ahci(pci_devices: Vec<PCIDevice>) -> Vec<AHCIController> {
    let mut ahci_controllers = Vec::<AHCIController>::with_capacity(64);
    for device in pci_devices {
        println!("{:?}", device);
        if device.class == 0x01 && device.subclass == 0x06 {
            // SATA/AHCI devices
            if [0x01, 0x85, 0x00].contains(&device.prog_if) {
                ahci_controllers.push(AHCIController { pci_device: device });
                println!(
                    "AHCI controller found at bus {} slot {}",
                    device.bus, device.slot
                );
            }
        }
    }

    ahci_controllers
}
