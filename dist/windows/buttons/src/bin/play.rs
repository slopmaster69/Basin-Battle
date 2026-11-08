//! Play: the one window between the player and the game. A borderless splash with a painting,
//! the game's name, one line saying what's happening and a thin gold progress line. It finds the
//! WoW 1.12.1 folder (asks once), brings the folder to the newest version when there is one,
//! starts the game logged in on the group's server, and closes. "Play alone" starts this PC's own
//! server quietly first and stops it when the game closes. The cog has the few choices there are.
//!
//!   Play.exe --no-update    don't look for a new version this time (the new Play after an update)
//!   Play.exe --solo         play alone at once
//!   Play.exe --settings     open the choices at once
//!   Play.exe --shot a.png   draw the splash, save a picture of it there, and stop (the build
//!                           keeps one, so the splash can be seen without running anything)
//!
//! What it did is kept in play.log beside it.
#![cfg_attr(all(windows, not(debug_assertions), not(feature = "console")), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::process::Child;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui::{self, Align2, Color32, FontFamily, FontId, Pos2, Rect, Vec2};
use launcher::folder::{self, Channel, Kind, Paths, Project, Settings};
use launcher::{game, net, proc, server, update, wow};

const W: f32 = 960.0;
const H: f32 = 540.0;
/// The splash waits this long once ready, so a hand on the mouse can reach the cog.
const PAUSE: Duration = Duration::from_millis(2500);

const SPLASH: &[u8] = include_bytes!("../../../art/splash.jpg");
const SHIELD: &[u8] = include_bytes!("../../../art/shield.png");
const TITLE_FONT: &[u8] = include_bytes!("../../fonts/CormorantGaramond-SemiBold.ttf");
const TEXT_FONT: &[u8] = include_bytes!("../../fonts/BarlowSemiCondensed-Medium.ttf");

fn gold() -> Color32 { Color32::from_rgb(232, 193, 90) }
fn ink() -> Color32 { Color32::from_rgb(240, 228, 205) }
fn dim() -> Color32 { Color32::from_rgb(190, 176, 150) }
fn ember() -> Color32 { Color32::from_rgb(240, 150, 120) }

/// A smooth patch of night: a grid of columns `xs` and rows `ys`, each `(position, darkness 0..1)`,
/// a vertex's darkness being the two multiplied. One mesh, so no banding.
fn shade(p: &egui::Painter, xs: &[(f32, f32)], ys: &[(f32, f32)]) {
    let mut mesh = egui::Mesh::default();
    for &(y, ay) in ys {
        for &(x, ax) in xs {
            let a = (ax * ay * 225.0).round().clamp(0.0, 255.0) as u8;
            mesh.colored_vertex(Pos2::new(x, y), Color32::from_rgba_unmultiplied(12, 9, 7, a));
        }
    }
    let w = xs.len() as u32;
    for r in 0..ys.len() as u32 - 1 {
        for c in 0..w - 1 {
            let i = r * w + c;
            mesh.add_triangle(i, i + 1, i + w);
            mesh.add_triangle(i + 1, i + w + 1, i + w);
        }
    }
    p.add(egui::Shape::mesh(mesh));
}

fn main() {
    // A window program has no console to complain in: anything that goes badly wrong is written
    // beside Play.exe instead, in play-crash.txt.
    let crash_file = std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.join("play-crash.txt")));
    std::panic::set_hook(Box::new(move |info| {
        if let Some(f) = &crash_file {
            let _ = std::fs::write(f, format!("Play stopped: {info}\n"));
        }
    }));
    // A folder Play may not read (macOS keeps an app out of Documents, Desktop and Downloads
    // until it's allowed): said in a dialog, not a splash that does nothing.
    if let Some(paths) = std::env::current_exe().ok().and_then(|e| Paths::find(&e)) {
        if let Err(e) = std::fs::read(paths.root.join("project.toml")) {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                blocked(&paths.root);
                return;
            }
        }
    }
    if let Err(e) = run() {
        if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.to_path_buf())) {
            let _ = std::fs::write(dir.join("play-crash.txt"), format!("Play couldn't open its window: {e}\n"));
        }
        std::process::exit(1);
    }
}

/// The system won't let Play read its own folder: what to do about it, in a dialog.
fn blocked(root: &std::path::Path) {
    let title = Project::built_in().title;
    let what = if cfg!(target_os = "macos") {
        format!(
            "macOS isn't letting {title} open its folder:\n{}\n\nAllow it in System Settings → Privacy & Security → Files & Folders (or add {title}.app to Full Disk Access), or move the folder out of Documents, Desktop and Downloads. Then open {title} again.",
            root.display()
        )
    } else {
        format!("Play isn't allowed to open its folder:\n{}\n\nCheck the folder's permissions, then open Play again.", root.display())
    };
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Warning)
        .set_title(format!("{title} can't open its folder"))
        .set_description(what)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

fn run() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let shot = args.iter().position(|a| a.eq_ignore_ascii_case("--shot")).and_then(|i| args.get(i + 1)).map(PathBuf::from);
    let opts = Options {
        no_update: args.iter().any(|a| a.eq_ignore_ascii_case("--no-update") || a.eq_ignore_ascii_case("-NoUpdate")) || shot.is_some(),
        solo: args.iter().any(|a| a.eq_ignore_ascii_case("--solo")),
        settings: args.iter().any(|a| a.eq_ignore_ascii_case("--settings")),
        shot,
    };
    let icon = eframe::icon_data::from_png_bytes(SHIELD).ok().map(Arc::new);
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([W, H])
        .with_min_inner_size([W, H])
        .with_max_inner_size([W, H])
        .with_decorations(false)
        .with_resizable(false)
        .with_title(Project::built_in().title);
    if let Some(icon) = icon {
        viewport = viewport.with_icon(icon);
    }
    let native = eframe::NativeOptions { viewport, centered: true, ..Default::default() };
    eframe::run_native("Elwynn Survival", native, Box::new(move |cc| Ok(Box::new(App::new(cc, opts)))))
}

struct Options {
    no_update: bool,
    solo: bool,
    settings: bool,
    /// Save a picture of the splash here and stop, instead of playing.
    shot: Option<PathBuf>,
}

/// What the splash is doing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Stage {
    /// Looking for WoW and asking GitHub.
    Opening,
    /// No WoW 1.12.1 found: the player picks the folder.
    NeedWow,
    /// A new version coming in.
    Updating,
    /// Everything's in place: the game starts in a moment, or on a click.
    Ready,
    /// The server asked, the game starting.
    Launching,
    /// The game is open (Play stays, hidden, while it runs its own server).
    Playing,
    /// Something to read, with what to do next.
    Failed,
}

/// What the threads tell the window.
enum Msg {
    Wow(Option<PathBuf>),
    Status(String),
    /// Nothing to update, or why GitHub couldn't be asked (shown small, not a failure).
    UpToDate(Option<String>),
    UpdateFound(net::Release),
    Progress(update::Progress),
    Updated(String),
    UpdateFailed(String),
    ServerStarted(Child),
    GameStarted(Child),
    Failed(String),
    /// A Mac's Play was rebuilt by the update: the new one takes over.
    Relaunch,
}

struct App {
    paths: Paths,
    project: Project,
    settings: Settings,
    version: String,
    channel: Channel,
    opts: Options,
    bg: Option<egui::TextureHandle>,
    bg_size: Vec2,
    stage: Stage,
    status: String,
    /// A second, smaller line.
    note: String,
    /// 0 to 1, or none for "busy".
    progress: Option<f32>,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    started: Instant,
    ready_since: Option<Instant>,
    /// When the pointer last moved over the window.
    pointer_moved: Option<Instant>,
    /// The game started (Ready → Launching) only once.
    launched: bool,
    /// The picture asked for once (`--shot`).
    shot_asked: bool,
    solo: bool,
    server_child: Option<Child>,
    game_child: Option<Child>,
    game_since: Option<Instant>,
    panel: bool,
    /// The cog's fields.
    f_name: String,
    f_place: String,
    f_address: String,
    f_game: String,
    f_hero: String,
    log: Option<std::fs::File>,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, opts: Options) -> Self {
        let exe = std::env::current_exe().unwrap_or_default();
        let paths = Paths::find(&exe).unwrap_or_else(|| Paths::download(PathBuf::from(".")));
        let project = Project::built_in();
        let settings = paths.read_settings();
        let version = paths.installed_version();
        let channel = folder::channel(&version);

        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert("title".into(), Arc::new(egui::FontData::from_static(TITLE_FONT)));
        fonts.font_data.insert("text".into(), Arc::new(egui::FontData::from_static(TEXT_FONT)));
        fonts.families.insert(FontFamily::Name("title".into()), vec!["title".into()]);
        fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "text".into());
        cc.egui_ctx.set_fonts(fonts);
        cc.egui_ctx.style_mut(|s| {
            s.visuals = egui::Visuals::dark();
            s.visuals.override_text_color = Some(ink());
            s.visuals.widgets.inactive.fg_stroke.color = ink();
            s.visuals.widgets.hovered.fg_stroke.color = gold();
            s.visuals.widgets.active.fg_stroke.color = gold();
            s.visuals.selection.bg_fill = Color32::from_rgb(90, 70, 30);
            s.spacing.item_spacing = egui::vec2(8.0, 6.0);
        });

        let (bg, bg_size) = match image::load_from_memory(SPLASH) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                let ci = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], rgba.as_raw());
                (Some(cc.egui_ctx.load_texture("splash", ci, egui::TextureOptions::LINEAR)), Vec2::new(w as f32, h as f32))
            }
            Err(_) => (None, Vec2::new(W, H)),
        };

        let (tx, rx) = std::sync::mpsc::channel();
        let log = std::fs::File::create(&paths.log).ok();
        let f_place = settings.get("place").cloned().filter(|p| ["group", "own", "address"].contains(&p.as_str())).unwrap_or_else(|| if project.server.is_empty() { "own".into() } else { "group".into() });
        let mut app = App {
            f_name: folder::account_name(&settings),
            f_address: settings.get("address").cloned().unwrap_or_default(),
            f_game: settings.get("game").cloned().unwrap_or_else(|| "survival".into()),
            f_hero: settings.get("hero").cloned().unwrap_or_default(),
            f_place,
            solo: opts.solo,
            panel: opts.settings,
            paths, project, settings, version, channel, opts, bg, bg_size,
            stage: Stage::Opening,
            status: String::new(),
            note: String::new(),
            progress: None,
            tx, rx,
            started: Instant::now(),
            ready_since: None,
            pointer_moved: None,
            launched: false,
            shot_asked: false,
            server_child: None,
            game_child: None,
            game_since: None,
            log,
        };
        app.log(&format!("Play {} in {} ({:?})", app.version, app.paths.root.display(), app.paths.kind));
        app.begin();
        app
    }

    fn log(&mut self, text: &str) {
        use std::io::Write;
        if let Some(f) = &mut self.log {
            let _ = writeln!(f, "{:>7.1}s {text}", self.started.elapsed().as_secs_f32());
        }
    }

    fn save(&mut self) {
        folder::save_settings(&self.paths.settings, &self.settings);
    }

    fn password(&self) -> String {
        self.settings.get("password").cloned().filter(|p| !p.is_empty()).unwrap_or_else(|| self.project.password.clone())
    }

    /// The slow look-ups, on a thread: WoW if it isn't known, then GitHub.
    fn begin(&mut self) {
        let wow_known = wow::data_folder_of(self.settings.get("wow").map(String::as_str).unwrap_or("")).is_some();
        self.status = if wow_known { "Checking for a new version…".into() } else { "Looking for your World of Warcraft…".into() };
        let tx = self.tx.clone();
        let paths = self.paths.clone();
        let project = self.project.clone();
        let check = !self.opts.no_update && self.channel != Channel::Fixed && self.paths.kind == Kind::Download;
        let check_checkout = !self.opts.no_update && self.paths.kind == Kind::Checkout;
        thread::spawn(move || {
            if !wow_known {
                let found = wow::find_wow_data(&paths.root);
                let _ = tx.send(Msg::Wow(found));
            }
            if check_checkout {
                // A Mac's folder: main from GitHub, then Play itself rebuilt if that changed it.
                match update::checkout(&paths, |s| { let _ = tx.send(Msg::Status(s)); }) {
                    update::CheckoutUpdate::NewPlay => { let _ = tx.send(Msg::Relaunch); }
                    update::CheckoutUpdate::Note(n) => { let _ = tx.send(Msg::UpToDate(n)); }
                }
            } else if check {
                let _ = tx.send(Msg::Status("Checking for a new version…".into()));
                match update::check(&paths, &project) {
                    Ok(Some(r)) => { let _ = tx.send(Msg::UpdateFound(r)); }
                    Ok(None) => { let _ = tx.send(Msg::UpToDate(None)); }
                    Err(why) => { let _ = tx.send(Msg::UpToDate(Some(why))); }
                }
            } else {
                let _ = tx.send(Msg::UpToDate(None));
            }
        });
    }

    fn become_ready(&mut self) {
        let wow_known = wow::data_folder_of(self.settings.get("wow").map(String::as_str).unwrap_or("")).is_some();
        // (A picture of the splash is taken on build machines with no WoW on them.)
        if !wow_known && self.paths.kind == Kind::Download && self.opts.shot.is_none() {
            self.stage = Stage::NeedWow;
            self.status = "Where is your World of Warcraft 1.12.1?".into();
            self.note = "The folder with WoW.exe in it. Asked once.".into();
            return;
        }
        self.stage = Stage::Ready;
        self.progress = Some(1.0);
        self.status = match self.f_place.as_str() {
            "own" => "Your own world, on this PC".into(),
            "address" => format!("Joining {}", self.f_address.trim()),
            _ => self.project.server_name.clone(),
        };
        if self.note.is_empty() {
            self.note = format!("Playing as {}", self.f_name);
        }
        self.ready_since = Some(Instant::now());
    }

    fn install(&mut self, r: net::Release) {
        if let Some(why) = update::not_now(&self.paths.root) {
            self.log(&format!("update {} waits: {why}", r.version));
            self.note = why.into();
            self.become_ready();
            return;
        }
        self.stage = Stage::Updating;
        self.status = format!("Downloading {}…", r.version);
        self.note = "Your settings and heroes stay as they are.".into();
        self.progress = Some(0.0);
        self.log(&format!("updating {} -> {} ({}, {} MB)", self.version, r.version, r.asset_name, r.asset_size / 1_000_000));
        let tx = self.tx.clone();
        let paths = self.paths.clone();
        thread::spawn(move || {
            let result = update::install(&paths, &r, |p| { let _ = tx.send(Msg::Progress(p)); });
            let _ = match result {
                Ok(v) => tx.send(Msg::Updated(v)),
                Err(why) => tx.send(Msg::UpdateFailed(why)),
            };
        });
    }

    /// The server (this PC's own, started if need be, or the chosen one asked whether it's up),
    /// then the game.
    fn launch(&mut self, solo: bool) {
        if self.launched {
            return;
        }
        self.launched = true;
        self.solo = solo || self.f_place == "own";
        self.stage = Stage::Launching;
        self.progress = None;
        self.note.clear();
        self.settings.insert("name".into(), self.f_name.clone());
        self.settings.insert("game".into(), self.f_game.clone());
        self.settings.insert("hero".into(), self.f_hero.trim().into());
        if !solo {
            self.settings.insert("place".into(), self.f_place.clone());
            self.settings.insert("address".into(), self.f_address.trim().into());
        }
        self.save();
        let Some(data) = wow::data_folder_of(self.settings.get("wow").map(String::as_str).unwrap_or("")) else {
            self.launched = false;
            return self.become_ready();
        };
        let (tx, paths, project) = (self.tx.clone(), self.paths.clone(), self.project.clone());
        let (solo, place, typed) = (self.solo, self.f_place.clone(), self.f_address.trim().to_string());
        let (game_key, user, pass, hero) = (self.f_game.clone(), self.f_name.clone(), self.password(), self.f_hero.trim().to_string());
        let ports = server::own_ports(&self.settings);
        self.status = if solo { "Starting your own world…".into() } else { format!("Asking {} if it's up…", if place == "address" { typed.clone() } else { project.server_name.clone() }) };
        thread::spawn(move || {
            let realm = project.game_name(&game_key).to_string();
            let address = if solo {
                if !server::up(ports, &game_key) {
                    match server::start_quiet(&paths, &server::args(&project, &data, ports)) {
                        Ok(child) => { let _ = tx.send(Msg::ServerStarted(child)); }
                        Err(why) => { let _ = tx.send(Msg::Failed(why)); return; }
                    }
                    let waited = server::wait_up(ports, &game_key, Duration::from_secs(20 * 60), |s| {
                        let _ = tx.send(Msg::Status(format!("Your world is loading ({s} s)…")));
                    });
                    if let Err(why) = waited {
                        let _ = tx.send(Msg::Failed(why));
                        return;
                    }
                }
                format!("127.0.0.1{}", ports.suffix())
            } else {
                let a = if place == "address" { typed } else { project.server.clone() };
                if a.is_empty() {
                    let _ = tx.send(Msg::Failed("Type the address the host gave you, under the cog.".into()));
                    return;
                }
                let (h, p) = net::split_address(&a);
                if !net::reach(&h, p, 4000) {
                    let who = if place == "address" { a.clone() } else { project.server_name.clone() };
                    let _ = tx.send(Msg::Failed(format!("{who} isn't up right now. Try again in a bit, or play alone.")));
                    return;
                }
                a
            };
            let _ = tx.send(Msg::Status("Starting the game…".into()));
            let launch = game::Launch { data: &data, address: &address, realm: &realm, user: &user, pass: &pass, hero: Some(&hero) };
            match game::start(&paths, &launch) {
                Ok(mut child) => {
                    // A Mac builds the client first: Play waits on screen until the game is up.
                    match game::wait_until_open(&paths, &mut child, |s| { let _ = tx.send(Msg::Status(s)); }) {
                        Ok(()) => { let _ = tx.send(Msg::GameStarted(child)); }
                        Err(why) => { let _ = tx.send(Msg::Failed(why)); }
                    }
                }
                Err(why) => { let _ = tx.send(Msg::Failed(why)); }
            }
        });
    }

    fn take_messages(&mut self, ctx: &egui::Context) {
        while let Ok(m) = self.rx.try_recv() {
            match m {
                Msg::Wow(found) => {
                    if let Some(p) = found {
                        self.log(&format!("found WoW: {}", p.display()));
                        self.settings.insert("wow".into(), p.to_string_lossy().to_string());
                        self.save();
                    }
                }
                Msg::Status(s) => self.status = s,
                Msg::UpToDate(why) => {
                    if let Some(w) = why {
                        self.log(&format!("update check: {w}"));
                        self.note = w;
                    }
                    if self.stage == Stage::Opening {
                        self.become_ready();
                    }
                }
                Msg::UpdateFound(r) => self.install(r),
                Msg::Progress(p) => match p {
                    update::Progress::Downloading { done, total } => {
                        self.progress = Some(if total > 0 { (done as f32 / total as f32).min(1.0) } else { 0.0 });
                        self.status = format!("Downloading the new version ({} of {} MB)…", done / 1_000_000, total.max(done) / 1_000_000);
                    }
                    update::Progress::Installing => {
                        self.progress = None;
                        self.status = "Installing…".into();
                    }
                },
                Msg::Updated(v) => {
                    self.log(&format!("updated to {v}; starting the new Play"));
                    self.status = format!("Updated to {v}. Starting it…");
                    let mut c = std::process::Command::new(self.paths.root.join(if cfg!(windows) { "Play.exe" } else { "Play" }));
                    c.arg("--no-update").current_dir(&self.paths.root);
                    if self.solo { c.arg("--solo"); }
                    if c.spawn().is_ok() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    } else {
                        self.note = "Open Play again to use it.".into();
                        self.stage = Stage::Failed;
                    }
                }
                Msg::UpdateFailed(why) => {
                    self.log(&format!("update failed: {why}"));
                    self.note = why;
                    self.become_ready();
                }
                Msg::Relaunch => {
                    self.log("Play was rebuilt by the update; starting the new one");
                    if update::start_new_play().is_ok() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    } else {
                        self.note = "Play was updated. Open it again to use the new one.".into();
                        self.become_ready();
                    }
                }
                Msg::ServerStarted(child) => self.server_child = Some(child),
                Msg::GameStarted(mut game) => {
                    self.log("the game is open");
                    self.game_since = Some(Instant::now());
                    self.stage = Stage::Playing;
                    self.status = "Have fun.".into();
                    self.progress = Some(1.0);
                    if let Some(mut server) = self.server_child.take() {
                        // Running the server for the game: a thread waits for the game to close,
                        // then stops the server and Play with it. (Not the window's own loop: a
                        // hidden window stops being drawn, so it would never notice.)
                        let log = self.paths.log.clone();
                        thread::spawn(move || {
                            let _ = game.wait();
                            let _ = server.kill();
                            let _ = server.wait();
                            if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(&log) {
                                use std::io::Write;
                                let _ = writeln!(f, "        the game closed; your server is stopped");
                            }
                            std::process::exit(0);
                        });
                        self.solo = true;
                    } else {
                        self.game_child = Some(game);
                    }
                }
                Msg::Failed(why) => {
                    self.log(&format!("failed: {why}"));
                    self.launched = false;
                    self.stage = Stage::Failed;
                    self.status = why;
                    self.note.clear();
                    self.progress = Some(0.0);
                    if let Some(mut s) = self.server_child.take() {
                        let _ = s.kill();
                    }
                }
            }
        }
    }

    fn make_test_copy(&mut self, ctx: &egui::Context) {
        match update::make_test_folder(&self.paths) {
            Ok(play) => {
                self.log(&format!("made the test folder: {}", play.display()));
                if std::process::Command::new(&play).current_dir(play.parent().unwrap_or(&self.paths.root)).spawn().is_ok() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
            Err(why) => {
                self.note = why;
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.take_messages(ctx);
        ctx.request_repaint_after(Duration::from_millis(if self.stage == Stage::Playing { 500 } else { 50 }));

        // Once the game is open: gone, or, running the server for it, hidden until the game
        // closes (the thread in `take_messages` ends Play then).
        if self.stage == Stage::Playing {
            let since = self.game_since.map(|t| t.elapsed()).unwrap_or_default();
            if since > Duration::from_millis(1200) {
                if self.solo {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                } else {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }

        let (hovered, moved) = ctx.input(|i| (i.pointer.hover_pos().is_some(), i.pointer.delta() != egui::Vec2::ZERO || i.pointer.any_down()));
        if hovered && moved {
            self.pointer_moved = Some(Instant::now());
        }
        // A hand on its way to the buttons holds the game back; a mouse merely resting where the
        // window happened to open (the middle of the screen, often) doesn't.
        let reaching = hovered && self.pointer_moved.map(|t| t.elapsed() < Duration::from_millis(1500)).unwrap_or(false);
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && self.stage != Stage::Playing {
            if self.panel { self.panel = false; } else { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
        }
        // A picture of the splash asked for: taken once it's ready and drawn, then done.
        if let Some(path) = self.opts.shot.clone() {
            let taken = ctx.input(|i| {
                i.events.iter().find_map(|e| match e {
                    egui::Event::Screenshot { image, .. } => Some(image.clone()),
                    _ => None,
                })
            });
            if let Some(img) = taken {
                let [w, h] = img.size;
                let rgba: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
                match image::save_buffer(&path, &rgba, w as u32, h as u32, image::ColorType::Rgba8) {
                    Ok(()) => self.log(&format!("saved a picture of the splash to {}", path.display())),
                    Err(e) => self.log(&format!("couldn't save the picture: {e}")),
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else if self.stage == Stage::Ready && !self.shot_asked && self.ready_since.map(|t| t.elapsed() > Duration::from_millis(600)).unwrap_or(false) {
                self.shot_asked = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            }
        }
        // Ready, and nobody reaching for the cog: the game starts by itself.
        if self.stage == Stage::Ready && !self.panel && self.opts.shot.is_none() {
            if reaching {
                self.ready_since = Some(Instant::now());
            } else if self.opts.solo || self.ready_since.map(|t| t.elapsed() > PAUSE).unwrap_or(false) {
                let solo = self.opts.solo;
                self.launch(solo);
            }
        }

        let screen = ctx.content_rect();
        egui::CentralPanel::default().frame(egui::Frame::new()).show(ctx, |ui| {
            let p = ui.painter();
            // The painting, edge to edge.
            if let Some(bg) = &self.bg {
                let scale = (screen.width() / self.bg_size.x).max(screen.height() / self.bg_size.y);
                let size = self.bg_size * scale;
                let rect = Rect::from_center_size(screen.center(), size);
                p.image(bg.id(), rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
            } else {
                p.rect_filled(screen, 0.0, Color32::from_rgb(24, 21, 17));
            }
            // Night rising from the bottom, for the words: smooth (one mesh, no steps), and dark by
            // the time it reaches the name.
            let band = 240.0;
            shade(p, &[(screen.min.x, 1.0), (screen.max.x, 1.0)], &[(screen.max.y - band, 0.0), (screen.max.y - band * 0.55, 0.42), (screen.max.y, 1.0)]);
            // The name, the line, the note.
            let left = screen.min.x + 40.0;
            let title_font = FontId::new(50.0, FontFamily::Name("title".into()));
            let status_font = FontId::new(19.0, FontFamily::Proportional);
            let note_font = FontId::new(14.5, FontFamily::Proportional);
            // A deeper shadow behind the words themselves, fading out to the right and upwards, so
            // they read over the brightest part of the painting without sitting in a box.
            let widest = [(&self.project.title, &title_font), (&self.status, &status_font), (&self.note, &note_font)]
                .iter()
                .map(|(s, f)| p.layout_no_wrap(s.to_string(), (*f).clone(), ink()).size().x)
                .fold(0.0f32, f32::max);
            let right = left + widest;
            shade(
                p,
                &[(screen.min.x, 0.85), (right - 60.0, 0.85), (right + 160.0, 0.0)],
                &[(screen.max.y - 200.0, 0.0), (screen.max.y - 140.0, 1.0), (screen.max.y, 1.0)],
            );
            p.text(Pos2::new(left, screen.max.y - 78.0), Align2::LEFT_BOTTOM, &self.project.title, title_font, ink());
            let status_color = if self.stage == Stage::Failed { ember() } else { ink() };
            p.text(Pos2::new(left + 2.0, screen.max.y - 46.0), Align2::LEFT_BOTTOM, &self.status, status_font, status_color);
            if !self.note.is_empty() {
                p.text(Pos2::new(left + 2.0, screen.max.y - 24.0), Align2::LEFT_BOTTOM, &self.note, note_font, dim());
            }
            // The version, small, in the corner, on a little shade of its own.
            let v = match self.channel {
                Channel::Latest => format!("TEST BUILD  {}", self.version),
                _ => self.version.clone(),
            };
            let version_font = FontId::new(12.5, FontFamily::Proportional);
            let vw = p.layout_no_wrap(v.clone(), version_font.clone(), ink()).size().x;
            shade(p, &[(screen.max.x - vw - 120.0, 0.0), (screen.max.x - vw - 30.0, 0.6), (screen.max.x, 0.6)], &[(screen.min.y, 1.0), (screen.min.y + 34.0, 1.0), (screen.min.y + 70.0, 0.0)]);
            p.text(Pos2::new(screen.max.x - 16.0, screen.min.y + 12.0), Align2::RIGHT_TOP, v, version_font, Color32::from_rgba_unmultiplied(240, 228, 205, 190));
            // The progress line along the bottom edge.
            let track = Rect::from_min_max(Pos2::new(screen.min.x, screen.max.y - 4.0), screen.max);
            p.rect_filled(track, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 120));
            match self.progress {
                Some(f) => {
                    let w = screen.width() * f.clamp(0.0, 1.0);
                    p.rect_filled(Rect::from_min_max(track.min, Pos2::new(screen.min.x + w, screen.max.y)), 0.0, gold());
                }
                None => {
                    // Busy: a short run of gold sliding along.
                    let t = self.started.elapsed().as_secs_f32() * 0.6 % 1.0;
                    let seg = screen.width() * 0.22;
                    let x = screen.min.x - seg + (screen.width() + seg) * t;
                    p.rect_filled(Rect::from_min_max(Pos2::new(x.max(screen.min.x), track.min.y), Pos2::new((x + seg).min(screen.max.x), screen.max.y)), 0.0, gold());
                }
            }
        });

        // The buttons, bottom right: on a hand's approach, or whenever there's something to decide.
        let show = hovered || matches!(self.stage, Stage::Failed | Stage::NeedWow) || self.panel;
        if show && self.stage != Stage::Playing {
            egui::Area::new(egui::Id::new("buttons")).anchor(Align2::RIGHT_BOTTOM, [-28.0, -22.0]).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let font = FontId::new(17.0, FontFamily::Proportional);
                    let button = |label: &str| egui::Button::new(egui::RichText::new(label).font(font.clone())).frame(false);
                    match self.stage {
                        Stage::NeedWow => {
                            if ui.add(button("Choose the WoW folder")).clicked() {
                                if let Some(data) = wow::pick_wow_data(&self.project.title) {
                                    self.settings.insert("wow".into(), data.to_string_lossy().to_string());
                                    self.save();
                                    self.note.clear();
                                    self.become_ready();
                                }
                            }
                        }
                        Stage::Failed => {
                            if ui.add(button("Try again")).clicked() {
                                self.become_ready();
                                self.ready_since = Some(Instant::now());
                            }
                            if ui.add(button("Play alone")).clicked() {
                                self.launch(true);
                            }
                        }
                        Stage::Ready => {
                            if ui.add(button("Play alone")).clicked() {
                                self.launch(true);
                            }
                            if ui.add(egui::Button::new(egui::RichText::new("Play").font(FontId::new(19.0, FontFamily::Proportional)).color(gold())).frame(false)).clicked() {
                                self.launch(false);
                            }
                        }
                        _ => {}
                    }
                    if self.stage != Stage::Launching && self.stage != Stage::Updating {
                        if ui.add(button("⚙")).on_hover_text("Choices").clicked() {
                            self.panel = !self.panel;
                        }
                    }
                });
            });
        }

        if self.panel {
            self.panel_window(ctx);
        }

        // Drag the window by its picture.
        if ctx.input(|i| i.pointer.primary_pressed()) && !ctx.is_pointer_over_area() {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
    }
}

impl App {
    /// The cog: the few choices there are.
    fn panel_window(&mut self, ctx: &egui::Context) {
        let frame = egui::Frame::popup(&ctx.style()).fill(Color32::from_rgba_unmultiplied(22, 18, 14, 240)).inner_margin(16.0);
        let mut close = false;
        let mut test = false;
        egui::Window::new("choices")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .frame(frame)
            .anchor(Align2::RIGHT_BOTTOM, [-28.0, -60.0])
            .fixed_size([340.0, 0.0])
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Your name").color(dim()).size(13.0));
                ui.text_edit_singleline(&mut self.f_name);
                ui.add_space(6.0);
                ui.label(egui::RichText::new("Where you play").color(dim()).size(13.0));
                if !self.project.server.is_empty() {
                    ui.radio_value(&mut self.f_place, "group".into(), &self.project.server_name);
                }
                ui.radio_value(&mut self.f_place, "own".into(), "On this PC, alone or hosting");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.f_place, "address".into(), "Someone else's:");
                    ui.add(egui::TextEdit::singleline(&mut self.f_address).hint_text("address the host gave you").desired_width(150.0));
                });
                ui.add_space(6.0);
                ui.label(egui::RichText::new("Game").color(dim()).size(13.0));
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.f_game, "survival".into(), &self.project.survival);
                    ui.radio_value(&mut self.f_game, "basin".into(), &self.project.basin);
                });
                ui.add_space(6.0);
                ui.label(egui::RichText::new("Straight into the world as").color(dim()).size(13.0));
                ui.add(egui::TextEdit::singleline(&mut self.f_hero).hint_text("a hero's name, or empty for the character screen"));
                ui.add_space(6.0);
                let wow_now = self.settings.get("wow").cloned().unwrap_or_default();
                ui.label(egui::RichText::new(format!("WoW 1.12.1: {}", if wow_now.is_empty() { "not found yet".to_string() } else { wow_now })).color(dim()).size(12.0));
                ui.horizontal(|ui| {
                    if ui.small_button("Change the WoW folder").clicked() {
                        if let Some(data) = wow::pick_wow_data(&self.project.title) {
                            self.settings.insert("wow".into(), data.to_string_lossy().to_string());
                            self.save();
                        }
                    }
                    if ui.small_button("Open this folder").clicked() {
                        proc::open_in_shell(&self.paths.root.to_string_lossy());
                    }
                });
                if self.paths.kind == Kind::Download && self.channel == Channel::Release {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.small_button("Make a test copy").on_hover_text("A second folder beside this one that plays the newest main, for playtests. Its own heroes and saves.").clicked() {
                            test = true;
                        }
                        ui.label(egui::RichText::new("plays the newest main, beside this folder").color(dim()).size(12.0));
                    });
                }
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Done").clicked() {
                        close = true;
                    }
                    ui.label(egui::RichText::new(format!("Password: {}", self.password())).color(dim()).size(12.0));
                });
            });
        if close {
            self.panel = false;
            self.ready_since = Some(Instant::now());
            if self.stage == Stage::Ready {
                self.become_ready();
            }
        }
        if test {
            self.make_test_copy(ctx);
        }
    }
}
