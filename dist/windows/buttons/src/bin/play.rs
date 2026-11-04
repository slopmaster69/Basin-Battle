//! Play.exe: starts launcher\play.ps1 (its own window draws the launcher) with no console window
//! behind it. Arguments are passed on (Play.exe -Change opens the choices at once).
#![windows_subsystem = "windows"]

use std::process::Command;

fn main() {
    let here = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_default();
    let mut c = Command::new("powershell.exe");
    c.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-STA", "-WindowStyle", "Hidden", "-File"])
        .arg(here.join("launcher").join("play.ps1"))
        .args(std::env::args_os().skip(1))
        .current_dir(&here);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: no console flashes up behind the launcher.
        c.creation_flags(0x0800_0000);
    }
    let _ = c.spawn();
}
