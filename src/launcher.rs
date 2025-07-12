use crate::client_switch::{ClientSwitch, ClientType};
use eframe::egui;

pub struct Launcher {
    pub switcher: ClientSwitch,
    pub status: String,
}

impl Default for Launcher {
    fn default() -> Self {
        Self {
            switcher: ClientSwitch::default(),
            status: String::new(),
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
        //TODO: Clean up resources if needed(include revert to default)
    }
}

impl Launcher {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Client Switcher");

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

        if ui.button("Launch Game").clicked() {
            let exe_path = self.switcher.game_path.clone();
            let proc_name = std::path::Path::new(&exe_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("YuanShen.exe");

            match kill_process_by_name(proc_name) {
                Ok(_) => {}
                Err(e) => {
                    self.status = format!("Failed to kill process: {}", e);
                    return;
                }
            }

            // 1. switch client
            let switch_result = self.switcher.switch();
            match switch_result {
                Ok(_) => {
                    let launch_result = launch_game(&self.switcher.game_path);
                    self.status = match launch_result {
                        Ok(_) => "Game launched successfully!".to_string(),
                        Err(e) => format!("Failed to launch game: {}", e),
                    };
                }
                Err(e) => {
                    self.status = format!("Switch failed: {}", e);
                }
            }

            //2. inject dll
            //TODO 
        }

        if ui.button("About").clicked() {
            self.status = "about_popup".to_string();
        }

        if self.status == "about_popup" {
            egui::Window::new("About GI-Toolkit")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.label("GI-Toolkit v1.0\n\n\
                        Copyright (c) 2025 Yoimiya\n\
                        MIT License\n\
                        https://github.com/Rukkhadevata123/min_hook_rs\n\n\
                        This software is provided \"as is\", without warranty of any kind.");
                    if ui.button("Close").clicked() {
                        self.status.clear();
                    }
                });
        }

        if !self.status.is_empty() {
            ui.label(&self.status);
        }
    }
}

pub fn kill_process_by_name(proc_name: &str) -> Result<(), String> {
    use std::mem;
    use windows_sys::Win32::Foundation::*;
    use windows_sys::Win32::System::Diagnostics::ToolHelp::*;
    use windows_sys::Win32::System::Threading::*;

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

pub fn launch_game(exe_path: &str) -> Result<(), String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use std::ptr;
    use windows_sys::Win32::Foundation::*;
    use windows_sys::Win32::System::Threading::*;

    let wide_exe: Vec<u16> = OsStr::new(exe_path).encode_wide().chain(Some(0)).collect();
    let dir = Path::new(exe_path).parent().unwrap();
    let wide_dir: Vec<u16> = dir.as_os_str().encode_wide().chain(Some(0)).collect();

    unsafe {
        let mut si = std::mem::zeroed::<STARTUPINFOW>();
        si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
        let mut pi = std::mem::zeroed::<PROCESS_INFORMATION>();

        let success = CreateProcessW(
            wide_exe.as_ptr(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            0,
            ptr::null(),
            wide_dir.as_ptr(),
            &mut si,
            &mut pi,
        );
        if success == 0 {
            return Err(format!("CreateProcessW failed: {}", GetLastError()));
        }
        CloseHandle(pi.hThread);
        CloseHandle(pi.hProcess);
    }
    Ok(())
}
