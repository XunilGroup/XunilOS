use crate::{arch::x86_64::paging::XunilFrameAllocator, mm::address_space::AddressSpace};

enum ProcessState {
    Ready,
    Running,
    Blocked,
    Zombie,
}

pub struct Process {
    pub pid: u64,
    pub state: ProcessState,
    // cpu_ctx: &[u8],
    pub address_space: AddressSpace,
    pub user_entry: u64,
}
impl Process {
    pub fn new(
        pid: u64,
        user_entry: u64,
        frame_allocator: &mut XunilFrameAllocator,
    ) -> Option<Process> {
        let address_space = AddressSpace::new(frame_allocator)?;

        Some(Process {
            pid,
            state: ProcessState::Ready,
            address_space,
            user_entry,
        })
    }
}
