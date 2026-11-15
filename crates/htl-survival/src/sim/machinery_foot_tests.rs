//! Sneed on foot, measured with a scripted warrior (docs/shredder-encounter.md, "Sneed on foot,
//! by measure"): can a hero who plays it right close to him, block his flurry and hit the opening?
use super::tests::{broken, emerged, fight, live, DT};
use super::*;

fn stride(s: &mut Survival, h: Id, to: Vec3, look: Vec3) {
    let me = s.players[&h].pos;
    let d = dist2d(me, to);
    let go = (s.content.player.run_speed * DT).min(d);
    let at = if d > 1e-3 { [me[0] + (to[0] - me[0]) / d * go, me[1] + (to[1] - me[1]) / d * go, me[2]] } else { me };
    s.move_player(h, at, facing(at, look), Moving::default());
}

const OPENING: f32 = 2.4;

/// What the warrior did in one run on Sneed.
#[derive(Debug, Default)]
struct Run {
    /// Seconds from his getting up (the warrior may act) to each blow that landed.
    hits: Vec<f32>,
    /// Seconds to the first time the warrior was within his 3.4 yd.
    closed: Option<f32>,
    /// Hits on the warrior, by wrench and by blow.
    wrenched: u32,
    struck: u32,
    /// Blows that landed in his open 1.4 sec.
    in_open: u32,
    flurries: u32,
    throws: u32,
    /// Each opening: the warrior's stamina as it began, and when their first blow landed in it.
    opens: Vec<(f32, Option<f32>)>,
    /// Seconds he lived, or the limit.
    dead_at: Option<f32>,
}

/// The warrior's rules. `guards`: puts the guard up while he flurries (reacting `react` sec after
/// the first blow's wind-up begins) and lets it down for the opening. `open_only`: swings only in
/// his opening (otherwise whenever in reach).
fn warrior(seed: u64, guards: bool, open_only: bool, react: f32, secs: f32, start: f32) -> Run {
    let (mut s, band, machine) = fight(1, seed);
    let h = band[0];
    let d = broken(&mut s, machine, h);
    emerged(&mut s, d);
    // His one repair is its own beat: this is his footwork.
    s.machinery.drivers.get_mut(&d).unwrap().stage = RepairStage::Spent;
    // Where the warrior stands as he gets up: `start` yd out, the far side from the wreck.
    let (him, wreck) = (s.mobs[&d].pos, s.mobs.values().find(|m| s.content.mobs[m.def].key == MACHINE).map_or(s.mobs[&d].home, |m| m.pos));
    let away = facing(wreck, him);
    s.move_player(h, [him[0] + away.cos() * start, him[1] + away.sin() * start, him[2]], facing([0.0; 3], [0.0; 3]), Moving::default());
    let t0 = s.clock;
    let mut run = Run::default();
    let mut last = String::new();
    let mut hp = s.mobs[&d].health;
    let mut mine = s.players[&h].health;
    let mut open_until = 0.0f32;
    let mut guard_up = false;
    let mut flurry_since: Option<f32> = None;
    while s.clock - t0 < secs {
        let Some(m) = s.mobs.get(&d).filter(|m| m.health > 0) else { run.dead_at = Some(s.clock - t0); break };
        let (me, him) = (s.players[&h].pos, m.pos);
        let dist = dist2d(me, him);
        let reach = s.weapon_of(&s.players[&h]).range + s.content.mobs[m.def].body();
        let flurrying = s.pattern_name(d) == Some("flurry") && open_until <= s.clock;
        if flurrying { flurry_since.get_or_insert(s.clock); } else { flurry_since = None; }
        let want_guard = guards && flurry_since.is_some_and(|t| s.clock - t >= react);
        if want_guard != guard_up {
            s.input(h, if want_guard { Input::Guard } else { Input::GuardUp });
            guard_up = want_guard;
        }
        let open = open_until > s.clock;
        if dist > reach - 0.3 {
            if !guard_up { stride(&mut s, h, him, him); }
        } else {
            s.move_player(h, me, facing(me, him), Moving::default());
            if !guard_up && (open || !open_only) { s.input(h, Input::Attack); }
        }
        if run.closed.is_none() && dist <= CORNERED { run.closed = Some(s.clock - t0); }
        if let Some(n) = s.pattern_name(d) { last = n.to_string(); }
        let ev = s.step(DT);
        for e in &ev {
            if let Event::MobWindup { id, .. } = e {
                if *id == d { match s.pattern_name(d) { Some("flurry") if s.mobs[&d].pattern.is_some_and(|(_, st)| st == 0) => run.flurries += 1, Some("wrench") => run.throws += 1, _ => {} } }
            }
            if let Event::HoldPose { id, clip: OPEN_CLIP, secs, .. } = e {
                if *id == d { open_until = s.clock + secs; run.opens.push((s.players[&h].stamina, None)); }
            }
        }
        if let Some(m) = s.mobs.get(&d) {
            if m.health < hp {
                run.hits.push(s.clock - t0);
                if open_until > s.clock {
                    run.in_open += 1;
                    if let Some(o) = run.opens.last_mut() { o.1.get_or_insert(OPENING - (open_until - s.clock)); }
                }
                hp = m.health;
            }
        }
        if s.players[&h].health < mine {
            if last == "wrench" { run.wrenched += 1; } else { run.struck += 1; }
            mine = s.players[&h].health;
        }
    }
    if live(&s, DRIVER).is_none() && run.dead_at.is_none() { run.dead_at = Some(s.clock - t0); }
    run
}

/// A warrior who strikes whenever in reach lands a first blow soon after he is up, whether they
/// start 4, 8 or 14 yd off, with or without the guard for his flurry (`-- --nocapture` prints it):
/// he backs off slower than they close and his wrench (a 1 sec wind-up) stops nobody.
#[test]
fn a_warrior_who_strikes_reaches_sneed_on_foot_from_4_8_and_14_yd() {
    for start in [4.0f32, 8.0, 14.0] {
        for guards in [false, true] {
            for seed in [2u64, 7, 19] {
                let r = warrior(seed, guards, false, 0.3, 40.0, start);
                println!("from {start} yd, guard {guards}, seed {seed}: first hit {:?}, {} hits, {} in his opening, closed {:?}, wrench hits {}, blows taken {}, dead {:?}, {} flurries, {} throws", r.hits.first(), r.hits.len(), r.in_open, r.closed, r.wrenched, r.struck, r.dead_at, r.flurries, r.throws);
                let first = r.hits.first().copied().expect("a hit");
                // 14 yd at a hero's 7 yd a sec is 2 sec of run, less what he backs off.
                assert!(first < 2.5, "from {start} yd: first hit {first:.2} sec in");
                assert!(r.dead_at.is_some(), "from {start} yd, seed {seed}: he falls inside 40 sec");
            }
        }
    }
}

/// His opening is 2.4 sec: the real floor's warrior who blocked the whole flurry (the last blow
/// tears the guard, staggers them 1.5 sec) first strikes 1.9 sec in (`shredder_rig.rs`).
#[test]
fn his_opening_outlasts_the_stagger_of_a_guard_his_last_blow_tore() {
    let (mut s, band, machine) = fight(1, 29);
    let d = broken(&mut s, machine, band[0]);
    emerged(&mut s, d);
    s.machinery.drivers.get_mut(&d).unwrap().stage = RepairStage::Spent;
    let at = s.mobs[&d].pos;
    s.move_player(band[0], [at[0] + 1.6, at[1], at[2]], std::f32::consts::PI, Moving::default());
    let mut open = None;
    for _ in 0..300 {
        let p = s.mobs[&d].pos;
        s.move_player(band[0], [p[0] + 1.6, p[1], p[2]], std::f32::consts::PI, Moving::default());
        for e in s.step(DT) {
            if let Event::HoldPose { id, clip: OPEN_CLIP, secs, .. } = e { if id == d { open = Some(secs); } }
        }
        if open.is_some() { break; }
    }
    assert_eq!(open, Some(OPENING));
}
