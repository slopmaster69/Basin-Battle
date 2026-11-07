//! The update: the whole folder brought to the newest release, so the game, the server and the
//! add-ons always come from one version. Kept: settings.txt, the game's own settings
//! (benilla-config\config.toml: your keys and add-on settings) and the server's worlds
//! (server\saves, backed up first).

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::folder::{channel, is_newer, Channel, Kind, Paths, Project};
use crate::net::{latest_release, Release};

/// What the update is doing, for a progress line.
#[derive(Clone, Debug)]
pub enum Progress {
    Downloading { done: u64, total: u64 },
    Installing,
}

/// A newer release than this folder's, if GitHub has one. `Ok(None)`: nothing newer (or a folder
/// that never updates). `Err`: GitHub couldn't be asked, in a player's words.
pub fn check(paths: &Paths, project: &Project) -> Result<Option<Release>, String> {
    if paths.kind == Kind::Checkout {
        return Ok(None);
    }
    let mine = paths.installed_version();
    let ch = channel(&mine);
    if ch == Channel::Fixed {
        return Ok(None);
    }
    let Some(r) = latest_release(&project.repository, ch)? else { return Ok(None) };
    Ok(is_newer(&r.version, &mine).then_some(r))
}

/// Why an update can't happen right now, or nothing.
pub fn not_now() -> Option<&'static str> {
    if crate::proc::server_running() {
        return Some("Your server is running from this folder, so the update waits. Close its window and open Play again.");
    }
    if crate::proc::game_running() {
        return Some("The game is open, so the update waits. Close it and open Play again.");
    }
    None
}

/// The update itself: download, unpack, copy over this folder, write the new version last. On
/// success the folder is the new release and the caller starts the new Play.exe (this one has
/// been renamed Play.exe.old, since a running program can't be written over).
pub fn install(paths: &Paths, r: &Release, mut report: impl FnMut(Progress)) -> Result<String, String> {
    let root = &paths.root;
    let tmp = std::env::temp_dir().join(format!("elwynn-update-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).map_err(|e| format!("Couldn't make a temporary folder ({e})."))?;
    let result = install_in(paths, r, &tmp, &mut report);
    let _ = fs::remove_dir_all(&tmp);
    result.map_err(|e| format!("{e} Nothing in {} was changed.", root.display()))
}

fn install_in(paths: &Paths, r: &Release, tmp: &Path, report: &mut impl FnMut(Progress)) -> Result<String, String> {
    let root = &paths.root;
    // The download, in pieces, so the bar moves.
    let zip_path = tmp.join("download.zip");
    {
        let resp = crate::net::agent(600).get(&r.asset_url).call().map_err(|_| "The download didn't start (offline?).".to_string())?;
        let total = resp.header("Content-Length").and_then(|v| v.parse::<u64>().ok()).unwrap_or(r.asset_size);
        let mut reader = resp.into_reader();
        let mut file = fs::File::create(&zip_path).map_err(|e| format!("Couldn't write the download ({e})."))?;
        let mut buf = vec![0u8; 256 * 1024];
        let mut done = 0u64;
        report(Progress::Downloading { done, total });
        loop {
            let n = reader.read(&mut buf).map_err(|_| "The download broke off.".to_string())?;
            if n == 0 {
                break;
            }
            std::io::Write::write_all(&mut file, &buf[..n]).map_err(|e| format!("Couldn't write the download ({e})."))?;
            done += n as u64;
            report(Progress::Downloading { done, total });
        }
        if r.asset_size > 0 && done != r.asset_size {
            return Err("The download is the wrong size.".into());
        }
    }
    report(Progress::Installing);
    // Unpacked, and the folder with the game in it found (the zip has one folder at its top).
    let unpacked = tmp.join("new");
    {
        let file = fs::File::open(&zip_path).map_err(|e| e.to_string())?;
        let mut zip = zip::ZipArchive::new(file).map_err(|_| "The download isn't a zip.".to_string())?;
        zip.extract(&unpacked).map_err(|e| format!("The download couldn't be unpacked ({e})."))?;
    }
    let new = find_game_folder(&unpacked).ok_or("The download has no game in it.")?;
    if !new.join("version.txt").is_file() {
        return Err("The download isn't laid out as this folder is.".into());
    }
    let new_version = fs::read_to_string(new.join("version.txt")).map(|t| t.lines().next().unwrap_or("").trim().to_string()).unwrap_or_default();

    // Your worlds first, in case the new server has a problem with them: the last 10 kept.
    let saves = paths.server_dir.join("saves");
    if saves.is_dir() {
        let backups = paths.server_dir.join("saves-backups");
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let backup = backups.join(format!("before-{}-{stamp}", r.version.replace(' ', "-")));
        copy_tree(&saves, &backup, &|_| true).map_err(|e| format!("Couldn't back up the saves ({e})."))?;
        let mut old: Vec<PathBuf> = fs::read_dir(&backups).map(|d| d.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect()).unwrap_or_default();
        old.sort();
        while old.len() > 10 {
            let _ = fs::remove_dir_all(old.remove(0));
        }
    }

    // From here the folder is being written: a version.txt written last means an update cut short
    // is done again next time, because the folder still says the old version.
    // A running Play.exe or Host.exe can't be written over, but can be renamed out of the way.
    for button in ["Play.exe", "Host.exe"] {
        let at = root.join(button);
        let _ = fs::remove_file(root.join(format!("{button}.old")));
        if at.is_file() && new.join(button).is_file() {
            fs::rename(&at, root.join(format!("{button}.old"))).map_err(|e| format!("Couldn't move {button} aside ({e}). Is it open twice?"))?;
        }
    }
    // What the release owns outright is made exactly the new one, so nothing stale is left in it.
    for owned in ["benilla-config/AddOns/BasinBattle", "benilla-config/AddOns/ElwynnSurvival", "server/content"] {
        if new.join(owned).is_dir() {
            let _ = fs::remove_dir_all(root.join(owned));
        }
    }
    // Everything else over the old, which keeps your files (the download has none of them)...
    let keep = |rel: &Path| -> bool {
        let s = rel.to_string_lossy().replace('\\', "/");
        !(s == "version.txt" || s == "settings.txt" || s == "benilla-config/config.toml" || s.starts_with("server/saves"))
    };
    copy_tree(&new, root, &keep).map_err(|e| format!("Copying the new version in failed ({e}). Open Play again to finish the update."))?;
    // ...and what older downloads had that this one doesn't.
    for gone in ["launcher", "scripts", "Game"] {
        let _ = fs::remove_dir_all(root.join(gone));
    }
    for gone in [
        "launcher.ps1", "Play Basin Battle.bat", "Play Elwynn Survival.bat", "Play.bat", "Host.bat",
        "1 Start my server.bat", "2 Play Basin Battle (my server).bat", "2 Play Elwynn Survival (my server).bat",
        "Join someone else's game.bat", "Update to the newest version.bat", "READ ME FIRST.txt",
    ] {
        let _ = fs::remove_file(root.join(gone));
    }
    // The game's memory of item and creature details: asked afresh of the new server.
    let _ = fs::remove_dir_all(root.join("benilla-config").join("cache"));
    fs::copy(new.join("version.txt"), &paths.version).map_err(|e| format!("Couldn't write version.txt ({e})."))?;
    Ok(new_version)
}

/// The folder in an unpacked download that holds the game (benilla.exe, or the server on a Mac).
fn find_game_folder(dir: &Path) -> Option<PathBuf> {
    if dir.join("benilla.exe").is_file() || dir.join("version.txt").is_file() {
        return Some(dir.to_path_buf());
    }
    for e in fs::read_dir(dir).ok()?.flatten() {
        if e.path().is_dir() {
            if let Some(f) = find_game_folder(&e.path()) {
                return Some(f);
            }
        }
    }
    None
}

/// `from` copied into `to` (made as needed), file by file, skipping what `keep` says to leave
/// as it is (by the path relative to `from`).
pub fn copy_tree(from: &Path, to: &Path, keep: &dyn Fn(&Path) -> bool) -> std::io::Result<()> {
    fn walk(base: &Path, from: &Path, to: &Path, keep: &dyn Fn(&Path) -> bool) -> std::io::Result<()> {
        fs::create_dir_all(to)?;
        for e in fs::read_dir(from)? {
            let e = e?;
            let src = e.path();
            let rel = src.strip_prefix(base).unwrap_or(&src).to_path_buf();
            if !keep(&rel) {
                continue;
            }
            let dst = to.join(e.file_name());
            if src.is_dir() {
                walk(base, &src, &dst, keep)?;
            } else {
                // A file in use (a running exe) is renamed aside rather than failing the update.
                if fs::copy(&src, &dst).is_err() {
                    let aside = dst.with_extension(format!("{}.old", dst.extension().map(|x| x.to_string_lossy().to_string()).unwrap_or_default()));
                    let _ = fs::remove_file(&aside);
                    fs::rename(&dst, &aside)?;
                    fs::copy(&src, &dst)?;
                }
            }
        }
        Ok(())
    }
    walk(from, from, to, keep)
}

/// A playtest copy of this folder beside it ("<name> (test)"): the same files, its own saves and
/// settings, on the `latest` channel, so it updates itself to the newest main. Returns its Play.
pub fn make_test_folder(paths: &Paths) -> Result<PathBuf, String> {
    let root = &paths.root;
    let name = root.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Elwynn Survival".into());
    let test = root.parent().unwrap_or(root).join(format!("{name} (test)"));
    let keep = |rel: &Path| -> bool {
        let s = rel.to_string_lossy().replace('\\', "/");
        !(s.starts_with("server/saves") || s.starts_with("server/logs") || s.starts_with("benilla-config/cache") || s.ends_with(".old") || s == "play.log")
    };
    copy_tree(root, &test, &keep).map_err(|e| format!("Couldn't make the test folder ({e})."))?;
    fs::write(test.join("version.txt"), "latest none\r\n").map_err(|e| e.to_string())?;
    Ok(test.join("Play.exe"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copying_keeps_what_it_is_told_to() {
        let dir = std::env::temp_dir().join(format!("launcher-copy-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let (from, to) = (dir.join("from"), dir.join("to"));
        fs::create_dir_all(from.join("server").join("saves")).unwrap();
        fs::create_dir_all(to.join("server").join("saves")).unwrap();
        fs::write(from.join("a.txt"), "new").unwrap();
        fs::write(from.join("settings.txt"), "theirs").unwrap();
        fs::write(from.join("server").join("saves").join("w.toml"), "fresh world").unwrap();
        fs::write(to.join("settings.txt"), "mine").unwrap();
        fs::write(to.join("server").join("saves").join("w.toml"), "my world").unwrap();
        let keep = |rel: &Path| {
            let s = rel.to_string_lossy().replace('\\', "/");
            !(s == "settings.txt" || s.starts_with("server/saves"))
        };
        copy_tree(&from, &to, &keep).unwrap();
        assert_eq!(fs::read_to_string(to.join("a.txt")).unwrap(), "new");
        assert_eq!(fs::read_to_string(to.join("settings.txt")).unwrap(), "mine");
        assert_eq!(fs::read_to_string(to.join("server").join("saves").join("w.toml")).unwrap(), "my world");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn the_game_folder_is_found_under_the_zips_top_folder() {
        let dir = std::env::temp_dir().join(format!("launcher-find-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let inner = dir.join("Elwynn Survival for Windows");
        fs::create_dir_all(&inner).unwrap();
        fs::write(inner.join("version.txt"), "v0.5.0").unwrap();
        assert_eq!(find_game_folder(&dir), Some(inner));
        let _ = fs::remove_dir_all(dir);
    }
}
