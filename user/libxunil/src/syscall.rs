pub const READ: usize = 0;
pub const WRITE: usize = 1;
pub const OPEN: usize = 2;
pub const CLOSE: usize = 3;
pub const STAT: usize = 4;
pub const LSEEK: usize = 8;
pub const MMAP: usize = 9;
pub const MUNMAP: usize = 9;
pub const BRK: usize = 12;
pub const GETPID: usize = 39;
pub const FORK: usize = 57;
pub const EXECVE: usize = 59;
pub const EXIT: usize = 60;
pub const WAIT4: usize = 61;
pub const KILL: usize = 62;
pub const CHDIR: usize = 80;
pub const MKDIR: usize = 83;
pub const UNLINK: usize = 87;
pub const GETDENTS64: usize = 217;
pub const CLOCK_GETTIME: usize = 228;
pub const EXIT_GROUP: usize = 231;
pub const SLEEP: usize = 909090; // zzz haha
pub const DRAW_PIXEL: usize = 5555;
pub const DRAW_BUFFER: usize = 7777;
pub const FRAMEBUFFER_SWAP: usize = 6666;

#[inline(always)]
pub unsafe fn syscall0(num: usize) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("rax") num,
            lateout("rax") ret,
            clobber_abi("sysv64"),
            options(nostack)
        );
    }

    ret
}

#[inline(always)]
pub unsafe fn syscall1(num: usize, arg0: isize) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("rax") num,
            in("rdi") arg0,
            lateout("rax") ret,
            clobber_abi("sysv64"),
            options(nostack)
        );
    }

    ret
}

#[inline(always)]
pub unsafe fn syscall3(num: usize, arg0: isize, arg1: isize, arg2: isize) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("rax") num,
            in("rdi") arg0,
            in("rsi") arg1,
            in("rdx") arg2,
            lateout("rax") ret,
            clobber_abi("sysv64"),
            options(nostack)
        );
    }

    ret
}
