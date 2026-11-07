//! The player's World of Warcraft 1.12.1 install: its Data folder, found or picked once.

use std::fs;
use std::path::{Path, PathBuf};

/// The Data folder, if `dir` is it or the WoW folder holding it (with dbc.MPQ in it: modern WoW
/// Classic has no MPQs, so it never matches).
pub fn data_folder_of(dir: &str) -> Option<PathBuf> {
    let d = dir.trim().trim_matches('"');
    if d.is_empty() {
        return None;
    }
    let p = PathBuf::from(d);
    if p.join("dbc.MPQ").is_file() {
        return Some(p);
    }
    let inner = p.join("Data");
    if inner.join("dbc.MPQ").is_file() {
        return Some(inner);
    }
    None
}

/// Where people keep it: the usual folder names on every local drive, Program Files, the
/// desktop, Documents, Downloads and Games folders, one level into any folder whose name says WoW.
pub fn find_wow_data(root: &Path) -> Option<PathBuf> {
    let mut places: Vec<PathBuf> = Vec::new();
    #[cfg(not(windows))]
    {
        // On a Mac, the places the scripts look first (beside this folder, in it, in Applications).
        for p in [root.join("WoW").join("Data"), root.join("..").join("WoW Classic").join("WoW-1.12.1").join("Data")] {
            if let Some(found) = data_folder_of(&p.to_string_lossy()) {
                return Some(found);
            }
        }
        places.push(root.to_path_buf());
        if let Some(parent) = root.parent() {
            places.push(parent.to_path_buf());
        }
        places.push(PathBuf::from("/Applications"));
        if let Some(h) = dirs::home_dir() {
            places.push(h.join("Applications"));
        }
    }
    #[cfg(windows)]
    let _ = root;
    for d in fixed_drives() {
        places.push(d.clone());
        places.push(d.join("Games"));
    }
    let home = dirs::home_dir();
    if let Some(h) = &home {
        places.push(h.clone());
    }
    if let Some(d) = dirs::desktop_dir() {
        places.push(d);
    }
    if let Some(d) = dirs::document_dir() {
        places.push(d);
    }
    if let Some(h) = &home {
        places.push(h.join("Downloads"));
        places.push(h.join("Games"));
    }
    for v in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(p) = std::env::var_os(v) {
            places.push(PathBuf::from(p));
        }
    }
    let wowish = |name: &str| {
        let n = name.to_lowercase();
        ["wow", "warcraft", "1.12", "vanilla", "azeroth"].iter().any(|w| n.contains(w))
    };
    let subdirs = |p: &Path| -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = fs::read_dir(p)
            .map(|r| r.flatten().filter(|e| e.path().is_dir()).map(|e| e.path()).collect())
            .unwrap_or_default();
        v.sort_by_key(|p| p.to_string_lossy().to_lowercase());
        v
    };
    for place in places {
        if !place.is_dir() {
            continue;
        }
        for f in subdirs(&place) {
            if !f.file_name().map(|n| wowish(&n.to_string_lossy())).unwrap_or(false) {
                continue;
            }
            if let Some(found) = data_folder_of(&f.to_string_lossy()) {
                return Some(found);
            }
            for inner in subdirs(&f) {
                if let Some(found) = data_folder_of(&inner.to_string_lossy()) {
                    return Some(found);
                }
            }
        }
    }
    None
}

/// Local drives only: a network drive could keep the search waiting.
#[cfg(windows)]
fn fixed_drives() -> Vec<PathBuf> {
    use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives};
    let mask = unsafe { GetLogicalDrives() };
    (0..26u8)
        .filter(|i| mask & (1 << i) != 0)
        .map(|i| format!("{}:\\", (b'A' + i) as char))
        .filter(|root| {
            let w: Vec<u16> = root.encode_utf16().chain([0]).collect();
            unsafe { GetDriveTypeW(w.as_ptr()) == 3 } // DRIVE_FIXED
        })
        .map(PathBuf::from)
        .collect()
}
#[cfg(not(windows))]
fn fixed_drives() -> Vec<PathBuf> {
    Vec::new()
}

/// The folder picker: the Data folder of what was picked, or `None` for a cancel. Asks again
/// when the pick isn't a 1.12.1 install.
pub fn pick_wow_data(title: &str) -> Option<PathBuf> {
    loop {
        let picked = rfd::FileDialog::new()
            .set_title(format!("{title}: pick your World of Warcraft 1.12.1 folder (the one with WoW.exe in it)"))
            .pick_folder()?;
        if let Some(data) = data_folder_of(&picked.to_string_lossy()) {
            return Some(data);
        }
        rfd::MessageDialog::new()
            .set_title(title)
            .set_description("That folder has no Data\\dbc.MPQ in it, so it isn't a WoW 1.12.1 install. Pick the folder that has WoW.exe in it. (Modern WoW Classic won't work: it has to be 1.12.1.)")
            .show();
    }
}
