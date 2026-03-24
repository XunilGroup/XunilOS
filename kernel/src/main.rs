#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::fmt::Write;

use limine::BaseRevision;
use limine::request::{
    FramebufferRequest, HhdmRequest, MemoryMapRequest, RequestsEndMarker, RequestsStartMarker,
};
pub mod arch;
pub mod driver;

use crate::arch::arch::{idle, init};
use crate::driver::graphics::base::rgb;
use crate::driver::graphics::framebuffer::{init_framebuffer, with_framebuffer};
use crate::driver::graphics::primitives::{
    circle_filled, circle_outline, rectangle_filled, rectangle_outline, triangle_outline,
};
use crate::driver::serial::{ConsoleWriter, init_serial_console, with_serial_console};

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

#[used]
#[unsafe(link_section = ".requests")]
static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
static MEMORY_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();

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
            let mut writer = ConsoleWriter {
                fb,
                console,
                should_center: false,
            };
            let _ = writer.write_fmt(args);
        });
        fb.swap();
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
            // boot_animation();
        }
    }

    if let Some(hhdm_response) = HHDM_REQUEST.get_response() {
        if let Some(memory_map_response) = MEMORY_MAP_REQUEST.get_response() {
            let mapper = init(hhdm_response, memory_map_response);
        } else {
            init_serial_console(0, 0);
            panic!("Could not get required info from Limine's memory map. ")
        }
    } else {
        init_serial_console(0, 0);
        panic!("Could not get required info from the Limine's higher-half direct mapping. ")
    }

    with_framebuffer(|mut fb| {
        let (width, height) = (fb.width.clone(), fb.height.clone());
        init_serial_console(0, 0);
        rectangle_filled(&mut fb, 0, 0, width, height, rgb(253, 129, 0), true);
        rectangle_filled(&mut fb, 700, 400, 200, 200, rgb(0, 0, 0), true);
        rectangle_outline(&mut fb, 400, 400, 100, 100, rgb(0, 0, 0));
        circle_filled(&mut fb, 200, 200, 100.0, rgb(0, 0, 0));
        circle_outline(&mut fb, 400, 200, 100.0, rgb(0, 0, 0));
        triangle_outline(&mut fb, 100, 400, 200, 400, 150, 600, rgb(0, 0, 0));
    });

    idle();
}

#[panic_handler]
fn rust_panic(_info: &core::panic::PanicInfo) -> ! {
    with_framebuffer(|mut fb| {
        let (width, height) = (fb.width.clone(), fb.height.clone());
        rectangle_filled(&mut fb, 0, 0, width, height, rgb(180, 0, 0), true);

        with_serial_console(|console| {
            console.clear(5, 5);

            let mut writer = ConsoleWriter {
                fb: &mut fb,
                console,
                should_center: true,
            };

            let _ = writer.write_str("KERNEL PANIC\n\n");
            let _ = writer.write_fmt(core::format_args!("{}", _info));
            fb.swap();
        });
    });

    idle();
}
