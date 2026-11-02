//! Building at the camp: moving and tearing down what's built, upgrades, walls on their grid and
//! the gate, and the buildings' jobs.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::*;

const DT: f32 = 0.05;

fn world() -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    // Building at once, so a test needn't stand out the work.
    c.work.build = 0.0;
    c.work.craft = 0.0;
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 1, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

fn hero(s: &mut Survival, pos: Vec3, facing: f32) -> Id {
    let id = s.join("ayla", "Ayla", None);
    s.move_player(id, pos, facing, Moving::default());
    let _ = s.step(0.0);
    id
}

fn stand(s: &mut Survival, p: Id, pos: Vec3, facing: f32) {
    s.move_player(p, pos, facing, Moving::default());
    let _ = s.step(DT);
}

/// A clear yard of the building ground for station `key`, and room `along` yards north of it.
fn yard(s: &Survival, key: &str, along: f32) -> Vec3 {
    let def = s.content.station(key).unwrap();
    let c = s.content.world.build_center;
    let mut r = 0.0f32;
    while r <= s.content.world.build_radius {
        let n = ((std::f32::consts::TAU * r).ceil() as usize).max(1);
        for k in 0..n {
            let a = k as f32 / n as f32 * std::f32::consts::TAU;
            let g = [c[0] + a.cos() * r, c[1] + a.sin() * r, 0.0];
            if [0.0, along].iter().all(|dx| s.site_problem(def, [g[0] + dx, g[1], 0.0], None).is_none()) {
                return g;
            }
        }
        r += 1.0;
    }
    panic!("no room for a {key}");
}

/// Build `key` with its ghost at `at` (the hero stands west of it, facing east).
fn build_at(s: &mut Survival, p: Id, key: &str, at: Vec3) -> Result<Id, String> {
    stand(s, p, [at[0], at[1] + BUILD_DISTANCE, 0.0], -std::f32::consts::FRAC_PI_2);
    let before: Vec<Id> = s.objs.keys().copied().collect();
    s.build_start(p, key)?;
    let _ = s.step(DT);
    s.build_confirm(p)?;
    let _ = s.step(DT);
    s.objs.iter().find(|(id, o)| !before.contains(id) && matches!(o.kind, ObjKind::Station { .. })).map(|(id, _)| *id).ok_or_else(|| "nothing built".to_string())
}

fn def_of(s: &Survival, station: Id) -> String {
    match s.objs[&station].kind {
        ObjKind::Station { def, .. } => s.content.stations[def].key.clone(),
        _ => panic!("not a station"),
    }
}

#[test]
fn a_station_moved_keeps_what_it_was_and_costs_nothing() {
    let mut s = world();
    s.free_build = true;
    let at = yard(&s, "cooking", 10.0);
    let p = hero(&mut s, at, 0.0);
    let fire = build_at(&mut s, p, "cooking", at).unwrap();
    s.free_build = false;
    if let Some(Obj { kind: ObjKind::Station { health, .. }, .. }) = s.objs.get_mut(&fire) {
        *health = 123;
    }
    // Picked up and set down 10 yards north.
    stand(&mut s, p, [at[0] + 2.0, at[1], 0.0], 0.0);
    s.move_start(p, fire).unwrap();
    stand(&mut s, p, [at[0] + 10.0, at[1] + BUILD_DISTANCE, 0.0], -std::f32::consts::FRAC_PI_2);
    assert_eq!(s.placement(p).unwrap().problem, None, "{:?}", s.placement(p));
    s.build_confirm(p).unwrap();
    let _ = s.step(DT);
    let o = &s.objs[&fire];
    assert!(dist2d(o.pos, [at[0] + 10.0, at[1], 0.0]) < 0.5, "moved: {:?}", o.pos);
    assert!(matches!(o.kind, ObjKind::Station { health: 123, .. }), "kept as it was");
    assert!(s.players[&p].preview.is_none());
    let stations = s.objs.values().filter(|o| matches!(o.kind, ObjKind::Station { .. })).count();
    assert_eq!(stations, 1, "the same one, not a second");
}

#[test]
fn a_station_torn_down_is_asked_first_and_gives_half_back() {
    let mut s = world();
    let at = yard(&s, "smithy", 0.0);
    let p = hero(&mut s, at, 0.0);
    let wood = s.content.item("wood").unwrap();
    let ore = s.content.item("ore").unwrap();
    *s.players.get_mut(&p).unwrap().bag.entry(wood).or_default() += 8;
    *s.players.get_mut(&p).unwrap().bag.entry(ore).or_default() += 4;
    let anvil = build_at(&mut s, p, "smithy", at).unwrap();
    s.storage.clear();
    stand(&mut s, p, [at[0] + 2.0, at[1], 0.0], 0.0);
    s.demolish(p, Some(anvil), false).unwrap();
    let ev = s.step(DT);
    assert!(ev.iter().any(|e| matches!(e, Event::Confirm { what, .. } if what == "demolish")), "asked first");
    assert!(s.objs.contains_key(&anvil), "still there until it's said");
    s.demolish(p, None, true).unwrap();
    assert!(!s.objs.contains_key(&anvil), "torn down");
    assert_eq!((s.storage.get(&wood).copied(), s.storage.get(&ore).copied()), (Some(4), Some(2)), "half of 8 timber and 4 copper");
}

#[test]
fn an_upgrade_stays_put_does_what_it_was_and_more() {
    let mut s = world();
    s.free_build = true;
    let at = yard(&s, "kitchen", 0.0);
    let p = hero(&mut s, at, 0.0);
    let fire = build_at(&mut s, p, "cooking", at).unwrap();
    let pos = s.objs[&fire].pos;
    stand(&mut s, p, [at[0] + 2.5, at[1], 0.0], 0.0);
    s.upgrade(p, fire).unwrap();
    let _ = s.step(DT);
    assert_eq!(def_of(&s, fire), "kitchen");
    assert_eq!(s.objs[&fire].pos, pos, "where it stood");
    assert!(s.has_station("cooking"), "a raid that needs a cooking fire still finds one");
    let kitchen = s.content.station("kitchen").unwrap();
    assert!(matches!(s.objs[&fire].kind, ObjKind::Station { health, .. } if health == s.content.stations[kitchen].health));
    // Its recipes are the fire's, each a roast more.
    let r = s.recipes(p, fire);
    assert!(r.iter().any(|(k, name, _, _)| k == "roast" && name.contains('x')), "{r:?}");
    let meat = s.content.item("meat").unwrap();
    let roast = s.content.item("roast").unwrap();
    *s.players.get_mut(&p).unwrap().bag.entry(meat).or_default() += 5;
    let before = s.players[&p].bag.get(&roast).copied().unwrap_or(0);
    s.craft(p, fire, "roast").unwrap();
    let _ = s.step(DT);
    let recipe = s.content.recipes.iter().find(|r| r.key == "roast").unwrap().count;
    assert_eq!(s.players[&p].bag.get(&roast).copied().unwrap_or(0) - before, recipe + 1, "one more");
    // Nothing further: as good as it gets.
    assert!(s.upgrade(p, fire).is_err());
}

#[test]
fn walls_sit_on_their_grid_cell_to_cell_and_the_gate_among_them() {
    let mut s = world();
    s.free_build = true;
    let snap = s.content.stations[s.content.station("palisade").unwrap()].snap;
    assert!(snap > 0.0);
    let at = yard(&s, "palisade", snap * 2.0);
    let p = hero(&mut s, at, 0.0);
    let a = build_at(&mut s, p, "palisade", at).unwrap();
    // A little off: it lands on the grid, a cell along.
    let b = build_at(&mut s, p, "palisade", [at[0] + snap + 0.6, at[1] + 0.4, 0.0]).unwrap();
    let c = s.content.world.build_center;
    for id in [a, b] {
        let o = &s.objs[&id];
        for (v, o0) in [(o.pos[0], c[0]), (o.pos[1], c[1])] {
            let k = (v - o0) / snap;
            assert!((k - k.round()).abs() < 0.01, "on the grid: {:?}", o.pos);
        }
        let q = o.facing / std::f32::consts::FRAC_PI_2;
        assert!((q - q.round()).abs() < 0.01, "a quarter turn: {}", o.facing);
    }
    assert!((dist2d(s.objs[&a].pos, s.objs[&b].pos) - snap).abs() < 0.6, "cell to cell");
    // The gate in the next cell along.
    let bp = s.objs[&b].pos;
    let g = build_at(&mut s, p, "gate", [bp[0] + snap, bp[1], 0.0]).unwrap();
    assert_eq!(def_of(&s, g), "gate");
    assert!(s.content.stations[s.content.station("gate").unwrap()].passable);
    // Not on the same cell twice.
    let ap = s.objs[&a].pos;
    assert!(build_at(&mut s, p, "palisade", ap).is_err());
}

#[test]
fn a_building_is_one_of_a_kind() {
    let mut s = world();
    s.free_build = true;
    let at = yard(&s, "barracks", 14.0);
    let p = hero(&mut s, at, 0.0);
    build_at(&mut s, p, "barracks", at).unwrap();
    let e = build_at(&mut s, p, "barracks", [at[0] + 14.0, at[1], 0.0]).unwrap_err();
    assert!(e.contains("already"), "{e}");
}

#[test]
fn the_buildings_do_their_jobs() {
    let mut s = world();
    s.free_build = true;
    // A barracks: up twice as soon, with more health.
    let (wait, more) = s.barracks();
    assert_eq!((wait, more), (1.0, 0.0));
    let at = yard(&s, "barracks", 0.0);
    let p = hero(&mut s, at, 0.0);
    build_at(&mut s, p, "barracks", at).unwrap();
    let (wait, more) = s.barracks();
    assert!(wait < 0.75 && more > 0.0, "{wait} {more}");
    // An infirmary: resting by it is faster.
    let at = yard(&s, "infirmary", 0.0);
    build_at(&mut s, p, "infirmary", at).unwrap();
    assert!(s.infirmary_at([at[0] + 3.0, at[1], 0.0]).is_some_and(|f| f > 1.0));
    assert!(s.infirmary_at([at[0] + 60.0, at[1], 0.0]).is_none());
    // A mill: timber into the chest every so often, not during a raid.
    let at = yard(&s, "mill", 0.0);
    build_at(&mut s, p, "mill", at).unwrap();
    let wood = s.content.item("wood").unwrap();
    s.storage.remove(&wood);
    let every = s.content.stations[s.content.station("mill").unwrap()].yields.as_ref().unwrap().every;
    for _ in 0..((every + 2.0) / 0.5) as usize {
        let _ = s.step(0.5);
    }
    assert!(s.storage.get(&wood).copied().unwrap_or(0) >= 2, "the mill's timber");
}

#[test]
fn free_building_and_stocking_the_chest_are_for_testing() {
    let mut s = world();
    let names = s.stock_for_building(7);
    assert!(!names.is_empty());
    let wood = s.content.item("wood").unwrap();
    assert_eq!(s.storage.get(&wood).copied(), Some(7));
    s.storage.clear();
    s.free_build = true;
    let at = yard(&s, "watchtower", 0.0);
    let p = hero(&mut s, at, 0.0);
    assert!(build_at(&mut s, p, "watchtower", at).is_ok(), "nothing to pay");
}
