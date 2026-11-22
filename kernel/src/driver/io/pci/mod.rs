use alloc::vec::Vec;
#[cfg(target_arch = "x86_64")]
use x86_64::instructions::port::Port;

const PCI_CONFIG_ADDR: u16 = 0xCF8;
const PCI_DATA_ADDR: u16 = 0xCFC;

#[derive(Debug, Clone, Copy)]
pub struct PCIDevice {
    pub bus: u8,
    pub slot: u8,
    pub function: u8,
    pub vendor_id: u32,
    pub class_code: u32,
    pub class: u32,
    pub subclass: u32,
    pub prog_if: u32,
}

#[cfg(target_arch = "x86_64")]
pub fn read_pci_config(bus: u8, slot: u8, function: u8, offset: u8) -> u32 {
    let address = (1 << 31)
        | (bus as u32) << 16
        | (slot as u32) << 11
        | (function as u32) << 8
        | ((offset & 0xFC) as u32);

    let mut pci_config_port = Port::<u32>::new(PCI_CONFIG_ADDR);
    let mut pci_data_port = Port::<u32>::new(PCI_DATA_ADDR);

    unsafe {
        pci_config_port.write(address);
        pci_data_port.read()
    }
}

pub fn pci_get_vendor(bus: u8, slot: u8, function: u8) -> Option<u32> {
    let vendor_id = read_pci_config(bus, slot, function, 0);

    if (vendor_id & 0xFFFF) == 0xFFFF {
        return None;
    }

    return Some(vendor_id);
}

pub fn discover_devices() -> Vec<PCIDevice> {
    let mut devices = Vec::<PCIDevice>::with_capacity(64);
    for bus in 0..=255 {
        for slot in 0..=31 {
            for function in 0..8 {
                if let Some(vendor_id) = pci_get_vendor(bus, slot, function) {
                    let class_code = read_pci_config(bus, slot, function, 0x08);
                    let prog_if = (class_code >> 8) & 0xFF;
                    let subclass = (class_code >> 16) & 0xFF;
                    let class = (class_code >> 24) & 0xFF;

                    if subclass & 0x7F == 0x7F {
                        continue; // phantom function
                    }

                    devices.push(PCIDevice {
                        bus,
                        slot,
                        function,
                        vendor_id,
                        class_code,
                        class,
                        subclass,
                        prog_if,
                    });

                    if function == 0 && subclass & 0x80 == 0 {
                        break; // single-function slot
                    }
                }
            }
        }
    }

    return devices;
}
