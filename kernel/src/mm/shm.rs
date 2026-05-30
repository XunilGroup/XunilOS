use core::sync::atomic::AtomicU64;

use alloc::{collections::btree_map::BTreeMap, string::String, vec::Vec};
use spin::mutex::Mutex;

pub const USER_SHM_BASE: u64 = 0x0000_4000_0000_0000;
pub static SHM_REGISTRY: Mutex<Option<BTreeMap<String, SharedMemory>>> = Mutex::new(None);
pub static NEXT_SHM_ID: AtomicU64 = AtomicU64::new(1);
pub const SHM_SLOT_SIZE: u64 = 64 * 1024 * 1024;

pub struct SharedMemory {
    pub id: u64,
    pub phys_pages: Vec<u64>,
}

pub fn init_shm() {
    *SHM_REGISTRY.lock() = Some(BTreeMap::new());
}
