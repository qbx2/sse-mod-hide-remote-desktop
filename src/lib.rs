mod log;

use detour::static_detour;
use std::ffi::c_void;
use std::intrinsics::transmute;
use std::io::Write;
use std::os::raw::c_char;
use winapi::um::libloaderapi::{GetModuleHandleA, GetProcAddress};
use crate::log::get_log;
use winapi::um::winuser::SM_REMOTESESSION;

enum InfoVersion {
    KInfoVersion = 1,
}

#[repr(C)]
pub struct PluginInfo {
    info_version: u32,
    name: *const c_char,
    version: u32,
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern fn SKSEPlugin_Query(_: *const c_void, info: *mut PluginInfo) -> bool {
    let mut info = unsafe { &mut *info };
    info.info_version = InfoVersion::KInfoVersion as u32;
    info.name = "hide-remote-desktop\0".as_ptr() as *const c_char;
    info.version = 1;
    return true;
}

static_detour! {
  static GetSystemMetrics: fn(i32) -> i32;
}

fn new_get_system_metrics(n_index: i32) -> i32 {
    // is remote desktop? no ;)
    let result = if n_index == SM_REMOTESESSION {
        0
    } else {
        GetSystemMetrics.call(n_index)
    };

    get_log().write_all(
        format!("GetSystemMetrics({:#x}) -> {}\n", n_index, result).as_bytes(),
    ).ok();
    return result;
}

#[allow(non_snake_case)]
#[no_mangle]
pub unsafe fn SKSEPlugin_Load(_: *const c_void) -> bool {
    let log = log::get_log();

    let result: Result<bool, Box<dyn std::error::Error>> = (|| {
        let user32 = GetModuleHandleA("User32.dll\0".as_ptr() as *const c_char);
        GetSystemMetrics.initialize(
            transmute(GetProcAddress(user32, "GetSystemMetrics\0".as_ptr() as *const c_char)),
            new_get_system_metrics,
        )?;
        GetSystemMetrics.enable()?;

        Ok(true)
    })();

    if let Err(err) = result {
        log.write_all(format!("error SKSEPlugin_Load: {}\n", err).as_bytes()).unwrap();
        return false;
    }

    return true;
}
