use core::{arch::global_asm, sync::atomic::Ordering};

use crate::{
    arch::syscall::{check_and_reschedule, syscall_dispatch},
    config::TIMER_FREQUENCY_HZ,
    driver::{
        graphics::framebuffer::with_framebuffer,
        io::virtio::{KEYBOARD_SLOT, MOUSE_SLOT, input::input_interrupt},
        serial::with_serial_console,
        timer::TIMER,
    },
    task::context::{UserContext, ctx_save},
};

global_asm!(
    r#"
  .section .text.vectors
  .global __exception_vectors
  .balign 0x800

  __exception_vectors:
  .balign 0x80; ldr x16, =current_el_sp0_sync; br x16
  .balign 0x80; ldr x16, =current_el_sp0_irq; br x16
  .balign 0x80; ldr x16, =current_el_sp0_fiq; br x16
  .balign 0x80; ldr x16, =current_el_sp0_serror; br x16

  .balign 0x80; ldr x16, =current_el_spx_sync; br x16
  .balign 0x80; ldr x16, =current_el_spx_irq; br x16
  .balign 0x80; ldr x16, =current_el_spx_fiq; br x16
  .balign 0x80; ldr x16, =current_el_spx_serror; br x16

  .balign 0x80; ldr x16, =lower_el_aarch64_sync; br x16
  .balign 0x80; ldr x16, =lower_el_aarch64_irq; br x16
  .balign 0x80; ldr x16, =lower_el_aarch64_fiq; br x16
  .balign 0x80; ldr x16, =lower_el_aarch64_serror; br x16

  .balign 0x80; ldr x16, =lower_el_aarch32_sync; br x16
  .balign 0x80; ldr x16, =lower_el_aarch32_irq; br x16
  .balign 0x80; ldr x16, =lower_el_aarch32_fiq; br x16
  .balign 0x80; ldr x16, =lower_el_aarch32_serror; br x16
  "#
);

fn timer_ticks_per_irq() -> u64 {
    let cntfrq: u64;
    unsafe { core::arch::asm!("mrs {}, cntfrq_el0", out(reg) cntfrq) };
    cntfrq / (TIMER_FREQUENCY_HZ as u64)
}

// stp allows storing in pairs, ldp loads in pairs
macro_rules! exception_handler {
    ($name:ident, $rust_handler:ident) => {
        #[unsafe(naked)]
        #[unsafe(no_mangle)]
        unsafe extern "C" fn $name() {
            core::arch::naked_asm!(
                "sub sp, sp, #288",
                "stp x0, x1, [sp, #0]",
                "stp x2, x3, [sp, #16]",
                "stp x4, x5, [sp, #32]",
                "stp x6, x7, [sp, #48]",
                "stp x8, x9, [sp, #64]",
                "stp x10, x11, [sp, #80]",
                "stp x12, x13, [sp, #96]",
                "stp x14, x15, [sp, #112]",
                "stp x16, x17, [sp, #128]",
                "stp x18, x19, [sp, #144]",
                "stp x20, x21, [sp, #160]",
                "stp x22, x23, [sp, #176]",
                "stp x24, x25, [sp, #192]",
                "stp x26, x27, [sp, #208]",
                "stp x28, x29, [sp, #224]",
                "mrs x2, sp_el0",
                "str x2, [sp, #280]",
                "mrs x0, elr_el1",
                "mrs x1, spsr_el1",
                "stp x30, x0, [sp, #240]",
                "str x1, [sp, #256]",
                "mrs x2, esr_el1",
                "mrs x3, far_el1",
                "stp x2, x3, [sp, #264]",
                "mov x0, sp",
                concat!("bl ", stringify!($rust_handler)),
                "ldr x2, [sp, #280]",
                "msr sp_el0, x2",
                "ldr x1, [sp, #256]",
                "ldp x30, x0, [sp, #240]",
                "msr spsr_el1, x1",
                "msr elr_el1, x0",
                "ldp x0, x1, [sp, #0]",
                "ldp x2, x3, [sp, #16]",
                "ldp x4, x5, [sp, #32]",
                "ldp x6, x7, [sp, #48]",
                "ldp x8, x9, [sp, #64]",
                "ldp x10, x11, [sp, #80]",
                "ldp x12, x13, [sp, #96]",
                "ldp x14, x15, [sp, #112]",
                "ldp x16, x17, [sp, #128]",
                "ldp x18, x19, [sp, #144]",
                "ldp x20, x21, [sp, #160]",
                "ldp x22, x23, [sp, #176]",
                "ldp x24, x25, [sp, #192]",
                "ldp x26, x27, [sp, #208]",
                "ldp x28, x29, [sp, #224]",
                "add sp, sp, #288",
                "eret"
            );
        }
    };
}

unsafe fn use_exception_vectors() {
    unsafe {
        core::arch::asm!(
            "adrp x0, __exception_vectors",
            "add x0, x0, :lo12:__exception_vectors",
            "msr vbar_el1, x0",
            "isb"
        );
    }
}

const GICD_BASE: u64 = 0xFFFF_0000_0800_0000;
const GICC_BASE: u64 = 0xFFFF_0000_0801_0000;
const ESR_EC_SHIFT: u64 = 26;
const ESR_EC_MASK: u64 = 0x3F;
const EC_SVC_AA64: u64 = 0x15;

unsafe fn gic_init() {
    let gicd_ctrl = GICD_BASE as *mut u32;
    unsafe {
        gicd_ctrl.write_volatile(1);
    }

    let gicc_ctrl = GICC_BASE as *mut u32;
    unsafe {
        gicc_ctrl.write_volatile(1);
    }

    let gicc_pmr = (GICC_BASE + 0x4) as *mut u32;
    unsafe {
        gicc_pmr.write_volatile(0xFF); // allow all priorities
    }
}

unsafe fn gic_enable_interrupt(id: u32) {
    let reg = (GICD_BASE + 0x100 + (id as u64 / 32 * 4)) as *mut u32;
    unsafe { reg.write_volatile(1 << (id % 32)) };
}

unsafe fn gic_acknowledge() -> u32 {
    let gicc_air = (GICC_BASE + 0xC) as *mut u32;
    unsafe { gicc_air.read_volatile() & 0x3FF } // interrupt id
}

unsafe fn gic_eoi(id: u32) {
    let gicc_eoir = (GICC_BASE + 0x10) as *mut u32;
    unsafe {
        gicc_eoir.write_volatile(id);
    }
}

pub fn init_interrupts() {
    unsafe { use_exception_vectors() };
    unsafe { gic_init() };
    enable_interrupts();
    unsafe { core::arch::asm!("isb") };
}

pub fn enable_interrupts() {
    unsafe {
        gic_enable_interrupt(30); // timer IRQ

        let keyboard_irq = 32 + 0x10 + KEYBOARD_SLOT.load(Ordering::Relaxed);
        let mouse_irq = 32 + 0x10 + MOUSE_SLOT.load(Ordering::Relaxed);

        gic_enable_interrupt(keyboard_irq as u32);
        gic_enable_interrupt(mouse_irq as u32);

        let ticks = timer_ticks_per_irq();
        core::arch::asm!("msr cntp_tval_el0, {}", in(reg) ticks);
        core::arch::asm!("msr cntp_ctl_el0, {}", in(reg) 1u64); // enable timer
    }
}

#[allow(unused_variables, dead_code)]
#[unsafe(no_mangle)]
unsafe extern "C" fn no_operation(ctx: *mut UserContext) {
    let interrupt_id = unsafe { gic_acknowledge() };

    unsafe {
        gic_eoi(interrupt_id);
    }
}

#[allow(unused_variables, dead_code)]
#[unsafe(no_mangle)]
unsafe extern "C" fn irq_handler(ctx: *mut UserContext) {
    let interrupt_id = unsafe { gic_acknowledge() };

    let keyboard_irq = 32 + 0x10 + KEYBOARD_SLOT.load(Ordering::Relaxed);
    let mouse_irq = 32 + 0x10 + MOUSE_SLOT.load(Ordering::Relaxed);

    match interrupt_id {
        30 => {
            TIMER.interrupt();

            let t = TIMER.interrupt_count.load(Ordering::Relaxed);

            if t % 60 == 0 {
                with_framebuffer(|fb| {
                    with_serial_console(|serial_console| serial_console.render(fb));
                    fb.present();
                });
            }

            let ticks = timer_ticks_per_irq();
            unsafe { core::arch::asm!("msr cntp_tval_el0, {}", in(reg) ticks) };
            unsafe { core::arch::asm!("msr cntp_ctl_el0, {}", in(reg) 1u64) };
        }
        interrupt_id if keyboard_irq == interrupt_id as u64 => {
            input_interrupt("kbd");
        }
        interrupt_id if mouse_irq == interrupt_id as u64 => {
            input_interrupt("mouse");
        }

        _ => {}
    }

    unsafe {
        gic_eoi(interrupt_id);
    }
}

fn handle_aborts(ec: u64, ctx: &UserContext) {
    match ec {
        ec if ec == 0x25 || ec == 0x24 => {
            panic!(
                "Data abort at VA={:#x} ELR={:#x} ESR={:#x}",
                ctx.far_el1, ctx.elr_el1, ctx.esr_el1
            );
        }
        ec if ec == 0x21 || ec == 0x20 => {
            panic!(
                "Instruction abort at VA={:#x} ELR={:#x}",
                ctx.far_el1, ctx.elr_el1
            );
        }
        _ => {
            panic!(
                "Unhandled sync abort VA={:#x} ELR={:#x} ESR={:#x}, ec={:#x}",
                ctx.far_el1, ctx.elr_el1, ctx.esr_el1, ec
            );
        }
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn sync_handler_user(ctx: *mut UserContext) {
    let ctx = unsafe { &mut *ctx };
    let ec = (ctx.esr_el1 >> ESR_EC_SHIFT) & ESR_EC_MASK;

    match ec {
        EC_SVC_AA64 => {
            ctx.x0 = unsafe {
                syscall_dispatch(
                    ctx.x8 as usize,
                    ctx.x0 as isize,
                    ctx.x1 as isize,
                    ctx.x2 as isize,
                    ctx.x3 as isize,
                    ctx.x4 as isize,
                    ctx.x5 as isize,
                )
            } as u64;

            ctx_save(ctx as *const UserContext);

            let _ = unsafe { check_and_reschedule() };
        }
        _ => handle_aborts(ec, ctx),
    };
}

#[unsafe(no_mangle)]
unsafe extern "C" fn sync_handler_kernel(ctx: *mut UserContext) {
    let ctx = unsafe { *ctx };
    let ec = (ctx.esr_el1 >> ESR_EC_SHIFT) & ESR_EC_MASK;
    handle_aborts(ec, &ctx)
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe fn run_next(ctx: *const UserContext, user_sp: u64) {
    core::arch::naked_asm!(
        "mov x30, x0",
        "msr sp_el0, x1",
        "ldr x2, [x30, #248]",
        "msr elr_el1, x2",
        "ldr x3, [x30, #256]",
        "msr spsr_el1, x3",
        "ldp x0, x1, [x30, #0]",
        "ldp x2, x3, [x30, #16]",
        "ldp x4, x5, [x30, #32]",
        "ldp x6, x7, [x30, #48]",
        "ldp x8, x9, [x30, #64]",
        "ldp x10, x11, [x30, #80]",
        "ldp x12, x13, [x30, #96]",
        "ldp x14, x15, [x30, #112]",
        "ldp x16, x17, [x30, #128]",
        "ldp x18, x19, [x30, #144]",
        "ldp x20, x21, [x30, #160]",
        "ldp x22, x23, [x30, #176]",
        "ldp x24, x25, [x30, #192]",
        "ldp x26, x27, [x30, #208]",
        "ldp x28, x29, [x30, #224]",
        "ldr x30, [x30, #240]",
        "isb",
        "eret"
    );
}

// these are UB and should not happen
exception_handler!(current_el_sp0_sync, no_operation);
exception_handler!(current_el_sp0_irq, no_operation);
exception_handler!(current_el_sp0_fiq, no_operation);
exception_handler!(current_el_sp0_serror, no_operation);
exception_handler!(lower_el_aarch32_sync, no_operation);
exception_handler!(lower_el_aarch32_irq, no_operation);
exception_handler!(lower_el_aarch32_fiq, no_operation);
exception_handler!(lower_el_aarch32_serror, no_operation);

// kernel
exception_handler!(current_el_spx_sync, sync_handler_kernel);
exception_handler!(current_el_spx_irq, irq_handler);
exception_handler!(current_el_spx_fiq, no_operation);
exception_handler!(current_el_spx_serror, no_operation);

// usermode
exception_handler!(lower_el_aarch64_sync, sync_handler_user);
exception_handler!(lower_el_aarch64_irq, irq_handler);
exception_handler!(lower_el_aarch64_fiq, no_operation);
exception_handler!(lower_el_aarch64_serror, no_operation);
