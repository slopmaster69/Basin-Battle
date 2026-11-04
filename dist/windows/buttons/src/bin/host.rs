//! Host.exe: runs launcher\host.ps1 in this console (the server's window: leave it open while
//! anyone plays) and ends with its exit code. Arguments go to the server (Host.exe --password x).

use std::process::Command;

fn main() {
    let here = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_default();
    let status = Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(here.join("launcher").join("host.ps1"))
        .args(std::env::args_os().skip(1))
        .current_dir(&here)
        .status();
    std::process::exit(status.ok().and_then(|s| s.code()).unwrap_or(1));
}
