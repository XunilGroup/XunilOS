use alloc::{collections::btree_map::BTreeMap, string::String, vec::Vec};
use bitflags::bitflags;
use spin::Mutex;

use crate::arch::arch::safe_lock;

type PID = u64;

bitflags! {
    #[derive(Debug)]
    pub struct Permissions: u32 {
        const READ   = 1 << 0;
        const WRITE  = 1 << 1;
        const MANAGE = 1 << 2;
    }
}

#[derive(Clone)]
pub struct Message {
    pub from: PID,
    pub content: String,
}

pub struct Port {
    pub messages: Vec<Message>,
    pub permissions: BTreeMap<PID, Permissions>,
    pub default_permissions: Permissions,
}

pub static PORTS: Mutex<Option<BTreeMap<String, Port>>> = Mutex::new(None);

pub fn init_ipc() {
    *PORTS.lock() = Some(BTreeMap::new());
}

pub fn create_port(name: String, pid: u64, default_permissions: Permissions) {
    let mut permissions = BTreeMap::new();
    permissions.insert(
        pid as PID,
        Permissions::READ | Permissions::WRITE | Permissions::MANAGE,
    );

    safe_lock(|| {
        PORTS.lock().as_mut().expect("IPC not initialized").insert(
            name,
            Port {
                messages: Vec::new(),
                permissions,
                default_permissions,
            },
        );
    });
}

pub fn read_port(name: String, pid: u64, from: i64) -> Option<Message> {
    safe_lock(|| {
        let mut guard = PORTS.lock();
        let ports = guard.as_mut().expect("IPC not initialized");

        if let Some(port) = ports.get_mut(&name) {
            let permissions = port
                .permissions
                .get(&pid)
                .unwrap_or(&port.default_permissions);

            if !permissions.contains(Permissions::READ) {
                return None;
            }

            if from != -1 {
                if let Some(index) = port
                    .messages
                    .iter()
                    .position(|message| message.from == from as u64)
                {
                    return Some(port.messages.remove(index));
                }

                None
            } else {
                if let Some(index) = port
                    .messages
                    .iter()
                    .position(|message| message.from != pid as u64)
                {
                    return Some(port.messages.remove(index));
                }
                None
            }
        } else {
            None
        }
    })
}

pub fn write_port(name: String, pid: u64, message: String) -> bool {
    return safe_lock(|| {
        let mut guard = PORTS.lock();
        let ports = guard.as_mut().expect("IPC not initialized");
        if let Some(port) = ports.get_mut(&name) {
            let permissions = port
                .permissions
                .get(&pid)
                .unwrap_or(&port.default_permissions);

            if permissions.contains(Permissions::WRITE) {
                port.messages.push(Message {
                    from: pid,
                    content: message,
                });

                return true;
            } else {
                return false;
            }
        } else {
            return false;
        }
    });
}

pub fn manage_port(name: String, pid: u64, pid_to_set: u64, new_permissions: Permissions) -> bool {
    return safe_lock(|| {
        let mut guard = PORTS.lock();
        let ports = guard.as_mut().expect("IPC not initialized");
        if let Some(port) = ports.get_mut(&name) {
            let permissions = port
                .permissions
                .get(&pid)
                .unwrap_or(&port.default_permissions);

            if permissions.contains(Permissions::MANAGE) {
                port.permissions.insert(pid_to_set, new_permissions);

                return true;
            } else {
                return false;
            }
        } else {
            return false;
        }
    });
}
