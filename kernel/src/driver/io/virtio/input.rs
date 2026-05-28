use crate::driver::io::{
    input::{InputEvent, enqueue_input_event},
    virtio::{
        KEYBOARD_SLOT, MOUSE_SLOT, VIRTIO_MMIO_BASE, VIRTIO_MMIO_STRIDE,
        queue::{QUEUE_SIZE, VirtqueueMem},
        transport::{VirtioMmio, VirtioMmioReg},
    },
};

#[repr(u8)]
pub enum VirtioInputCfgSelect {
    Unset = 0x00,
    IdName = 0x01,
    IdSerial = 0x02,
    IdDevids = 0x03,
    PropBits = 0x10,
    EvBits = 0x11,
    AbsInfo = 0x12,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VirtioInputEvent {
    event_type: u16,
    code: u16,
    value: u32,
}

pub static mut VIRTIO_KEYBOARD_QUEUE: VirtqueueMem = unsafe { core::mem::zeroed() };
pub static mut VIRTIO_MOUSE_QUEUE: VirtqueueMem = unsafe { core::mem::zeroed() };

pub fn read_device_name(device: &VirtioMmio) -> &str {
    return unsafe {
        let select_ptr = (device.base + 0x100) as *mut u8;
        let subsel_ptr = (device.base + 0x101) as *mut u8;
        let size_ptr = (device.base + 0x102) as *mut u8;
        let name_ptr = (device.base + 0x108) as *mut u8;

        select_ptr.write_volatile(VirtioInputCfgSelect::IdName as u8);
        subsel_ptr.write_volatile(0x00);

        let size = size_ptr.read_volatile() as usize;

        if size == 0 || size > 128 {
            return "Error";
        }

        let slice = core::slice::from_raw_parts(name_ptr, size);
        core::str::from_utf8(slice).unwrap_or("Error")
    };
}

pub fn input_interrupt(device_type: &str) {
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

    let device = VirtioMmio::new(
        VIRTIO_MMIO_BASE + {
            if device_type == "kbd" {
                KEYBOARD_SLOT.load(core::sync::atomic::Ordering::Relaxed)
            } else if device_type == "mouse" {
                MOUSE_SLOT.load(core::sync::atomic::Ordering::Relaxed)
            } else {
                0
            }
        } * VIRTIO_MMIO_STRIDE,
    );

    if queue.is_none() {
        return;
    }

    let queue = queue.unwrap();

    let status = device.read(VirtioMmioReg::InterruptStatus);
    device.write(VirtioMmioReg::InterruptAck, status);

    loop {
        let used_idx = unsafe { core::ptr::read_volatile(&queue.used.idx) };
        let last_idx = queue.last_used_idx;

        if last_idx == used_idx {
            return;
        }

        let used_element = &queue.used.ring[last_idx as usize % QUEUE_SIZE];
        let desc_idx = used_element.id;

        queue.last_used_idx = last_idx.wrapping_add(1);

        let event = unsafe { core::ptr::read_volatile(&queue.buffers[desc_idx as usize]) };

        handle_event(&event);

        let avail_idx = unsafe { core::ptr::read_volatile(&queue.avail.idx) };
        queue.avail.ring[(avail_idx as usize) % QUEUE_SIZE] = desc_idx as u16;

        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
        unsafe {
            core::ptr::write_volatile(&mut queue.avail.idx, avail_idx.wrapping_add(1));
        }
    }
}

#[allow(static_mut_refs)]
pub fn handle_event(event: &VirtioInputEvent) {
    if event.event_type == 0 || event.code == 0 {
        return;
    }

    enqueue_input_event(InputEvent {
        event_type: event.event_type,
        code: event.code,
        value: event.value,
    });
}
