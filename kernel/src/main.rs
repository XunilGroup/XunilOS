#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate alloc;
use core::fmt::Write;

use limine::BaseRevision;
use limine::request::{
    DateAtBootRequest, FramebufferRequest, HhdmRequest, MemoryMapRequest, MpRequest,
    RequestsEndMarker, RequestsStartMarker,
};
pub mod arch;
pub mod driver;
pub mod userspace_stub;
pub mod util;

use crate::arch::arch::{infinite_idle, init, kernel_crash, sleep};
use crate::driver::graphics::base::rgb;
use crate::driver::graphics::framebuffer::{init_framebuffer, with_framebuffer};
use crate::driver::graphics::primitives::{
    circle_filled, circle_outline, rectangle_filled, rectangle_outline, triangle_outline,
};
use crate::driver::mouse::MOUSE;
use crate::driver::serial::{ConsoleWriter, init_serial_console, with_serial_console};
use crate::driver::timer::TIMER;
use crate::userspace_stub::userspace_init;
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

#[used]
#[unsafe(link_section = ".requests")]
static DATE_AT_BOOT_REQUEST: DateAtBootRequest = DateAtBootRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
static MP_REQUEST: MpRequest = MpRequest::new();

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
    });
}

#[unsafe(no_mangle)]
unsafe extern "C" fn kmain() -> ! {
    // All limine requests must also be referenced in a called function, otherwise they may be
    // removed by the linker.
    assert!(BASE_REVISION.is_supported());

    let mapper;
    let frame_allocator;

    if let Some(hhdm_response) = HHDM_REQUEST.get_response() {
        if let Some(memory_map_response) = MEMORY_MAP_REQUEST.get_response() {
            (mapper, frame_allocator) = init(hhdm_response, memory_map_response);
        } else {
            kernel_crash(); // Could not get required info from Limine's memory map.
        }
    } else {
        kernel_crash(); // Could not get required info from the Limine's higher-half direct mapping.
    }

    if let Some(framebuffer_response) = FRAMEBUFFER_REQUEST.get_response() {
        if let Some(limine_framebuffer) = framebuffer_response.framebuffers().next() {
            init_framebuffer(&limine_framebuffer);
        }
    }

    init_serial_console(0, 0);

    if let Some(date_at_boot_response) = DATE_AT_BOOT_REQUEST.get_response() {
        TIMER.set_date_at_boot(date_at_boot_response.timestamp().as_secs());
    } else {
        println!("Could not get date at boot. Will default to 0.")
    }

    userspace_init()
}

#[panic_handler]
fn rust_panic(_info: &core::panic::PanicInfo) -> ! {
    with_framebuffer(|mut fb| {
        let (width, height) = (fb.width.clone(), fb.height.clone());
        rectangle_filled(&mut fb, 0, 0, width, height, rgb(180, 0, 0));

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

    infinite_idle();
}
