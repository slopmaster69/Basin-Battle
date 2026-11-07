//! Other programs: starting one without a console window, and whether the game or the server is
//! open.

use std::process::Command;

/// A console program started without a console window behind it (benilla and htl-server are
/// console programs).
#[cfg(windows)]
pub fn quiet(c: &mut Command) -> &mut Command {
    use std::os::windows::process::CommandExt;
    c.creation_flags(0x0800_0000) // CREATE_NO_WINDOW
}
#[cfg(not(windows))]
pub fn quiet(c: &mut Command) -> &mut Command {
    c
}

/// Whether a process called `exe` ("benilla.exe") is running.
#[cfg(windows)]
pub fn running(exe: &str) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
    };
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE {
            return false;
        }
        let mut e: PROCESSENTRY32W = std::mem::zeroed();
        e.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut found = false;
        let mut ok = Process32FirstW(snap, &mut e) != 0;
        while ok {
            let len = e.szExeFile.iter().position(|&c| c == 0).unwrap_or(e.szExeFile.len());
            if String::from_utf16_lossy(&e.szExeFile[..len]).eq_ignore_ascii_case(exe) {
                found = true;
                break;
            }
            ok = Process32NextW(snap, &mut e) != 0;
        }
        CloseHandle(snap);
        found
    }
}
#[cfg(not(windows))]
pub fn running(exe: &str) -> bool {
    let name = exe.trim_end_matches(".exe");
    Command::new("pgrep")
        .args(["-x", name])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn game_running() -> bool {
    running("benilla.exe")
}

pub fn server_running() -> bool {
    running("htl-server.exe")
}

/// Open a folder or a web page the way the desktop does.
pub fn open_in_shell(target: &str) {
    #[cfg(windows)]
    let mut c = Command::new("explorer");
    #[cfg(target_os = "macos")]
    let mut c = Command::new("open");
    #[cfg(not(any(windows, target_os = "macos")))]
    let mut c = Command::new("xdg-open");
    let _ = c.arg(target).spawn();
}
