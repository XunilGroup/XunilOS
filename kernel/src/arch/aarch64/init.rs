use crate::arch::aarch64::{
    heap::init_heap,
    paging::{AArchPageTable, initialize_paging_aarch64},
};
use limine::response::{ExecutableAddressResponse, HhdmResponse, MemoryMapResponse};

pub fn init_aarch64<'a>(
    hhdm_response: &HhdmResponse,
    memory_map_response: &'a MemoryMapResponse,
    executable_address_response: &ExecutableAddressResponse,
) -> AArchPageTable {
    let mut mapper = initialize_paging_aarch64(
        hhdm_response,
        memory_map_response,
        executable_address_response,
    );

    init_heap(&mut mapper);

    return mapper;
}
