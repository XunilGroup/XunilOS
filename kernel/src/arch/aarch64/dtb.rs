// TODO: actually use this?

use fdt::Fdt;

fn get_midr() -> u64 {
    let midr: u64;
    unsafe { core::arch::asm!("mrs {}, MIDR_EL1", out(reg) midr, options(nomem, nostack)) };
    midr
}

pub fn is_qemu() -> bool {
    let midr = get_midr();
    let implementer = (midr >> 24) & 0xFF;

    implementer == 0x00
}

pub unsafe fn read_dtb<'a>() -> Fdt<'a> {
    match is_qemu() {
        true => {
            let ptr: *mut u32 = 0x4000_0000 as *mut u32;

            let magic = u32::from_be(unsafe { ptr.read_volatile() });
            assert_eq!(magic, 0xd00dfeed);

            let size = u32::from_be(unsafe { ptr.add(1).read_volatile() }) as usize;
            let bytes = unsafe { core::slice::from_raw_parts(ptr as *const u8, size) };

            return Fdt::new(bytes).unwrap();
        }
        false => panic!("Can't read DTB of non-qemu device. Unimplemented."),
    }
}
