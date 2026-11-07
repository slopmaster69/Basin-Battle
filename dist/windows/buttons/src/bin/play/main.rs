//! Play.exe: the launcher. Its window is the War Table menu (launcher\ui\index.html, drawn by
//! the WebView2 that comes with Windows 10 and 11, or by the Mac's own WebKit); what it does
//! behind the menu is in backend.rs. Without WebView2, or if the menu can't draw its map, it
//! starts the plain launcher instead (classic.rs), so Play always plays.
//! On a Mac it runs from the repository (Basin Battle.app, made by Play Basin Battle.command).
//!   Play.exe -Change       opens the server list at once
//!   Play.exe -Game basin   opens on Basin Battle
//!   Play.exe -NoUpdate     doesn't look for a new version this time
//!   Play.exe -Classic      the plain launcher
//!   Play.exe -Bench        measures how smoothly this PC draws the map
//!   Play.exe -Map2D        draws the map with a 2D canvas instead of WebGL (a Mac does unless -WebGL)
#![windows_subsystem = "windows"]

mod backend;
mod classic;

use backend::{Backend, Kind, Options, Paths};
use serde_json::Value;
use std::borrow::Cow;
use std::ffi::OsString;
use std::path::{Component, Path};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tao::dpi::LogicalSize;
use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::{ResizeDirection, WindowBuilder};
use wry::http::{header::CONTENT_TYPE, Response};
use wry::{WebContext, WebViewBuilder};

enum Ui {
    /// A message for the menu.
    Js(String),
    /// A message from the menu.
    Ipc(String),
    Show,
    /// The menu never said hello: the plain launcher instead.
    Stalled,
    Quit,
}

fn main() {
    let exe = std::env::current_exe().unwrap_or_default();
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let flag = |f: &str| args.iter().any(|a| a.eq_ignore_ascii_case(f));
    let Some(paths) = Paths::find(&exe) else {
        // A Play.exe without its menu beside it: the plain launcher, if it's there.
        classic::start(&Paths::download(exe.parent().map(Path::to_path_buf).unwrap_or_default()), &args);
        return;
    };
    if flag("-Classic") {
        classic::start(&paths, &args);
        return;
    }
    // A menu Play may not read (macOS keeps an app out of Documents, Desktop and Downloads until
    // it's allowed): said in a window, not a blank one.
    if let Err(e) = std::fs::read(paths.ui.join("index.html")) {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            blocked(&paths.root);
            return;
        }
    }
    #[cfg(target_os = "macos")]
    refresh_on_a_mac(&paths, &exe);
    let game = args.iter().position(|a| a.eq_ignore_ascii_case("-Game")).and_then(|i| args.get(i + 1)).map(|g| g.to_string_lossy().to_string());
    let opts = Options { change: flag("-Change"), no_update: flag("-NoUpdate"), game, bench: flag("-Bench") };
    let (log, kind, root) = (paths.log.clone(), paths.kind, paths.root.clone());
    if let Err(e) = run(paths, opts) {
        // No WebView2 on this PC (or it wouldn't start): the plain launcher.
        let _ = std::fs::write(log, format!("Play: the menu couldn't open ({e}), so the plain launcher instead.\r\n"));
        classic::start(&if kind == Kind::Download { Paths::download(root) } else { Paths::checkout(root) }, &args);
    }
}

/// The system won't let Play read its own folder: what to do about it, in a dialog.
fn blocked(root: &Path) {
    let what = if cfg!(target_os = "macos") {
        format!(
            "macOS isn't letting Basin Battle open its folder:\n{}\n\nAllow it in System Settings → Privacy & Security → Files & Folders (or add Basin Battle.app to Full Disk Access), or move the folder out of Documents, Desktop and Downloads. Then open Basin Battle again.",
            root.display()
        )
    } else {
        format!("Play isn't allowed to open its folder:\n{}\n\nCheck the folder's permissions, then open Play again.", root.display())
    };
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Warning)
        .set_title("Basin Battle can't open its folder")
        .set_description(what)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

/// On a Mac, Play is Basin Battle.app's own program, so macOS asks for the folders it needs on
/// the app's behalf. Before the window it does what the app's script once did: Play rebuilt if
/// its code changed (scripts/mac-launcher.sh, seconds when nothing did), and started again if it
/// was; then the dispatches written from this folder's history.
#[cfg(target_os = "macos")]
fn refresh_on_a_mac(paths: &Paths, exe: &Path) {
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};
    let in_app = exe.ancestors().any(|a| a.extension().is_some_and(|x| x == "app"));
    if paths.kind != Kind::Checkout || !in_app || std::env::var_os("BB_REFRESHED").is_some() {
        return;
    }
    let root = &paths.root;
    // Opened from the Finder, an app has the system's PATH only: cargo is where rustup puts it.
    let mut path = OsString::new();
    if let Some(home) = dirs::home_dir() {
        path.push(home.join(".cargo").join("bin"));
        path.push(":");
    }
    path.push(std::env::var_os("PATH").unwrap_or_else(|| "/usr/bin:/bin:/usr/sbin:/sbin".into()));
    let quiet = |c: &mut Command| {
        let _ = c.current_dir(root).env("PATH", &path).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status();
    };
    let stamp = |p: &Path| std::fs::metadata(p).ok().map(|m| (m.len(), m.modified().ok()));
    let before = stamp(exe);
    quiet(Command::new("bash").arg(root.join("scripts").join("mac-launcher.sh")).arg("--quiet"));
    // python3 comes with Apple's command line tools (without them it would ask to install them).
    let tools = Command::new("xcode-select").arg("-p").stdout(Stdio::null()).stderr(Stdio::null()).status();
    if tools.is_ok_and(|s| s.success()) {
        quiet(Command::new("python3").arg(root.join("scripts").join("launcher-news.py")).arg(paths.ui.join("news.json")));
    }
    if stamp(exe) != before {
        // The new Play, in this process. If it can't start, this one carries on.
        let _ = Command::new(exe).args(std::env::args_os().skip(1)).env("BB_REFRESHED", "1").exec();
    }
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase().as_str() {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "png" => "image/png",
        "webp" => "image/webp",
        "jpg" | "jpeg" => "image/jpeg",
        "svg" => "image/svg+xml",
        "woff2" => "font/woff2",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    }
}

/// A file of the menu's, from its folder only (or the shield, from the art beside it).
fn serve(ui: &Path, art: &Path, uri_path: &str) -> Response<Cow<'static, [u8]>> {
    let rel = uri_path.trim_start_matches('/');
    let rel = if rel.is_empty() { "index.html" } else { rel };
    let rel = Path::new(rel);
    let safe = rel.components().all(|c| matches!(c, Component::Normal(_)));
    let mut path = ui.join(rel);
    if !path.is_file() && rel == Path::new("shield.png") { path = art.join(rel); }
    match std::fs::read(&path) {
        Ok(bytes) if safe => Response::builder()
            .header(CONTENT_TYPE, content_type(&path))
            .header("Cache-Control", "no-store")
            .body(Cow::Owned(bytes))
            .unwrap(),
        _ => Response::builder().status(404).body(Cow::Borrowed(&b"not found"[..])).unwrap(),
    }
}

fn run(paths: Paths, opts: Options) -> Result<(), String> {
    let (root, ui, art, kind) = (paths.root.clone(), paths.ui.clone(), paths.art.clone(), paths.kind);
    let event_loop = EventLoopBuilder::<Ui>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let bench = opts.bench;
    let backend = {
        let (p1, p2) = (proxy.clone(), proxy.clone());
        Backend::new(
            paths,
            opts,
            Arc::new(move |v: Value| { let _ = p1.send_event(Ui::Js(v.to_string())); }),
            Arc::new(move || { let _ = p2.send_event(Ui::Quit); }),
        )
    };

    // Big enough for the map, never bigger than the screen.
    let (mut w, mut h) = (1280.0, 800.0);
    if let Some(m) = event_loop.primary_monitor() {
        let s = m.size().to_logical::<f64>(m.scale_factor());
        w = f64::min(w, s.width * 0.92);
        h = f64::min(h, s.height * 0.88);
    }
    let wb = WindowBuilder::new()
        .with_title(&backend.project.title)
        .with_inner_size(LogicalSize::new(w, h))
        .with_min_inner_size(LogicalSize::new(f64::min(960.0, w), f64::min(600.0, h)))
        .with_visible(false);
    // Windows and Linux: no frame, the menu's own title bar. A Mac keeps its traffic lights,
    // over the menu's title bar.
    #[cfg(not(target_os = "macos"))]
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut wb = wb.with_decorations(false);
    #[cfg(target_os = "macos")]
    let wb = {
        use tao::platform::macos::WindowBuilderExtMacOS;
        wb.with_titlebar_transparent(true).with_title_hidden(true).with_fullsize_content_view(true)
    };
    #[cfg(windows)]
    {
        use tao::platform::windows::{IconExtWindows, WindowBuilderExtWindows};
        let icon = root.join("launcher").join("icon.ico");
        if let Ok(i) = tao::window::Icon::from_path(&icon, None) { wb = wb.with_window_icon(Some(i)); }
        // WebView2 does its own drag and drop.
        wb = wb.with_drag_and_drop(false).with_undecorated_shadow(true);
    }
    let window = wb.build(&event_loop).map_err(|e| e.to_string())?;
    window.set_outer_position(centre(&window, LogicalSize::new(w, h)));

    // WebView2 keeps its own files (its cache) outside the game's folder, which updates replace.
    let data = dirs::data_local_dir().unwrap_or_else(std::env::temp_dir).join(&backend.project.title).join("WebView2");
    let mut context = WebContext::new(Some(data));
    let (ui_dir, art_dir) = (ui.clone(), art.clone());
    let p_ipc = proxy.clone();
    // #map2d: the map drawn with a 2D canvas instead of WebGL (Play -Map2D, or BB_MAP2D=1). A Mac
    // draws it that way unless told -WebGL: there its WebGL can draw without reaching the screen
    // (seen on GitHub's Mac runners), while its 2D canvas always does, as fast and as faithful.
    let arg = |f: &str| std::env::args_os().any(|a| a.eq_ignore_ascii_case(f));
    let map2d = arg("-Map2D") || std::env::var_os("BB_MAP2D").is_some() || (cfg!(target_os = "macos") && !arg("-WebGL"));
    let hash = [(bench, "bench"), (map2d, "map2d")].iter().filter(|(on, _)| *on).map(|(_, n)| *n).collect::<Vec<_>>().join("-");
    let url = if hash.is_empty() { "bb://localhost/index.html".to_string() } else { format!("bb://localhost/index.html#{hash}") };
    let builder = WebViewBuilder::new_with_web_context(&mut context)
        .with_url(&url)
        .with_background_color((16, 12, 8, 255))
        .with_asynchronous_custom_protocol("bb".into(), move |_, req, responder| {
            let (ui, art, path) = (ui_dir.clone(), art_dir.clone(), req.uri().path().to_string());
            std::thread::spawn(move || responder.respond(serve(&ui, &art, &path)));
        })
        .with_ipc_handler(move |req| { let _ = p_ipc.send_event(Ui::Ipc(req.body().clone())); })
        // The menu is one page: links to the web open in the browser instead (the menu asks).
        .with_navigation_handler(|u| u.starts_with("bb://") || u.starts_with("http://bb.localhost") || u.starts_with("https://bb.localhost") || u == "about:blank")
        .with_new_window_req_handler(|_, _| wry::NewWindowResponse::Deny)
        .with_focused(true);
    #[cfg(windows)]
    let builder = {
        use wry::WebViewBuilderExtWindows;
        builder.with_browser_accelerator_keys(false).with_default_context_menus(false)
    };
    #[cfg(target_os = "linux")]
    let webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        builder.build_gtk(window.default_vbox().ok_or("no GTK box")?).map_err(|e| e.to_string())?
    };
    #[cfg(not(target_os = "linux"))]
    let webview = builder.build(&window).map_err(|e| e.to_string())?;

    // Shown once the menu has drawn (no white flash); if it never says hello, the plain
    // launcher takes over.
    let hello = Arc::new(AtomicBool::new(false));
    {
        let (p, hello) = (proxy.clone(), hello.clone());
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(4));
            let _ = p.send_event(Ui::Show);
            std::thread::sleep(Duration::from_secs(16));
            if !hello.load(Ordering::SeqCst) { let _ = p.send_event(Ui::Stalled); }
        });
    }
    let mut shown = false;
    let classic_paths = if kind == Kind::Download { Paths::download(root.clone()) } else { Paths::checkout(root.clone()) };
    event_loop.run(move |event, _, flow| {
        *flow = ControlFlow::Wait;
        let _ = &context;
        match event {
            Event::NewEvents(StartCause::Init) => {}
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *flow = ControlFlow::Exit,
            Event::UserEvent(Ui::Js(json)) => {
                let _ = webview.evaluate_script(&format!("window.launcherRecv && launcherRecv({json})"));
            }
            Event::UserEvent(Ui::Show) => {
                if !shown { shown = true; window.set_visible(true); window.set_focus(); }
            }
            Event::UserEvent(Ui::Quit) => *flow = ControlFlow::Exit,
            Event::UserEvent(Ui::Stalled) => {
                classic::start(&classic_paths, &std::env::args_os().skip(1).collect::<Vec<_>>());
                *flow = ControlFlow::Exit;
            }
            Event::UserEvent(Ui::Ipc(body)) => {
                let Ok(m) = serde_json::from_str::<Value>(&body) else { return };
                let what = m.get("do").and_then(|v| v.as_str()).unwrap_or("");
                match what {
                    "hello" => { hello.store(true, Ordering::SeqCst); backend.handle(&m); }
                    "ready" => {
                        backend.log("the menu is drawn");
                        if !shown { shown = true; window.set_visible(true); window.set_focus(); }
                    }
                    "drag" => { let _ = window.drag_window(); }
                    "max" => window.set_maximized(!window.is_maximized()),
                    "min" => window.set_minimized(true),
                    "close" => *flow = ControlFlow::Exit,
                    "resize" => {
                        let d = match m.get("edge").and_then(|v| v.as_str()).unwrap_or("") {
                            "n" => ResizeDirection::North, "s" => ResizeDirection::South,
                            "e" => ResizeDirection::East, "w" => ResizeDirection::West,
                            "ne" => ResizeDirection::NorthEast, "nw" => ResizeDirection::NorthWest,
                            "se" => ResizeDirection::SouthEast, _ => ResizeDirection::SouthWest,
                        };
                        if !window.is_maximized() { let _ = window.drag_resize_window(d); }
                    }
                    "pick-wow" => {
                        let picked = rfd::FileDialog::new()
                            .set_title("Pick your World of Warcraft 1.12.1 folder (the one with a Data folder in it)")
                            .set_parent(&window)
                            .pick_folder();
                        match picked {
                            Some(dir) => { backend.picked_wow(&dir); }
                            None => { let _ = webview.evaluate_script("window.launcherRecv && launcherRecv({t:'wow',cancelled:true})"); }
                        }
                    }
                    "classic" => {
                        backend.log(&format!("the menu can't draw here ({}): the plain launcher instead", m.get("why").and_then(|v| v.as_str()).unwrap_or("")));
                        classic::start(&classic_paths, &std::env::args_os().skip(1).collect::<Vec<_>>());
                        *flow = ControlFlow::Exit;
                    }
                    _ => backend.handle(&m),
                }
            }
            _ => {}
        }
    });
}

/// The middle of the screen the window opens on.
fn centre(window: &tao::window::Window, size: LogicalSize<f64>) -> tao::dpi::PhysicalPosition<i32> {
    match window.current_monitor().or_else(|| window.primary_monitor()) {
        Some(m) => {
            let size = size.to_physical::<u32>(m.scale_factor());
            let (mp, ms) = (m.position(), m.size());
            tao::dpi::PhysicalPosition::new(mp.x + (ms.width as i32 - size.width as i32) / 2, mp.y + (ms.height as i32 - size.height as i32) / 2)
        }
        None => tao::dpi::PhysicalPosition::new(100, 100),
    }
}
