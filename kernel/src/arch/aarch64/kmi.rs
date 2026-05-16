use crate::arch::arch::safe_lock;

const KMI0_BASE: u64 = 0xFFFF_0000_0905_0000;
const KMI1_BASE: u64 = 0xFFFF_0000_0906_0000;
const KMI_CR: u64 = 0x00;
const KMI_STAT: u64 = 0x04;
const KMI_DATA: u64 = 0x08;

pub unsafe fn kmi_write(base: u64, kmi_type: u64, val: u8) {
    let data = (base + kmi_type) as *mut u32;
    unsafe { data.write_volatile(val as u32) };
}

pub fn kmi_read(base: u64, kmi_type: u64) -> u8 {
    let data = (base + kmi_type) as *const u32;
    unsafe { data.read_volatile() as u8 }
}

pub unsafe fn read_mouse_control() -> u8 {
    kmi_read(KMI1_BASE, KMI_STAT)
}

pub unsafe fn read_mouse_data() -> u8 {
    kmi_read(KMI1_BASE, KMI_DATA)
}

pub unsafe fn read_keyboard_control() -> u8 {
    kmi_read(KMI0_BASE, KMI_STAT)
}
pub unsafe fn read_keyboard_data() -> u8 {
    kmi_read(KMI0_BASE, KMI_DATA)
}

pub unsafe fn setup_kmi() -> u8 {
    safe_lock(|| unsafe {
        let cr0 = (KMI0_BASE + KMI_CR) as *mut u32;
        cr0.write_volatile(0x14); // enable keyboard

        kmi_write(KMI0_BASE, KMI_DATA, 0xFF); // reset keyboard

        if kmi_read(KMI0_BASE, KMI_DATA) != 0xFA {
            // ACK
            return 2;
        }

        if kmi_read(KMI0_BASE, KMI_DATA) != 0xAA {
            // self-test passed
            return 3;
        }

        kmi_write(KMI0_BASE, KMI_DATA, 0xF4); // enable keyboard data reporting

        if kmi_read(KMI0_BASE, KMI_DATA) != 0xFA {
            // ACK
            return 4;
        }

        let cr1 = (KMI1_BASE + KMI_CR) as *mut u32;
        cr1.write_volatile(0x14); // enable mouse

        kmi_write(KMI1_BASE, KMI_DATA, 0xFF); // reset mouse

        if kmi_read(KMI1_BASE, KMI_DATA) != 0xFA {
            // ACK
            return 5;
        }

        if kmi_read(KMI1_BASE, KMI_DATA) != 0xAA {
            // self-test passed
            return 6;
        }

        if kmi_read(KMI1_BASE, KMI_DATA) != 0x00 {
            // mouse ID
            return 7;
        }

        kmi_write(KMI1_BASE, KMI_DATA, 0xF4); // enable mouse data reporting

        if kmi_read(KMI1_BASE, KMI_DATA) != 0xFA {
            // ACK
            return 8;
        }

        return 9;
    })
}
