//! Runs an add-on (Basin Battle's or Elwynn Survival's) in benilla's UI script host with stand-ins
//! for the stock interface, feeds it server lines and frames, and prints every Lua error.
//!
//! `addon-harness <addon.lua> <lines> <script> [prefix]`: the prefix of the server's lines is
//! `BB` unless given (Elwynn Survival's is `ES`).
use benilla_ui::script::{QuadContent, ScriptValue, TexCoords, UiScript};

const STUBS: &str = include_str!("stubs.lua");

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let addon = std::fs::read(&args[1]).unwrap();
    let lines = std::fs::read_to_string(&args[2]).unwrap();
    let extra = std::fs::read_to_string(&args[3]).unwrap_or_default();
    let prefix = args.get(4).cloned().unwrap_or_else(|| "BB".to_string());
    let path = std::path::Path::new(&args[1]);
    let name = path.file_stem().and_then(|n| n.to_str()).unwrap_or("Addon").to_string();
    let mut s = UiScript::new().unwrap();
    // HARNESS_SCREEN="w,h": the UI's own size (the client's is 768 high, whatever the window).
    let (sw, sh) = std::env::var("HARNESS_SCREEN")
        .ok()
        .and_then(|v| {
            let mut it = v.split(',').map(|n| n.trim().parse::<f32>());
            Some((it.next()?.ok()?, it.next()?.ok()?))
        })
        .unwrap_or((1920.0, 1080.0));
    s.set_screen_size(sw, sh);
    s.run(STUBS).expect("stubs");
    if let Err(e) = s.run_chunk_named(&addon, &format!("@{name}.lua")) {
        println!("LOAD ERROR: {e}");
    }
    // The rest of the add-on's files, in its .toc's order after this one, as the client loads them.
    if let Ok(toc) = std::fs::read_to_string(path.with_extension("toc")) {
        let first = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        for file in toc.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#') && l.ends_with(".lua") && *l != first) {
            match std::fs::read(path.with_file_name(file)) {
                Ok(code) => {
                    if let Err(e) = s.run_chunk_named(&code, &format!("@{file}")) {
                        println!("LOAD ERROR ({file}): {e}");
                    }
                }
                Err(e) => println!("MISSING ({file}): {e}"),
            }
        }
    }
    // The add-on's Bindings.xml, beside it, as the client registers it after the files.
    let bindings_path = std::path::Path::new(&args[1]).with_file_name("Bindings.xml");
    if let Ok(text) = std::fs::read_to_string(&bindings_path) {
        match benilla_ui::bindings_xml::parse(&text) {
            Ok(b) => s.register_addon_bindings(&name, &b),
            Err(e) => println!("BINDINGS ERROR: {e:?}"),
        }
    }
    let mut errors = Vec::new();
    let tick = |s: &mut UiScript, n: usize, errors: &mut Vec<String>| {
        for _ in 0..n {
            s.tick(0.05);
            for e in s.take_errors() {
                errors.push(e);
            }
        }
    };
    s.fire_event("VARIABLES_LOADED", vec![]);
    s.fire_event("PLAYER_ENTERING_WORLD", vec![]);
    tick(&mut s, 4, &mut errors);
    let addon_line = |s: &mut UiScript, l: &str| {
        s.fire_event(
            "CHAT_MSG_ADDON",
            vec![ScriptValue::Str(prefix.clone().into()), ScriptValue::Str(l.into()), ScriptValue::Str("WHISPER".into()), ScriptValue::Str("Server".into())],
        );
    };
    for l in lines.lines().chain(extra.lines()) {
        if l.starts_with("#") || l.is_empty() {
            continue;
        }
        if let Some(ev) = l.strip_prefix("@") {
            let mut parts = ev.split(' ');
            let name = parts.next().unwrap().to_string();
            let args = parts.map(|a| match a.parse::<f64>() { Ok(n) => ScriptValue::Number(n), Err(_) => ScriptValue::Str(a.into()) }).collect();
            s.fire_event(&name, args);
        } else if let Some(path) = l.strip_prefix("%SNAP ") {
            // The render list as it stands, for tools/addon-harness/snap.py to draw.
            tick(&mut s, 2, &mut errors);
            snap(&s, path.trim());
        } else if let Some(code) = l.strip_prefix("!") {
            if let Err(e) = s.run(code) {
                errors.push(format!("[{code}] {e}"));
            }
        } else {
            addon_line(&mut s, l);
        }
        tick(&mut s, 2, &mut errors);
        for e in s.take_errors() {
            errors.push(format!("after {:.40}: {e}", l));
        }
    }
    tick(&mut s, 200, &mut errors);
    errors.extend(s.take_errors());
    let mut seen = std::collections::BTreeMap::new();
    for e in errors {
        *seen.entry(e).or_insert(0) += 1;
    }
    for (e, n) in &seen {
        println!("{n}x {e}");
    }
    println!("{} distinct errors", seen.len());
    if let Ok(said) = s.eval::<String>("return table.concat(SAID, '\\n')") {
        println!("--- said ---\n{said}");
    }
}

fn esc(t: &str) -> String {
    let mut o = String::new();
    for c in t.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            c if (c as u32) < 0x20 => o.push(' '),
            c => o.push(c),
        }
    }
    o
}

fn col(c: Option<[f32; 4]>) -> String {
    c.map_or("null".to_string(), |c| format!("[{},{},{},{}]", c[0], c[1], c[2], c[3]))
}

/// Every quad the screen draws now (textures and text, in painter order) as JSON lines.
fn snap(s: &UiScript, path: &str) {
    let mut out = String::new();
    if let Ok(size) = s.eval::<String>("return UIParent:GetWidth() .. ' ' .. UIParent:GetHeight()") {
        let mut it = size.split(' ');
        if let (Some(w), Some(h)) = (it.next(), it.next()) {
            out.push_str(&format!("{{\"k\":\"screen\",\"r\":[0,0,{w},{h}]}}\n"));
        }
    }
    for q in s.extract() {
        let Some(r) = q.rect else { continue };
        if q.alpha <= 0.001 {
            continue;
        }
        let rect = format!("[{},{},{},{}]", r.left, r.bottom, r.right, r.top);
        match &q.content {
            QuadContent::Texture { path, color, tex_coords, desaturated, .. } => {
                let tc = match tex_coords {
                    Some(TexCoords::Rect(t)) => format!("[{},{},{},{}]", t[0], t[1], t[2], t[3]),
                    Some(TexCoords::Corners(c)) => format!("[{},{},{},{}]", c[0][0], c[1][0], c[0][1], c[2][1]),
                    None => "null".into(),
                };
                out.push_str(&format!(
                    "{{\"k\":\"tex\",\"r\":{rect},\"a\":{},\"path\":{},\"color\":{},\"tc\":{tc},\"desat\":{}}}\n",
                    q.alpha,
                    path.as_ref().map_or("null".to_string(), |p| format!("\"{}\"", esc(p))),
                    col(*color),
                    desaturated
                ));
            }
            QuadContent::Text { text: Some(t), color, justify_h, font, font_height, outline, .. } if !t.is_empty() => {
                out.push_str(&format!(
                    "{{\"k\":\"text\",\"r\":{rect},\"a\":{},\"text\":\"{}\",\"color\":{},\"j\":\"{:?}\",\"font\":\"{}\",\"h\":{},\"outline\":\"{:?}\",\"scale\":{}}}\n",
                    q.alpha,
                    esc(t),
                    col(*color),
                    justify_h,
                    esc(font.as_deref().unwrap_or("")),
                    font_height.unwrap_or(12.0),
                    outline,
                    q.scale
                ));
            }
            _ => {}
        }
    }
    let _ = std::fs::write(path, out);
}
