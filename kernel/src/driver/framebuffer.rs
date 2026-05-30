use crate::arch::arch::safe_lock;
use limine::framebuffer::Framebuffer as LimineFramebuffer;
use spin::Mutex;

#[cfg(target_arch = "x86_64")]
use x86_64::structures::paging::OffsetPageTable;

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
        Framebuffer {
            addr: limine_fb.addr().cast::<u32>(),
            width,
            height,
            pitch,
            user_fb: UserFrameBuffer {
                buf_virt: 0 as *mut u32,
                width,
                height,
                pitch,
            },
            meta: FramebufferMeta {
                buf_phys: 0,
                buf_len: 0,
                struct_phys: 0,
            },
        }
    }

    #[cfg(target_arch = "x86_64")]
    pub fn setup_x86_64(&mut self, mapper: &mut OffsetPageTable) {
        use crate::arch::arch::FRAME_ALLOCATOR;
        use x86_64::{
            PhysAddr, VirtAddr,
            structures::paging::{
                FrameAllocator, Mapper, Page, PageTableFlags, PhysFrame, Size4KiB,
            },
        };
        const KERNEL_FB_BASE: u64 = 0xffffffffc0000000;
        let buf_len = self.pitch * self.height;
        let byte_len = buf_len * core::mem::size_of::<u32>();
        let pixel_frames = (byte_len + 4095) / 4096;

        let mut fa = safe_lock(|| FRAME_ALLOCATOR.lock());

        let struct_frame: PhysFrame<Size4KiB> =
            fa.allocate_frame().expect("framebuffer struct frame");
        let struct_phys = struct_frame.start_address().as_u64();

        let first_pixel_frame: PhysFrame<Size4KiB> =
            fa.allocate_frame().expect("framebuffer pixel frame 0");
        let buf_phys = first_pixel_frame.start_address().as_u64();
        for _ in 1..pixel_frames {
            fa.allocate_frame().expect("framebuffer pixel frame");
        }

        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE;

        let struct_page = Page::<Size4KiB>::containing_address(VirtAddr::new(KERNEL_FB_BASE));
        unsafe {
            mapper
                .map_to(struct_page, struct_frame, flags, &mut *fa)
                .unwrap()
                .flush();
        }

        for i in 0..pixel_frames {
            let frame = PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(
                buf_phys + i as u64 * 4096,
            ));
            let page = Page::<Size4KiB>::containing_address(VirtAddr::new(
                KERNEL_FB_BASE + 0x1000 + i as u64 * 4096,
            ));
            unsafe {
                mapper.map_to(page, frame, flags, &mut *fa).unwrap().flush();
            }
        }

        let struct_virt = KERNEL_FB_BASE as *mut UserFrameBuffer;
        let buf_virt_kernel = (KERNEL_FB_BASE + 0x1000) as *mut u32;
        drop(fa);

        unsafe { core::ptr::write_bytes(buf_virt_kernel, 0, buf_len) };

        unsafe {
            struct_virt.write(UserFrameBuffer {
                buf_virt: (USER_FB_BASE + 0x1000) as *mut u32,
                width: self.width,
                height: self.height,
                pitch: self.pitch,
            });
        };

        self.user_fb.buf_virt = buf_virt_kernel;
        self.meta.buf_phys = buf_phys;
        self.meta.buf_len = buf_len;
        self.meta.struct_phys = struct_phys;
    }

    #[cfg(target_arch = "aarch64")]
    pub fn setup_aarch64(&mut self) {
        let hhdm_offset = HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
        use crate::arch::arch::{FRAME_ALLOCATOR, HHDM_OFFSET};
        let buf_len = self.pitch * self.height;
        let byte_len = buf_len * core::mem::size_of::<u32>();
        let pixel_frames = (byte_len + 4095) / 4096;

        let mut fa = safe_lock(|| FRAME_ALLOCATOR.lock());

        let struct_phys: u64 = fa.allocate_frame().expect("framebuffer struct frame");
        let struct_virt = (struct_phys + hhdm_offset) as *mut UserFrameBuffer;

        let buf_phys: u64 = fa.allocate_frame().expect("framebuffer pixel frame 0");
        for _ in 1..pixel_frames {
            fa.allocate_frame().expect("framebuffer pixel frame");
        }
        let buf_virt_kernel = (buf_phys + hhdm_offset) as *mut u32;
        drop(fa);

        unsafe { core::ptr::write_bytes(buf_virt_kernel, 0, buf_len) };

        unsafe {
            struct_virt.write(UserFrameBuffer {
                buf_virt: (USER_FB_BASE + 0x1000) as *mut u32,
                width: self.width,
                height: self.height,
                pitch: self.pitch,
            });
        };

        self.user_fb.buf_virt = buf_virt_kernel;
        self.meta.buf_phys = buf_phys;
        self.meta.buf_len = buf_len;
        self.meta.struct_phys = struct_phys;
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
    safe_lock(|| {
        let mut guard = FRAMEBUFFER.lock();
        if let Some(fb) = guard.as_mut() {
            f(fb);
        }
    });
}
