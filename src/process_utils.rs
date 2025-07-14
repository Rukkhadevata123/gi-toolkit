use std::mem;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Diagnostics::ToolHelp::*;
use windows_sys::Win32::System::Threading::*;

pub fn is_process_running(proc_name: &str) -> bool {
    for_each_process_by_name(proc_name, |_| {})
}

pub fn kill_process_by_name(proc_name: &str) -> Result<(), String> {
    let mut killed = false;
    for_each_process_by_name(proc_name, |pid| unsafe {
        let h_process = OpenProcess(PROCESS_TERMINATE, 0, pid);
        if !h_process.is_null() && h_process != INVALID_HANDLE_VALUE {
            TerminateProcess(h_process, 0);
            CloseHandle(h_process);
            killed = true;
        }
    });
    if killed {
        Ok(())
    } else {
        Err("Process not found".to_string())
    }
}

pub fn get_main_thread_id(process_id: u32) -> u32 {
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

pub fn for_each_process_by_name<F>(proc_name: &str, mut action: F) -> bool
where
    F: FnMut(u32),
{
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return false;
        }
        let mut entry: PROCESSENTRY32W = mem::zeroed();
        entry.dwSize = mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut found = false;
        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                let exe_name = String::from_utf16_lossy(&entry.szExeFile);
                let exe_name = exe_name.trim_end_matches('\0');
                if exe_name.eq_ignore_ascii_case(proc_name) {
                    found = true;
                    action(entry.th32ProcessID);
                }
                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snapshot);
        found
    }
}
