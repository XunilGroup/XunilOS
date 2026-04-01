#[inline(always)]
pub unsafe fn syscall0(num: usize) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("rax") num,
            lateout("rax") ret,
            options(nostack)
        );
    }

    ret
}

#[inline(always)]
pub unsafe fn syscall3(num: usize, arg0: usize, arg1: usize, arg2: usize) -> isize {
    let ret: isize;
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("rax") num,
            in("rdi") arg0,
            in("rsi") arg1,
            in("rdx") arg2,
            lateout("rax") ret,
            options(nostack)
        );
    }

    ret
}
