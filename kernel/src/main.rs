#![no_std]
#![no_main]
#![cfg_attr(target_arch = "x86_64", feature(abi_x86_interrupt))]
#![feature(naked_functions_rustic_abi)]
extern crate alloc;
use core::fmt::Write;

#[cfg(target_arch = "aarch64")]
use aarch64_cpu::registers::{DAIF, Writeable};
use limine::BaseRevision;
use limine::request::{
    DateAtBootRequest, ExecutableAddressRequest, FramebufferRequest, HhdmRequest, MemoryMapRequest,
    RequestsEndMarker, RequestsStartMarker,
};
pub mod arch;
pub mod config;
pub mod driver;
pub mod mm;
pub mod task;
pub mod util;

#[cfg(target_arch = "aarch64")]
use crate::arch::aarch64::interrupts::enable_interrupts;
#[cfg(target_arch = "aarch64")]
use crate::arch::aarch64::paging::AArchPageTable;
use crate::arch::arch::{HHDM_OFFSET, infinite_idle, init, kernel_crash, serial_print};
#[cfg(target_arch = "x86_64")]
use crate::driver::elf::loader::run_elf;
use crate::driver::graphics::base::rgb;
use crate::driver::graphics::framebuffer::{init_framebuffer, with_framebuffer};
#[cfg(target_arch = "aarch64")]
use crate::driver::graphics::primitives::rectangle_filled;
use crate::driver::keyboard::init_keyboard;
use crate::driver::serial::{ConsoleWriter, init_serial_console, with_serial_console};
use crate::driver::timer::TIMER;

#[repr(C, align(16))]
#[allow(dead_code)]
struct AlignedElf([u8; include_bytes!("../../assets/init").len()]);
#[allow(dead_code)]
static INIT_ELF: AlignedElf = AlignedElf(*include_bytes!("../../assets/init"));
#[allow(dead_code)]
static INIT_ELF_BYTES: &[u8] = &INIT_ELF.0;

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
static EXECUTABLE_ADDRESS_REQUEST: ExecutableAddressRequest = ExecutableAddressRequest::new();

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
    // mask everything early to prevent stack corruption and other undefined behavior
    #[cfg(target_arch = "aarch64")]
    {
        DAIF.write(DAIF::D::Masked + DAIF::A::Masked + DAIF::I::Masked + DAIF::F::Masked);
    }

    if let Some(hhdm_response) = HHDM_REQUEST.get_response() {
        HHDM_OFFSET.store(
            hhdm_response.offset(),
            core::sync::atomic::Ordering::Relaxed,
        );
        #[allow(unused_variables)]
        if let Some(memory_map_response) = MEMORY_MAP_REQUEST.get_response() {
            #[cfg(target_arch = "aarch64")]
            if let Some(executable_address_response) = EXECUTABLE_ADDRESS_REQUEST.get_response() {
                use crate::arch::aarch64::init::preinit_aarch64;
                preinit_aarch64(
                    hhdm_response,
                    memory_map_response,
                    executable_address_response,
                );

                loop {}
            } else {
                kernel_crash()
            }

            #[cfg(target_arch = "x86_64")]
            unsafe {
                kernel_main_x86_64()
            }
        } else {
            kernel_crash(); // Could not get required info from Limine's memory map.
        }
    } else {
        kernel_crash(); // Could not get required info from Limine's higher-half direct mapping.
    }
}

#[cfg(target_arch = "aarch64")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kernel_main_aarch64(mapper: &mut AArchPageTable) -> ! {
    // All limine requests must also be referenced in a called function, otherwise they may be
    // removed by the linker.
    assert!(BASE_REVISION.is_supported());
    init(mapper);

    if let Some(framebuffer_response) = FRAMEBUFFER_REQUEST.get_response() {
        if let Some(limine_framebuffer) = framebuffer_response.framebuffers().next() {
            init_framebuffer(&limine_framebuffer);
            with_framebuffer(|fb| fb.setup_aarch64());
        } else {
            serial_print("no framebuffers found");
        }
    }

    init_serial_console();

    init_keyboard();

    if let Some(date_at_boot_response) = DATE_AT_BOOT_REQUEST.get_response() {
        TIMER.set_date_at_boot(date_at_boot_response.timestamp().as_secs());
    } else {
        println!("Could not get date at boot. Will default to 0.")
    }

    println!("Hello from Aarch64!");
    with_framebuffer(|fb| {
        with_serial_console(|sc| sc.render(fb));
        rectangle_filled(fb, 100, 100, 20, 20, rgb(255, 255, 255));
    });

    enable_interrupts();

    loop {}
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn kernel_main_x86_64() -> ! {
    // All limine requests must also be referenced in a called function, otherwise they may be
    // removed by the linker.
    assert!(BASE_REVISION.is_supported());

    if let Some(hhdm_response) = HHDM_REQUEST.get_response() {
        if let Some(memory_map_response) = MEMORY_MAP_REQUEST.get_response() {
            init(hhdm_response, memory_map_response);
        } else {
            kernel_crash(); // Could not get required info from Limine's memory map.
        }
    } else {
        kernel_crash(); // Could not get required info from Limine's higher-half direct mapping.
    }

    if let Some(framebuffer_response) = FRAMEBUFFER_REQUEST.get_response() {
        if let Some(limine_framebuffer) = framebuffer_response.framebuffers().next() {
            init_framebuffer(&limine_framebuffer);
            with_framebuffer(|fb| fb.setup_x86_64());
        } else {
            serial_print("no framebuffers found");
        }
    }

    init_serial_console();

    init_keyboard();

    if let Some(date_at_boot_response) = DATE_AT_BOOT_REQUEST.get_response() {
        TIMER.set_date_at_boot(date_at_boot_response.timestamp().as_secs());
    } else {
        println!("Could not get date at boot. Will default to 0.")
    }

    run_elf(INIT_ELF_BYTES, false);

    loop {}
}

struct BufWriter<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> BufWriter<'a> {
    fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }
}

impl core::fmt::Write for BufWriter<'_> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        let remaining = &mut self.buf[self.pos..];
        let len = bytes.len().min(remaining.len());
        remaining[..len].copy_from_slice(&bytes[..len]);
        self.pos += len;
        Ok(())
    }
}

#[panic_handler]
fn rust_panic(_info: &core::panic::PanicInfo) -> ! {
    serial_print("\nKERNEL PANIC:\n");
    let mut buf = [0u8; 512];
    let msg = {
        let mut w = BufWriter::new(&mut buf);
        let _ = core::fmt::write(&mut w, core::format_args!("{}", _info));
        let len = w.pos;
        core::str::from_utf8(&buf[..len]).unwrap_or("(utf8 error)")
    };
    serial_print(msg);
    serial_print("\n");
    with_framebuffer(|mut fb| {
        fb.clear(rgb(180, 0, 0));

        with_serial_console(|console| {
            console.clear();

            let mut writer = ConsoleWriter {
                fb: &mut fb,
                console,
                should_center: true,
            };

            let _ = writer.write_str("KERNEL PANIC\n\n");
            let _ = writer.write_fmt(core::format_args!("{}", _info));
        });
    });

    infinite_idle();
}
