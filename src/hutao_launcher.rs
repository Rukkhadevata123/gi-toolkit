use crate::client_switch::{ClientSwitch, ClientType};
use crate::hutao_config::{
    ASSETS_PATH, CHINESE_OFFSETS, IslandEnvironment, IslandState, SHARED_MEMORY_NAME,
};
use crate::process_utils::{get_main_thread_id, is_process_running, kill_process_by_name};
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
use windows_sys::Win32::System::Environment::*;
use windows_sys::Win32::System::LibraryLoader::*;
use windows_sys::Win32::System::Memory::*;
use windows_sys::Win32::System::Threading::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

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
    game_pid: u32,
    game_process: Option<HANDLE>,
    game_thread: Option<HANDLE>,
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
            game_pid: 0,
            game_process: None,
            game_thread: None,
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
            // persistent storage
            let response = ui.text_edit_singleline(&mut self.switcher.game_path);
            if response.changed() {
                let _ = std::fs::write(
                    format!("{ASSETS_PATH}/game_path.txt"),
                    &self.switcher.game_path,
                );
            }
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
                let exe_path = self.switcher.game_path.trim().to_string();
                let proc_name = Path::new(&exe_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("YuanShen.exe");
                let process_found = is_process_running(proc_name);

                if process_found {
                    self.status = "confirm_kill_popup".to_string();
                } else {
                    self.launch_game();
                }
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

        if self.status == "confirm_kill_popup" {
            egui::Window::new("Confirm Termination")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.label("The game process is running.\nThis will terminate game process, do you want to continue?");
                    ui.horizontal(|ui| {
                        if ui.button("Yes").clicked() {
                            let exe_path = self.switcher.game_path.trim().to_string();
                            let proc_name = Path::new(&exe_path)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("YuanShen.exe");
                            let _ = kill_process_by_name(proc_name);
                            self.status.clear();
                            self.launch_game();
                        }
                        if ui.button("No").clicked() {
                            self.status.clear();
                        }
                    });
                });
        }

        if self.status == "about_popup" {
            egui::Window::new("About GI-Toolkit")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.label(
                        "GI-Toolkit v1.0\n\n\
                        Acknowledge: https://github.com/DGP-Studio/Snap.Hutao\n\n\
                        Copyright (c) 2025 Yoimiya\n\
                        MIT License\n\
                        https://github.com/Rukkhadevata123/min_hook_rs\n\
                        https://github.com/Rukkhadevata123/gi-toolkit\n\n\
                        This software is provided \"as is\", without warranty of any kind.",
                    );
                    if ui.button("Close").clicked() {
                        self.status.clear();
                    }
                });
        }

        if !self.status.is_empty() && self.status != "about_popup" {
            ui.label(&self.status);
        }
    }

    fn launch_game(&mut self) {
        // Clean up
        self.cleanup();

        self.switcher.game_path = std::fs::read_to_string(format!("{ASSETS_PATH}/game_path.txt"))
            .unwrap_or_else(|_| self.switcher.game_path.clone());

        let exe_path = self.switcher.game_path.trim().to_string();

        // hutao_minhook
        let path_str = format!("{ASSETS_PATH}/dlls/hutao_minhook.dll");
        let hutao_dll_dst = Path::new(path_str.as_str());
        if !hutao_dll_dst.exists() {
            let dll_src = Path::new("target/release/hutao_minhook.dll");
            if dll_src.exists() {
                let _ = fs::copy(dll_src, hutao_dll_dst);
            }
        }
        if !hutao_dll_dst.exists() {
            self.status = "DLL not found: hutao_minhook.dll".to_string();
            return;
        }

        // Switch client if needed
        let switch_result = self.switcher.switch();
        if let Err(e) = switch_result {
            self.status = format!("Switch failed: {e}");
            return;
        }

        let game_dir = Path::new(&exe_path).parent().unwrap();
        let game_dir_str = game_dir.to_str().unwrap();
        unsafe {
            let env_name = "__COMPAT_LAYER\0".encode_utf16().collect::<Vec<u16>>();
            let env_value = "RunAsInvoker\0".encode_utf16().collect::<Vec<u16>>();
            SetEnvironmentVariableW(env_name.as_ptr(), env_value.as_ptr());

            let mut si = mem::zeroed::<STARTUPINFOA>();
            si.cb = mem::size_of::<STARTUPINFOA>() as u32;
            let mut pi = mem::zeroed::<PROCESS_INFORMATION>();

            // Launch options
            let launch_args = "";
            let cmd_line = format!("\"{exe_path}\" {launch_args}");
            let cmd_line_c = CString::new(cmd_line).unwrap();
            let dir_c = CString::new(game_dir_str).unwrap();

            let ok = CreateProcessA(
                ptr::null(),
                cmd_line_c.as_ptr() as *mut u8,
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
                self.status = format!("CreateProcessA failed: {}", GetLastError());
                return;
            }
            self.game_pid = pi.dwProcessId;
            self.game_process = Some(pi.hProcess);
            self.game_thread = Some(pi.hThread);

            CloseHandle(pi.hThread);

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

            thread::sleep(Duration::from_secs(10));

            self.game_pid = pi.dwProcessId;
            self.game_process = Some(pi.hProcess);
            let hutao_result = self.inject_hutao_dll(hutao_dll_dst.to_str().unwrap());
            match hutao_result {
                Ok(_) => {
                    self.status = "Game launched, DLL injected successfully!".to_string();
                }
                Err(e) => {
                    self.status = format!("Hutao DLL injection failed: {e}");
                    CloseHandle(pi.hProcess);
                    self.game_process = None;
                }
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
                // Zero out the memory
                std::ptr::write_bytes(ptr, 0, 1);

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

    fn inject_hutao_dll(&self, dll_path: &str) -> Result<(), String> {
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
