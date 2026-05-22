use core::sync::atomic::AtomicU64;

use crate::{
    arch::arch::serial_print,
    driver::io::virtio::{
        input::read_device_name,
        queue::setup_queue,
        transport::{VirtioMmio, VirtioMmioReg},
    },
    util::U64Buf,
};

pub mod input;
pub mod queue;
pub mod transport;

pub const VIRTIO_MMIO_BASE: u64 = 0xFFFF_0000_0a00_0000;
pub const VIRTIO_MMIO_STRIDE: u64 = 0x200;
pub const VIRTIO_MMIO_COUNT: u64 = 32;

const VIRTIO_MAGIC: u32 = 0x74726976;

const VIRTIO_DEVICE_INPUT: u32 = 0x12;
const VIRTIO_DEVICE_BLOCK: u32 = 0x02;

const DEVICE_ACKNOWLEDGE: u32 = 0x01;
const DEVICE_DRIVER: u32 = 0x02;
const DEVICE_FEATURES_OK: u32 = 0x08;
const DEVICE_DRIVER_OK: u32 = 0x04;
const DEVICE_FAILED: u32 = 0x80;

pub static KEYBOARD_SLOT: AtomicU64 = AtomicU64::new(u64::MAX);
pub static MOUSE_SLOT: AtomicU64 = AtomicU64::new(u64::MAX);

pub fn get_device(slot: u64) -> Option<VirtioMmio> {
    let base = VIRTIO_MMIO_BASE + slot * VIRTIO_MMIO_STRIDE;
    let device = VirtioMmio::new(base);

    if device.read(VirtioMmioReg::MagicValue) != VIRTIO_MAGIC {
        return None;
    }

    if device.read(VirtioMmioReg::Version) != 0x02 {
        return None;
    }

    let device_id = device.read(VirtioMmioReg::DeviceID);

    if device_id == 0 {
        return None;
    }

    Some(device)
}

pub fn scan_virtio_devices() {
    for i in 0..VIRTIO_MMIO_COUNT {
        let base = VIRTIO_MMIO_BASE + i * VIRTIO_MMIO_STRIDE;
        let device = VirtioMmio::new(base);

        if device.read(VirtioMmioReg::MagicValue) != VIRTIO_MAGIC {
            continue;
        }

        if device.read(VirtioMmioReg::Version) != 0x02 {
            continue;
        }

        let device_id = device.read(VirtioMmioReg::DeviceID);

        if device_id == 0 {
            continue;
        }

        serial_print("Slot #");
        serial_print(U64Buf::new(i).as_str());
        serial_print(" device_id=");
        serial_print(U64Buf::new(device.read(VirtioMmioReg::DeviceID) as u64).as_str());
        serial_print("\n");

        match device_id {
            VIRTIO_DEVICE_INPUT => {
                serial_print("Input device found!");
                serial_print("\n");
                if init_device(&device) {
                    serial_print("Successfully initialized device!\n");
                } else {
                    serial_print("Device initialization failed.\n");
                }

                let device_name = read_device_name(&device);

                serial_print("Found name of device: ");
                serial_print(device_name);
                serial_print("\n");

                serial_print("Setting up queue for device...\n");

                if read_device_name(&device).contains("QEMU Virtio Keyboard") {
                    KEYBOARD_SLOT.store(i, core::sync::atomic::Ordering::Relaxed);
                    setup_queue(&device, "kbd");
                } else if read_device_name(&device).contains("QEMU Virtio Mouse") {
                    MOUSE_SLOT.store(i, core::sync::atomic::Ordering::Relaxed);
                    setup_queue(&device, "mouse");
                } else {
                    serial_print("Could not recognize device.");
                    continue;
                }

                device.write(
                    VirtioMmioReg::Status,
                    DEVICE_ACKNOWLEDGE | DEVICE_DRIVER | DEVICE_FEATURES_OK | DEVICE_DRIVER_OK,
                );

                serial_print("Successfully set up queue for device and notified it.\n");
            }
            VIRTIO_DEVICE_BLOCK => {
                // TODO: add block device support
            }
            _ => {}
        }
    }
}

pub fn init_device(device: &VirtioMmio) -> bool {
    device.write(VirtioMmioReg::Status, 0); // reset
    device.write(VirtioMmioReg::Status, DEVICE_ACKNOWLEDGE);
    device.write(VirtioMmioReg::Status, DEVICE_ACKNOWLEDGE | DEVICE_DRIVER);
    device.write(VirtioMmioReg::DriverFeatures, 0); // no features for now
    device.write(
        VirtioMmioReg::Status,
        DEVICE_ACKNOWLEDGE | DEVICE_DRIVER | DEVICE_FEATURES_OK,
    );

    if device.read(VirtioMmioReg::Status) & DEVICE_FEATURES_OK == 0 {
        device.write(VirtioMmioReg::Status, DEVICE_FAILED);
        return false;
    }

    true
}
