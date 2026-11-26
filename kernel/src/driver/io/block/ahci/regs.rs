#[derive(Debug)]
pub struct AHCICap {
    pub s64: u32,
    pub ncs: u32,
    pub np: u32,
}

#[derive(Clone)]
pub struct AHCIPortRegs {
    pub base: u64,
}

impl AHCIPortRegs {
    pub fn read(&self, offset: u64) -> u32 {
        return unsafe { core::ptr::read_volatile((self.base + offset) as *const u32) };
    }

    pub fn write(&self, offset: u64, data: u32) {
        unsafe { core::ptr::write_volatile((self.base + offset) as *mut u32, data) };
    }

    pub fn clb(&self) -> u32 {
        return self.read(0x00);
    }

    pub fn clbu(&self) -> u32 {
        return self.read(0x04);
    }

    pub fn set_clb(&self, data: u32) {
        return self.write(0x00, data);
    }

    pub fn set_clbu(&self, data: u32) {
        return self.write(0x04, data);
    }

    pub fn fb(&self) -> u32 {
        return self.read(0x08);
    }

    pub fn fbu(&self) -> u32 {
        return self.read(0x0C);
    }

    pub fn set_fb(&self, data: u32) {
        return self.write(0x08, data);
    }

    pub fn set_fbu(&self, data: u32) {
        return self.write(0x0C, data);
    }

    pub fn is(&self) -> u32 {
        return self.read(0x10);
    }

    pub fn set_is(&self, data: u32) {
        return self.write(0x10, data);
    }

    pub fn ie(&self) -> u32 {
        return self.read(0x14);
    }

    pub fn cmd(&self) -> u32 {
        return self.read(0x18);
    }

    pub fn set_cmd(&self, data: u32) {
        return self.write(0x18, data);
    }

    pub fn sig(&self) -> u32 {
        return self.read(0x24);
    }

    pub fn tfd(&self) -> u32 {
        return self.read(0x20);
    }

    pub fn ssts(&self) -> u32 {
        return self.read(0x28);
    }

    pub fn serr(&self) -> u32 {
        return self.read(0x30);
    }

    pub fn set_serr(&self, data: u32) {
        return self.write(0x30, data);
    }

    pub fn sact(&self) -> u32 {
        return self.read(0x34);
    }

    pub fn set_sact(&self, data: u32) {
        return self.write(0x34, data);
    }

    pub fn ci(&self) -> u32 {
        return self.read(0x38);
    }

    pub fn set_ci(&self, data: u32) {
        return self.write(0x38, data);
    }
}

pub struct AHCIRegs {
    pub base: u64,
}

impl AHCIRegs {
    pub fn read(&self, offset: u64) -> u32 {
        return unsafe { core::ptr::read_volatile((self.base + offset) as *const u32) };
    }

    pub fn write(&self, offset: u64, data: u32) {
        unsafe { core::ptr::write_volatile((self.base + offset) as *mut u32, data) };
    }

    pub fn cap(&self) -> AHCICap {
        let raw = self.read(0x00);

        // S64A, NCS, NP
        return AHCICap {
            s64: (raw >> 31) & 1,
            ncs: ((raw >> 8) & 0x1F) + 1,
            np: (raw & 0x1F) + 1,
        };
    }

    pub fn is(&self) -> u32 {
        return self.read(0x08);
    }

    pub fn ghc(&self) -> u32 {
        return self.read(0x04);
    }

    pub fn set_ghc(&self, value: u32) {
        return self.write(0x04, value);
    }

    pub fn pi(&self) -> u32 {
        return self.read(0x0C);
    }

    pub fn vs(&self) -> u32 {
        return self.read(0x10);
    }

    pub fn port(&self, index: u64) -> AHCIPortRegs {
        return AHCIPortRegs {
            base: self.base + 0x100 + index * 0x80,
        };
    }
}
