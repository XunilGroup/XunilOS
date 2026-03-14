#![no_std]
#![no_main]

use core::arch::asm;

use limine::BaseRevision;
use limine::request::{FramebufferRequest, RequestsEndMarker, RequestsStartMarker};

pub mod driver;

use crate::driver::graphics::{circle_filled, circle_outline, rectangle_filled, rectangle_outline, rgb, triangle_outline};
use crate::driver::framebuffer::Framebuffer;

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

#[unsafe(no_mangle)]
unsafe extern "C" fn kmain() -> ! {
    // All limine requests must also be referenced in a called function, otherwise they may be
    // removed by the linker.
    assert!(BASE_REVISION.is_supported());
    
    if let Some(framebuffer_response) = FRAMEBUFFER_REQUEST.get_response() {
        if let Some(limine_framebuffer) = framebuffer_response.framebuffers().next() {
            let mut fb = Framebuffer::new(&limine_framebuffer);
            rectangle_filled(&mut fb, 0, 0, limine_framebuffer.width() as usize, limine_framebuffer.height() as usize, rgb(253, 129, 0));
            rectangle_filled(&mut fb, 700, 400, 200, 200, rgb(0, 0, 0));
            rectangle_outline(&mut fb, 400, 400, 100, 100, rgb(0, 0, 0));
            circle_filled(&mut fb, 200, 200, 100.0, rgb(0, 0, 0));
            circle_outline(&mut fb, 400, 200, 100.0, rgb(0, 0, 0));
            triangle_outline(&mut fb, 100, 400, 200, 400, 150, 600, rgb(0, 0, 0));
        }
    }

    idle();
}

#[panic_handler]
fn rust_panic(_info: &core::panic::PanicInfo) -> ! {
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
