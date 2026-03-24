use crate::driver::graphics::base::rgb;
use crate::driver::graphics::font_render::render_text;
use crate::driver::graphics::framebuffer::Framebuffer;
use core::fmt::{self, Write};
use spin::Mutex;

#[cfg(target_arch = "x86_64")]
use crate::arch::x86_64::interrupts::without_interrupts;

pub struct ConsoleWriter<'a> {
    pub fb: &'a mut Framebuffer,
    pub console: &'a mut SerialConsole,
    pub should_center: bool,
}

impl Write for ConsoleWriter<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.console.render_text(self.fb, s, 2, false);
        Ok(())
    }
}

pub struct SerialConsole {
    start_x: usize,
    pub current_x: usize,
    current_y: usize,
}

impl SerialConsole {
    pub fn new(start_x: usize, start_y: usize) -> SerialConsole {
        SerialConsole {
            start_x,
            current_x: start_x,
            current_y: start_y,
        }
    }

    pub fn render_text(
        &mut self,
        fb: &mut Framebuffer,
        text: &str,
        font_size: usize,
        should_center: bool,
    ) {
        let (new_x, new_y) = render_text(
            fb,
            if should_center {
                self.current_x - (text.len() - text.matches('\n').count()) * (font_size * 4)
            } else {
                self.current_x
            },
            self.current_y,
            text,
            font_size,
            rgb(255, 255, 255),
            self.start_x,
        );
        self.current_x = new_x;
        self.current_y = new_y;
    }

    pub fn clear(&mut self, start_x: usize, start_y: usize) {
        self.start_x = start_x;
        self.current_x = start_x;
        self.current_y = start_y;
    }
}

pub static SERIAL_CONSOLE: Mutex<Option<SerialConsole>> = Mutex::new(None);

pub fn init_serial_console(start_x: usize, start_y: usize) {
    *SERIAL_CONSOLE.lock() = Some(SerialConsole::new(start_x, start_y));
}

pub fn with_serial_console<F: FnOnce(&mut SerialConsole)>(f: F) {
    without_interrupts(|| {
        let mut guard = SERIAL_CONSOLE.lock();
        if let Some(fb) = guard.as_mut() {
            f(fb);
        }
    })
}
