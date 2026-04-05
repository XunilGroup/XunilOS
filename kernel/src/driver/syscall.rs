use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::null_mut,
};

use x86_64::{
    VirtAddr,
    structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB},
};

use crate::{
    arch::arch::{FRAME_ALLOCATOR, get_allocator, infinite_idle},
    driver::graphics::framebuffer::with_framebuffer,
    println,
    task::scheduler::SCHEDULER,
    util::{align_up, serial_print},
};

const READ: usize = 0;
const WRITE: usize = 1;
const OPEN: usize = 2;
const CLOSE: usize = 3;
const STAT: usize = 4;
const LSEEK: usize = 8;
const MMAP: usize = 9;
const MUNMAP: usize = 9;
const BRK: usize = 12;
const GETPID: usize = 39;
const FORK: usize = 57;
const EXECVE: usize = 59;
const EXIT: usize = 60;
const WAIT4: usize = 61;
const KILL: usize = 62;
const CHDIR: usize = 80;
const MKDIR: usize = 83;
const UNLINK: usize = 87;
const GETDENTS64: usize = 217;
const CLOCK_GETTIME: usize = 228;
const EXIT_GROUP: usize = 231;

pub unsafe fn malloc(size: usize, align: usize) -> *mut u8 {
    let align = if align < 1 {
        1
    } else {
        align.next_power_of_two()
    };
    let layout = match Layout::from_size_align(size, align) {
        Ok(l) => l,
        Err(_) => return null_mut(),
    };

    unsafe { GlobalAlloc::alloc(get_allocator(), layout) }
}

pub unsafe fn free(ptr: *mut u8, size: usize, align: usize) {
    if ptr.is_null() {
        // very important, do not double free
        return;
    }

    let align = if align < 1 {
        1
    } else {
        align.next_power_of_two()
    };

    if let Ok(layout) = Layout::from_size_align(size.max(1), align.max(1)) {
        unsafe { GlobalAlloc::dealloc(get_allocator(), ptr, layout) }
    }
}

pub unsafe fn memset(ptr: *mut u8, val: u8, count: usize) {
    unsafe { core::ptr::write_bytes(ptr, val, count) };
}

pub unsafe fn sbrk(increment: isize) -> isize {
    serial_print("sbrk called");
    let mut scheduler = SCHEDULER.lock();
    if scheduler.current_process == -1 {
        return -1;
    }
    let pid = scheduler.current_process as u64;
    drop(scheduler);

    let mut frame_allocator = FRAME_ALLOCATOR.lock();
    return SCHEDULER
        .with_process(pid as u64, |mut process| {
            let (heap_end, heap_base, stack_top) =
                (process.heap_end, process.heap_base, process.stack_top);

            let old = heap_end;
            let new = if increment >= 0 {
                old.checked_add(increment as u64)
            } else {
                let dec = increment.unsigned_abs() as u64;
                old.checked_sub(dec)
            }
            .unwrap_or(old);

            if new < heap_base {
                return -1;
            }
            if new > stack_top - 3 * 4096 {
                return -1;
            }
            if new > old {
                let map_start = align_up(old, 4096);
                let map_end = align_up(new, 4096);

                for addr in (map_start..map_end).step_by(4096) {
                    let frame = frame_allocator.allocate_frame().unwrap();

                    // TODO: do not use x86_64 only
                    let virt_addr = VirtAddr::new(addr);
                    let page = Page::<Size4KiB>::containing_address(virt_addr);
                    unsafe {
                        process
                            .address_space
                            .mapper
                            .map_to(
                                page,
                                frame,
                                PageTableFlags::PRESENT
                                    | PageTableFlags::WRITABLE
                                    | PageTableFlags::USER_ACCESSIBLE
                                    | PageTableFlags::NO_EXECUTE,
                                &mut *frame_allocator,
                            )
                            .unwrap()
                            .flush();
                    }
                }
            }
            drop(frame_allocator);

            process.heap_end = new;

            serial_print("sbrk finished");

            return old as isize;
        })
        .unwrap_or(-1);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn syscall_dispatch(
    num: usize,
    arg0: isize,
    arg1: isize,
    arg2: isize,
) -> isize {
    match num {
        SYS_BRK => sbrk(arg0),
        SYS_WRITE => {
            let buf_ptr = arg1 as *const u8;
            let len = arg2 as usize;
            let bytes: &[u8] = unsafe { core::slice::from_raw_parts(buf_ptr, len) };
            let s = core::str::from_utf8(bytes).unwrap_or("<non-utf8>");
            println!("SYS_WRITE called: {:?} {:?}", s, len);
            with_framebuffer(|fb| fb.swap());
            0
        }
        SYS_EXIT => {
            println!("Program exit.");
            with_framebuffer(|fb| fb.swap());
            infinite_idle();
        }
        _ => -38, // syscall not found
    }
}

pub type Fd = i32;
pub type Off = i64;
