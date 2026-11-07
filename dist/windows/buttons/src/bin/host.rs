//! Host: this PC's server for both games, the one that came in the download (nothing to build).
//! A console window: leave it open while anyone plays, close it to stop the server. It brings the
//! folder to the newest version first, prints the addresses friends type, then runs the server.
//!
//!   Host.exe --password secret    the password everyone logs in with
//!   Host.exe --game survival      only one of the games (or --game basin)
//!   Host.exe --version            say the folder's version and stop (the build checks this)
//! Any other option goes to the server as given (Host.exe --help lists them).

use std::io::Write;
use std::process::Command;
use std::time::Duration;

use launcher::folder::{self, Paths, Project};
use launcher::{net, server, update, wow};

fn say(text: &str) {
    println!("{text}");
}

fn stop_here(text: &str) -> ! {
    println!("\n{text}");
    print!("Press Enter to close. ");
    let _ = std::io::stdout().flush();
    let mut s = String::new();
    let _ = std::io::stdin().read_line(&mut s);
    std::process::exit(1);
}

#[cfg(windows)]
fn console_title(title: &str) {
    use windows_sys::Win32::System::Console::SetConsoleTitleW;
    let w: Vec<u16> = title.encode_utf16().chain([0]).collect();
    unsafe { SetConsoleTitleW(w.as_ptr()) };
}
#[cfg(not(windows))]
fn console_title(_title: &str) {}

fn main() {
    let extra: Vec<String> = std::env::args().skip(1).collect();
    let exe = std::env::current_exe().unwrap_or_default();
    let paths = Paths::find(&exe).unwrap_or_else(|| Paths::download(std::path::PathBuf::from(".")));
    let project = Project::built_in();
    let version = paths.installed_version();
    if extra.iter().any(|a| a == "--version") {
        println!("{} {}", project.title, if version.is_empty() { "(no version.txt)" } else { &version });
        return;
    }
    console_title(&format!("{} server (close this window to stop it)", project.title));
    say(&format!("{} {}", project.title, version));

    // The newest version first: the new Host takes over.
    if !extra.iter().any(|a| a == "--no-update") {
        if let Some(code) = update_and_restart(&paths, &project, &extra) {
            std::process::exit(code);
        }
    }

    if !paths.server_exe.is_file() {
        stop_here("server\\htl-server.exe is missing. Your anti-virus may have taken it: restore it, or download the game again.");
    }
    let settings = folder::read_settings(&paths.settings);
    let ports = server::own_ports(&settings);
    for port in [ports.login, ports.basin, ports.survival] {
        if net::reach("127.0.0.1", port, 400) {
            stop_here(&format!("Port {port} is taken: another server (another copy of the game, or a test build) is running on this PC. Close it first."));
        }
    }

    // The player's game files: the server builds its worlds from them.
    let mut settings = settings;
    let data = match wow::data_folder_of(settings.get("wow").map(String::as_str).unwrap_or("")) {
        Some(d) => d,
        None => {
            say("Looking for your World of Warcraft 1.12.1...");
            let found = wow::find_wow_data(&paths.root).or_else(|| wow::pick_wow_data(&project.title));
            let Some(d) = found else { stop_here("No WoW 1.12.1 folder picked: the server builds its worlds from your game files. Run Host again when you have one.") };
            settings.insert("wow".into(), d.to_string_lossy().to_string());
            folder::save_settings(&paths.settings, &settings);
            d
        }
    };

    // What friends type.
    let suffix = ports.suffix();
    println!();
    say("Give your friends one of these addresses (they put it under the cog in Play, \"Someone else's\"):");
    if let Some(lan) = net::lan_address() {
        say(&format!("  Same house or Wi-Fi:      {lan}{suffix}"));
    }
    if let Some(ts) = net::tailscale_address() {
        say(&format!("  Over Tailscale:           {ts}{suffix}"));
    }
    let mut args = server::args(&project, &data, ports);
    if !extra.iter().any(|a| a == "--public-host") {
        if let Some(public) = net::public_address() {
            say(&format!("  Over the internet:        {public}{suffix}   (once your router forwards TCP {}, {} and {} to this PC)", ports.login, ports.basin, ports.survival));
            args.push("--public-host".into());
            args.push(public);
        }
    }
    if !extra.iter().any(|a| a == "--password") && project.password != "hold" {
        args.push("--password".into());
        args.push(project.password.clone());
    }
    let password = extra.iter().position(|a| a == "--password").and_then(|i| extra.get(i + 1)).cloned().unwrap_or_else(|| project.password.clone());
    say(&format!("Password: {password}"));
    say("If Windows asks whether htl-server may use the network, click Allow.");
    println!();

    let _ = std::fs::create_dir_all(paths.server_dir.join("saves"));
    let _ = std::fs::create_dir_all(paths.server_dir.join("logs"));
    args.extend(extra.iter().filter(|a| *a != "--no-update").cloned());
    loop {
        let status = Command::new(&paths.server_exe).args(&args).current_dir(&paths.server_dir).status();
        let code = match status {
            Ok(s) => s.code().unwrap_or(1),
            Err(e) => stop_here(&format!("The server didn't start ({e}).")),
        };
        // 75: a warned restart (Restart Basin Battle, or the minutes written in restart.request).
        // The new version, if there is one, comes in between.
        if code != 75 {
            stop_here(&format!("The server stopped (code {code})."));
        }
        say("Restarting...");
        std::thread::sleep(Duration::from_secs(1));
        if let Some(code) = update_and_restart(&paths, &project, &extra) {
            std::process::exit(code);
        }
    }
}

/// A newer version brought in and the new Host run in its place: its exit code, or `None` when
/// this Host carries on.
fn update_and_restart(paths: &Paths, project: &Project, extra: &[String]) -> Option<i32> {
    match update::check(paths, project) {
        Ok(None) => None,
        Err(why) => {
            say(&why);
            None
        }
        Ok(Some(r)) => {
            if let Some(why) = update::not_now() {
                say(&format!("{} is out, but: {why}", r.version));
                return None;
            }
            say(&format!("A new version is out: {} (you have {}). Updating...", r.version, paths.installed_version()));
            let mut last = 0u64;
            let result = update::install(paths, &r, |p| match p {
                update::Progress::Downloading { done, total } => {
                    if done / 5_000_000 > last {
                        last = done / 5_000_000;
                        println!("  {} of {} MB", done / 1_000_000, total.max(done) / 1_000_000);
                    }
                }
                update::Progress::Installing => println!("  Installing..."),
            });
            match result {
                Ok(v) => {
                    say(&format!("Updated to {v}."));
                    let host = paths.root.join(if cfg!(windows) { "Host.exe" } else { "Host" });
                    let code = Command::new(host).args(extra).arg("--no-update").current_dir(&paths.root).status().ok().and_then(|s| s.code()).unwrap_or(1);
                    Some(code)
                }
                Err(why) => {
                    say(&why);
                    None
                }
            }
        }
    }
}
