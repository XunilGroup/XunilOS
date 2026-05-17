#[repr(u64)]
pub enum VirtioMmioReg {
    MagicValue = 0x000,
    Version = 0x004,
    DeviceID = 0x008,
    VendorID = 0x00c,
    DeviceFeatures = 0x010,
    DriverFeatures = 0x020,
    QueueSel = 0x030,
    QueueNumMax = 0x034,
    QueueNum = 0x038,
    QueueReady = 0x044,
    QueueNotify = 0x050,
    InterruptStatus = 0x060,
    InterruptAck = 0x064,
    Status = 0x070,
    QueueDescLow = 0x080,
    QueueDescHigh = 0x084,
    QueueDriverLow = 0x090,
    QueueDriverHigh = 0x094,
    QueueDeviceLow = 0x0a0,
    QueueDeviceHigh = 0x0a4,
    ConfigGeneration = 0x0fc,
    Config = 0x100,
}

pub struct VirtioMmio {
    pub base: u64,
}

impl VirtioMmio {
    pub fn new(base: u64) -> Self {
        Self { base }
    }

    pub fn read(&self, reg: VirtioMmioReg) -> u32 {
        unsafe {
            let ptr = (self.base + reg as u64) as *const u32;
            ptr.read_volatile()
        }
    }

    pub fn write(&self, reg: VirtioMmioReg, value: u32) {
        unsafe {
            let ptr = (self.base + reg as u64) as *mut u32;
            ptr.write_volatile(value);
        }
    }
}
