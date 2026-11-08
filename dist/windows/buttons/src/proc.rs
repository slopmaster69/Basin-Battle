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

/// Whether a process called `exe` is running from inside `root` (this folder's own game or
/// server, not another folder's: a lab or a second copy on the same PC held every update back,
/// Nov 7). One whose path can't be read counts, to be safe.
#[cfg(windows)]
pub fn running_in(exe: &str, root: &std::path::Path) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    let root = root.to_string_lossy().trim_end_matches(['\\', '/']).to_lowercase().replace('/', "\\");
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
                let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, e.th32ProcessID);
                if h.is_null() {
                    found = true;
                    break;
                }
                let mut buf = [0u16; 1024];
                let mut size = buf.len() as u32;
                let got = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut size) != 0;
                CloseHandle(h);
                let path = String::from_utf16_lossy(&buf[..size as usize]).to_lowercase();
                if !got || path.starts_with(&format!("{root}\\")) {
                    found = true;
                    break;
                }
            }
            ok = Process32NextW(snap, &mut e) != 0;
        }
        CloseHandle(snap);
        found
    }
}
#[cfg(not(windows))]
pub fn running_in(exe: &str, _root: &std::path::Path) -> bool {
    running(exe)
}

/// This folder's own server is running (Host's, or Play alone's).
pub fn server_running_in(root: &std::path::Path) -> bool {
    running_in("htl-server.exe", root)
}

/// This folder's own game is open.
pub fn game_running_in(root: &std::path::Path) -> bool {
    running_in("benilla.exe", root)
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

#[cfg(all(test, windows))]
mod tests {
    use super::running_in;

    #[test]
    fn a_program_counts_only_from_inside_its_own_folder() {
        // This test's own program: running from its folder, not from another.
        let me = std::env::current_exe().unwrap();
        let name = me.file_name().unwrap().to_string_lossy().to_string();
        let dir = me.parent().unwrap();
        assert!(running_in(&name, dir));
        assert!(running_in(&name, dir.parent().unwrap()), "a folder above it holds it too");
        assert!(!running_in(&name, std::path::Path::new(r"C:\Nowhere Elwynn Survival")));
        assert!(!running_in("no-such-program.exe", dir));
    }
}
