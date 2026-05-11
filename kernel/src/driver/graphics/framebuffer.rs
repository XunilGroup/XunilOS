use limine::framebuffer::Framebuffer as LimineFramebuffer;
use spin::Mutex;

#[cfg(target_arch = "x86_64")]
use x86_64::instructions::interrupts::without_interrupts;
use x86_64::structures::paging::{FrameAllocator, PhysFrame, Size4KiB};

use crate::arch::arch::FRAME_ALLOCATOR;

#[repr(C)]
pub struct UserFrameBuffer {
    pub buf_virt: *mut u32,
    pub width: usize,
    pub height: usize,
    pub pitch: usize,
}

pub struct Framebuffer {
    addr: *mut u32,
    pub width: usize,
    pub height: usize,
    pitch: usize,
    pub user_fb: UserFrameBuffer,
    pub meta: FramebufferMeta,
}

pub const USER_FB_BASE: u64 = 0x0000_7F00_0000_0000;

pub struct FramebufferMeta {
    pub buf_phys: u64,
    pub buf_len: usize,
    pub struct_phys: u64,
}

impl Framebuffer {
    pub fn new(limine_fb: &LimineFramebuffer) -> Framebuffer {
        let width = limine_fb.width() as usize;
        let height = limine_fb.height() as usize;
        let pitch = limine_fb.pitch() as usize / 4;
        let buf_len = pitch * height;
        let byte_len = buf_len * core::mem::size_of::<u32>();
        let pixel_frames = (byte_len + 4095) / 4096;

        let mut fa = FRAME_ALLOCATOR.lock();

        let struct_frame: PhysFrame<Size4KiB> =
            fa.allocate_frame().expect("framebuffer struct frame");
        let struct_phys = struct_frame.start_address().as_u64();
        let struct_virt = (struct_phys + fa.hhdm_offset) as *mut UserFrameBuffer;

        let first_pixel_frame: PhysFrame<Size4KiB> =
            fa.allocate_frame().expect("framebuffer pixel frame 0");
        let buf_phys = first_pixel_frame.start_address().as_u64();
        for _ in 1..pixel_frames {
            fa.allocate_frame().expect("framebuffer pixel frame");
        }
        let buf_virt_kernel = (buf_phys + fa.hhdm_offset) as *mut u32;
        drop(fa);

        unsafe { core::ptr::write_bytes(buf_virt_kernel, 0, buf_len) };

        unsafe {
            struct_virt.write(UserFrameBuffer {
                buf_virt: (USER_FB_BASE + 0x1000) as *mut u32,
                width,
                height,
                pitch,
            });
        }

        Framebuffer {
            addr: limine_fb.addr().cast::<u32>(),
            width,
            height,
            pitch,
            user_fb: UserFrameBuffer {
                buf_virt: buf_virt_kernel,
                width,
                height,
                pitch,
            },
            meta: FramebufferMeta {
                buf_phys,
                buf_len,
                struct_phys,
            },
        }
    }

    #[inline(always)]
    pub fn put_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = y * self.pitch + x;
        if idx >= self.meta.buf_len {
            return;
        }
        unsafe { self.user_fb.buf_virt.add(idx).write(color) };
    }

    #[inline(always)]
    pub fn fill_span(&mut self, x: usize, y: usize, len: usize, color: u32) {
        if y >= self.height || x >= self.width || len == 0 {
            return;
        }
        let len = core::cmp::min(len, self.width - x);
        let start = y * self.pitch + x;
        unsafe {
            let slice = core::slice::from_raw_parts_mut(self.user_fb.buf_virt.add(start), len);
            slice.fill(color);
        }
    }

    pub fn present(&mut self) {
        unsafe {
            core::ptr::copy_nonoverlapping(self.user_fb.buf_virt, self.addr, self.meta.buf_len);
        }
    }

    pub fn clear(&mut self, color: u32) {
        unsafe {
            let slice = core::slice::from_raw_parts_mut(self.user_fb.buf_virt, self.meta.buf_len);
            slice.fill(color);
        }
    }
}

unsafe impl Send for Framebuffer {}
unsafe impl Send for UserFrameBuffer {}

pub static FRAMEBUFFER: Mutex<Option<Framebuffer>> = Mutex::new(None);

pub fn init_framebuffer(raw: &LimineFramebuffer) {
    *FRAMEBUFFER.lock() = Some(Framebuffer::new(raw));
}

pub fn with_framebuffer<F: FnOnce(&mut Framebuffer)>(f: F) {
    without_interrupts(|| {
        let mut guard = FRAMEBUFFER.lock();
        if let Some(fb) = guard.as_mut() {
            f(fb);
        }
    });
}
