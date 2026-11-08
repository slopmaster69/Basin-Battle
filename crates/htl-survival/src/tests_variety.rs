//! Variety within a kind (Corey, Nov 8: two brawlers and a caster shouldn't be "the exact same
//! fight" every time): each creature's own quirks, and a fight's plan (`[[plan]]`).

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::{Foe, MobState, Role};
use crate::sim::*;

const DT: f32 = 0.05;

fn world(plans: &[&str]) -> (Survival, Id) {
    let mut c = Content::dev();
    c.pvp = Default::default();
    if !plans.is_empty() {
        c.plans.retain(|p| plans.contains(&p.key.as_str()));
    }
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    let p = s.join("ayla", "Ayla", None);
    s.move_player(p, [0.0, 0.0, 0.0], 0.0, Moving::default());
    let _ = s.step(0.0);
    if let Some(h) = s.players.get_mut(&p) {
        h.max_health = 100_000;
        h.health = 100_000;
    }
    (s, p)
}

fn on_hero(s: &mut Survival, p: Id, key: &str, at: Vec3) -> Id {
    let def = s.content.mob(key).unwrap_or_else(|| panic!("no {key}"));
    let id = s.spawn_mob(def, at, None, false);
    s.mobs.get_mut(&id).unwrap().state = MobState::Chase { foe: Foe::Hero(p) };
    id
}

fn plan_line(ev: &[Event]) -> Option<String> {
    ev.iter().find_map(|e| match e {
        Event::Log(t) if t.starts_with("A fight begins, ") => Some(t.clone()),
        _ => None,
    })
}

#[test]
fn each_pirate_is_its_own_within_its_kind() {
    let (mut s, p) = world(&[]);
    let q = s.content.ai.quirk;
    assert!(q > 0.0, "survival.toml sets a quirk");
    let ids: Vec<Id> = (0..8).map(|k| on_hero(&mut s, p, "dm_pirate", [20.0 + k as f32, 0.0, 0.0])).collect();
    let paces: Vec<f32> = ids.iter().map(|id| s.mobs[id].quirk_pace).collect();
    assert!(paces.iter().all(|x| (1.0 - q - 1e-4..=1.0 + q + 1e-4).contains(x)), "{paces:?}");
    let spread = paces.iter().cloned().fold(f32::MIN, f32::max) - paces.iter().cloned().fold(f32::MAX, f32::min);
    assert!(spread > q * 0.5, "eight pirates, eight paces: {paces:?}");
    let favs: std::collections::BTreeSet<_> = ids.iter().map(|id| s.mobs[id].favourite).collect();
    assert!(favs.len() >= 2, "not all one favourite string: {favs:?}");
}

#[test]
fn two_brawlers_and_a_caster_dont_always_fight_the_same_way() {
    let mut plans = std::collections::BTreeSet::new();
    for k in 0..30 {
        let (mut s, p) = world(&[]);
        s.clock = 10.0 + k as f32 * 7.3;
        let ids = vec![
            on_hero(&mut s, p, "dm_pirate", [3.0, 0.0, 0.0]),
            on_hero(&mut s, p, "dm_pirate", [3.0, 2.0, 0.0]),
            on_hero(&mut s, p, "dm_squallshaper", [14.0, 0.0, 0.0]),
        ];
        s.plan_fight(&ids);
        let ev = s.step(0.0);
        let line = plan_line(&ev).expect("a plan for two pirates and a caster");
        plans.insert(line.split(':').next().unwrap().to_string());
    }
    assert!(plans.len() >= 3, "{plans:?}");
    // Waves needs three fighters up close: never for two.
    assert!(!plans.iter().any(|l| l.contains("waves")), "{plans:?}");
}

#[test]
fn a_boss_has_no_quirks() {
    let (mut s, p) = world(&[]);
    let b = on_hero(&mut s, p, "dm_vancleef", [3.0, 0.0, 0.0]);
    let m = &s.mobs[&b];
    assert_eq!((m.quirk_pace, m.quirk_retreat, m.favourite), (1.0, 1.0, None));
}

#[test]
fn a_boss_fight_has_no_plan() {
    let (mut s, p) = world(&[]);
    let ids = vec![on_hero(&mut s, p, "dm_smite", [3.0, 0.0, 0.0]), on_hero(&mut s, p, "dm_pirate", [3.0, 2.0, 0.0])];
    s.plan_fight(&ids);
    assert!(plan_line(&s.step(0.0)).is_none());
    assert!(ids.iter().all(|id| s.mobs[id].role == Role::None));
}

#[test]
fn a_held_back_pirate_stays_out_of_reach_until_its_fellow_is_hurt() {
    let (mut s, p) = world(&["bait"]);
    let a = on_hero(&mut s, p, "dm_pirate", [2.5, 0.0, 0.0]);
    let b = on_hero(&mut s, p, "dm_pirate", [2.5, 1.5, 0.0]);
    s.plan_fight(&[a, b]);
    let (front, back) = if s.mobs[&a].role == Role::Reserve { (b, a) } else { (a, b) };
    assert_eq!(s.mobs[&back].role, Role::Reserve);
    // (It starts beside its fellow, in reach: it backs off first.)
    let mut nearest = f32::MAX;
    for k in 0..(6.0 / DT) as usize {
        s.step(DT);
        if k as f32 * DT >= 2.5 {
            nearest = nearest.min(dist2d(s.mobs[&back].pos, [0.0, 0.0, 0.0]));
        }
        assert!(!matches!(s.mobs[&back].state, MobState::Windup { .. }), "held back, it never swings");
    }
    assert!(nearest > 4.5, "out of reach while its fellow fights: {nearest}");
    // Its fellow hurt past half: it comes in.
    let hurt = s.mobs[&front].max_health * 6 / 10;
    s.hurt_mob(front, p, hurt);
    let mut swung = false;
    for _ in 0..(6.0 / DT) as usize {
        s.step(DT);
        swung |= matches!(s.mobs[&back].state, MobState::Windup { .. });
    }
    assert_eq!(s.mobs[&back].role, Role::None);
    assert!(swung, "in it comes");
}

#[test]
fn a_guard_waits_between_its_hero_and_the_caster() {
    let (mut s, p) = world(&["screen"]);
    let g = on_hero(&mut s, p, "dm_pirate", [-3.0, 0.0, 0.0]);
    let c = on_hero(&mut s, p, "dm_squallshaper", [14.0, 0.0, 0.0]);
    s.plan_fight(&[g, c]);
    assert_eq!(s.mobs[&g].role, Role::Guard);
    let (mut waits, mut on_line) = (0, 0);
    for _ in 0..(8.0 / DT) as usize {
        s.step(DT);
        if matches!(s.mobs[&g].state, MobState::Chase { .. }) {
            waits += 1;
            let at = s.mobs[&g].pos;
            let toward = facing([0.0, 0.0, 0.0], at);
            let caster = facing([0.0, 0.0, 0.0], s.mobs[&c].pos);
            let off = ((toward - caster + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI).abs();
            if off < 0.8 {
                on_line += 1;
            }
        }
    }
    assert!(waits > 20, "it waited: {waits}");
    assert!(on_line * 2 > waits, "mostly on the caster's side of the hero: {on_line} of {waits}");
}

#[test]
fn a_flanker_strikes_from_its_heros_back() {
    let (mut s, p) = world(&["pincer"]);
    // The hero faces +x, the pirates in front of them.
    let a = on_hero(&mut s, p, "dm_pirate", [2.5, 0.5, 0.0]);
    let b = on_hero(&mut s, p, "dm_pirate", [2.5, -1.0, 0.0]);
    s.plan_fight(&[a, b]);
    let f = if s.mobs[&a].role == Role::Flanker { a } else { b };
    assert_eq!(s.mobs[&f].role, Role::Flanker);
    let mut first = None;
    for _ in 0..(8.0 / DT) as usize {
        s.step(DT);
        if first.is_none() && matches!(s.mobs[&f].state, MobState::Windup { .. }) {
            first = Some(s.behind(f, p));
        }
    }
    assert_eq!(first, Some(true), "its first blow comes from behind");
}

#[test]
fn under_every_plan_two_never_wind_up_at_the_hero_at_once() {
    for plan in ["all_in", "pincer", "screen", "bait", "waves"] {
        let (mut s, p) = world(&[plan]);
        let ids = vec![
            on_hero(&mut s, p, "dm_pirate", [3.0, 0.0, 0.0]),
            on_hero(&mut s, p, "dm_pirate", [3.0, 2.0, 0.0]),
            on_hero(&mut s, p, "dm_pirate", [-3.0, 1.0, 0.0]),
            on_hero(&mut s, p, "dm_squallshaper", [14.0, 0.0, 0.0]),
        ];
        s.plan_fight(&ids);
        for _ in 0..(12.0 / DT) as usize {
            s.step(DT);
            let winding = ids
                .iter()
                .filter(|id| !s.content.mobs[s.mobs[id].def].attack.ranged && matches!(s.mobs[id].state, MobState::Windup { .. }))
                .count();
            assert!(winding <= 1, "{plan}: {winding} winding up at once");
        }
    }
}
