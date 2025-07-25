use min_hook_rs::*;
use std::ffi::{CString, c_void};
use std::mem;
use std::ptr;
use std::sync::Mutex;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::LibraryLoader::*;
use windows_sys::Win32::System::Memory::*;
use windows_sys::Win32::System::SystemServices::*;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::core::*;

// IL2CPP structures
#[repr(C)]
struct Il2CppObject {
    klass: *mut c_void,
    monitor: *mut c_void,
}

#[repr(C)]
struct Il2CppString {
    object: Il2CppObject,
    length: i32,
    chars: [u16; 32],
}

// Function types
type FindString = unsafe extern "system" fn(*const i8) -> *mut Il2CppString;
type SetFieldOfViewMethod = unsafe extern "system" fn(*mut c_void, f32);
type SetEnableFogRenderingMethod = unsafe extern "system" fn(bool);
type SetTargetFrameRateMethod = unsafe extern "system" fn(i32);
type OpenTeamMethod = unsafe extern "system" fn();
type OpenTeamPageAccordinglyMethod = unsafe extern "system" fn(bool);
type CheckCanEnterMethod = unsafe extern "system" fn() -> bool;
type CraftEntryMethod = unsafe extern "system" fn(*mut c_void);
type CraftEntryMethodPartner = unsafe extern "system" fn(
    *mut Il2CppString,
    *mut c_void,
    *mut c_void,
    *mut c_void,
    *mut c_void,
) -> bool;

// Environment structure
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum IslandState {
    None = 0,
    Error = 1,
    Started = 2,
    Stopped = 3,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct FunctionOffsets {
    find_string: u32,
    set_field_of_view: u32,
    set_enable_fog_rendering: u32,
    set_target_frame_rate: u32,
    open_team: u32,
    open_team_page_accordingly: u32,
    check_can_enter: u32,
    craft_entry: u32,
    craft_entry_partner: u32,
}

#[repr(C)]
#[derive(Debug)]
struct IslandEnvironment {
    state: IslandState,
    last_error: u32,
    function_offsets: FunctionOffsets,
    field_of_view: f32,
    fix_low_fov_scene: i32, // BOOL
    disable_fog: i32,       // BOOL
    target_frame_rate: i32,
    remove_open_team_progress: i32, // BOOL
    redirect_craft_entry: i32,      // BOOL
}

// Original function pointers
#[derive(Default)]
struct OriginalFunctions {
    find_string: Option<FindString>,
    set_field_of_view: Option<SetFieldOfViewMethod>,
    set_enable_fog_rendering: Option<SetEnableFogRenderingMethod>,
    set_target_frame_rate: Option<SetTargetFrameRateMethod>,
    open_team: Option<OpenTeamMethod>,
    open_team_page_accordingly: Option<OpenTeamPageAccordinglyMethod>,
    check_can_enter: Option<CheckCanEnterMethod>,
    craft_entry: Option<CraftEntryMethod>,
    craft_entry_partner: Option<CraftEntryMethodPartner>,
}

// Global state
static ISLAND_ENVIRONMENT_NAME: &str = "4F3E8543-40F7-4808-82DC-21E48A6037A7";
static mut P_ENVIRONMENT: *mut IslandEnvironment = ptr::null_mut();
static ORIGINALS: Mutex<OriginalFunctions> = Mutex::new(OriginalFunctions {
    find_string: None,
    set_field_of_view: None,
    set_enable_fog_rendering: None,
    set_target_frame_rate: None,
    open_team: None,
    open_team_page_accordingly: None,
    check_can_enter: None,
    craft_entry: None,
    craft_entry_partner: None,
});

// Memory protection disabling
unsafe extern "system" {
    unsafe fn LdrAddRefDll(flags: u32, dll_handle: *mut c_void) -> i32;
}

const LDR_ADDREF_DLL_PIN: u32 = 0x00000001;

fn disable_protect_virtual_memory() {
    unsafe {
        let ntdll = GetModuleHandleA(b"ntdll.dll\0".as_ptr());
        if ntdll.is_null() {
            return;
        }

        let p_nt_protect_virtual_memory =
            GetProcAddress(ntdll, b"NtProtectVirtualMemory\0".as_ptr());
        let p_nt_query_section = GetProcAddress(ntdll, b"NtQuerySection\0".as_ptr());

        if let (Some(protect_fn), Some(query_fn)) =
            (p_nt_protect_virtual_memory, p_nt_query_section)
        {
            let mut old = 0u32;
            VirtualProtect(
                protect_fn as *mut c_void,
                1,
                PAGE_EXECUTE_READWRITE,
                &mut old,
            );

            let protect_ptr = protect_fn as *mut u64;
            let query_ptr = query_fn as *const u64;
            let query_offset_ptr = (query_fn as usize + 4) as *const u32;

            *protect_ptr = (*query_ptr & !(0xFFu64 << 32)) | ((*query_offset_ptr as u64 - 1) << 32);

            VirtualProtect(protect_fn as *mut c_void, 1, old, &mut old);
        }
    }
}

// FOV endpoint handler
unsafe extern "system" fn set_field_of_view_endpoint(p_this: *mut c_void, value: f32) {
    unsafe {
        if P_ENVIRONMENT.is_null() {
            return;
        }

        let env = &*P_ENVIRONMENT;

        let (frame_rate_fn, fov_fn, fog_fn) = {
            let originals = ORIGINALS.lock().unwrap();
            (
                originals.set_target_frame_rate,
                originals.set_field_of_view,
                originals.set_enable_fog_rendering,
            )
        };

        // Set target frame rate
        if let Some(frame_rate_fn) = frame_rate_fn {
            frame_rate_fn(env.target_frame_rate);
        }

        // Handle FOV and fog based on current view value
        if value.floor() <= 30.0 {
            // Low FOV scene
            if let Some(fog_fn) = fog_fn {
                fog_fn(false); // Always disable fog in low FOV scenes
            }
            if let Some(fov_fn) = fov_fn {
                let fov_value = if env.fix_low_fov_scene != 0 {
                    env.field_of_view // Use user-defined FOV
                } else {
                    value // Keep original low FOV
                };
                fov_fn(p_this, fov_value);
            }
        } else {
            // Normal FOV scene
            if let Some(fog_fn) = fog_fn {
                fog_fn(env.disable_fog == 0); // Respect user fog setting
            }
            if let Some(fov_fn) = fov_fn {
                fov_fn(p_this, env.field_of_view); // Use user-defined FOV
            }
        }
    }
}

unsafe extern "system" fn open_team_endpoint() {
    unsafe {
        if P_ENVIRONMENT.is_null() {
            return;
        }

        let env = &*P_ENVIRONMENT;

        let (check_fn, page_fn, team_fn) = {
            let originals = ORIGINALS.lock().unwrap();
            (
                originals.check_can_enter,
                originals.open_team_page_accordingly,
                originals.open_team,
            )
        };

        let should_use_page_fn =
            env.remove_open_team_progress != 0 && check_fn.is_some_and(|f| f());

        if should_use_page_fn {
            if let Some(page_fn) = page_fn {
                page_fn(false);
            }
        } else if let Some(team_fn) = team_fn {
            team_fn();
        }
    }
}

unsafe extern "system" fn craft_entry_endpoint(p_this: *mut c_void) {
    unsafe {
        if P_ENVIRONMENT.is_null() {
            return;
        }

        let env = &*P_ENVIRONMENT;

        let (partner_fn, craft_partner_fn, craft_fn) = {
            let originals = ORIGINALS.lock().unwrap();
            (
                originals.find_string,
                originals.craft_entry_partner,
                originals.craft_entry,
            )
        };

        if env.redirect_craft_entry != 0 {
            if let (Some(partner_fn), Some(craft_partner_fn)) = (partner_fn, craft_partner_fn) {
                let synthesis_page = CString::new("SynthesisPage").unwrap();
                let page_string = partner_fn(synthesis_page.as_ptr());
                craft_partner_fn(
                    page_string,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                );
                return;
            }
        }

        if let Some(craft_fn) = craft_fn {
            craft_fn(p_this);
        }
    }
}

// Install MinHooks
fn install_min_hooks(base: u64, env: &IslandEnvironment) -> Result<()> {
    unsafe {
        initialize()?;

        let mut originals = ORIGINALS.lock().unwrap();

        // Store original function pointers
        originals.find_string = Some(mem::transmute::<*mut c_void, FindString>(
            (base + env.function_offsets.find_string as u64) as *mut c_void,
        ));
        originals.set_enable_fog_rendering =
            Some(mem::transmute::<*mut c_void, SetEnableFogRenderingMethod>(
                (base + env.function_offsets.set_enable_fog_rendering as u64) as *mut c_void,
            ));
        originals.set_target_frame_rate =
            Some(mem::transmute::<*mut c_void, SetTargetFrameRateMethod>(
                (base + env.function_offsets.set_target_frame_rate as u64) as *mut c_void,
            ));
        originals.open_team_page_accordingly = Some(mem::transmute::<
            *mut c_void,
            OpenTeamPageAccordinglyMethod,
        >(
            (base + env.function_offsets.open_team_page_accordingly as u64) as *mut c_void,
        ));
        originals.check_can_enter = Some(mem::transmute::<*mut c_void, CheckCanEnterMethod>(
            (base + env.function_offsets.check_can_enter as u64) as *mut c_void,
        ));
        originals.craft_entry_partner =
            Some(mem::transmute::<*mut c_void, CraftEntryMethodPartner>(
                (base + env.function_offsets.craft_entry_partner as u64) as *mut c_void,
            ));

        // Create hooks
        let target = (base + env.function_offsets.set_field_of_view as u64) as *mut c_void;
        let trampoline = create_hook(target, set_field_of_view_endpoint as *mut c_void)?;
        originals.set_field_of_view = Some(mem::transmute::<*mut c_void, SetFieldOfViewMethod>(
            trampoline,
        ));

        let target = (base + env.function_offsets.open_team as u64) as *mut c_void;
        let trampoline = create_hook(target, open_team_endpoint as *mut c_void)?;
        originals.open_team = Some(mem::transmute::<*mut c_void, OpenTeamMethod>(trampoline));

        let target = (base + env.function_offsets.craft_entry as u64) as *mut c_void;
        let trampoline = create_hook(target, craft_entry_endpoint as *mut c_void)?;
        originals.craft_entry = Some(mem::transmute::<*mut c_void, CraftEntryMethod>(trampoline));

        // Enable all hooks
        enable_hook(ALL_HOOKS)?;

        Ok(())
    }
}

// Main DLL thread
extern "system" fn island_thread(lp_param: *mut c_void) -> u32 {
    unsafe {
        let env_name_c = CString::new(ISLAND_ENVIRONMENT_NAME).unwrap();

        let h_file = OpenFileMappingA(
            FILE_MAP_READ | FILE_MAP_WRITE,
            FALSE,
            env_name_c.as_ptr() as *const u8,
        );
        if h_file.is_null() || h_file == INVALID_HANDLE_VALUE {
            return GetLastError();
        }

        let lp_view = MapViewOfFile(h_file, FILE_MAP_READ | FILE_MAP_WRITE, 0, 0, 0);
        if lp_view.Value.is_null() {
            CloseHandle(h_file);
            return GetLastError();
        }

        P_ENVIRONMENT = lp_view.Value as *mut IslandEnvironment;
        (*P_ENVIRONMENT).state = IslandState::Started;

        let base = GetModuleHandleA(ptr::null()) as u64;

        // Install hooks
        if install_min_hooks(base, &(*P_ENVIRONMENT)).is_err() {
            (*P_ENVIRONMENT).state = IslandState::Error;
            (*P_ENVIRONMENT).last_error = GetLastError();
            UnmapViewOfFile(lp_view);
            CloseHandle(h_file);
            return GetLastError();
        }

        // Wait indefinitely
        WaitForSingleObject(GetCurrentThread(), u32::MAX);

        // Cleanup
        let _ = disable_hook(ALL_HOOKS);
        let _ = uninitialize();

        (*P_ENVIRONMENT).state = IslandState::Stopped;
        UnmapViewOfFile(lp_view);
        CloseHandle(h_file);

        FreeLibraryAndExitThread(lp_param as HMODULE, 0);
    }
}

// Hook procedure for exports
extern "system" fn island_get_window_hook_impl(
    code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    unsafe { CallNextHookEx(ptr::null_mut(), code, w_param, l_param) }
}

// Export functions
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllGetWindowsHookForHutao(p_hook_proc: *mut *mut c_void) -> HRESULT {
    unsafe {
        *p_hook_proc = island_get_window_hook_impl as *mut c_void;
        0 // S_OK
    }
}

// DLL entry point
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllMain(
    h_module: HINSTANCE,
    ul_reason_for_call: u32,
    _lp_reserved: *mut c_void,
) -> BOOL {
    unsafe {
        match ul_reason_for_call {
            DLL_PROCESS_ATTACH => {
                DisableThreadLibraryCalls(h_module);
                LdrAddRefDll(LDR_ADDREF_DLL_PIN, h_module);
                disable_protect_virtual_memory();
                CreateThread(
                    ptr::null_mut(),
                    0,
                    Some(island_thread),
                    h_module,
                    0,
                    ptr::null_mut(),
                );
            }
            DLL_PROCESS_DETACH => {
                let _ = disable_hook(ALL_HOOKS);
                let _ = uninitialize();
                Sleep(500);
            }
            _ => {}
        }
        TRUE
    }
}
