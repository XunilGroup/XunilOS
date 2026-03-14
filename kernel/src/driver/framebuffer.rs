use limine::framebuffer::Framebuffer as LimineFramebuffer;

const MAX_BACKBUFFER_PIXELS: usize = 1920 * 1080;
static mut BACK_BUFFER: [u32; MAX_BACKBUFFER_PIXELS] = [0; MAX_BACKBUFFER_PIXELS];

pub struct Framebuffer {
    addr: *mut u32,
    back_buffer: *mut u32,
    width: usize,
    height: usize,
    pitch: usize,
    back_buffer_len: usize,
}

impl Framebuffer {
    pub fn new(limine_fb: &LimineFramebuffer) -> Framebuffer {
        let width = limine_fb.width() as usize;
        let height = limine_fb.height() as usize;
        let pitch = limine_fb.pitch() as usize / 4;
        let needed = pitch.saturating_mul(height);
        let back_buffer_len = core::cmp::min(needed, MAX_BACKBUFFER_PIXELS);

        Framebuffer {
            addr: limine_fb.addr().cast::<u32>(),
            back_buffer: core::ptr::addr_of_mut!(BACK_BUFFER).cast::<u32>(),
            width,
            height,
            pitch,
            back_buffer_len,
        }
    }

    pub fn put_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x >= self.width || y >= self.height {
            return;
        }

        let idx = y.saturating_mul(self.pitch).saturating_add(x);
        if idx >= self.back_buffer_len {
            return;
        }

        unsafe {
            *self.back_buffer.add(idx) = color;
        }
    }

    pub fn swap(&mut self) {
        unsafe {
            core::ptr::copy_nonoverlapping(self.back_buffer, self.addr, self.back_buffer_len);
        }
    }
}