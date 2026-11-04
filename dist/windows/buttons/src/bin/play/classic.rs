//! The plain launcher: launcher\play.ps1, the window Play had before the War Table. Play.exe
//! starts it when this PC has no WebView2 (or the menu can't draw its map), and for
//! Play.exe -Classic. Arguments are passed on (-Change opens the choices at once).

use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

pub fn start(root: &Path, args: &[OsString]) {
    let mut c = Command::new("powershell.exe");
    c.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-STA", "-WindowStyle", "Hidden", "-File"])
        .arg(root.join("launcher").join("play.ps1"))
        .args(args.iter().filter(|a| !a.eq_ignore_ascii_case("-Classic") && !a.eq_ignore_ascii_case("-Bench")))
        .current_dir(root);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: no console flashes up behind the launcher.
        c.creation_flags(0x0800_0000);
    }
    let _ = c.spawn();
}
