use alloc::vec;
use alloc::vec::Vec;
use limine::framebuffer::Framebuffer as LimineFramebuffer;
use spin::Mutex;

#[cfg(target_arch = "x86_64")]
use x86_64::instructions::interrupts::without_interrupts;

pub struct Framebuffer {
    addr: *mut u32,
    back_buffer: Vec<u32>,
    pub width: usize,
    pub height: usize,
    pitch: usize,
    back_buffer_len: usize,
}

impl Framebuffer {
    pub fn new(limine_fb: &LimineFramebuffer) -> Framebuffer {
        let width = limine_fb.width() as usize;
        let height = limine_fb.height() as usize;
        let pitch = limine_fb.pitch() as usize / 4;
        let back_buffer_len = width * height;

        Framebuffer {
            addr: limine_fb.addr().cast::<u32>(),
            back_buffer: vec![0u32; width * height],
            width,
            height,
            pitch,
            back_buffer_len,
        }
    }

    #[inline(always)]
    pub fn put_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x >= self.width || y >= self.height {
            return;
        }

        let idx = y.saturating_mul(self.pitch).saturating_add(x);
        if idx >= self.back_buffer_len {
            return;
        }

        self.back_buffer[idx] = color;
    }

    #[inline(always)]
    pub fn fill_span(&mut self, x: usize, y: usize, len: usize, color: u32) {
        if y >= self.height || x >= self.width || len == 0 {
            return;
        }
        let max_len = self.width - x;
        let len = core::cmp::min(len, max_len);
        let start = y * self.pitch + x;
        let end = start + len;
        unsafe {
            self.back_buffer.get_unchecked_mut(start..end).fill(color);
        }
    }

    pub fn swap(&mut self) {
        unsafe {
            core::ptr::copy_nonoverlapping(
                self.back_buffer.as_ptr(),
                self.addr,
                self.back_buffer_len,
            );
        }
    }

    pub unsafe fn load_from_ptr(
        &mut self,
        src_ptr: *const u32,
        src_width: usize,
        src_height: usize,
    ) {
        let h = core::cmp::min(src_height, self.height);
        let w = core::cmp::min(src_width, self.width);

        for y in 0..h {
            let src_row = unsafe { src_ptr.add(y * src_width) };
            let dst_row = unsafe { self.back_buffer.as_mut_ptr().add(y * self.pitch) };
            unsafe { core::ptr::copy_nonoverlapping(src_row, dst_row, w) };
        }
    }

    pub fn clear(&mut self, color: u32) {
        self.back_buffer.fill(color);
    }
}

unsafe impl Send for Framebuffer {}

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
    })
}
