use crate::driver::graphics::font_render::render_text;
use crate::driver::graphics::framebuffer::Framebuffer;
use crate::{arch::arch::safe_lock, driver::graphics::base::rgb};
use alloc::string::String;
use core::fmt::{self, Write};
use spin::Mutex;

pub struct ConsoleWriter<'a> {
    pub fb: &'a mut Framebuffer,
    pub console: &'a mut SerialConsole,
    pub should_center: bool,
}

impl Write for ConsoleWriter<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.console.print(s, self.fb);
        Ok(())
    }
}

pub struct SerialConsole {
    text: String,
    font_size: usize,
    dirty: bool,
}

impl SerialConsole {
    pub fn new() -> SerialConsole {
        SerialConsole {
            text: String::new(),
            font_size: 2,
            dirty: false,
        }
    }

    pub fn print(&mut self, text: &str, fb: &mut Framebuffer) {
        let max_height = fb.height / (12 * self.font_size);
        if self.text.split('\n').count() > max_height {
            self.clear();
        }
        self.text.push_str(text);
        self.dirty = true;
    }

    pub fn render(&mut self, fb: &mut Framebuffer) {
        if self.dirty {
            fb.clear(rgb(0, 0, 0));
            self.dirty = false;
            render_text(fb, 0, 0, &self.text, self.font_size, rgb(255, 255, 255), 0);
        }
    }

    pub fn clear(&mut self) {
        self.dirty = true;
        self.text.clear();
    }
}

pub static SERIAL_CONSOLE: Mutex<Option<SerialConsole>> = Mutex::new(None);

pub fn init_serial_console() {
    *SERIAL_CONSOLE.lock() = Some(SerialConsole::new());
}

pub fn with_serial_console<F: FnOnce(&mut SerialConsole)>(f: F) {
    safe_lock(|| {
        let mut guard = SERIAL_CONSOLE.lock();
        if let Some(sc) = guard.as_mut() {
            f(sc);
        }
    });
}
