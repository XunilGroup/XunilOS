use crate::driver::graphics::framebuffer::{with_framebuffer, Framebuffer};
use crate::driver::graphics::font_render::render_text;
use crate::driver::graphics::base::rgb;
use spin::Mutex;
use core::fmt::{self, Write};

const DEFAULT_FONT_SIZE: usize = 3;

pub struct ConsoleWriter<'a> {
    pub fb: &'a mut Framebuffer,
    pub console: &'a mut SerialConsole,
}

impl Write for ConsoleWriter<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.console.render_text(self.fb, s);
        Ok(())
    }
}

pub struct SerialConsole {
    start_x: usize,
    current_y: usize
}

impl SerialConsole {
    pub fn new(start_x: usize, start_y: usize) -> SerialConsole {
        SerialConsole {
            start_x,
            current_y: start_y
        }
    }

    pub fn render_text(&mut self, fb: &mut Framebuffer, text: &str) {
        self.current_y = render_text(fb, self.start_x - ((text.len() - text.matches('\n').count()) * DEFAULT_FONT_SIZE * 4), self.current_y, text, DEFAULT_FONT_SIZE, rgb(255, 255, 255));
        self.current_y = render_text(fb, self.start_x - ((text.len() - text.matches('\n').count()) * DEFAULT_FONT_SIZE * 4), self.current_y, "\n", DEFAULT_FONT_SIZE, rgb(255, 255, 255)); // add a newline
    }

    pub fn clear(&mut self, start_y: usize) {
        self.current_y = start_y;
    }
}

pub static SERIAL_CONSOLE: Mutex<Option<SerialConsole>> = Mutex::new(None);

pub fn init_serial_console(start_x: usize, start_y: usize) {
    *SERIAL_CONSOLE.lock() = Some(SerialConsole::new(start_x, start_y));
}

pub fn with_serial_console<F: FnOnce(&mut SerialConsole)>(f: F) {
    let mut guard = SERIAL_CONSOLE.lock();
    if let Some(fb) = guard.as_mut() {
        f(fb);
    }
}