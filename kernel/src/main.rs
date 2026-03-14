#![no_std]
#![no_main]

use core::arch::asm;
use core::fmt::Write;

use limine::BaseRevision;
use limine::request::{FramebufferRequest, RequestsEndMarker, RequestsStartMarker};

pub mod driver;
pub mod arch;

use spin::Mutex;
use crate::arch::serial::{ConsoleWriter, SerialConsole, init_serial_console, with_serial_console};
use crate::driver::graphics::font_render::render_text;
use crate::driver::graphics::primitives::{circle_filled, circle_outline, rectangle_filled, rectangle_outline, triangle_outline};
use crate::driver::graphics::base::rgb;
use crate::driver::graphics::framebuffer::{Framebuffer, init_framebuffer, with_framebuffer};

/// Sets the base revision to the latest revision supported by the crate.
/// See specification for further info.
/// Be sure to mark all limine requests with #[used], otherwise they may be removed by the compiler.
#[used]
// The .requests section allows limine to find the requests faster and more safely.
#[unsafe(link_section = ".requests")]
static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[unsafe(link_section = ".requests")]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

/// Define the stand and end markers for Limine requests.
#[used]
#[unsafe(link_section = ".requests_start_marker")]
static _START_MARKER: RequestsStartMarker = RequestsStartMarker::new();
#[used]
#[unsafe(link_section = ".requests_end_marker")]
static _END_MARKER: RequestsEndMarker = RequestsEndMarker::new();

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::_print(core::format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::_print(core::format_args!("\n"))
    };
    ($($arg:tt)*) => {
        $crate::_print(core::format_args!("{}\n", core::format_args!($($arg)*)))
    };
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    with_framebuffer(|fb| {
        with_serial_console(|console| {
            let mut writer = ConsoleWriter { fb, console };
            let _ = writer.write_fmt(args);
        });
    });
}

#[unsafe(no_mangle)]
unsafe extern "C" fn kmain() -> ! {
    // All limine requests must also be referenced in a called function, otherwise they may be
    // removed by the linker.
    assert!(BASE_REVISION.is_supported());
    
    if let Some(framebuffer_response) = FRAMEBUFFER_REQUEST.get_response() {
        if let Some(limine_framebuffer) = framebuffer_response.framebuffers().next() {
            init_framebuffer(&limine_framebuffer);
            with_framebuffer(|mut fb| {
                let (width, height) = (fb.width.clone(), fb.height.clone());
                init_serial_console(width / 2, height / 3);
                rectangle_filled(&mut fb, 0, 0, width, height, rgb(253, 129, 0), true);
                rectangle_filled(&mut fb, 700, 400, 200, 200, rgb(0, 0, 0), true);
                rectangle_outline(&mut fb, 400, 400, 100, 100, rgb(0, 0, 0));
                circle_filled(&mut fb, 200, 200, 100.0, rgb(0, 0, 0));
                circle_outline(&mut fb, 400, 200, 100.0, rgb(0, 0, 0));
                triangle_outline(&mut fb, 100, 400, 200, 400, 150, 600, rgb(0, 0, 0));
            });
            panic!("idk, test");
        }
    }

    idle();
}

#[panic_handler]
fn rust_panic(_info: &core::panic::PanicInfo) -> ! {    
    with_framebuffer(|mut fb|{
        let (width, height) = (fb.width.clone(), fb.height.clone());
        rectangle_filled(&mut fb, 0, 0, width, height, rgb(180, 0, 0), true);
        with_serial_console(|serial_console| {
            serial_console.clear(height / 3);
            serial_console.render_text(&mut fb, "Kernel Panic! :C\n\n\n");
            if let Some(message) = _info.message().as_str() {
                serial_console.render_text(&mut fb, "Message:");
                serial_console.render_text(&mut fb, message);
            }
            crate::println!("Kernel Panic! :C");
            crate::print!("Message: ");
            crate::println!("{}", _info);
        });
    });

    idle();
}

fn idle() -> ! {
    loop {
        unsafe {
            #[cfg(target_arch = "x86_64")]
            asm!("hlt");
            #[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
            asm!("wfi");
            #[cfg(target_arch = "loongarch64")]
            asm!("idle 0");
        }
    }
}
