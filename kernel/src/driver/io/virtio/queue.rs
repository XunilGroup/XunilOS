use core::{mem::offset_of, sync::atomic::Ordering};

use crate::{
    KERNEL_PHYS_BASE, KERNEL_VIRT_BASE,
    driver::io::virtio::{
        input::{VIRTIO_KEYBOARD_QUEUE, VIRTIO_MOUSE_QUEUE, VirtioInputEvent},
        transport::{VirtioMmio, VirtioMmioReg},
    },
};

const VIRTQ_DESC_F_WRITE: u16 = 0x02;
pub const QUEUE_SIZE: usize = 64;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VirtQDesc {
    pub addr: u64,
    pub len: u32,
    pub flags: u16,
    pub next: u16,
}

impl VirtQDesc {
    pub const fn writable() -> Self {
        Self {
            addr: 0,
            len: 0,
            flags: VIRTQ_DESC_F_WRITE,
            next: 0,
        }
    }
}

#[repr(C)]
pub struct VirtQAvail {
    pub flags: u16,
    pub idx: u16,
    pub ring: [u16; QUEUE_SIZE],
}

#[repr(C)]
pub struct VirtQUsedElem {
    pub id: u32,
    pub len: u32,
}

#[repr(C)]
pub struct VirtQUsed {
    pub flags: u16,
    pub idx: u16,
    pub ring: [VirtQUsedElem; QUEUE_SIZE],
}

#[repr(C, align(16))]
pub struct VirtqueueMem {
    pub desc: [VirtQDesc; QUEUE_SIZE],
    pub avail: VirtQAvail,
    pub used: VirtQUsed,
    pub buffers: [VirtioInputEvent; QUEUE_SIZE],
    pub last_used_idx: u16,
}

pub fn fill_descriptors(queue: &mut VirtqueueMem) {
    let kernel_virt_base = KERNEL_VIRT_BASE.load(core::sync::atomic::Ordering::Relaxed);
    let kernel_phys_base = KERNEL_PHYS_BASE.load(core::sync::atomic::Ordering::Relaxed);
    for i in 0..QUEUE_SIZE {
        let virt_addr = &queue.buffers[i] as *const VirtioInputEvent as u64;
        let phys_addr = virt_addr - kernel_virt_base + kernel_phys_base;
        queue.desc[i].addr = phys_addr;
        queue.desc[i].len = size_of::<VirtioInputEvent>() as u32;
        queue.desc[i].flags = VIRTQ_DESC_F_WRITE;
        queue.avail.ring[i] = i as u16;
    }
    queue.avail.idx = QUEUE_SIZE as u16;
}
pub fn setup_queue(device: &VirtioMmio, device_type: &str) {
    #[allow(static_mut_refs)]
    let queue: Option<&mut VirtqueueMem> = {
        if device_type == "kbd" {
            Some(unsafe { &mut VIRTIO_KEYBOARD_QUEUE })
        } else if device_type == "mouse" {
            Some(unsafe { &mut VIRTIO_MOUSE_QUEUE })
        } else {
            None
        }
    };

    if queue.is_none() {
        return;
    }

    let queue = queue.unwrap();

    let phys_base: u64 = queue as *const VirtqueueMem as u64
        - KERNEL_VIRT_BASE.load(Ordering::Relaxed)
        + KERNEL_PHYS_BASE.load(Ordering::Relaxed);

    device.write(VirtioMmioReg::QueueSel, 0);
    device.write(VirtioMmioReg::QueueNum, QUEUE_SIZE as u32);

    let desc_phys = phys_base + offset_of!(VirtqueueMem, desc) as u64;
    device.write(VirtioMmioReg::QueueDescLow, (desc_phys & 0xFFFFFFFF) as u32);
    device.write(VirtioMmioReg::QueueDescHigh, (desc_phys >> 32) as u32);

    let avail_phys = phys_base + offset_of!(VirtqueueMem, avail) as u64;
    device.write(
        VirtioMmioReg::QueueDriverLow,
        (avail_phys & 0xFFFFFFFF) as u32,
    );
    device.write(VirtioMmioReg::QueueDriverHigh, (avail_phys >> 32) as u32);

    let used_phys = phys_base + offset_of!(VirtqueueMem, used) as u64;
    device.write(
        VirtioMmioReg::QueueDeviceLow,
        (used_phys & 0xFFFFFFFF) as u32,
    );
    device.write(VirtioMmioReg::QueueDeviceHigh, (used_phys >> 32) as u32);

    fill_descriptors(queue);

    core::sync::atomic::fence(Ordering::SeqCst);
    device.write(VirtioMmioReg::QueueReady, 1);
    device.write(VirtioMmioReg::QueueNotify, 0);
}
