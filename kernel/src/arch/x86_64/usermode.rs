use x86_64::{
    registers::segmentation::{DS, ES, Segment},
    structures::gdt::SegmentSelector,
};

use crate::arch::x86_64::gdt::{user_code_selector, user_data_selector};

fn with_rpl3(ss: SegmentSelector) -> u64 {
    (ss.0 as u64) | 3
}

// entry point and stack
pub fn enter_usermode_x86_64(user_rip: u64, user_rsp: u64) -> ! {
    let user_cs = with_rpl3(user_code_selector());
    let user_ss = with_rpl3(user_data_selector());

    unsafe {
        DS::set_reg(user_data_selector());
        ES::set_reg(user_data_selector());
    }

    let rflags: u64 = 0x202;

    unsafe {
        core::arch::asm!(
            "cli",
            "push {user_ss}",
            "push {user_rsp}",
            "push {rflags}",
            "push {user_cs}",
            "push {user_rip}",
            "iretq",

            user_ss = in(reg) user_ss,
            user_rsp = in(reg) user_rsp,
            rflags = in(reg) rflags,
            user_cs = in(reg) user_cs,
            user_rip = in(reg) user_rip,
            options(noreturn)
        );
    }
}
