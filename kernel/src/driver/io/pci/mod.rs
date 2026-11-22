use alloc::vec::Vec;
#[cfg(target_arch = "x86_64")]
use x86_64::instructions::port::Port;

const PCI_CONFIG_ADDR: u16 = 0xCF8;
const PCI_DATA_ADDR: u16 = 0xCFC;

#[derive(Debug, Clone, Copy)]
pub struct PCIBar {
    pub index: u8,
    pub base: u64,
    pub prefetchable: bool,
    pub size: u32,
    pub is_io: bool,
    pub is_64bit: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct PCIDevice {
    pub bus: u8,
    pub slot: u8,
    pub function: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class_code: u32,
    pub class: u32,
    pub subclass: u32,
    pub prog_if: u32,
    pub bars: [Option<PCIBar>; 6],
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

#[cfg(target_arch = "x86_64")]
pub fn write_pci_config(bus: u8, slot: u8, function: u8, offset: u8, data: u32) {
    let address = (1 << 31)
        | (bus as u32) << 16
        | (slot as u32) << 11
        | (function as u32) << 8
        | ((offset & 0xFC) as u32);

    let mut pci_config_port = Port::<u32>::new(PCI_CONFIG_ADDR);
    let mut pci_data_port = Port::<u32>::new(PCI_DATA_ADDR);

    unsafe {
        pci_config_port.write(address);
        pci_data_port.write(data);
    }
}

pub fn read_bar(bus: u8, slot: u8, function: u8, index: u8) -> Option<PCIBar> {
    let raw = read_pci_config(bus, slot, function, 0x10 + 4 * index);

    if raw == 0 || raw == 0xFFFF_FFFF {
        return None;
    };

    let bar_type = raw & 1;
    let prefetchable = ((raw >> 3) & 1) == 1;

    // I/O BAR
    if bar_type != 0 {
        return Some(PCIBar {
            index,
            base: (raw & !0x3) as u64,
            size: 0,
            is_io: true,
            is_64bit: false,
            prefetchable: false,
        });
    }

    return match (raw >> 1) & 0x3 {
        // 32 bit memory BAR
        0b00 => Some(PCIBar {
            index,
            base: (raw & !0xF) as u64,
            prefetchable,
            size: 0,
            is_io: false,
            is_64bit: false,
        }),
        // 64 bit memory BAR
        0b10 if index < 5 => {
            let upper = read_pci_config(bus, slot, function, 0x10 + 4 * (index + 1));

            Some(PCIBar {
                index,
                base: (raw & !0xF) as u64 | ((upper as u64) << 32), // combine the lower and upper half of the address
                prefetchable,
                size: 0,
                is_io: false,
                is_64bit: true,
            })
        }
        _ => None,
    };
}

pub fn pci_get_ids(bus: u8, slot: u8, function: u8) -> Option<(u16, u16)> {
    let ids = read_pci_config(bus, slot, function, 0);

    if (ids & 0xFFFF) == 0xFFFF {
        return None;
    }

    return Some(((ids & 0xFFFF) as u16, (ids >> 16) as u16));
}

pub fn enable_memory_and_bus_master(bus: u8, slot: u8, function: u8) {
    let cmd = (read_pci_config(bus, slot, function, 0x04) & 0xFFFF) as u16;
    write_pci_config(
        bus,
        slot,
        function,
        0x04,
        (cmd | (1 << 1) | (1 << 2)) as u32,
    );
}

pub fn discover_devices() -> Vec<PCIDevice> {
    let mut devices = Vec::<PCIDevice>::with_capacity(64);
    for bus in 0..=255 {
        for slot in 0..=31 {
            if pci_get_ids(bus, slot, 0).is_none() {
                continue;
            }

            let header0 = (read_pci_config(bus, slot, 0, 0x0C) >> 16) as u8;
            let functions = if header0 & 0x80 != 0 { 0..=7 } else { 0..=0 };

            for function in functions {
                if let Some((vendor_id, device_id)) = pci_get_ids(bus, slot, function) {
                    let class_code = read_pci_config(bus, slot, function, 0x08);
                    let header_type = (read_pci_config(bus, slot, function, 0x0C) >> 16) as u8;

                    let prog_if = (class_code >> 8) & 0xFF;
                    let subclass = (class_code >> 16) & 0xFF;
                    let class = (class_code >> 24) & 0xFF;

                    let bar_count = if header_type & 0x7F == 0x01 { 2 } else { 6 };
                    let mut bars = [None; 6];
                    let mut i = 0u8;
                    while i < bar_count {
                        bars[i as usize] = read_bar(bus, slot, function, i);
                        i += match bars[i as usize] {
                            Some(bar) if bar.is_64bit => 2,
                            _ => 1,
                        };
                    }

                    devices.push(PCIDevice {
                        bus,
                        slot,
                        function,
                        vendor_id,
                        device_id,
                        class_code,
                        class,
                        subclass,
                        prog_if,
                        bars,
                    });
                }
            }
        }
    }

    return devices;
}
