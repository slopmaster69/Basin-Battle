//! The Proving Ring: the book's page in, waves that never end, a fall that costs nothing, the ways
//! out.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::Foe;
use crate::sim::*;

const DT: f32 = 0.05;

fn world() -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 1, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        out.extend(s.step(DT));
    }
    out
}

/// A hero at the book, and into the ring by its page.
fn in_the_ring(s: &mut Survival, name: &str) -> Id {
    let book = s.book_id();
    let at = s.objs[&book].pos;
    let id = s.join(&name.to_lowercase(), name, None);
    s.move_player(id, at, 0.0, Moving::default());
    s.travel(id, "arena").unwrap();
    id
}

fn standing_foes(s: &Survival) -> Vec<Id> {
    s.arena.foes.iter().copied().filter(|f| s.mobs.get(f).is_some_and(|m| m.health > 0)).collect()
}

fn ring() -> crate::content::ArenaDef {
    Content::dev().arena.unwrap()
}

#[test]
fn the_rings_page_is_open_from_the_start_and_its_first_wave_comes_for_the_heroes_in_it() {
    let mut s = world();
    assert_eq!(s.camp.raids_won, 0);
    let a = ring();
    let id = in_the_ring(&mut s, "Ayla");
    assert!(s.in_arena(id));
    let p = s.players[&id].pos;
    assert!(dist2d(p, [a.at[0], a.at[1], 0.0]) < a.radius, "set down in the ring: {p:?}");
    assert!(s.travel(id, "arena").is_err(), "in it already");
    assert!(s.travel(id, "first").is_err(), "the Deadmines' pages only from camp");
    // Its braziers and banner stand there.
    assert!(s.objs.values().any(|o| o.kind == ObjKind::ArenaBanner));
    // Nothing comes until the first wave's time.
    run(&mut s, a.first_gap - 1.0);
    assert!(standing_foes(&s).is_empty());
    let ev = run(&mut s, 1.5);
    let foes = standing_foes(&s);
    assert!(!foes.is_empty(), "the first wave");
    assert_eq!(s.arena.wave, 1);
    assert!(ev.iter().any(|e| matches!(e, Event::Banner { player: Some(h), title, .. } if *h == id && title == "Wave 1")));
    for f in &foes {
        let m = &s.mobs[f];
        assert!(dist2d(m.pos, [a.at[0], a.at[1], 0.0]) <= a.radius + 0.5, "in from the edge");
        assert_eq!(m.state.foe(), Some(Foe::Hero(id)), "on the hero");
    }
    s.travel(id, "camp").unwrap();
    assert!(standing_foes(&s).is_empty(), "the last hero gone, the ring clears");
    assert_eq!(s.arena.wave, 0);
}

#[test]
fn a_wave_beaten_mends_its_heroes_and_each_after_is_worth_more_and_tougher_a_champion_every_fifth() {
    let mut s = world();
    let a = ring();
    let id = in_the_ring(&mut s, "Ayla");
    run(&mut s, a.first_gap + 0.5);
    let max = s.players[&id].max_health;
    s.players.get_mut(&id).unwrap().health = max / 2;
    for f in standing_foes(&s) {
        s.hurt_mob(f, id, 100_000);
    }
    let ev = run(&mut s, 0.2);
    assert!(!s.arena.fighting);
    assert!(ev.iter().any(|e| matches!(e, Event::Banner { title, .. } if title == "Wave 1 Beaten")));
    let mended = max / 2 + (max as f32 * a.mend).round() as u32;
    assert_eq!(s.players[&id].health, mended.min(max), "a breather");
    assert_eq!(s.arena_best(), 1, "the camp's best");
    run(&mut s, a.gap + 0.2);
    assert_eq!(s.arena.wave, 2);
    assert!(!standing_foes(&s).is_empty());

    // Worth more as they go, up to the most at once; a champion first on every fifth.
    let worth = |s: &Survival, list: &[usize]| -> f32 {
        list.iter().map(|d| a.roster.iter().find(|f| f.mob == s.content.mobs[*d].key).map_or(0.0, |f| f.cost)).sum()
    };
    let w1 = s.arena_wave(1, 1);
    let w4 = s.arena_wave(4, 1);
    assert!(worth(&s, &w4) > worth(&s, &w1));
    assert!(s.arena_wave(40, 4).len() <= a.most);
    assert!(s.arena_wave(2, 3).len() >= s.arena_wave(2, 1).len(), "more for more heroes");
    let five = s.arena_wave(5, 1);
    assert!(s.content.mobs[five[0]].boss, "a champion on the fifth");
    assert_eq!(s.content.mobs[five[0]].key, a.bosses[0]);
    let ten = s.arena_wave(10, 1);
    assert_eq!(s.content.mobs[ten[0]].key, a.bosses[1]);
    let six = s.arena_wave(6, 1);
    assert!(!s.content.mobs[six[0]].boss);
    let one = s.arena_wave(1, 1);
    for d in one {
        assert!(a.roster.iter().any(|f| f.mob == s.content.mobs[d].key && f.from <= 1), "only the first wave's foes");
    }
    // Tougher: wave 2's foes have more than their kind's health.
    for f in standing_foes(&s) {
        let m = &s.mobs[&f];
        assert!(m.max_health > s.content.mobs[m.def].health);
    }
    // A champion's wave beaten pays into the chest.
    s.arena.wave = 4;
    for f in standing_foes(&s) {
        s.hurt_mob(f, id, 100_000);
    }
    run(&mut s, a.gap + 0.5);
    assert_eq!(s.arena.wave, 5);
    // (Its trophy, the first time, aside.)
    s.camp.counts.insert(format!("kill:{}", a.bosses[0]), 1);
    let before: u32 = s.storage.values().sum();
    for _ in 0..3 {
        for f in standing_foes(&s) {
            s.hurt_mob(f, id, 1_000_000);
        }
        run(&mut s, 0.5);
    }
    assert!(!s.arena.fighting, "the champion's wave beaten");
    let after: u32 = s.storage.values().sum();
    assert_eq!(after - before, a.reward.values().sum::<u32>());
    assert_eq!(s.arena_best(), 5);
}

#[test]
fn a_fall_in_the_ring_ends_the_run_and_costs_nothing() {
    let mut s = world();
    let a = ring();
    let id = in_the_ring(&mut s, "Ayla");
    let wood = s.content.item("wood").unwrap();
    s.players.get_mut(&id).unwrap().bag.insert(wood, 5);
    run(&mut s, a.first_gap + 0.5);
    let foes = standing_foes(&s);
    assert!(!foes.is_empty());
    let hp = s.players[&id].health;
    s.hurt_player(id, hp);
    assert!(!s.in_arena(id));
    assert!(!s.objs.values().any(|o| matches!(o.kind, ObjKind::Cache { .. })), "no sack left in the ring");
    assert_eq!(s.players[&id].bag.get(&wood), Some(&5));
    run(&mut s, 0.2);
    for f in foes {
        assert!(!s.mobs.contains_key(&f), "the ring clears");
    }
    // Up again at the camp.
    let secs = s.content.player.respawn_secs;
    run(&mut s, secs + 1.0);
    let w = s.content.world.clone();
    assert!(dist2d(s.players[&id].pos, [w.spawn[0], w.spawn[1], 0.0]) < 2.0);
    assert!(!s.in_arena(id));
}

#[test]
fn the_banner_takes_a_hero_home_walking_out_ends_a_run_and_a_hero_saved_in_the_ring_wakes_at_camp() {
    let mut s = world();
    let a = ring();
    let w = s.content.world.clone();
    let ayla = in_the_ring(&mut s, "Ayla");
    let bren = in_the_ring(&mut s, "Bren");
    assert_eq!(s.arena.heroes.len(), 2);
    assert!(dist2d(s.players[&ayla].pos, s.players[&bren].pos) > 1.0, "side by side, not on top of each other");
    run(&mut s, a.first_gap + 0.5);
    // Between waves, the banner calls the next at once.
    assert!(s.arena_next_now(ayla).is_err(), "a wave on");
    // Home by the banner.
    s.travel(ayla, "camp").unwrap();
    assert!(!s.in_arena(ayla));
    assert!(dist2d(s.players[&ayla].pos, [w.spawn[0], w.spawn[1], 0.0]) < 1.0);
    for f in standing_foes(&s) {
        assert_eq!(s.mobs[&f].state.foe(), Some(Foe::Hero(bren)), "on whoever's left");
    }
    // Walking out of the ring.
    let p = s.players[&bren].pos;
    s.move_player(bren, [a.at[0] + a.radius + 10.0, a.at[1], p[2]], 0.0, Moving::default());
    run(&mut s, 0.1);
    assert!(!s.in_arena(bren));
    assert!(s.arena.foes.is_empty(), "nobody left: the ring clears");
    // Saved mid-run: back at the camp.
    let cara = in_the_ring(&mut s, "Cara");
    let save = s.leave(cara).unwrap();
    assert!(dist2d(save.pos, [w.spawn[0], w.spawn[1], 0.0]) < 1.0);
    assert!(s.arena.heroes.is_empty());
    let back = s.join("cara", "Cara", Some(&save));
    assert!(!s.in_arena(back));
    // Between waves the banner brings the next at once.
    let dara = in_the_ring(&mut s, "Dara");
    s.arena_next_now(dara).unwrap();
    run(&mut s, 0.1);
    assert_eq!(s.arena.wave, 1);
}
