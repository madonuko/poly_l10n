use windows::{Win32::Foundation::LPARAM, Win32::Globalization::EnumSystemLocalesEx};
use windows_core::{BOOL, PCWSTR};

unsafe extern "system" fn enum_locales_proc(ptr: PCWSTR, _dw_flags: u32, l_param: LPARAM) -> BOOL {
    // SAFETY: The type of `locales` is guaranteed in `get_system_locales()`
    let locales = unsafe { (l_param.0 as *mut Vec<String>).as_mut() }.expect("not a vec of string");

    // https://docs.rs/windows-core/latest/windows_core/struct.PCWSTR.html#method.as_wide
    // SAFETY: `to_string()` requires the same safety guarantees as `as_wide()`, and the string is
    // valid if windows doesn't break
    match unsafe { ptr.to_string() } {
        Ok(locale) => locales.push(locale),
        Err(_) if !cfg!(feature = "tracing") => {}
        Err(err) => tracing::error!(?err, "cannot convert utf16"),
    };

    BOOL(1) // Continue enumeration
}

pub(super) fn get_system_locales() -> Vec<String> {
    let mut locales = Vec::new();

    // SAFETY: The callback and pointer usage are properly handled within the unsafe block
    #[allow(unused_variables)]
    if let Err(err) = unsafe {
        EnumSystemLocalesEx(
            Some(enum_locales_proc),
            0, // Enumerate all locales
            LPARAM(&mut locales as *mut _ as isize),
            None,
        )
    } {
        #[cfg(feature = "tracing")]
        tracing::error!(?err, "EnumSystemLocalesEx failed");
        return vec![];
    }
    locales
}
