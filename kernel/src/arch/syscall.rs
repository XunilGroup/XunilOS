#![allow(dead_code, unused_imports)]
use core::cmp::Reverse;
use core::sync::atomic::Ordering;

#[cfg(target_arch = "x86_64")]
use crate::arch::x86_64::paging::create_and_map_multiple_pages;
use crate::driver::io::fs::vfs::vfs_write;
use crate::driver::io::input::{InputEvent, process_input};
#[cfg(target_arch = "x86_64")]
use crate::driver::io::ps2::process_scancodes;
use crate::mm::usercopy::copy_from_user;
use alloc::vec;
use alloc::{string::String, vec::Vec};
#[cfg(target_arch = "x86_64")]
use x86_64::{
    PhysAddr, VirtAddr,
    instructions::interrupts,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTableFlags, PhysFrame, Size4KiB,
    },
};

#[cfg(target_arch = "aarch64")]
use crate::arch::aarch64::paging::AArchPageTable;
use crate::{
    arch::arch::{FRAME_ALLOCATOR, GLOBAL_TICK_COUNT, serial_print},
    config::TIMER_FREQUENCY_HZ,
    driver::{
        elf::loader::run_elf,
        framebuffer::{FRAMEBUFFER, USER_FB_BASE, with_framebuffer},
        io::fs::vfs::{vfs_close, vfs_lseek, vfs_open, vfs_read},
        ipc::{Permissions, create_port, manage_port, read_port, write_port},
        timer::TIMER,
    },
    mm::{
        shm::{NEXT_SHM_ID, SHM_REGISTRY, SHM_SLOT_SIZE, SharedMemory, USER_SHM_BASE},
        usercopy::copy_cstr_to_user,
    },
    print, println,
    task::{
        process::ProcessState,
        scheduler::{SCHEDULER, current_pid},
    },
    util::{U64Buf, align_down, align_up},
};

use crate::{
    arch::arch::safe_lock,
    mm::usercopy::{copy_cstr_from_user, copy_to_user},
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
const INPUT_READ: usize = 666;
const SLEEP: usize = 909090; // zzz haha
const IPC_CREATE: usize = 500;
const IPC_READ: usize = 501;
const IPC_WRITE: usize = 502;
const IPC_MANAGE: usize = 503;
const SHM_OPEN: usize = 600;
pub const MAP_FRAMEBUFFER: usize = 5555;
pub const FRAMEBUFFER_SWAP: usize = 6666;

#[cfg(target_arch = "aarch64")]
type PageTable = AArchPageTable;

#[cfg(target_arch = "x86_64")]
type PageTable<'a> = OffsetPageTable<'a>;

fn map_framebuffer() -> isize {
    let pid = current_pid().unwrap_or(0);
    if pid != 1 {
        return -1;
    }

    let framebuffer = FRAMEBUFFER.lock();
    let fb = match framebuffer.as_ref() {
        Some(fb) => fb,
        None => return -1,
    };

    let struct_phys = fb.meta.struct_phys;
    let buf_phys = fb.meta.buf_phys;
    let buf_size = (fb.meta.buf_len * size_of::<u32>()) as u64;
    let pixel_map_start = align_down(buf_phys, 4096);
    let pixel_map_end = align_up(buf_phys + buf_size, 4096);
    drop(framebuffer);

    SCHEDULER
        .with_process(pid, |process| {
            let address_space = match process.address_space.as_mut() {
                Some(a) => a,
                None => return -1,
            };
            #[allow(dead_code, unused_mut, unused_variables)]
            let mut map_page = |virt: u64, phys: u64| {};
            #[cfg(target_arch = "x86_64")]
            let mut map_page = |virt: u64, phys: u64| unsafe {
                let frame = PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(phys));
                let page = Page::<Size4KiB>::containing_address(VirtAddr::new(virt));
                safe_lock(|| {
                    let mut frame_allocator = FRAME_ALLOCATOR.lock();

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

                    drop(frame_allocator);
                });
            };
            #[cfg(target_arch = "aarch64")]
            let map_page = |virt: u64, phys: u64| {
                use crate::arch::aarch64::paging::user_data_flags;

                address_space.mapper.map_page(virt, phys, user_data_flags());
            };

            map_page(USER_FB_BASE, struct_phys);

            for offset in (0..pixel_map_end - pixel_map_start).step_by(4096) {
                map_page(USER_FB_BASE + 0x1000 + offset, pixel_map_start + offset);
            }

            0
        })
        .unwrap_or(-1)
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
                if let Some(address_space) = process.address_space.as_mut() {
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
            let address_space = process.address_space.as_mut().ok_or::<isize>(-1)?;
            let path = copy_cstr_from_user(&mut address_space.mapper, path as *const u8, 256)?;
            let mode = copy_cstr_from_user(&mut address_space.mapper, mode as *const u8, 16)?;

            Ok::<isize, isize>(vfs_open(&path, &mode) as isize)
        })
        .unwrap_or(Err(-1))
        .unwrap_or(-1)
}

fn write_fd(ptr: isize, size: isize, count: isize, fd: isize) -> isize {
    let pid = current_pid().unwrap_or(0);
    if pid == 0 {
        return -1;
    }

    SCHEDULER
        .with_process(pid, |process| {
            let len = (size as usize).checked_mul(count as usize).ok_or(-1isize)?;
            if len == 0 {
                return Ok(0isize);
            }

            let address_space = process.address_space.as_mut().ok_or(-1isize)?;

            let mut buf: alloc::vec::Vec<u8> = alloc::vec![0u8; len];
            copy_from_user(
                &mut address_space.mapper,
                buf.as_mut_ptr(),
                ptr as *const u8,
                len,
            )
            .map_err(|_| -14isize)?;

            Ok(
                vfs_write(buf.as_ptr(), size as usize, count as usize, fd as i64).unwrap_or(0)
                    as isize,
            )
        })
        .unwrap_or(Err(-1))
        .unwrap_or(-1)
}

fn close(fd: isize) -> isize {
    vfs_close(fd as i64) as isize
}

fn input_read(user_ptr: *mut InputEvent, max_events: isize) -> isize {
    let pid = current_pid().unwrap_or(0);

    if pid != 1 {
        return -1;
    }

    if max_events <= 0 || user_ptr.is_null() {
        return -1;
    }

    return SCHEDULER
        .with_process(pid as u64, |process| {
            let to_copy = (max_events as usize).min(process.input_buffer.len());
            if let Some(address_space) = process.address_space.as_mut() {
                if let Ok(_) = copy_to_user(
                    &mut address_space.mapper,
                    user_ptr as *mut u8,
                    process.input_buffer.as_ptr() as *const u8,
                    to_copy * size_of::<InputEvent>(),
                ) {
                    process.input_buffer.drain(0..to_copy);

                    return to_copy as isize;
                } else {
                    return -1;
                };
            } else {
                return -1;
            }
        })
        .unwrap_or(-1);
}

pub unsafe fn sbrk(increment: isize) -> isize {
    let pid = current_pid().unwrap_or(0);

    if pid == 0 {
        return -1;
    }

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
            if new > stack_top - 65535 * 4096 {
                // 262 mib max
                return -1;
            }

            if new > old {
                let map_start = align_up(old, 4096);
                let page_count = (align_up(new, 4096) - map_start) / 4096;
                if let Some(address_space) = process.address_space.as_mut() {
                    #[cfg(target_arch = "x86_64")]
                    create_and_map_multiple_pages(
                        &mut address_space.mapper,
                        page_count,
                        map_start,
                        PageTableFlags::PRESENT
                            | PageTableFlags::WRITABLE
                            | PageTableFlags::USER_ACCESSIBLE
                            | PageTableFlags::NO_EXECUTE,
                    );
                    #[cfg(target_arch = "aarch64")]
                    {
                        use crate::arch::aarch64::paging::{
                            create_and_map_multiple_pages, user_data_flags,
                        };

                        create_and_map_multiple_pages(
                            &mut address_space.mapper,
                            page_count,
                            map_start,
                            user_data_flags(),
                        );
                    }
                } else {
                    return -1;
                }
            }

            process.heap_end = new;

            return old as isize;
        })
        .unwrap_or(-1);
}

pub fn exec(arg0: isize) -> isize {
    let pid = current_pid().unwrap_or(0);
    if pid == 0 {
        return -1;
    }

    let path = match SCHEDULER.with_process(pid, |process| {
        let address_space = process.address_space.as_mut().ok_or::<isize>(-1)?;
        copy_cstr_from_user(&mut address_space.mapper, arg0 as *const u8, 256)
    }) {
        Some(Ok(p)) => p,
        _ => return -1,
    };

    let fd = vfs_open(path.as_str(), "r");
    if fd < 0 {
        return -1;
    }

    const SEEK_SET: i32 = 0;
    const SEEK_CUR: i32 = 1;
    const SEEK_END: i32 = 2;

    if vfs_lseek(fd, 0, SEEK_END) < 0 {
        vfs_close(fd);
        return -1;
    }
    let size = vfs_lseek(fd, 0, SEEK_CUR);
    if size <= 0 {
        vfs_close(fd);
        return -1;
    }
    if vfs_lseek(fd, 0, SEEK_SET) < 0 {
        vfs_close(fd);
        return -1;
    }

    let mut buf = vec![0u8; size as usize];
    let mut off = 0usize;

    while off < buf.len() {
        let want = buf.len() - off;
        let Some((src, n)) = vfs_read(fd, want) else {
            break;
        };
        if n == 0 {
            break;
        }
        unsafe {
            core::ptr::copy_nonoverlapping(src, buf.as_mut_ptr().add(off), n);
        }
        off += n;
    }

    vfs_close(fd);

    if off != buf.len() {
        return -1;
    }

    run_elf(&buf, true, false);
    0
}

pub fn kill(pid: isize, exit_code: isize) -> isize {
    let pid = pid as u64;
    println!("PID {} Exited.", pid);

    SCHEDULER.terminate_process(pid, exit_code);

    if pid == current_pid().unwrap_or(0) {
        SCHEDULER.switch_next(false);
        crate::arch::arch::infinite_idle();
    } else {
        0
    }
}

fn sleep(ticks: isize) -> isize {
    let ticks = (ticks as usize) * TIMER_FREQUENCY_HZ / 1000;

    let pid = current_pid().unwrap_or(0);

    if pid == 0 {
        return 0;
    }

    safe_lock(|| {
        let mut guard = SCHEDULER.lock();
        if let Some(process) = guard.processes.get_mut(&pid) {
            let wake_at = TIMER.now().elapsed() + ticks as u64;
            process.info.wake_tick = Some(wake_at);
            process.state = ProcessState::Blocked;
            process.in_ready_queue = false;
            guard.sleep_queue.push(Reverse((wake_at, pid)));
        };
    });

    0
}

fn ipc_create(name_ptr: isize, default_permissions: isize) -> isize {
    let pid = current_pid().unwrap_or(0);

    if pid == 0 {
        return 0;
    }

    let default_permissions = Permissions::from_bits_retain(default_permissions as u32);

    let name = match SCHEDULER.with_process(pid, |process| {
        let address_space = process.address_space.as_mut().ok_or::<isize>(-1)?;
        copy_cstr_from_user(&mut address_space.mapper, name_ptr as *const u8, 256)
    }) {
        Some(Ok(p)) => p,
        _ => return -1,
    };

    create_port(name, pid, default_permissions);

    0
}

fn ipc_read(name_ptr: isize, output_ptr: isize, from: isize) -> isize {
    let pid = current_pid().unwrap_or(0);
    if pid == 0 {
        return 0;
    }
    let name = match SCHEDULER.with_process(pid, |process| {
        let address_space = process.address_space.as_mut().ok_or::<isize>(-1)?;
        copy_cstr_from_user(&mut address_space.mapper, name_ptr as *const u8, 256)
    }) {
        Some(Ok(p)) => p,
        _ => return 0,
    };
    return match read_port(name, pid, from as i64) {
        Some(message) => {
            let sender = message.from as isize;
            SCHEDULER
                .with_process(pid, |process| {
                    process
                        .address_space
                        .as_mut()
                        .map(|address_space| {
                            copy_cstr_to_user(
                                &mut address_space.mapper,
                                message.content,
                                output_ptr as *mut u8,
                            )
                        })
                        .unwrap_or(Ok(()))
                })
                .unwrap_or(Ok(()))
                .map(|_| sender)
                .unwrap_or(0)
        }
        None => 0,
    };
}

fn ipc_write(name_ptr: isize, message_ptr: isize) -> isize {
    let pid = current_pid().unwrap_or(0);

    if pid == 0 {
        return 0;
    }

    let (name, message) = match SCHEDULER.with_process(pid, |process| {
        let address_space = process.address_space.as_mut().ok_or::<isize>(-1)?;
        let name = copy_cstr_from_user(&mut address_space.mapper, name_ptr as *const u8, 256)?;
        let message =
            copy_cstr_from_user(&mut address_space.mapper, message_ptr as *const u8, 256)?;
        Ok::<(String, String), isize>((name, message))
    }) {
        Some(Ok((name, message))) => (name, message),
        _ => return -1,
    };

    return match write_port(name, pid, message) {
        true => 0,
        _ => -1,
    };
}

fn ipc_manage(name_ptr: isize, pid_to_set: isize, permissions: isize) -> isize {
    let pid = current_pid().unwrap_or(0);

    if pid == 0 {
        return 0;
    }

    let permissions = Permissions::from_bits_retain(permissions as u32);

    let name = match SCHEDULER.with_process(pid, |process| {
        let address_space = process.address_space.as_mut().ok_or::<isize>(-1)?;
        copy_cstr_from_user(&mut address_space.mapper, name_ptr as *const u8, 256)
    }) {
        Some(Ok(p)) => p,
        _ => return -1,
    };

    return match manage_port(name, pid, pid_to_set as u64, permissions) {
        true => 0,
        _ => -1,
    };
}

fn shm_open(name_ptr: isize, size: isize) -> isize {
    if size > (SHM_SLOT_SIZE as f64 * 0.8) as isize {
        return -1;
    }

    let pid = current_pid().unwrap_or(0);

    if pid == 0 {
        return 0;
    }

    SCHEDULER
        .with_process(pid, |process| -> Result<isize, isize> {
            let address_space = process.address_space.as_mut().ok_or::<isize>(-1)?;

            let name = copy_cstr_from_user(&mut address_space.mapper, name_ptr as *const u8, 256)?;

            #[allow(dead_code, unused_mut, unused_variables)]
            let mut map_page = |virt: u64, phys: u64| {};

            #[cfg(target_arch = "x86_64")]
            let mut map_page = |virt: u64, phys: u64| unsafe {
                let frame = PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(phys));

                let page = Page::<Size4KiB>::containing_address(VirtAddr::new(virt));

                safe_lock(|| {
                    let mut frame_allocator = FRAME_ALLOCATOR.lock();

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
                });
            };

            #[cfg(target_arch = "aarch64")]
            let map_page = |virt: u64, phys: u64| {
                use crate::arch::aarch64::paging::user_data_flags;

                address_space.mapper.map_page(virt, phys, user_data_flags());
            };

            let size = size as usize;

            let mut shm_registry_opt = SHM_REGISTRY.lock();

            let shm_registry = shm_registry_opt
                .as_mut()
                .expect("Could not get SHM registry");

            if let Some(shared_memory) = shm_registry.get(&name) {
                for (i, page) in shared_memory.phys_pages.iter().enumerate() {
                    map_page(
                        USER_SHM_BASE + shared_memory.id * SHM_SLOT_SIZE + i as u64 * 4096,
                        *page,
                    );
                }
                #[cfg(target_arch = "aarch64")]
                crate::arch::aarch64::paging::tlb_flush();

                Ok(shared_memory.id as isize)
            } else {
                let mut page_count = align_up(size as u64, 4096) / 4096;
                if size as u64 % 4096 == 0 {
                    // Map one extra page to tolerate end-boundary reads.
                    page_count += 1;
                }

                let mut phys_pages = Vec::new();

                #[cfg(target_arch = "aarch64")]
                for _ in 0..page_count {
                    use crate::arch::aarch64::paging::alloc_frame;

                    phys_pages.push(alloc_frame().expect("Could not allocate frame"));
                }

                #[cfg(target_arch = "x86_64")]
                for _ in 0..page_count {
                    safe_lock(|| {
                        let mut frame_allocator = FRAME_ALLOCATOR.lock();

                        phys_pages.push(
                            frame_allocator
                                .allocate_frame()
                                .expect("Could not allocate frame")
                                .start_address()
                                .as_u64(),
                        );
                    });
                }

                let id = NEXT_SHM_ID.fetch_add(1, Ordering::Relaxed);

                shm_registry.insert(name.clone(), SharedMemory { id, phys_pages });

                let shared_memory = shm_registry.get(&name.clone()).ok_or::<isize>(-1)?;
                for (i, page) in shared_memory.phys_pages.iter().enumerate() {
                    map_page(
                        USER_SHM_BASE + shared_memory.id * SHM_SLOT_SIZE + i as u64 * 4096,
                        *page,
                    );
                }
                #[cfg(target_arch = "aarch64")]
                crate::arch::aarch64::paging::tlb_flush();

                Ok(shared_memory.id as isize)
            }
        })
        .unwrap_or(Err(-1))
        .unwrap_or(-1)
}

#[allow(unused_variables)]
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
    match num {
        BRK => unsafe { sbrk(arg0) },
        READ => read(arg0, arg1, arg2, arg3) as isize,
        WRITE => {
            if arg0 == 1 {
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
            } else {
                write_fd(arg0, arg1, arg2, arg3);
            }

            0
        }
        OPEN => open(arg0, arg1),
        CLOSE => close(arg0),
        LSEEK => vfs_lseek(arg0 as i64, arg1 as i64, arg2 as i32) as isize,
        EXIT => kill(current_pid().unwrap() as isize, arg0),
        SLEEP => sleep(arg0),
        EXECVE => exec(arg0),
        CLOCK_GETTIME => {
            (TIMER.now().elapsed() + TIMER.get_date_at_boot() * TIMER_FREQUENCY_HZ as u64) as isize
        }
        MAP_FRAMEBUFFER => map_framebuffer(),
        INPUT_READ => input_read(arg0 as *mut InputEvent, arg1),
        GETPID => {
            let pid = current_pid().unwrap_or(0);

            match pid {
                0 => return -1,
                _ => return pid as isize,
            }
        }
        IPC_CREATE => ipc_create(arg0, arg1),
        IPC_READ => ipc_read(arg0, arg1, arg2),
        IPC_WRITE => ipc_write(arg0, arg1),
        IPC_MANAGE => ipc_manage(arg0, arg1, arg2),
        SHM_OPEN => shm_open(arg0, arg1),
        KILL => kill(arg0, -9),
        FRAMEBUFFER_SWAP => {
            with_framebuffer(|fb| fb.present());
            0
        }

        _ => -38, // syscall not found
    }
}

pub type Fd = i32;
pub type Off = i64;
