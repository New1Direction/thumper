//! Cross-platform process ancestry detection for Thumper.
//!
//! This module determines whether the current process (or its ancestors)
//! is a Thumper binary (`thump`, `thumper`, `bunny`, `thump-cli`).
//! It is the foundation for the "prefer native Bun execution" heuristic.
//!
//! Platform implementations:
//! - Windows: Uses `windows-sys` + `NtQueryInformationProcess` (gold standard)
//! - macOS:   Uses `libc` + `proc_pidinfo(PROC_PIDTASKINFO)`
//! - Linux:   Parses /proc/<pid>/stat and /proc/<pid>/cmdline

use std::path::PathBuf;

/// Structured report for --json output
#[derive(serde::Serialize)]
pub struct AncestryReport {
    pub os: String,
    pub current_pid: u32,
    pub ancestors: Vec<AncestorEntry>,
    pub verdict: bool,
    pub reason: String,
}

#[derive(serde::Serialize)]
pub struct AncestorEntry {
    pub pid: u32,
    pub name: String,
}

/// Returns a structured ancestry report (used by `thump internal debug-ancestry --json`).
pub fn get_ancestry_report() -> AncestryReport {
    let signatures = ["thump", "thumper", "bunny", "thump-cli", "api-anything"];

    let os = if cfg!(windows) {
        "windows".to_string()
    } else if cfg!(target_os = "macos") {
        "macos".to_string()
    } else if cfg!(target_os = "linux") {
        "linux".to_string()
    } else {
        "unknown".to_string()
    };

    let current_pid = std::process::id();
    let mut ancestors = Vec::new();
    let mut verdict = false;
    let mut reason = "No Thumper ancestor found in process tree".to_string();

    let mut pid = current_pid;

    for _ in 0..16 {
        if let Some(parent_pid) = get_parent_of(pid) {
            if let Some(name) = get_process_name(parent_pid) {
                ancestors.push(AncestorEntry {
                    pid: parent_pid,
                    name: name.clone(),
                });

                let name_lower = name.to_lowercase();
                if signatures.iter().any(|sig| name_lower.contains(sig)) {
                    verdict = true;
                    reason = format!("Found Thumper-family process: {}", name);
                    break;
                }
            }
            pid = parent_pid;
        } else {
            break;
        }
    }

    if ancestors.is_empty() {
        reason = "Could not walk process tree (insufficient permissions or unsupported platform)"
            .to_string();
    }

    AncestryReport {
        os,
        current_pid,
        ancestors,
        verdict,
        reason,
    }
}

/// Returns the parent process ID of the current process, if detectable.
pub fn get_parent_pid() -> Option<u32> {
    #[cfg(windows)]
    {
        windows_impl::get_parent_pid()
    }
    #[cfg(target_os = "macos")]
    {
        macos_impl::get_parent_pid()
    }
    #[cfg(target_os = "linux")]
    {
        linux_impl::get_parent_pid()
    }
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        None
    }
}

/// Walks the process tree (up to 16 levels) looking for any Thumper-family binary.
pub fn is_launched_by_thumper() -> bool {
    let signatures = ["thump", "thumper", "bunny", "thump-cli", "api-anything"];

    let mut current_pid = std::process::id();

    for _ in 0..16 {
        if let Some(parent_pid) = get_parent_of(current_pid) {
            if let Some(name) = get_process_name(parent_pid) {
                let name_lower = name.to_lowercase();
                if signatures.iter().any(|sig| name_lower.contains(sig)) {
                    return true;
                }
            }
            current_pid = parent_pid;
        } else {
            break;
        }
    }
    false
}

/// Returns a human-readable ancestry report for `--debug-ancestry`.
pub fn get_ancestry_diagnostics() -> String {
    use std::fmt::Write;

    let mut output = String::new();

    let _ = writeln!(
        output,
        "============================================================="
    );
    let _ = writeln!(output, "THUMPER ANCESTRY DIAGNOSTICS");
    let _ = writeln!(
        output,
        "============================================================="
    );

    #[cfg(windows)]
    let _ = writeln!(output, "Target OS: Windows (x86_64)");
    #[cfg(target_os = "macos")]
    let _ = writeln!(output, "Target OS: macOS (Darwin)");
    #[cfg(target_os = "linux")]
    let _ = writeln!(output, "Target OS: Linux");

    let current_pid = std::process::id();
    let _ = writeln!(output, "Current PID: {}", current_pid);

    let mut pid = current_pid;
    let mut depth = 0;

    while depth < 12 {
        if let Some(parent) = get_parent_of(pid) {
            let name = get_process_name(parent).unwrap_or_else(|| "<unknown>".to_string());
            let _ = writeln!(output, "Parent PID: {} ({})", parent, name);

            // Simple heuristic for common Windows shells
            if name.to_lowercase().contains("powershell") || name.contains("pwsh") {
                let _ = writeln!(output, "Detected: PowerShell / pwsh");
            } else if name.to_lowercase().contains("cmd") {
                let _ = writeln!(output, "Detected: cmd.exe");
            } else if name.to_lowercase().contains("bash") || name.contains("git-bash") {
                let _ = writeln!(output, "Detected: Git Bash / MSYS2");
            }

            pid = parent;
        } else {
            break;
        }
        depth += 1;
    }

    let prefers = is_launched_by_thumper();
    let _ = writeln!(
        output,
        "Verdict: PREFER_NATIVE -> {} ({})",
        prefers,
        if prefers {
            "Thumper ancestor found"
        } else {
            "No Thumper ancestor detected"
        }
    );
    let _ = writeln!(
        output,
        "============================================================="
    );

    output
}

// ---------------------------------------------------------------------------
// Internal helpers (platform agnostic interface)
// ---------------------------------------------------------------------------

fn get_parent_of(pid: u32) -> Option<u32> {
    if pid == 0 {
        return None;
    }

    #[cfg(windows)]
    {
        windows_impl::get_parent_of(pid)
    }
    #[cfg(target_os = "macos")]
    {
        macos_impl::get_parent_of(pid as i32).map(|p| p as u32)
    }
    #[cfg(target_os = "linux")]
    {
        linux_impl::get_parent_of(pid)
    }
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        None
    }
}

fn get_process_name(pid: u32) -> Option<String> {
    #[cfg(windows)]
    {
        windows_impl::get_process_name(pid)
    }
    #[cfg(target_os = "macos")]
    {
        macos_impl::get_process_name(pid)
    }
    #[cfg(target_os = "linux")]
    {
        linux_impl::get_process_name(pid)
    }
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        None
    }
}

fn is_thumper_binary(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("thump") || lower.contains("bunny") || lower.contains("api-anything")
}

// ---------------------------------------------------------------------------
// Platform modules (stubs that will be filled with real implementations)
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod windows_impl {
    pub fn get_parent_pid() -> Option<u32> {
        get_parent_of(std::process::id())
    }

    pub fn get_parent_of(pid: u32) -> Option<u32> {
        use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
        use windows_sys::Win32::System::Threading::{
            NtQueryInformationProcess, ProcessBasicInformation, PROCESS_BASIC_INFORMATION,
            PROCESS_QUERY_LIMITED_INFORMATION,
        };

        unsafe {
            let handle: HANDLE = windows_sys::Win32::System::Threading::OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION,
                0,
                pid,
            );

            if handle.is_null() {
                return None;
            }

            let mut pbi: PROCESS_BASIC_INFORMATION = std::mem::zeroed();
            let mut return_length: u32 = 0;

            let status = NtQueryInformationProcess(
                handle,
                ProcessBasicInformation as i32,
                &mut pbi as *mut _ as *mut _,
                std::mem::size_of::<PROCESS_BASIC_INFORMATION>() as u32,
                &mut return_length,
            );

            CloseHandle(handle);

            if status == 0 {
                // InheritedFromUniqueProcessId is the real parent on Windows
                let parent = pbi.InheritedFromUniqueProcessId as u32;
                if parent != 0 && parent != pid {
                    return Some(parent);
                }
            }
            None
        }
    }

    pub fn get_process_name(pid: u32) -> Option<String> {
        // Lightweight implementation: use GetProcessImageFileNameW if available
        use windows_sys::Win32::Foundation::HANDLE;
        use windows_sys::Win32::System::ProcessStatus::GetProcessImageFileNameW;
        use windows_sys::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION;

        unsafe {
            let handle: HANDLE = windows_sys::Win32::System::Threading::OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION,
                0,
                pid,
            );

            if handle.is_null() {
                return None;
            }

            let mut buffer = [0u16; 260];
            let len = GetProcessImageFileNameW(handle, buffer.as_mut_ptr(), buffer.len() as u32);

            windows_sys::Win32::Foundation::CloseHandle(handle);

            if len > 0 {
                let path = String::from_utf16_lossy(&buffer[..len as usize]);
                // Extract just the filename
                std::path::Path::new(&path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
            } else {
                None
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod macos_impl {
    use libc::{c_char, c_int, c_void, proc_pidinfo};
    use std::ffi::CStr;
    use std::path::Path;

    const PROC_PIDTASKALLINFO: c_int = 2;
    const PROC_PIDPATHINFO: c_int = 11;
    const PROC_PIDPATHINFO_SIZE: c_int = 1024;

    #[repr(C)]
    #[derive(Copy, Clone)]
    struct proc_bsdinfo {
        pbi_flags: u32,
        pbi_status: u32,
        pbi_xstatus: u32,
        pbi_pid: u32,
        pbi_ppid: u32, // Exact parent PID
        pbi_uid: u32,
        pbi_gid: u32,
        pbi_ruid: u32,
        pbi_rgid: u32,
        pbi_svuid: u32,
        pbi_svgid: u32,
        rfu_1: u32,
        pbi_comm: [c_char; 16],
        pbi_name: [c_char; 32],
        pbi_nfiles: u32,
        pbi_pgid: u32,
        pbi_pjobc: u32,
        pbi_tgid: u32,
        pbi_flag_ext: u32,
        pbi_rgid_owner: u32,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    struct proc_taskinfo {
        pti_virtual_size: u64,
        pti_resident_size: u64,
        pti_total_user: u64,
        pti_total_system: u64,
        pti_threads_user: u64,
        pti_threads_system: u64,
        pti_policy: i32,
        pti_faults: i32,
        pti_pageins: i32,
        pti_cow_faults: i32,
        pti_messages_sent: i32,
        pti_messages_received: i32,
        pti_syscalls_mach: i32,
        pti_syscalls_unix: i32,
        pti_csw: i32,
        pti_threadnum: i32,
        pti_numrunning: i32,
        pti_priority: i32,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    struct proc_taskallinfo {
        pbsd: proc_bsdinfo,
        ptinfo: proc_taskinfo,
    }

    pub fn get_parent_pid() -> Option<u32> {
        get_parent_of(std::process::id() as i32)
    }

    pub fn get_parent_of(pid: i32) -> Option<u32> {
        let mut info: proc_taskallinfo = unsafe { std::mem::zeroed() };
        let size = std::mem::size_of::<proc_taskallinfo>() as c_int;

        let ret = unsafe {
            proc_pidinfo(
                pid,
                PROC_PIDTASKALLINFO,
                0,
                &mut info as *mut _ as *mut c_void,
                size,
            )
        };

        if ret == size {
            let ppid = info.pbsd.pbi_ppid;
            if ppid != 0 && ppid != pid as u32 {
                return Some(ppid);
            }
        }
        None
    }

    pub fn get_process_name(pid: u32) -> Option<String> {
        let mut path_buf: [u8; PROC_PIDPATHINFO_SIZE as usize] =
            [0; PROC_PIDPATHINFO_SIZE as usize];

        let ret = unsafe {
            proc_pidinfo(
                pid as i32,
                PROC_PIDPATHINFO,
                0,
                path_buf.as_mut_ptr() as *mut c_void,
                PROC_PIDPATHINFO_SIZE,
            )
        };

        if ret > 0 {
            // Find first null terminator
            if let Some(pos) = path_buf.iter().position(|&b| b == 0) {
                if let Ok(cstr) = CStr::from_bytes_with_nul(&path_buf[..=pos]) {
                    if let Ok(path) = cstr.to_str() {
                        return Path::new(path)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .map(|s| s.to_string());
                    }
                }
            }
        }
        None
    }
}

#[cfg(target_os = "linux")]
mod linux_impl {
    use std::fs;

    pub fn get_parent_pid() -> Option<u32> {
        get_parent_of(std::process::id())
    }

    pub fn get_parent_of(pid: u32) -> Option<u32> {
        let path = format!("/proc/{}/stat", pid);
        let content = fs::read_to_string(path).ok()?;
        // Format: pid (comm) state ppid ...
        let start = content.rfind(')')? + 1;
        let parts: Vec<&str> = content[start..].split_whitespace().collect();
        if parts.len() > 2 {
            parts[2].parse::<u32>().ok()
        } else {
            None
        }
    }

    pub fn get_process_name(pid: u32) -> Option<String> {
        let path = format!("/proc/{}/comm", pid);
        fs::read_to_string(path).ok().map(|s| s.trim().to_string())
    }
}
