#[allow(unused_variables)]
pub fn enter_usermode_aarch64(entry: u64, stack_ptr: u64, should_swapgs: bool) -> ! {
    unsafe {
        core::arch::asm!(
            "msr sp_el0, {sp}",
            "msr elr_el1, {entry}",
            "msr spsr_el1, xzr",
            "isb",
            "eret",
            sp = in(reg) stack_ptr,
            entry = in(reg) entry,
            options(noreturn)
        )
    };
}
