use core::sync::atomic::{AtomicI16, AtomicU8, AtomicUsize, Ordering};

use crate::driver::graphics::{base::rgb, framebuffer::Framebuffer};

pub struct Mouse {
    left_button_pressed: AtomicU8,
    right_button_pressed: AtomicU8,
    middle_button_pressed: AtomicU8,
    x_delta: AtomicI16,
    y_delta: AtomicI16,
    status: AtomicU8,
    mouse_x: AtomicUsize,
    mouse_y: AtomicUsize,
}

static CURSOR_BYTES: &[u8] = include_bytes!("../../../assets/cursors/default.bmp");
const BMP_HEADER_SIZE: usize = 138;
const CURSOR_W: usize = 24;
const CURSOR_H: usize = 24;

impl Mouse {
    const fn new() -> Mouse {
        Mouse {
            left_button_pressed: AtomicU8::new(0),
            right_button_pressed: AtomicU8::new(0),
            middle_button_pressed: AtomicU8::new(0),
            x_delta: AtomicI16::new(0),
            y_delta: AtomicI16::new(0),
            status: AtomicU8::new(0),
            mouse_x: AtomicUsize::new(100),
            mouse_y: AtomicUsize::new(100),
        }
    }

    pub fn interrupt(
        &self,
        left_button_pressed: u8,
        right_button_pressed: u8,
        middle_button_pressed: u8,
        x_delta: i16,
        y_delta: i16,
    ) {
        self.left_button_pressed
            .store(left_button_pressed, Ordering::Relaxed);
        self.right_button_pressed
            .store(right_button_pressed, Ordering::Relaxed);
        self.middle_button_pressed
            .store(middle_button_pressed, Ordering::Relaxed);
        self.x_delta.fetch_add(x_delta, Ordering::Relaxed);
        self.y_delta.fetch_add(y_delta, Ordering::Relaxed);
    }

    pub fn is_left_button_pressed(&self) -> u8 {
        self.left_button_pressed.load(Ordering::Relaxed)
    }
    pub fn is_right_button_pressed(&self) -> u8 {
        self.right_button_pressed.load(Ordering::Relaxed)
    }
    pub fn is_middle_button_pressed(&self) -> u8 {
        self.middle_button_pressed.load(Ordering::Relaxed)
    }
    pub fn get_x_delta(&self) -> i16 {
        self.x_delta.swap(0, Ordering::Relaxed)
    }
    pub fn get_y_delta(&self) -> i16 {
        self.y_delta.swap(0, Ordering::Relaxed)
    }
    pub fn set_status(&self, status: u8) {
        self.status.store(status, Ordering::Relaxed);
    }
    pub fn get_status(&self) -> u8 {
        self.status.load(Ordering::Relaxed)
    }

    pub fn update(&self, width: usize, height: usize) {
        let x_delta = self.get_x_delta() / 5;
        let y_delta = self.get_y_delta() / 5;

        if x_delta != 0 {
            self.mouse_x.store(
                (self.mouse_x.load(Ordering::Relaxed) as isize + x_delta as isize).max(0) as usize,
                Ordering::Relaxed,
            );
        }

        if y_delta != 0 {
            self.mouse_y.store(
                (self.mouse_y.load(Ordering::Relaxed) as isize + y_delta as isize).max(0) as usize,
                Ordering::Relaxed,
            );
        }

        if self.mouse_x.load(Ordering::Relaxed) > width {
            self.mouse_x.store(width - CURSOR_W, Ordering::Relaxed);
        }

        if self.mouse_y.load(Ordering::Relaxed) > height {
            self.mouse_y.store(height - CURSOR_H, Ordering::Relaxed);
        }
    }

    pub fn draw(&self, fb: &mut Framebuffer) {
        let pixels = &CURSOR_BYTES[BMP_HEADER_SIZE..]; // remove header

        for row in 0..CURSOR_H {
            let src_row = (CURSOR_H - 1 - row) * CURSOR_W * 4;
            for col in 0..CURSOR_W {
                let i = src_row + col * 4; // 4 because rgba

                let b = pixels[i];
                let g = pixels[i + 1];
                let r = pixels[i + 2];
                let a = pixels[i + 3];

                if a < 255 {
                    continue;
                }

                let color = rgb(r, g, b);

                fb.put_pixel(
                    (self.mouse_x.load(Ordering::Relaxed) + col) as usize,
                    (self.mouse_y.load(Ordering::Relaxed) + row) as usize,
                    color,
                );
            }
        }
    }
}

pub static MOUSE: Mouse = Mouse::new();
