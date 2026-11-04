//! Play.exe: the launcher. Its window is the War Table menu (launcher\ui\index.html, drawn by
//! the WebView2 that comes with Windows 10 and 11); what it does behind the menu is in
//! backend.rs. Without WebView2, or if the menu can't draw its map, it starts the plain launcher
//! instead (launcher\play.ps1, classic.rs), so Play always plays.
//!   Play.exe -Change       opens the server list at once
//!   Play.exe -Game basin   opens on Basin Battle
//!   Play.exe -NoUpdate     doesn't look for a new version this time
//!   Play.exe -Classic      the plain launcher
//!   Play.exe -Bench        measures how smoothly this PC draws the map
#![windows_subsystem = "windows"]

mod backend;
mod classic;

use backend::{Backend, Options};
use serde_json::Value;
use std::borrow::Cow;
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};
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
    let root = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)).unwrap_or_default();
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let flag = |f: &str| args.iter().any(|a| a.eq_ignore_ascii_case(f));
    let ui = root.join("launcher").join("ui");
    if flag("-Classic") || !ui.join("index.html").is_file() {
        classic::start(&root, &args);
        return;
    }
    let game = args.iter().position(|a| a.eq_ignore_ascii_case("-Game")).and_then(|i| args.get(i + 1)).map(|g| g.to_string_lossy().to_string());
    let opts = Options { change: flag("-Change"), no_update: flag("-NoUpdate"), game, bench: flag("-Bench") };
    if let Err(e) = run(root.clone(), ui, opts) {
        // No WebView2 on this PC (or it wouldn't start): the plain launcher.
        let _ = std::fs::write(root.join("launcher").join("play.log"), format!("Play: the menu couldn't open ({e}), so the plain launcher instead.\r\n"));
        classic::start(&root, &args);
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

/// A file of the menu's, from launcher\ui only.
fn serve(ui: &Path, uri_path: &str) -> Response<Cow<'static, [u8]>> {
    let rel = uri_path.trim_start_matches('/');
    let rel = if rel.is_empty() { "index.html" } else { rel };
    let rel = Path::new(rel);
    let safe = rel.components().all(|c| matches!(c, Component::Normal(_)));
    let path = ui.join(rel);
    match std::fs::read(&path) {
        Ok(bytes) if safe => Response::builder()
            .header(CONTENT_TYPE, content_type(&path))
            .header("Cache-Control", "no-store")
            .body(Cow::Owned(bytes))
            .unwrap(),
        _ => Response::builder().status(404).body(Cow::Borrowed(&b"not found"[..])).unwrap(),
    }
}

fn run(root: PathBuf, ui: PathBuf, opts: Options) -> Result<(), String> {
    let event_loop = EventLoopBuilder::<Ui>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let bench = opts.bench;
    let backend = {
        let (p1, p2) = (proxy.clone(), proxy.clone());
        Backend::new(
            root.clone(),
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
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut wb = WindowBuilder::new()
        .with_title(&backend.project.title)
        .with_inner_size(LogicalSize::new(w, h))
        .with_min_inner_size(LogicalSize::new(f64::min(960.0, w), f64::min(600.0, h)))
        .with_decorations(false)
        .with_visible(false);
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
    let ui_dir = ui.clone();
    let p_ipc = proxy.clone();
    let url = if bench { "bb://localhost/index.html#bench" } else { "bb://localhost/index.html" };
    let builder = WebViewBuilder::new_with_web_context(&mut context)
        .with_url(url)
        .with_background_color((16, 12, 8, 255))
        .with_asynchronous_custom_protocol("bb".into(), move |_, req, responder| {
            let (ui, path) = (ui_dir.clone(), req.uri().path().to_string());
            std::thread::spawn(move || responder.respond(serve(&ui, &path)));
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
    let classic_root = root.clone();
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
                classic::start(&classic_root, &std::env::args_os().skip(1).collect::<Vec<_>>());
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
                            .set_title("Pick your World of Warcraft 1.12.1 folder (the one with WoW.exe and a Data folder in it)")
                            .set_parent(&window)
                            .pick_folder();
                        match picked {
                            Some(dir) => { backend.picked_wow(&dir); }
                            None => { let _ = webview.evaluate_script("window.launcherRecv && launcherRecv({t:'wow',cancelled:true})"); }
                        }
                    }
                    "classic" => {
                        backend.log(&format!("the menu can't draw here ({}): the plain launcher instead", m.get("why").and_then(|v| v.as_str()).unwrap_or("")));
                        classic::start(&classic_root, &std::env::args_os().skip(1).collect::<Vec<_>>());
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
