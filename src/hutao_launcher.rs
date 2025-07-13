use crate::client_switch::{ClientSwitch, ClientType};
use eframe::egui;
use std::ffi::{CString, c_void};
use std::fs;
use std::mem;
use std::path::Path;
use std::ptr;
use std::thread;
use std::time::Duration;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Security::*;
use windows_sys::Win32::System::Diagnostics::ToolHelp::*;
use windows_sys::Win32::System::LibraryLoader::*;
use windows_sys::Win32::System::Memory::*;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FunctionOffsets {
    pub find_string: u32,
    pub set_field_of_view: u32,
    pub set_enable_fog_rendering: u32,
    pub set_target_frame_rate: u32,
    pub open_team: u32,
    pub open_team_page_accordingly: u32,
    pub check_can_enter: u32,
    pub craft_entry: u32,
    pub craft_entry_partner: u32,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IslandState {
    None = 0,
    Error = 1,
    Started = 2,
    Stopped = 3,
}

#[repr(C)]
#[derive(Debug)]
pub struct IslandEnvironment {
    pub state: IslandState,
    pub last_error: u32,
    pub function_offsets: FunctionOffsets,
    pub field_of_view: f32,
    pub fix_low_fov_scene: i32, // BOOL
    pub disable_fog: i32,       // BOOL
    pub target_frame_rate: i32,
    pub remove_open_team_progress: i32, // BOOL
    pub redirect_craft_entry: i32,      // BOOL
}

const SHARED_MEMORY_NAME: &str = "4F3E8543-40F7-4808-82DC-21E48A6037A7";
const CHINESE_OFFSETS: FunctionOffsets = FunctionOffsets {
    find_string: 4830752,
    set_field_of_view: 17204528,
    set_enable_fog_rendering: 277807600,
    set_target_frame_rate: 277729120,
    open_team: 118414576,
    open_team_page_accordingly: 118384496,
    check_can_enter: 156982512,
    craft_entry: 127845632,
    craft_entry_partner: 201143472,
};

pub struct Launcher {
    pub switcher: ClientSwitch,
    pub status: String,
    pub target_fps: i32,
    pub field_of_view: f32,
    pub disable_fog: bool,
    pub fix_low_fov: bool,
    pub remove_team_anim: bool,
    pub redirect_craft: bool,
    // Inner state
    shared_mem_handle: Option<HANDLE>,
    shared_mem_ptr: Option<*mut IslandEnvironment>,
    game_process: Option<HANDLE>,
    game_pid: u32,
}

impl Default for Launcher {
    fn default() -> Self {
        Self {
            switcher: ClientSwitch::default(),
            status: String::new(),
            target_fps: 60,
            field_of_view: 45.0,
            disable_fog: false,
            fix_low_fov: false,
            remove_team_anim: true,
            redirect_craft: true,
            shared_mem_handle: None,
            shared_mem_ptr: None,
            game_process: None,
            game_pid: 0,
        }
    }
}

impl eframe::App for Launcher {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.ui(ui);
        });
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.cleanup();
    }
}

impl Launcher {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("GI-Toolkit Launcher");

        ui.horizontal(|ui| {
            ui.label("Game Path:");
            ui.text_edit_singleline(&mut self.switcher.game_path);
        });

        ui.horizontal(|ui| {
            ui.radio_value(
                &mut self.switcher.client_type,
                ClientType::Official,
                "Official",
            );
            ui.radio_value(
                &mut self.switcher.client_type,
                ClientType::Bilibili,
                "Bilibili",
            );
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Target FPS:");
            ui.add(egui::DragValue::new(&mut self.target_fps).range(30..=239));
            ui.checkbox(&mut self.remove_team_anim, "Remove Team Animation");
        });

        ui.horizontal(|ui| {
            ui.label("Field of View:");
            ui.add(egui::DragValue::new(&mut self.field_of_view).range(1.0..=120.0));
            ui.checkbox(&mut self.fix_low_fov, "Fix Low FOV Scenes");
        });

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.disable_fog, "Disable Fog");
            ui.checkbox(&mut self.redirect_craft, "Redirect Crafting Table");
        });

        ui.horizontal(|ui| {
            if ui.button("Launch Game").clicked() {
                self.launch_game();
            }
            if ui.button("Apply").clicked() {
                self.apply_settings();
            }
            if ui.button("Reset").clicked() {
                self.reset_settings();
            }
            if ui.button("About").clicked() {
                self.status = "about_popup".to_string();
            }
            if ui.button("Exit").clicked() {
                self.cleanup();
                std::process::exit(0);
            }
        });

        if self.status == "about_popup" {
            egui::Window::new("About GI-Toolkit")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.label(
                        "GI-Toolkit v1.0\n\n\
                        Copyright (c) 2025 Yoimiya\n\
                        MIT License\n\
                        https://github.com/Rukkhadevata123/min_hook_rs\n\n\
                        This software is provided \"as is\", without warranty of any kind.",
                    );
                    if ui.button("Close").clicked() {
                        self.status.clear();
                    }
                });
        }

        if !self.status.is_empty() && self.status != "about_popup" && self.status != "usage_popup" {
            ui.label(&self.status);
        }
    }

    fn launch_game(&mut self) {
        // no need because we have DragValue with range
        if self.target_fps < 30 {
            self.status = "FPS must be at least 30".to_string();
            return;
        }
        if self.field_of_view < 1.0 {
            self.status = "FOV must be at least 1.0".to_string();
            return;
        }

        let exe_path = self.switcher.game_path.trim().to_string();
        let proc_name = Path::new(&exe_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("YuanShen.exe");

        if self.switcher.client_type == ClientType::Bilibili {
            let game_dir = Path::new(&exe_path).parent().unwrap();
            let login_src = Path::new("assets/login.json");
            let login_dst = game_dir.join("login.json");
            if login_src.exists() {
                let _ = fs::copy(login_src, &login_dst);
            }
        }

        let dll_dst = Path::new("assets/dlls/hutao_minhook.dll");
        if !dll_dst.exists() {
            let dll_src = Path::new("target/release/hutao_minhook.dll");
            if dll_src.exists() {
                let _ = fs::copy(dll_src, dll_dst);
            }
        }
        if !dll_dst.exists() {
            self.status = "DLL not found: hutao_minhook.dll".to_string();
            return;
        }

        match kill_process_by_name(proc_name) {
            Ok(_) => {}
            Err(e) => {
                if e != "Process not found" {
                    self.status = format!("Failed to kill process: {e}");
                    return;
                }
            }
        }

        // Switch client if needed
        let switch_result = self.switcher.switch();
        if let Err(e) = switch_result {
            self.status = format!("Switch failed: {e}");
            return;
        }

        // Create shared memory
        match self.create_shared_memory() {
            Ok(_) => {}
            Err(e) => {
                self.status = format!("Failed to create shared memory: {e}");
                return;
            }
        }

        // Configure environment
        self.configure_environment();

        // Launch game process
        match self.launch_game_process(&exe_path) {
            Ok(_) => {}
            Err(e) => {
                self.status = format!("Failed to launch game: {e}");
                return;
            }
        }

        // DLL injection
        match self.inject_dll(dll_dst.to_str().unwrap()) {
            Ok(_) => {
                self.status = "Game launched and DLL injected successfully!".to_string();
            }
            Err(e) => {
                self.status = format!("DLL injection failed: {e}");
                return;
            }
        }
    }

    fn create_shared_memory(&mut self) -> Result<(), String> {
        unsafe {
            let mut sa = SECURITY_ATTRIBUTES {
                nLength: mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: ptr::null_mut(),
                bInheritHandle: TRUE,
            };
            let name = CString::new(SHARED_MEMORY_NAME).unwrap();
            let h_map = CreateFileMappingA(
                INVALID_HANDLE_VALUE,
                &mut sa,
                PAGE_READWRITE,
                0,
                mem::size_of::<IslandEnvironment>() as u32,
                name.as_ptr() as *const u8,
            );
            if h_map.is_null() {
                return Err("CreateFileMappingA failed".to_string());
            }
            let p_mem = MapViewOfFile(
                h_map,
                FILE_MAP_ALL_ACCESS,
                0,
                0,
                mem::size_of::<IslandEnvironment>(),
            );
            if p_mem.Value.is_null() {
                CloseHandle(h_map);
                return Err("MapViewOfFile failed".to_string());
            }
            self.shared_mem_handle = Some(h_map);
            self.shared_mem_ptr = Some(p_mem.Value as *mut IslandEnvironment);
        }
        Ok(())
    }

    fn configure_environment(&mut self) {
        if let Some(ptr) = self.shared_mem_ptr {
            unsafe {
                let env = &mut *ptr;
                env.function_offsets = CHINESE_OFFSETS;
                env.field_of_view = self.field_of_view;
                env.fix_low_fov_scene = if self.fix_low_fov { 1 } else { 0 };
                env.disable_fog = if self.disable_fog { 1 } else { 0 };
                env.target_frame_rate = self.target_fps;
                env.remove_open_team_progress = if self.remove_team_anim { 1 } else { 0 };
                env.redirect_craft_entry = if self.redirect_craft { 1 } else { 0 };
                env.state = IslandState::Started;
            }
        }
    }

    fn launch_game_process(&mut self, exe_path: &str) -> Result<(), String> {
        let game_dir = Path::new(exe_path).parent().unwrap();
        let game_dir_str = game_dir.to_str().unwrap();
        unsafe {
            let mut si = mem::zeroed::<STARTUPINFOA>();
            si.cb = mem::size_of::<STARTUPINFOA>() as u32;
            let mut pi = mem::zeroed::<PROCESS_INFORMATION>();
            let exe_c = CString::new(exe_path).unwrap();
            let dir_c = CString::new(game_dir_str).unwrap();
            let ok = CreateProcessA(
                exe_c.as_ptr() as *const u8,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                FALSE,
                0,
                ptr::null_mut(),
                dir_c.as_ptr() as *const u8,
                &mut si,
                &mut pi,
            );
            if ok == 0 {
                return Err(format!("CreateProcessA failed: {}", GetLastError()));
            }
            CloseHandle(pi.hThread);
            self.game_process = Some(pi.hProcess);
            self.game_pid = pi.dwProcessId;
            SetPriorityClass(pi.hProcess, HIGH_PRIORITY_CLASS);
            thread::sleep(Duration::from_secs(10));
        }
        Ok(())
    }

    fn inject_dll(&self, dll_path: &str) -> Result<(), String> {
        unsafe {
            let dll_c = CString::new(dll_path).unwrap();
            let h_dll = LoadLibraryA(dll_c.as_ptr() as *const u8);
            if h_dll.is_null() {
                return Err("LoadLibraryA failed".to_string());
            }
            // Get the address of the hook function
            let proc_name = CString::new("DllGetWindowsHookForHutao").unwrap();
            let p_get_hook = GetProcAddress(h_dll, proc_name.as_ptr() as *const u8);
            let p_get_hook = if p_get_hook.is_some() {
                p_get_hook
            } else {
                let proc_name2 = CString::new("IslandGetWindowHook").unwrap();
                GetProcAddress(h_dll, proc_name2.as_ptr() as *const u8)
            };
            if let Some(get_hook_fn) = p_get_hook {
                type GetHookFn = unsafe extern "system" fn(*mut HOOKPROC) -> i32;
                let get_hook: GetHookFn = mem::transmute(get_hook_fn);
                let mut hook_proc: HOOKPROC = None;
                let result = get_hook(&mut hook_proc as *mut HOOKPROC);
                if result != 0 || hook_proc.is_none() {
                    FreeLibrary(h_dll);
                    return Err("Failed to get hook function from DLL".to_string());
                }
                // Get the main thread ID of the game process
                let thread_id = get_main_thread_id(self.game_pid);
                if thread_id == 0 {
                    FreeLibrary(h_dll);
                    return Err("Failed to get main thread ID".to_string());
                }
                let h_hook = SetWindowsHookExA(WH_GETMESSAGE, hook_proc, h_dll, thread_id);
                if h_hook.is_null() {
                    FreeLibrary(h_dll);
                    return Err("SetWindowsHookEx failed".to_string());
                }
                PostThreadMessageA(thread_id, WM_NULL, 0, 0);
                thread::sleep(Duration::from_millis(500));
                UnhookWindowsHookEx(h_hook);
                FreeLibrary(h_dll);
                Ok(())
            } else {
                FreeLibrary(h_dll);
                Err("Failed to get hook function from DLL".to_string())
            }
        }
    }

    pub fn apply_settings(&mut self) {
        // just update the settings
        self.configure_environment();
        self.status = "Settings applied.".to_string();
    }

    pub fn reset_settings(&mut self) {
        self.target_fps = 60;
        self.field_of_view = 45.0;
        self.disable_fog = false;
        self.fix_low_fov = false;
        self.remove_team_anim = true;
        self.redirect_craft = true;
        self.configure_environment();
        self.status = "Settings reset to default.".to_string();
    }

    pub fn cleanup(&mut self) {
        unsafe {
            if let Some(ptr) = self.shared_mem_ptr {
                let env = &mut *ptr;
                env.target_frame_rate = 60;
                env.field_of_view = 45.0;
                env.fix_low_fov_scene = 0;
                env.disable_fog = 0;
                env.remove_open_team_progress = 0;
                env.redirect_craft_entry = 0;
                env.state = IslandState::Stopped;
                UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS {
                    Value: ptr as *mut c_void,
                });
                self.shared_mem_ptr = None;
            }
            if let Some(h_map) = self.shared_mem_handle {
                CloseHandle(h_map);
                self.shared_mem_handle = None;
            }
            if let Some(h_proc) = self.game_process {
                CloseHandle(h_proc);
                self.game_process = None;
            }
        }
    }
}

pub fn kill_process_by_name(proc_name: &str) -> Result<(), String> {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return Err("CreateToolhelp32Snapshot failed".to_string());
        }
        let mut entry: PROCESSENTRY32W = mem::zeroed();
        entry.dwSize = mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut found = false;
        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                let exe_name = String::from_utf16_lossy(&entry.szExeFile);
                let exe_name = exe_name.trim_end_matches('\0');
                if exe_name.eq_ignore_ascii_case(proc_name) {
                    let h_process = OpenProcess(PROCESS_TERMINATE, 0, entry.th32ProcessID);
                    if !h_process.is_null() && h_process != INVALID_HANDLE_VALUE {
                        TerminateProcess(h_process, 0);
                        CloseHandle(h_process);
                        found = true;
                    }
                }
                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snapshot);
        if found {
            Ok(())
        } else {
            Err("Process not found".to_string())
        }
    }
}

fn get_main_thread_id(process_id: u32) -> u32 {
    unsafe {
        let h_snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        if h_snapshot == INVALID_HANDLE_VALUE {
            return 0;
        }
        let mut te32 = THREADENTRY32 {
            dwSize: mem::size_of::<THREADENTRY32>() as u32,
            ..mem::zeroed()
        };
        let mut thread_id = 0u32;
        let mut earliest_time = FILETIME {
            dwLowDateTime: u32::MAX,
            dwHighDateTime: u32::MAX,
        };
        if Thread32First(h_snapshot, &mut te32) != 0 {
            loop {
                if te32.th32OwnerProcessID == process_id {
                    let h_thread = OpenThread(THREAD_QUERY_INFORMATION, FALSE, te32.th32ThreadID);
                    if !h_thread.is_null() {
                        let mut creation_time = mem::zeroed();
                        let mut exit_time = mem::zeroed();
                        let mut kernel_time = mem::zeroed();
                        let mut user_time = mem::zeroed();
                        if GetThreadTimes(
                            h_thread,
                            &mut creation_time,
                            &mut exit_time,
                            &mut kernel_time,
                            &mut user_time,
                        ) != 0
                        {
                            let creation_u64 = ((creation_time.dwHighDateTime as u64) << 32)
                                | (creation_time.dwLowDateTime as u64);
                            let earliest_u64 = ((earliest_time.dwHighDateTime as u64) << 32)
                                | (earliest_time.dwLowDateTime as u64);
                            if creation_u64 < earliest_u64 {
                                earliest_time = creation_time;
                                thread_id = te32.th32ThreadID;
                            }
                        }
                        CloseHandle(h_thread);
                    }
                }
                if Thread32Next(h_snapshot, &mut te32) == 0 {
                    break;
                }
            }
        }
        CloseHandle(h_snapshot);
        thread_id
    }
}
