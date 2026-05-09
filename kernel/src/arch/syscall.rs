#![allow(dead_code)]

use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::null_mut,
};

use x86_64::{
    VirtAddr,
    instructions::interrupts,
    structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB},
};

use crate::{
    arch::arch::{FRAME_ALLOCATOR, get_allocator, infinite_idle, sleep},
    driver::{
        fs::vfs::{vfs_close, vfs_lseek, vfs_open, vfs_read},
        graphics::framebuffer::with_framebuffer,
        keyboard::{KeyboardEvent, process_scancodes},
        timer::TIMER,
    },
    mm::usercopy::{copy_cstr_from_user, copy_to_user},
    print, println,
    task::scheduler::{SCHEDULER, current_pid},
    util::align_up,
};

const READ: usize = 0;
const WRITE: usize = 1;
const OPEN: usize = 2;
const CLOSE: usize = 3;
const STAT: usize = 4;
const LSEEK: usize = 8;
const MMAP: usize = 9;
const MUNMAP: usize = 11;
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
const KBD_READ: usize = 666;
const SLEEP: usize = 909090; // zzz haha
pub const MAP_FRAMEBUFFER: usize = 5555;
pub const FRAMEBUFFER_SWAP: usize = 6666;

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

fn map_framebuffer() -> isize {
    0
}

fn read(ptr: isize, size: isize, nmemb: isize, fd: isize) -> isize {
    let pid = current_pid().unwrap_or(0);
    if pid == 0 {
        return -1;
    }

    SCHEDULER
        .with_process(pid, |process| {
            let len = size * nmemb;
            let to_read = vfs_read(fd as i64, len as usize);

            if let Some(read_ptr) = to_read {
                let address_space = process.address_space.as_mut().unwrap();
                if copy_to_user(
                    &mut address_space.mapper,
                    ptr as *mut u8,
                    read_ptr.0,
                    read_ptr.1,
                )
                .is_err()
                {
                    return -1;
                };

                return (read_ptr.1 as isize) / size;
            } else {
                return -1;
            }
        })
        .unwrap_or(-1)
}

fn open(path: isize, mode: isize) -> isize {
    let pid = current_pid().unwrap_or(0);
    if pid == 0 {
        return -1;
    }

    SCHEDULER
        .with_process(pid, |process| {
            let address_space = process.address_space.as_mut().unwrap();
            let path = copy_cstr_from_user(&mut address_space.mapper, path as *const u8, 256)?;
            let mode = copy_cstr_from_user(&mut address_space.mapper, mode as *const u8, 16)?;

            Ok::<isize, isize>(vfs_open(&path, &mode) as isize)
        })
        .unwrap_or(Err(-1))
        .unwrap_or(-1)
}

fn close(fd: isize) -> isize {
    vfs_close(fd as i64) as isize
}

fn kbd_read(user_ptr: *mut KeyboardEvent, max_events: isize) -> isize {
    process_scancodes();
    if max_events <= 0 || user_ptr.is_null() {
        return -1;
    }

    let pid = current_pid().unwrap_or(0);

    if pid == 0 {
        return -1;
    }

    return SCHEDULER
        .with_process(pid as u64, |process| {
            let to_copy = (max_events as usize).min(process.kbd_buffer.len());
            let address_space = process.address_space.as_mut().unwrap();
            if let Ok(_) = copy_to_user(
                &mut address_space.mapper,
                user_ptr as *mut u8,
                process.kbd_buffer.as_ptr() as *const u8,
                to_copy * size_of::<KeyboardEvent>(),
            ) {
                process.kbd_buffer.drain(0..to_copy);
                return to_copy as isize;
            } else {
                return -1;
            };
        })
        .unwrap_or(-1);
}

pub unsafe fn sbrk(increment: isize) -> isize {
    let pid = current_pid().unwrap_or(0);

    if pid == 0 {
        return -1;
    }

    let mut frame_allocator = FRAME_ALLOCATOR.lock();
    return SCHEDULER
        .with_process(pid as u64, |process| {
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
            if new > stack_top - 16384 * 4096 {
                // 67 mib max
                return -1;
            }

            if new > old {
                let map_start = align_up(old, 4096);
                let map_end = align_up(new, 4096);

                for addr in (map_start..map_end).step_by(4096) {
                    if let Some(frame) = frame_allocator.allocate_frame() {
                        // TODO: do not use x86_64 only
                        let virt_addr = VirtAddr::new(addr);
                        let page = Page::<Size4KiB>::containing_address(virt_addr);
                        let address_space = process.address_space.as_mut().unwrap();
                        unsafe {
                            address_space
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

                            core::ptr::write_bytes(virt_addr.as_mut_ptr::<u8>(), 0, 4096);
                        }
                    } else {
                        return -1;
                    }
                }
            }
            drop(frame_allocator);

            process.heap_end = new;

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
    arg3: isize,
    arg4: isize,
    arg5: isize,
) -> isize {
    interrupts::enable();
    match num {
        BRK => unsafe { sbrk(arg0) },
        READ => read(arg0, arg1, arg2, arg3) as isize,
        WRITE => {
            let buf_ptr = arg1 as *const u8;
            let len = arg2 as usize;
            let bytes: &[u8] = unsafe { core::slice::from_raw_parts(buf_ptr, len) };
            if let Ok(s) = core::str::from_utf8(bytes) {
                print!("{}", s);
            } else {
                for byte in bytes {
                    if *byte == b'\0' {
                        continue;
                    }
                    print!("{}", *byte as char);
                }
            }

            with_framebuffer(|fb| fb.swap());
            0
        }
        OPEN => open(arg0, arg1),
        CLOSE => close(arg0),
        LSEEK => vfs_lseek(arg0 as i64, arg1 as i64, arg2 as i32) as isize,
        EXIT => {
            println!("Program exit: {}", arg0);
            with_framebuffer(|fb| fb.swap());
            infinite_idle();
        }
        SLEEP => {
            sleep(arg0 as u64);
            0
        }
        CLOCK_GETTIME => TIMER.now().elapsed() as isize,
        MAP_FRAMEBUFFER => map_framebuffer(),
        KBD_READ => kbd_read(arg0 as *mut KeyboardEvent, arg1),
        FRAMEBUFFER_SWAP => {
            with_framebuffer(|fb| {
                fb.swap();
            });
            0
        }
        _ => -38, // syscall not found
    }
}

pub type Fd = i32;
pub type Off = i64;
