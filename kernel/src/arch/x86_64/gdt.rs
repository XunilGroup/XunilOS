use core::mem::MaybeUninit;

use alloc::boxed::Box;
use lazy_static::lazy_static;
use spin::mutex::Mutex;
use x86_64::VirtAddr;
use x86_64::instructions::segmentation::{CS, DS, ES, SS, Segment};
use x86_64::instructions::tables::load_tss;
use x86_64::structures::{
    gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
    tss::TaskStateSegment,
};
pub const TIMER_IST_INDEX: u16 = 1;
pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

pub static TSS_MUTEX: Mutex<Option<&'static mut TaskStateSegment>> = Mutex::new(None);

#[allow(static_mut_refs)]
fn make_tss() -> &'static mut TaskStateSegment {
    static mut TSS_STORAGE: MaybeUninit<TaskStateSegment> = MaybeUninit::uninit();

    unsafe {
        let tss_ptr = TSS_STORAGE.as_mut_ptr();
        tss_ptr.write(TaskStateSegment::new());
        let tss: &mut TaskStateSegment = &mut *tss_ptr;

        const STACK_SIZE: usize = 4096 * 5;
        static mut DOUBLE_FAULT_STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
        let df_start = VirtAddr::from_ptr(&DOUBLE_FAULT_STACK as *const _);
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = df_start + STACK_SIZE as u64;

        static mut PRIVILEGE_STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
        let p_start = VirtAddr::from_ptr(&PRIVILEGE_STACK as *const _);
        tss.privilege_stack_table[0] = p_start + STACK_SIZE as u64;

        static mut TIMER_IST_STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
        let ti_start = VirtAddr::from_ptr(&TIMER_IST_STACK as *const _);
        tss.interrupt_stack_table[TIMER_IST_INDEX as usize] = ti_start + STACK_SIZE as u64;

        tss
    }
}

lazy_static! {
    pub static ref GDT: (
        GlobalDescriptorTable,
        (
            SegmentSelector,
            SegmentSelector,
            SegmentSelector,
            SegmentSelector,
            SegmentSelector,
            SegmentSelector
        )
    ) = {
        let tss: &'static mut TaskStateSegment = make_tss();
        let tss_ptr: *mut TaskStateSegment = tss as *mut _;
        *TSS_MUTEX.lock() = Some(tss);

        let mut gdt = GlobalDescriptorTable::new();
        let kernel_code_selector = gdt.append(Descriptor::kernel_code_segment());
        let kernel_data_selector = gdt.append(Descriptor::kernel_data_segment());

        let user_code32_selector = gdt.append(Descriptor::UserSegment(
            (x86_64::structures::gdt::DescriptorFlags::USER_SEGMENT
                | x86_64::structures::gdt::DescriptorFlags::PRESENT
                | x86_64::structures::gdt::DescriptorFlags::EXECUTABLE
                | x86_64::structures::gdt::DescriptorFlags::DPL_RING_3)
                .bits(),
        ));

        let user_data_selector = gdt.append(Descriptor::user_data_segment());
        let user_code_selector = gdt.append(Descriptor::user_code_segment());

        let tss_ref: &'static TaskStateSegment = unsafe { &*tss_ptr };
        let tss_selector = gdt.append(Descriptor::tss_segment(tss_ref));

        (
            gdt,
            (
                kernel_code_selector,
                kernel_data_selector,
                user_code32_selector,
                user_data_selector,
                user_code_selector,
                tss_selector,
            ),
        )
    };
}

pub fn load_gdt_x86_64() {
    GDT.0.load();

    unsafe {
        CS::set_reg(GDT.1.0);
        DS::set_reg(GDT.1.1);
        ES::set_reg(GDT.1.1);
        SS::set_reg(GDT.1.1);
        load_tss(GDT.1.5);
    }
}

pub fn user_code_selector() -> SegmentSelector {
    GDT.1.4
}

pub fn user_data_selector() -> SegmentSelector {
    GDT.1.3
}
