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
pub mod util;

use crate::arch::arch::{infinite_idle, init, kernel_crash, sleep};
use crate::driver::graphics::base::rgb;
use crate::driver::graphics::framebuffer::{init_framebuffer, with_framebuffer};
use crate::driver::graphics::primitives::{
    circle_filled, circle_outline, rectangle_filled, rectangle_outline, triangle_outline,
};
use crate::driver::serial::{ConsoleWriter, init_serial_console, with_serial_console};
use crate::driver::timer::TIMER;
use crate::util::test_performance;
use alloc::{boxed::Box, string::ToString, vec::Vec};

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
        fb.swap();
    });
}

#[unsafe(no_mangle)]
unsafe extern "C" fn kmain() -> ! {
    // All limine requests must also be referenced in a called function, otherwise they may be
    // removed by the linker.
    assert!(BASE_REVISION.is_supported());

    if let Some(hhdm_response) = HHDM_REQUEST.get_response() {
        if let Some(memory_map_response) = MEMORY_MAP_REQUEST.get_response() {
            let (mapper, frame_allocator) = init(hhdm_response, memory_map_response);
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

    boot_animation();

    let x = Box::new(41);
    let mut test_vec: Vec<u16> = Vec::new();
    test_vec.push(5);
    println!("Before: {:?}", test_vec);
    test_vec.push(9);
    println!("After: {:?}", test_vec);

    sleep(500);

    loop {
        with_serial_console(|serial_console| serial_console.clear(0, 0));

        with_framebuffer(|mut fb| {
            fb.clear(rgb(253, 129, 0));

            // rectangle_filled(&mut fb, 700, 400, 200, 200, rgb(0, 0, 0));
            // rectangle_outline(&mut fb, 400, 400, 100, 100, rgb(0, 0, 0));
            // circle_filled(&mut fb, 200, 200, 100.0, rgb(0, 0, 0));

            circle_outline(&mut fb, 400, 200, 100.0, rgb(0, 0, 0));
            triangle_outline(&mut fb, 100, 400, 200, 400, 150, 600, rgb(0, 0, 0));
        });

        let (hours, minutes, seconds) =
            unix_to_hms(TIMER.get_date_at_boot() + (TIMER.now().elapsed()) / 1000);

        print!("{:?}:{:?}:{:?}", hours, minutes, seconds);

        sleep(16);
    }
}

fn boot_animation() {
    let mut i = 1;

    while i < 10 {
        let mut width = 0;
        let mut height = 0;

        with_framebuffer(|fb| {
            fb.clear(rgb(253, 129, 0));
            width = fb.width;
            height = fb.height;
        });

        let text_width = ("XunilOS Loading".len() + ".".repeat(i).len()) * 4 * 2;

        with_serial_console(|serial_console| {
            serial_console.clear(width / 2 - text_width / 2, height / 2)
        });

        println!(
            "{}",
            "XunilOS Loading".to_string() + &".".repeat(i).as_str()
        );

        i += 1;

        sleep(200);
    }

    with_serial_console(|serial_console| {
        serial_console.clear(0, 0);
    });

    with_framebuffer(|fb| {
        fb.clear(rgb(253, 129, 0));
    });
}

fn unix_to_hms(timestamp: u64) -> (u64, u64, u64) {
    let seconds = timestamp % 86400;
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    let s = seconds % 60;
    (h, m, s)
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
