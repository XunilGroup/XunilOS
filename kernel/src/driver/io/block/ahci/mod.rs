#[cfg(target_arch = "x86_64")]
use x86_64::instructions::port::Port;

#[cfg(target_arch = "x86_64")]
use crate::println;

const PCI_CONFIG_ADDR: u16 = 0xCF8;
const PCI_DATA_ADDR: u16 = 0xCFC;

#[cfg(target_arch = "x86_64")]
pub fn discover_drives() {
    for bus in 0..=255 {
        for device in 0..=31 {
            let vendor_id = read_pci_config(bus, device, 0, 0x00);

            if vendor_id == 0xFFFF {
                continue;
            }

            let class_code = read_pci_config(bus, device, 0, 0x08);
            let prog_if = (class_code >> 8) & 0xFF;
            let subclass = (class_code >> 16) & 0xFF;
            let class = (class_code >> 24) & 0xFF;
            if class == 0x01 && subclass == 0x06 {
                // SATA/AHCI devices
                if [0x01, 0x85, 0x00].contains(&prog_if) {
                    println!("AHCI controller found at bus {} device {}", bus, device);
                }
            }
        }
    }
}

#[cfg(target_arch = "x86_64")]
fn read_pci_config(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    let address = (1 << 31)
        | (bus as u32) << 16
        | (device as u32) << 11
        | (function as u32) << 8
        | ((offset & 0xFC) as u32);

    let mut pci_config_port = Port::<u32>::new(PCI_CONFIG_ADDR);
    let mut pci_data_port = Port::<u32>::new(PCI_DATA_ADDR);

    unsafe {
        pci_config_port.write(address);
        pci_data_port.read()
    }
}
