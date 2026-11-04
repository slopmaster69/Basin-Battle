//! The plain launcher, for when the map can't be drawn (no WebView2 on this PC, or no WebGL2),
//! and for Play.exe -Classic. Arguments are passed on (-Change opens the choices at once).
//! - The Windows download: launcher\play.ps1, the window Play had before the War Table.
//! - A Mac (or Linux) checkout: scripts/play.sh in a Terminal window, as the Mac scripts play.

use crate::backend::{Kind, Paths};
use std::ffi::OsString;
use std::process::Command;

pub fn start(paths: &Paths, args: &[OsString]) {
    let root = &paths.root;
    if paths.kind == Kind::Checkout {
        let script = root.join("scripts").join("play.sh");
        #[cfg(target_os = "macos")]
        {
            let cmd = format!("cd {} && bash {}", sh_quote(&root.to_string_lossy()), sh_quote(&script.to_string_lossy()));
            let _ = Command::new("osascript")
                .args(["-e", &format!("tell application \"Terminal\" to do script \"{}\"", cmd.replace('\\', "\\\\").replace('"', "\\\"")), "-e", "tell application \"Terminal\" to activate"])
                .spawn();
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = Command::new("x-terminal-emulator").arg("-e").arg("bash").arg(&script).current_dir(root).spawn()
                .or_else(|_| Command::new("bash").arg(&script).current_dir(root).spawn());
        }
        let _ = args;
        return;
    }
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

#[cfg(target_os = "macos")]
fn sh_quote(s: &str) -> String { format!("'{}'", s.replace('\'', "'\\''")) }
