use min_hook_rs::*;
use std::ffi::{CString, c_char, c_void};
use std::fs;
use std::ptr;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::LibraryLoader::*;

type LoginCallBackHandler = unsafe extern "stdcall" fn(*const c_char, i32);
type LoadLibraryWFn = unsafe extern "system" fn(*const u16) -> HMODULE;

static mut ORIGINAL_LOAD_LIBRARY: Option<LoadLibraryWFn> = None;

extern "stdcall" fn hook_login(
    _app_key: *const c_char,
    _back_to_login: bool,
    callback: LoginCallBackHandler,
) -> i32 {
    let data = match fs::read_to_string("assets/login.json") {
        Ok(content) => content.lines().next().unwrap_or("").trim().to_string(),
        Err(_) => {
            let error_data = r#"{"code":-1,"data":{"message":"file not found"}}"#;
            let error_cstring = CString::new(error_data).unwrap();
            unsafe {
                callback(error_cstring.as_ptr(), error_data.len() as i32);
            }
            return 0;
        }
    };

    if serde_json::from_str::<serde_json::Value>(&data).is_err() {
        let error_data = r#"{"code":-1,"data":{"message":"invalid json format"}}"#;
        let error_cstring = CString::new(error_data).unwrap();
        unsafe {
            callback(error_cstring.as_ptr(), error_data.len() as i32);
        }
        return 0;
    }

    let data_cstring = CString::new(data.as_str()).unwrap();
    unsafe {
        callback(data_cstring.as_ptr(), data.len() as i32);
    }

    0
}

extern "system" fn new_load_library_w(file_name: *const u16) -> HMODULE {
    let module = unsafe {
        if let Some(original) = ORIGINAL_LOAD_LIBRARY {
            original(file_name)
        } else {
            return ptr::null_mut();
        }
    };

    if module.is_null() {
        return module;
    }

    let func_name = CString::new("SDKShowLoginPanel").unwrap();
    let sdk_login_func = unsafe { GetProcAddress(module, func_name.as_ptr() as *const u8) };

    if let Some(func_ptr) = sdk_login_func {
        let target = func_ptr as *mut c_void;
        if create_hook(target, hook_login as *mut c_void).is_ok() {
            let _ = enable_hook(target);
        }
    }

    module
}

#[unsafe(no_mangle)]
pub extern "system" fn DllMain(_module: HMODULE, reason: u32, _reserved: *mut c_void) -> i32 {
    match reason {
        1 => {
            if initialize().is_err() {
                return 0;
            }

            let load_library_addr = LoadLibraryW as *mut c_void;
            match create_hook(load_library_addr, new_load_library_w as *mut c_void) {
                Ok(trampoline) => {
                    unsafe {
                        ORIGINAL_LOAD_LIBRARY = Some(std::mem::transmute::<
                            *mut c_void,
                            unsafe extern "system" fn(*const u16) -> *mut c_void,
                        >(trampoline));
                    }

                    if enable_hook(load_library_addr).is_err() {
                        return 0;
                    }
                }
                Err(_) => return 0,
            }

            1
        }
        0 => {
            let _ = uninitialize();
            1
        }
        _ => 1,
    }
}
