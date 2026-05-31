use crate::{
    arch::{
        aarch64::{
            heap::init_heap,
            interrupts::init_interrupts,
            paging::{AArchPageTable, initialize_paging_aarch64},
        },
        arch::KERNEL_MAPPER,
    },
    driver::{
        io::{fs::vfs::init_vfs, virtio::scan_virtio_devices},
        ipc::init_ipc,
    },
    mm::shm::init_shm,
};
use limine::response::{ExecutableAddressResponse, HhdmResponse, MemoryMapResponse};

// needs to be page aligned since we map it into the page table
#[repr(align(4096))]
pub struct Stack(pub [u8; 2048 * 1024]);

pub static KERNEL_STACK: Stack = Stack([0; 2048 * 1024]);

#[unsafe(naked)]
pub unsafe fn init_aarch64_trampoline(mapper: &mut AArchPageTable) {
    // fix stack, since limine's bootloader stack is not mapped
    core::arch::naked_asm!(
        "msr spsel, #1",
        "adrp x1, {stack}",
        "add x1, x1, :lo12:{stack}",
        "mov x2, {size}",
        "add sp, x1, x2",
        "b kernel_main_aarch64",
        stack = sym KERNEL_STACK,
        size = const 2048usize * 1024,
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn init_aarch64(mapper: &mut AArchPageTable) {
    init_heap(mapper);
    scan_virtio_devices();
    init_interrupts();
    init_ipc();
    init_shm();
    init_vfs();
}

pub fn preinit_aarch64<'a>(
    hhdm_response: &HhdmResponse,
    memory_map_response: &'a MemoryMapResponse,
    executable_address_response: &ExecutableAddressResponse,
) {
    let mapper: AArchPageTable = initialize_paging_aarch64(
        hhdm_response,
        memory_map_response,
        executable_address_response,
    );
    #[allow(static_mut_refs)]
    unsafe {
        *KERNEL_MAPPER.get_mut() = Some(mapper)
    };
    #[allow(static_mut_refs)]
    unsafe {
        init_aarch64_trampoline(KERNEL_MAPPER.get_mut().as_mut().unwrap())
    };
}
