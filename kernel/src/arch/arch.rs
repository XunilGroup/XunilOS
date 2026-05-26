#![allow(dead_code, unused_imports)]
#[cfg(target_arch = "x86_64")]
pub use crate::arch::x86_64::paging::FRAME_ALLOCATOR_X86_64 as FRAME_ALLOCATOR;
#[cfg(target_arch = "x86_64")]
use crate::arch::x86_64::{init::init_x86_64, usermode::enter_usermode_x86_64};
#[cfg(target_arch = "aarch64")]
use limine::response::ExecutableAddressResponse;
use spin::mutex::Mutex;
#[cfg(target_arch = "x86_64")]
use x86_64::{
    instructions::interrupts::without_interrupts,
    structures::paging::{FrameAllocator, OffsetPageTable, PhysFrame, Size4KiB},
};

#[cfg(target_arch = "aarch64")]
use aarch64_cpu::registers::{DAIF, Writeable};

#[cfg(target_arch = "aarch64")]
pub use crate::arch::aarch64::paging::FRAME_ALLOCATOR_AARCH64 as FRAME_ALLOCATOR;
#[cfg(target_arch = "aarch64")]
use crate::arch::aarch64::{init::init_aarch64, paging::AArchPageTable};

use crate::{
    config::TIMER_FREQUENCY_HZ,
    driver::timer::TIMER,
    task::scheduler::{CURRENT_PID, SCHEDULER},
    util::align_up,
};
use core::{
    arch::asm,
    sync::atomic::{AtomicU64, Ordering},
};
use limine::{
    memory_map::{Entry, EntryType},
    response::{HhdmResponse, MemoryMapResponse},
};

#[cfg(target_arch = "aarch64")]
const UART: *mut u8 = 0x0900_0000 as *mut u8;

pub static HHDM_OFFSET: AtomicU64 = AtomicU64::new(0);

#[cfg(target_arch = "aarch64")]
type PageTable = AArchPageTable;

#[cfg(target_arch = "x86_64")]
type PageTable<'a> = OffsetPageTable<'a>;

pub static mut KERNEL_MAPPER: Mutex<Option<PageTable>> = Mutex::new(None);

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct UsableRegion {
    pub base: u64,
    pub length: u64,
}

const EMPTY_REGION: UsableRegion = UsableRegion { base: 0, length: 0 };

// NOTE: dont change name to frameallocator as that is the name of the trait of x86_64
pub struct XunilFrameAllocator {
    pub hhdm_offset: u64,
    pub usable_regions: [UsableRegion; 1024],
    pub usable_region_count: usize,
    pub region_index: usize,
    pub region_offset: usize,
}

impl XunilFrameAllocator {
    pub const fn new() -> Self {
        Self {
            hhdm_offset: 0,
            usable_regions: [EMPTY_REGION; 1024],
            usable_region_count: 0,
            region_index: 0,
            region_offset: 0,
        }
    }

    pub fn initialize(&mut self, memory_map: &[&Entry]) {
        let mut regions = [EMPTY_REGION; 1024];
        let mut count = 0usize;

        for region in memory_map.iter().copied() {
            if region.entry_type != EntryType::USABLE {
                continue;
            }

            if count < regions.len() && region.length >= 4096 {
                let aligned_base = align_up(region.base, 4096);
                let base_offset = aligned_base - region.base;
                let aligned_length = region.length.saturating_sub(base_offset);
                if aligned_length >= 4096 {
                    regions[count] = UsableRegion {
                        base: aligned_base,
                        length: aligned_length,
                    };
                    count += 1;
                }
            }
        }

        self.hhdm_offset = HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
        self.usable_regions = regions;
        self.usable_region_count = count;
        self.region_index = 0;
        self.region_offset = 0;
    }
}

#[cfg(target_arch = "x86_64")]
pub fn init<'a>(
    hhdm_response: &HhdmResponse,
    memory_map_response: &'a MemoryMapResponse,
) -> OffsetPageTable<'a> {
    init_x86_64(hhdm_response, memory_map_response)
}

#[cfg(target_arch = "aarch64")]
pub fn init(mapper: &mut AArchPageTable) {
    init_aarch64(mapper);
}

pub static GLOBAL_TICK_COUNT: AtomicU64 = AtomicU64::new(0);

#[unsafe(no_mangle)]
pub extern "C" fn do_interrupt() {
    TIMER.interrupt();
    GLOBAL_TICK_COUNT.fetch_add(1, Ordering::Relaxed);
}

#[cfg(target_arch = "x86_64")]
pub fn safe_lock<R, F: FnOnce() -> R>(f: F) -> R {
    return without_interrupts(|| f());
}

#[cfg(target_arch = "aarch64")]
pub fn safe_lock<R, F: FnOnce() -> R>(f: F) -> R {
    let old_daif: u64;
    unsafe { core::arch::asm!("mrs {}, daif", out(reg) old_daif) };
    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
    DAIF.write(DAIF::D::Masked + DAIF::A::Masked + DAIF::I::Masked + DAIF::F::Masked);
    let r = f();
    unsafe { core::arch::asm!("msr daif, {}", in(reg) old_daif) };
    r
}

#[cfg(target_arch = "x86_64")]
pub fn serial_print_byte(b: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") b);
    }
}

#[cfg(target_arch = "aarch64")]
pub fn serial_print_byte(b: u8) {
    unsafe {
        let buf = [b];
        core::arch::asm!(
            "hlt #0xF000",
            in("x0") 0x03u64,  // SYS_WRITEC
            in("x1") buf.as_ptr(),
            options(nostack)
        );
    }
}

pub fn serial_print(s: &str) {
    for &b in s.as_bytes() {
        serial_print_byte(b);
    }
}

pub fn idle() {
    unsafe {
        #[cfg(target_arch = "x86_64")]
        asm!("hlt");
        #[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
        asm!("wfi");
        #[cfg(target_arch = "loongarch64")]
        asm!("idle 0");
    }
}

pub fn infinite_idle() -> ! {
    loop {
        idle()
    }
}

#[inline(always)]
pub fn kernel_crash() -> ! {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("ud2")
    };

    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!("udf #0")
    };

    #[cfg(target_arch = "riscv64")]
    unsafe {
        core::arch::asm!("unimp")
    };

    loop {} // satisfies -> ! on unknown archs
}
