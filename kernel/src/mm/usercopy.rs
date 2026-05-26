use alloc::{
    ffi::CString,
    string::{String, ToString},
    vec::Vec,
};

#[cfg(target_arch = "x86_64")]
use x86_64::{
    VirtAddr,
    structures::paging::{OffsetPageTable, PageTableFlags, Translate, mapper::TranslateResult},
};

#[cfg(target_arch = "aarch64")]
use crate::arch::aarch64::paging::AArchPageTable;

#[cfg(target_arch = "aarch64")]
type PageTable = AArchPageTable;

#[cfg(target_arch = "x86_64")]
type PageTable<'a> = OffsetPageTable<'a>;

#[allow(unused_variables)]
pub fn copy_to_user(
    mapper: &mut PageTable,
    buf: *mut u8,
    src: *const u8,
    len: usize,
) -> Result<(), isize> {
    let start = buf as u64;
    let end = start + len as u64;
    #[allow(unused_mut)]
    let mut page_addr = start & !0xFFF;

    #[cfg(target_arch = "x86_64")]
    {
        while page_addr < end {
            let translate_result = mapper.translate(VirtAddr::new(page_addr));
            #[allow(non_shorthand_field_patterns)]
            if let TranslateResult::Mapped {
                frame: _,
                offset: _,
                flags: flags,
            } = translate_result
            {
                if flags.contains(PageTableFlags::USER_ACCESSIBLE)
                    && flags.contains(PageTableFlags::WRITABLE)
                {
                } else {
                    return Err(-13);
                }
            } else {
                return Err(-1);
            }
            page_addr += 0x1000;
        }

        unsafe { core::ptr::copy_nonoverlapping(src, buf, len) };
    }

    #[cfg(target_arch = "aarch64")]
    {
        // TODO: add checks
        unsafe { core::ptr::copy_nonoverlapping(src, buf, len) };
    }

    Ok(())
}

#[allow(unused_variables)]
pub fn copy_from_user(
    mapper: &mut PageTable,
    buf: *mut u8,
    src: *const u8,
    len: usize,
) -> Result<(), isize> {
    if len == 0 {
        return Ok(());
    }
    if buf.is_null() || src.is_null() {
        return Err(-14);
    }

    let start = src as u64;
    let end = start
        .checked_add(len as u64)
        .ok_or(-1)
        .map_err(|err| err as isize)?;
    #[allow(unused_mut)]
    let mut page_addr = start & !0xFFF;

    #[cfg(target_arch = "x86_64")]
    {
        while page_addr < end {
            let translate_result = mapper.translate(VirtAddr::new(page_addr));
            #[allow(non_shorthand_field_patterns)]
            if let TranslateResult::Mapped {
                frame: _,
                offset: _,
                flags: flags,
            } = translate_result
            {
                if !flags.contains(PageTableFlags::USER_ACCESSIBLE) {
                    return Err(-13);
                }
            } else {
                return Err(-1);
            }
            page_addr += 0x1000;
        }

        unsafe { core::ptr::copy_nonoverlapping(src, buf, len) };
    }

    #[cfg(target_arch = "aarch64")]
    {
        // TODO: add checks
        unsafe { core::ptr::copy_nonoverlapping(src, buf, len) };
    }

    Ok(())
}

pub fn copy_cstr_from_user(
    mapper: &mut PageTable,
    user_ptr: *const u8,
    max_len: usize,
) -> Result<String, isize> {
    if user_ptr.is_null() {
        return Err(-14);
    }

    let mut buf: Vec<u8> = Vec::with_capacity(max_len);

    for i in 0..max_len {
        let mut byte = 0u8;
        copy_from_user(mapper, &mut byte as *mut u8, unsafe { user_ptr.add(i) }, 1)?;
        if byte == 0 {
            return core::str::from_utf8(&buf)
                .map(|s| s.to_string())
                .map_err(|_| -84);
        }
        buf.push(byte);
    }

    Err(-36)
}

pub fn copy_cstr_to_user(
    mapper: &mut PageTable,
    kernel_str: String,
    user_ptr: *mut u8,
) -> Result<(), isize> {
    if user_ptr.is_null() {
        return Err(-14);
    }
    let c_string = CString::new(kernel_str).map_err(|_| -14isize)?;
    let len = c_string.count_bytes();
    let _ = copy_to_user(mapper, user_ptr, c_string.into_raw() as *const u8, len);
    Ok(())
}
