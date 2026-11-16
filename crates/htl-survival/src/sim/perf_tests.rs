//! Tick-cost benchmarks (run by hand, `#[ignore]`d): the cost of `Survival::step` in heavy scenes
//! against the server's 50 ms tick. Release build:
//! `cargo test --release -p htl-survival perf_ -- --ignored --nocapture --test-threads 1`

use super::*;
use crate::land::FlatLand;
use super::mobs::Foe;
use std::time::Instant;

const DT: f32 = 0.05;
const BUDGET_MS: f64 = 50.0;

fn report(name: &str, mut ms: Vec<f64>, mobs: usize) {
    ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let q = |p: f64| ms[((ms.len() - 1) as f64 * p) as usize];
    eprintln!(
        "PERF {name}: {mobs} creatures, {} ticks: median {:.3} ms, p99 {:.3} ms, max {:.3} ms ({:.2}% of the {BUDGET_MS} ms tick at the median, {:.2}% worst)",
        ms.len(),
        q(0.5),
        q(0.99),
        ms[ms.len() - 1],
        q(0.5) / BUDGET_MS * 100.0,
        ms[ms.len() - 1] / BUDGET_MS * 100.0
    );
}

fn heroes(s: &mut Survival, n: usize) -> Vec<Id> {
    (0..n)
        .map(|k| {
            let h = s.join(&format!("h{k}"), &format!("H{k}"), None);
            s.move_player(h, [3.0 + k as f32, k as f32 * 2.0, 0.0], std::f32::consts::PI, Moving::default());
            let p = s.players.get_mut(&h).unwrap();
            p.health = 1_000_000;
            p.max_health = 1_000_000;
            h
        })
        .collect()
}

fn keep_up(s: &mut Survival, band: &[Id], tick: usize) {
    for h in band {
        if let Some(p) = s.players.get_mut(h) {
            p.health = p.max_health;
            p.stamina = 100.0;
        }
        if tick % 8 == 0 {
            s.input(*h, Input::Attack);
        }
    }
}

fn timed(name: &str, s: &mut Survival, band: &[Id], ticks: usize) {
    let mut ms = Vec::with_capacity(ticks);
    for tick in 0..ticks {
        keep_up(s, band, tick);
        let t = Instant::now();
        let ev = s.step(DT);
        ms.push(t.elapsed().as_secs_f64() * 1000.0);
        std::hint::black_box(ev);
    }
    let live = s.mobs.values().filter(|m| m.health > 0).count();
    report(name, ms, live);
}

fn crowd(s: &mut Survival, band: &[Id], keys: &[&str], n: usize, raider: bool) {
    for i in 0..n {
        let key = keys[i % keys.len()];
        let Some(def) = s.content.mob(key) else { continue };
        let a = i as f32 * 0.7;
        let r = 8.0 + (i % 5) as f32 * 2.0;
        let id = s.spawn_mob(def, [a.cos() * r, a.sin() * r, 0.0], None, raider);
        if raider {
            s.raid.raiders.push(id);
        }
        s.engage(id, Foe::Hero(band[i % band.len()]));
    }
}

fn big_wave(n: usize) -> (Survival, Vec<Id>) {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 1, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    let band = heroes(&mut s, 5);
    let _ = s.step(0.0);
    crowd(&mut s, &band, &["blackrock_grunt", "blackrock_caster", "prowler", "kobold_tunneler", "grunt", "archer", "shaman", "wolf"], n, true);
    (s, band)
}

#[test]
#[ignore]
fn perf_elwynn_wave_5_heroes_60_raiders() {
    let (mut s, band) = big_wave(60);
    timed("elwynn wave, 5 heroes", &mut s, &band, 1200);
}

#[test]
#[ignore]
fn perf_elwynn_wave_5_heroes_150_raiders() {
    let (mut s, band) = big_wave(150);
    timed("elwynn wave x150, 5 heroes", &mut s, &band, 1200);
}

#[test]
#[ignore]
fn perf_elwynn_idle_world_with_roaming_creatures() {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 1, 1_000_000.0);
    let band = heroes(&mut s, 5);
    timed("elwynn open world, 5 heroes", &mut s, &band, 1200);
}

#[test]
#[ignore]
fn perf_dungeon_boss_with_adds() {
    for key in ["dm_vancleef", "dm_shredder", "dm_sneed"] {
        let mut c = Content::dev();
        c.pvp = Default::default();
        let mut s = Survival::bare(c, Box::new(FlatLand::default()), 3, 0.0);
        let band = heroes(&mut s, 5);
        let boss = s.spawn_mob(s.content.mob(key).unwrap(), [0.0; 3], None, false);
        s.engage(boss, Foe::Hero(band[0]));
        crowd(&mut s, &band, &["blackrock_grunt", "prowler", "kobold_tunneler", "grunt"], 20, false);
        timed(&format!("boss {key} + 20 adds, 5 heroes"), &mut s, &band, 1200);
    }
}

#[test]
#[ignore]
fn perf_basin_battle_siege_biggest_waves() {
    let mut c = Content::dev();
    c.dev.full_tree = false;
    let mut s = Survival::new_siege(c, Box::new(FlatLand::default()), 1, 1_000_000.0);
    s.events.clear();
    let mut band = Vec::new();
    for k in 0..4 {
        let h = s.join(&format!("h{k}"), &format!("H{k}"), None);
        s.siege_arrive(h);
        s.siege_join(h, "1v1v1v1").unwrap();
        band.push(h);
    }
    if !matches!(s.siege.as_ref().unwrap().phase, siege::SiegePhase::Countdown { .. }) {
        s.siege_start_now(band[0]).unwrap();
    }
    for _ in 0..1000 {
        let _ = s.step(DT);
        if matches!(s.siege.as_ref().unwrap().phase, siege::SiegePhase::Running) {
            break;
        }
    }
    let mut ms = Vec::new();
    let mut peak = 0;
    for tick in 0..2400 {
        for h in &band {
            if let Some(p) = s.players.get_mut(h) {
                p.health = p.max_health;
            }
        }
        // A wave every 4 s, so they pile up as late waves would.
        if tick % 80 == 0 {
            if let Some(g) = s.siege.as_mut() {
                g.timer = 0.0;
            }
        }
        let t = Instant::now();
        let ev = s.step(DT);
        ms.push(t.elapsed().as_secs_f64() * 1000.0);
        std::hint::black_box(ev);
        peak = peak.max(s.mobs.values().filter(|m| m.health > 0).count());
    }
    let mut top: Vec<(usize, f64)> = ms.iter().cloned().enumerate().collect();
    top.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    eprintln!("PERF slowest siege ticks (tick, tick % 80, ms): {:?}", top.iter().take(10).map(|(i, m)| (*i, i % 80, (*m * 100.0).round() / 100.0)).collect::<Vec<_>>());
    report("basin battle siege 1v1v1v1, waves every 4 s", ms, peak);
}

/// Elwynn round three, Stage 11: the seal-cutter's escort in the whole built-in world. Pell (a
/// friendly walker) and the watchman (a friendly archer that `shoots`) fight the knives (creature
/// against creature, `Foe::Mob`), a hero beside Pell; a knife the hero fells every 5 s so the waves
/// come and he keeps walking the route.
#[test]
#[ignore]
fn perf_elwynn_escort_pell_with_friendly_archers() {
    use crate::tests_story::{bh_hero, fell, open_at};
    let mut c = Content::builtin();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 5, 1_000_000.0);
    let h = bh_hero(&mut s, [-9243.0, 178.0, 0.0]);
    s.camp.counts.insert("open:marzon_satchel".into(), 1);
    open_at(&mut s, h, "pell_satchel");
    let pell = |s: &Survival| s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "aldric_pell").map(|(id, _)| *id).expect("Pell");
    let mut ms = Vec::new();
    let (mut peak, mut mob_foes) = (0, 0);
    for tick in 0..1600 {
        if tick % 100 == 99 {
            let knives: Vec<Id> = s.mobs.iter().filter(|(_, m)| m.health > 0 && !m.hidden && !m.friendly && m.pack.is_some_and(|k| s.content.packs[k].key.starts_with("pell_"))).map(|(id, _)| *id).collect();
            for k in knives {
                fell(&mut s, h, k);
            }
        }
        let at = s.mobs[&pell(&s)].pos;
        s.move_player(h, [at[0] - 3.0, at[1], at[2]], 0.0, Moving::default());
        let t = Instant::now();
        let ev = s.step(DT);
        ms.push(t.elapsed().as_secs_f64() * 1000.0);
        std::hint::black_box(ev);
        if let Some(p) = s.players.get_mut(&h) {
            p.health = p.max_health;
        }
        peak = peak.max(s.mobs.values().filter(|m| m.health > 0).count());
        mob_foes = mob_foes.max(s.mobs.values().filter(|m| m.health > 0 && matches!(m.state.foe(), Some(Foe::Mob(_)))).count());
    }
    eprintln!("PERF escort: at most {mob_foes} creatures at once fighting another creature");
    assert!(mob_foes > 0, "the escort never set creature against creature: the bench measures nothing");
    report("elwynn escort (Pell, friendly archers, knives), 1 hero", ms, peak);
}

/// Stage 13: Sable the stalker in the whole built-in world, a hero who keeps turning with her (she
/// circles) and then gives her his back (she winds up), over and over.
#[test]
#[ignore]
fn perf_elwynn_sable_stalker_circling_a_hero() {
    use crate::tests_story::bh_hero;
    let mut c = Content::builtin();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    let h = bh_hero(&mut s, [0.0, 0.0, 0.0]);
    let k = s.spawn_mob(s.content.mob("sable").unwrap(), [2.5, 0.0, 0.0], None, false);
    s.engage(k, Foe::Hero(h));
    let mut ms = Vec::new();
    for tick in 0..1200 {
        let (me, her) = (s.players[&h].pos, s.mobs[&k].pos);
        let toward = (her[1] - me[1]).atan2(her[0] - me[0]);
        let f = if (tick / 100) % 2 == 0 { toward } else { toward + std::f32::consts::PI };
        s.move_player(h, me, f, Moving::default());
        let t = Instant::now();
        let ev = s.step(DT);
        ms.push(t.elapsed().as_secs_f64() * 1000.0);
        std::hint::black_box(ev);
        if let Some(p) = s.players.get_mut(&h) {
            p.health = p.max_health;
        }
        if s.mobs[&k].health == 0 {
            break;
        }
    }
    let live = s.mobs.values().filter(|m| m.health > 0).count();
    report("elwynn Sable (stalker) circling a hero", ms, live);
}
