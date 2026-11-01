//! The Deadmines' rules: lamps, Draughts, the one-use sack, rest, doors, the book at camp.

use std::collections::BTreeMap;

use crate::content::{Content, DoorDef, LampDef, PackDef};
use crate::land::FlatLand;
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.05;

fn lamp(key: &str, at: Vec3) -> LampDef {
    LampDef {
        key: key.into(),
        name: format!("The {key}"),
        at,
        facing: 0.0,
    }
}

fn pack(mob: &str, key: &str, at: [f32; 2], elite: bool) -> PackDef {
    PackDef {
        mob: mob.into(),
        at,
        z: None,
        radius: 0.0,
        count: 1,
        link: 0.0,
        route: Vec::new(),
        wait: 8.0,
        camp: String::new(),
        key: key.into(),
        elite,
        hidden: 0.0, wake: String::new(), spots: Vec::new(), rare: false,
    }
}

/// The rules with a small Deadmines: two lamps 60 yards apart, a grunt between them, a warlord
/// (a boss) in a room barred by a fog door, and a shortcut opened from the east.
fn content() -> Content {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let d = c.dungeon.as_mut().unwrap();
    d.lamps = vec![
        lamp("first", [0.0, 0.0, 0.0]),
        lamp("second", [60.0, 0.0, 0.0]),
    ];
    d.packs = vec![
        pack("grunt", "", [30.0, 0.0], false),
        pack("warlord", "boss", [100.0, 0.0], false),
    ];
    d.doors = vec![
        DoorDef {
            key: "fog".into(),
            name: "Fog".into(),
            kind: "fog".into(),
            display: 1,
            at: [90.0, 0.0, 0.0],
            facing: 0.0,
            scale: 1.0,
            from: None,
            boss: "boss".into(),
            bars: Some([[90.0, -10.0], [90.0, 10.0]]),
            text: String::new(),
            ..Default::default()
        },
        DoorDef {
            key: "short".into(),
            name: "Gate".into(),
            kind: "shortcut".into(),
            display: 1,
            at: [0.0, 40.0, 0.0],
            facing: 0.0,
            scale: 1.0,
            from: Some([0.0, 45.0, 0.0]),
            boss: String::new(),
            bars: Some([[-5.0, 40.0], [5.0, 40.0]]),
            text: String::new(),
            ..Default::default()
        },
        DoorDef {
            key: "lift".into(),
            name: "Chain Lift".into(),
            kind: "lift".into(),
            display: 0,
            at: [-30.0, 0.0, 0.0],
            facing: 0.0,
            scale: 1.0,
            from: Some([-30.0, 30.0, 0.0]),
            boss: String::new(),
            bars: None,
            text: String::new(),
            ..Default::default()
        },
    ];
    d.exit_boss = "boss".into();
    d.exit = [120.0, 0.0, 0.0];
    d.hazards.clear();
    d.boxes = vec![crate::content::BoxDef { name: String::new(),
        key: "box".into(),
        at: [5.0, 5.0, 0.0],
        facing: 0.0,
        loot: BTreeMap::from([
            ("draught_flask".to_string(), 1),
            ("plunder".to_string(), 10),
        ]),
    }];
    c
}

fn below() -> Survival {
    Survival::new_dungeon(content(), Box::new(FlatLand::default()), 3, 1_000_000.0)
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        out.extend(s.step(DT));
    }
    out
}

fn obj(s: &Survival, f: impl Fn(&ObjKind) -> bool) -> Id {
    *s.objs
        .iter()
        .find(|(_, o)| f(&o.kind))
        .map(|(id, _)| id)
        .expect("the object")
}

fn draughts(s: &Survival, id: Id) -> u32 {
    let d = s.content.item("draught").unwrap();
    s.players[&id].bag.get(&d).copied().unwrap_or(0)
}

#[test]
fn a_hero_arrives_at_the_first_lamp_with_three_draughts_and_no_food() {
    let mut s = below();
    let mut save = crate::save::save_player(&s.content, &{
        let mut camp = Survival::new(content(), Box::new(FlatLand::default()), 1, 0.0);
        let id = camp.join("ayla", "Ayla", None);
        let roast = camp.content.item("roast").unwrap();
        camp.players.get_mut(&id).unwrap().bag.insert(roast, 3);
        camp.leave(id).map(|_| ()).unwrap();
        let id = camp.join("ayla", "Ayla", None);
        camp.players.remove(&id).unwrap()
    });
    save.bag.insert("roast".into(), 3);
    let id = s.join("ayla", "Ayla", Some(&save));
    assert_eq!(s.players[&id].pos, [0.0, 0.0, 0.5], "at the first lamp, set down just over it");
    assert_eq!(draughts(&s, id), 3);
    let roast = s.content.item("roast").unwrap();
    assert!(
        !s.usable_items(&s.players[&id]).contains(&roast),
        "food doesn't work below"
    );
    assert_eq!(
        s.quick_item(&s.players[&id]),
        s.content.item("draught"),
        "R drinks a Draught"
    );
    // A Draught heals.
    s.players.get_mut(&id).unwrap().health = 50;
    s.input(id, Input::Use);
    run(&mut s, 2.0);
    assert!(
        s.players[&id].health > 50 + 60,
        "healed by the Draught: {}",
        s.players[&id].health
    );
    assert_eq!(draughts(&s, id), 2);
    // Leaving, the Draughts stay below; coming back, they're in hand again.
    let left = s.leave(id).unwrap();
    assert!(!left.bag.contains_key("draught"), "not carried up");
    let id = s.join("ayla", "Ayla", Some(&left));
    assert_eq!(draughts(&s, id), 2);
}

#[test]
fn lighting_a_lamp_makes_it_a_page_and_resting_fills_the_draughts_and_wakes_the_mine() {
    let mut s = below();
    let id = s.join("ayla", "Ayla", None);
    let grunt = *s
        .mobs
        .keys()
        .find(|m| s.content.mobs[s.mobs[*m].def].key == "grunt")
        .unwrap();
    let lamp = obj(&s, |k| matches!(k, ObjKind::Lamp { key } if key == "first"));
    s.interact(id, Some(lamp));
    let ev = s.step(DT);
    assert!(ev
        .iter()
        .any(|e| matches!(e, Event::Banner { title, .. } if title == "Lamp Lit")));
    assert!(ev.iter().any(|e| matches!(e, Event::ToCamp { counts, .. } if counts.iter().any(|(k, _)| k == "delve:first"))), "the camp hears of it");
    assert_eq!(
        s.lit_lamps(),
        vec![("first".to_string(), "The first".to_string())]
    );
    // The grunt falls; a Draught's drunk.
    s.hurt_mob(grunt, id, 100_000);
    s.input(id, Input::Use);
    run(&mut s, 2.0);
    assert_eq!(draughts(&s, id), 2);
    assert!(!s
        .mobs
        .values()
        .any(|m| s.content.mobs[m.def].key == "grunt" && m.health > 0));
    run(&mut s, 30.0);
    assert!(
        !s.mobs
            .values()
            .any(|m| s.content.mobs[m.def].key == "grunt" && m.health > 0),
        "no timer brings it back below"
    );
    s.rest(id).unwrap();
    assert_eq!(draughts(&s, id), 3, "full again");
    assert!(
        s.mobs
            .values()
            .any(|m| s.content.mobs[m.def].key == "grunt" && m.health > 0),
        "the mine stirs"
    );
}

#[test]
fn a_boss_stays_dead_and_its_fog_door_bars_the_room_while_it_fights() {
    let mut s = below();
    let id = s.join("ayla", "Ayla", None);
    s.move_player(id, [95.0, 0.0, 0.0], 0.0, Moving::default());
    let boss = *s
        .mobs
        .keys()
        .find(|m| s.content.mobs[s.mobs[*m].def].key == "warlord")
        .unwrap();
    s.engage(boss, mobs::Foe::Hero(id));
    s.step(DT);
    assert!(!s.door_is_open("fog"), "barred while it fights");
    // Out through the fog: held back.
    s.move_player(id, [85.0, 0.0, 0.0], 0.0, Moving::default());
    assert!(s.players[&id].pos[0] > 90.0, "the fog holds them in");
    s.hurt_mob(boss, id, 1_000_000);
    s.step(DT);
    assert!(s.slain("boss"));
    assert!(s.door_is_open("fog"));
    // Rest: it doesn't come back.
    let lamp = obj(
        &s,
        |k| matches!(k, ObjKind::Lamp { key } if key == "second"),
    );
    s.move_player(id, [60.0, 0.0, 0.0], 0.0, Moving::default());
    s.interact(id, Some(lamp));
    run(&mut s, 5.0);
    s.rest(id).unwrap();
    assert!(
        !s.mobs
            .values()
            .any(|m| s.content.mobs[m.def].key == "warlord" && m.health > 0),
        "a boss is gone for good"
    );
    // And the front door lets them out, home.
    s.move_player(id, [120.0, 0.0, 0.0], 0.0, Moving::default());
    let ev = s.step(DT);
    assert!(ev
        .iter()
        .any(|e| matches!(e, Event::Travel { player, to: None } if *player == id)));
}

#[test]
fn a_shortcut_opens_from_its_far_side_only() {
    let mut s = below();
    let id = s.join("ayla", "Ayla", None);
    let door = obj(&s, |k| matches!(k, ObjKind::Door { key } if key == "short"));
    s.move_player(id, [0.0, 36.0, 0.0], 0.0, Moving::default());
    s.interact(id, Some(door));
    assert!(!s.door_is_open("short"), "barred from this side");
    // Walking through it: held back.
    s.move_player(id, [0.0, 44.0, 0.0], 0.0, Moving::default());
    assert!(s.players[&id].pos[1] < 40.0);
    s.players.get_mut(&id).unwrap().pos = [0.0, 45.0, 0.0];
    s.interact(id, Some(door));
    assert!(s.door_is_open("short"), "opened from its far side");
    s.move_player(id, [0.0, 35.0, 0.0], 0.0, Moving::default());
    assert!(s.players[&id].pos[1] < 40.0, "and through it");
}

#[test]
fn a_fall_below_leaves_one_sack_and_falling_again_loses_it() {
    let mut s = below();
    let id = s.join("ayla", "Ayla", None);
    let plunder = s.content.item("plunder").unwrap();
    s.players.get_mut(&id).unwrap().bag.insert(plunder, 12);
    s.move_player(id, [20.0, 0.0, 0.0], 0.0, Moving::default());
    s.fall_damage(id, 100_000);
    let respawn = s.content.player.respawn_secs;
    run(&mut s, 0.1);
    let sacks = |s: &Survival| {
        s.objs
            .values()
            .filter(|o| matches!(o.kind, ObjKind::Cache { .. }))
            .count()
    };
    assert_eq!(sacks(&s), 1);
    assert!(!s.players[&id].bag.contains_key(&plunder));
    run(&mut s, respawn + 1.0);
    assert_eq!(s.players[&id].pos, [0.0, 0.0, 0.0], "awake at the lamp");
    assert_eq!(s.players[&id].health, s.players[&id].max_health, "whole");
    // Fall again with fresh plunder before winning the sack back: the first is lost.
    s.players.get_mut(&id).unwrap().bag.insert(plunder, 3);
    s.fall_damage(id, 100_000);
    run(&mut s, 0.1);
    assert_eq!(sacks(&s), 1, "only the new sack");
    let in_sack: u32 = s
        .objs
        .values()
        .filter_map(|o| match &o.kind {
            ObjKind::Cache { items, .. } => items.get(&plunder).copied(),
            _ => None,
        })
        .sum();
    assert_eq!(in_sack, 3);
}

#[test]
fn a_strongbox_gives_a_flask_and_plunder_once() {
    let mut s = below();
    let id = s.join("ayla", "Ayla", None);
    let b = obj(&s, |k| matches!(k, ObjKind::Strongbox { .. }));
    s.move_player(id, [3.0, 3.0, 0.0], 0.0, Moving::default());
    s.interact(id, Some(b));
    assert_eq!(draughts(&s, id), 4, "a Draught more");
    let plunder = s.content.item("plunder").unwrap();
    assert_eq!(s.players[&id].bag.get(&plunder), Some(&10));
    assert!(!s.objs.contains_key(&b));
    let lamp = obj(&s, |k| matches!(k, ObjKind::Lamp { key } if key == "first"));
    s.interact(id, Some(lamp));
    s.rest(id).unwrap();
    assert_eq!(draughts(&s, id), 4, "and every rest after");
    // Bought at the lamp: a whetstone for plunder.
    let name = s.buy(id, "whetstone").unwrap();
    assert!(name.contains("Sharpening"));
    assert_eq!(s.players[&id].bag.get(&plunder), Some(&4));
}

#[test]
fn the_book_at_camp_opens_after_the_first_raid_and_sends_a_hero_down() {
    // The book's gate, set for this test (the built-in dungeons start open for now).
    let mut c = content();
    for d in c.dungeon.iter_mut().chain(c.more_dungeons.iter_mut()) {
        d.gate_raids = 1;
    }
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 1, 0.0);
    s.mobs.clear();
    let book = obj(&s, |k| *k == ObjKind::Book);
    let at = s.objs[&book].pos;
    let id = s.join("ayla", "Ayla", None);
    s.move_player(id, at, 0.0, Moving::default());
    assert!(
        s.travel(id, "first").is_err(),
        "closed until the camp holds a raid"
    );
    s.camp.raids_won = 1;
    assert!(s.travel(id, "second").is_err(), "a page not lit yet");
    s.travel(id, "first").unwrap();
    let ev = s.step(DT);
    assert!(ev.iter().any(
        |e| matches!(e, Event::Travel { player, to: Some(k) } if *player == id && k == "first")
    ));
    s.set_pages(vec!["first".into(), "second".into()]);
    // Both of this dungeon's, and each other dungeon's first page after them, the easiest first.
    let pages: Vec<String> = s.book_pages().into_iter().map(|(k, _)| k).collect();
    assert_eq!(&pages[..2], &["first", "second"]);
    let mut more: Vec<&crate::content::DungeonDef> = s.content.more_dungeons.iter().collect();
    more.sort_by_key(|d| d.tier);
    let firsts: Vec<String> = more.iter().map(|d| d.lamps[0].key.clone()).collect();
    assert_eq!(pages[2..].to_vec(), firsts);
    let books = s.book_dungeons();
    assert_eq!(books[0].key, "deadmines");
    assert_eq!(books.len(), 1 + s.content.more_dungeons.len());
    assert!(books.iter().all(|b| b.open));
}

#[test]
fn the_world_below_is_kept_across_a_restart() {
    let mut s = below();
    let id = s.join("ayla", "Ayla", None);
    let lamp = obj(&s, |k| matches!(k, ObjKind::Lamp { key } if key == "first"));
    s.interact(id, Some(lamp));
    let b = obj(&s, |k| matches!(k, ObjKind::Strongbox { .. }));
    s.move_player(id, [3.0, 3.0, 0.0], 0.0, Moving::default());
    s.interact(id, Some(b));
    let boss = *s
        .mobs
        .keys()
        .find(|m| s.content.mobs[s.mobs[*m].def].key == "warlord")
        .unwrap();
    s.hurt_mob(boss, id, 1_000_000);
    s.step(DT);
    let save = s.save(&BTreeMap::new());
    let text = crate::save::to_toml(&save).unwrap();
    let back = crate::save::from_toml(&text).unwrap();
    let t = Survival::load_dungeon(
        content(),
        Box::new(FlatLand::default()),
        3,
        1_000_000.0,
        &back,
    );
    assert!(t.lit_lamps().iter().any(|(k, _)| k == "first"));
    assert!(t.slain("boss"));
    assert!(
        !t.mobs
            .values()
            .any(|m| t.content.mobs[m.def].key == "warlord"),
        "the boss stays beaten"
    );
    assert!(
        !t.objs
            .values()
            .any(|o| matches!(o.kind, ObjKind::Strongbox { .. })),
        "the box stays empty"
    );
    assert_eq!(t.below.as_ref().unwrap().heroes["ayla"].flasks, 1);
    assert!(
        !back.players["ayla"].bag.contains_key("draught"),
        "the Draughts are kept below, not in the save's bags"
    );
    assert_eq!(back.below.as_ref().unwrap().heroes["ayla"].draughts, 4);
    let _ = MobState::Return;
}

#[test]
fn a_lift_is_set_going_from_its_far_end_then_rides_both_ways() {
    let mut s = below();
    let id = s.join("ayla", "Ayla", None);
    let near = obj(&s, |k| matches!(k, ObjKind::Door { key } if key == "lift"));
    let far = obj(&s, |k| matches!(k, ObjKind::LiftEnd { key } if key == "lift"));
    s.move_player(id, [-30.0, 2.0, 0.0], 0.0, Moving::default());
    s.interact(id, Some(near));
    assert_eq!(s.players[&id].pos, [-30.0, 2.0, 0.0], "it won't go from here yet");
    s.players.get_mut(&id).unwrap().pos = [-30.0, 28.0, 0.0];
    s.interact(id, Some(far));
    assert!(s.door_is_open("lift"), "set going from its far end");
    s.players.get_mut(&id).unwrap().pos = [-30.0, 2.0, 0.0];
    s.interact(id, Some(near));
    assert!(s.players[&id].pos[1] > 29.0, "a ride to the far end: {:?}", s.players[&id].pos);
    // A fog draws its veil while shut, and none once open.
    let veils = s.objs.values().filter(|o| matches!(&o.kind, ObjKind::Veil { key } if key == "fog")).count();
    assert!(veils >= 8, "a column every few yards: {veils}");
    let v = *s.objs.iter().find(|(_, o)| matches!(&o.kind, ObjKind::Veil { .. })).unwrap().0;
    assert!(s.object(v).is_none(), "open: no veil drawn");
}

#[test]
fn a_machine_that_falls_lets_its_driver_out_and_comes_back_whole_if_he_wins() {
    let mut c = content();
    c.dungeon.as_mut().unwrap().packs[1].mob = "dm_shredder".into();
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    let id = s.join("ayla", "Ayla", None);
    s.move_player(id, [95.0, 0.0, 0.0], 0.0, Moving::default());
    let find = |s: &Survival, key: &str| {
        s.mobs
            .iter()
            .find(|(_, m)| m.health > 0 && s.content.mobs[m.def].key == key)
            .map(|(id, _)| *id)
    };
    let shredder = find(&s, "dm_shredder").expect("the shredder");
    s.engage(shredder, mobs::Foe::Hero(id));
    s.step(DT);
    s.hurt_mob(shredder, id, 1_000_000);
    s.step(DT);
    let sneed = find(&s, "dm_sneed").expect("Sneed climbs out");
    assert!(!s.slain("boss"), "the pack stands while Sneed does");
    assert!(!s.door_is_open("fog"), "and the fog holds");
    assert!(
        !matches!(s.mobs[&sneed].state, MobState::Idle { .. }),
        "he goes straight for his killer"
    );
    // He wins (the fight is off) and the hero rests: the shredder is back, whole, and Sneed in it.
    s.mobs.get_mut(&sneed).unwrap().state = MobState::Idle { wander_at: 1e9 };
    let lamp = obj(&s, |k| matches!(k, ObjKind::Lamp { key } if key == "second"));
    s.move_player(id, [60.0, 0.0, 0.0], 0.0, Moving::default());
    s.interact(id, Some(lamp));
    run(&mut s, 5.0);
    s.rest(id).unwrap();
    assert!(find(&s, "dm_sneed").is_none());
    let back = find(&s, "dm_shredder").expect("the shredder is back");
    assert_eq!(s.mobs[&back].health, s.mobs[&back].max_health);
    // Beaten both ways: gone for good.
    s.engage(back, mobs::Foe::Hero(id));
    s.hurt_mob(back, id, 1_000_000);
    s.step(DT);
    let sneed = find(&s, "dm_sneed").unwrap();
    s.hurt_mob(sneed, id, 1_000_000);
    s.step(DT);
    assert!(s.slain("boss"));
}

#[test]
fn a_hero_who_falls_through_the_world_is_put_back_and_unstuck_goes_to_the_lamp() {
    let mut s = below();
    let id = s.join("ayla", "Ayla", None);
    s.step(DT);
    let safe = s.players[&id].last_safe;
    // Set down over the lamp's floor, never level with it.
    let arrived = s.players[&id].pos;
    assert!(arrived[2] > safe[2] + 0.2 && arrived[2] < safe[2] + 1.0, "lands from just above: {arrived:?} over {safe:?}");
    // Just arrived: a short drop through a floor that wasn't there yet is caught at once.
    s.move_player(id, [safe[0], safe[1], safe[2] - 15.0], 0.0, Moving::default());
    let ev = s.step(DT);
    assert!((s.players[&id].pos[2] - safe[2]).abs() < 1.0, "caught on arrival: {:?}", s.players[&id].pos);
    assert!(ev.iter().any(|e| matches!(e, Event::Teleport { player, .. } if *player == id)));
    assert!((s.players[&id].last_safe[2] - safe[2]).abs() < 0.01, "the firm ground stays the floor's");
    s.move_player(id, safe, 0.0, Moving::default());
    for _ in 0..(9.0 / DT) as usize {
        s.step(DT);
    }
    // Long after: a 15 yard drop is a fall, not the world.
    s.move_player(id, [safe[0], safe[1], safe[2] - 15.0], 0.0, Moving::default());
    s.step(DT);
    assert!(s.players[&id].pos[2] < safe[2] - 10.0, "a real fall stands");
    s.move_player(id, [safe[0], safe[1], safe[2] - 200.0], 0.0, Moving::default());
    let ev = s.step(DT);
    assert!((s.players[&id].pos[2] - safe[2]).abs() < 1.0, "back on firm ground");
    assert!(ev.iter().any(|e| matches!(e, Event::Teleport { player, .. } if *player == id)));
    s.move_player(id, [30.0, 20.0, 0.0], 0.0, Moving::default());
    s.unstuck(id).unwrap();
    let p = &s.players[&id];
    assert!(p.pos[0].abs() < 0.01 && p.pos[1].abs() < 0.01, "at the first lamp");
}

#[test]
fn a_hero_carried_to_a_far_lamp_is_held_still_a_moment_while_the_floor_comes() {
    let mut s = below();
    let id = s.join("ayla", "Ayla", None);
    run(&mut s, 3.0);
    let first = obj(&s, |k| matches!(k, ObjKind::Lamp { key } if key == "first"));
    s.interact(id, Some(first));
    s.step(DT);
    let second = obj(&s, |k| matches!(k, ObjKind::Lamp { key } if key == "second"));
    let at = s.objs[&second].pos;
    s.move_player(id, at, 0.0, Moving::default());
    s.interact(id, Some(second));
    s.step(DT);
    s.move_player(id, [0.0, 0.0, 0.0], 0.0, Moving::default());
    run(&mut s, 0.5);
    s.travel(id, "second").unwrap();
    let ev = run(&mut s, 0.2);
    let teleport = ev.iter().position(|e| matches!(e, Event::Teleport { player, .. } if *player == id));
    let root = ev.iter().position(|e| matches!(e, Event::Rooted { player, on: true } if *player == id));
    assert!(teleport.is_some() && root.is_some() && teleport < root, "set down, then held: {ev:?}");
    let ev = run(&mut s, 1.6);
    assert!(ev.iter().any(|e| matches!(e, Event::Rooted { player, on: false } if *player == id)), "then let go");
    assert!(!s.players[&id].rooted);
}

#[test]
fn a_beaten_run_starts_over_a_while_after_once_nobody_is_below() {
    let mut c = content();
    c.dungeon.as_mut().unwrap().reset_minutes = 1.0;
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    let id = s.join("ayla", "Ayla", None);
    // The boss beaten, the strongbox emptied, the second lamp lit.
    let boss = *s.mobs.keys().find(|m| s.content.mobs[s.mobs[*m].def].key == "warlord").unwrap();
    s.hurt_mob(boss, id, 1_000_000);
    s.step(DT);
    assert!(s.slain("boss"));
    let b = obj(&s, |k| matches!(k, ObjKind::Strongbox { .. }));
    s.move_player(id, [3.0, 3.0, 0.0], 0.0, Moving::default());
    s.interact(id, Some(b));
    let lamp = obj(&s, |k| matches!(k, ObjKind::Lamp { key } if key == "second"));
    s.move_player(id, [60.0, 0.0, 0.0], 0.0, Moving::default());
    s.interact(id, Some(lamp));
    run(&mut s, 5.0);
    let lit = s.below.as_ref().unwrap().lit.clone();
    assert!(lit.contains("second"), "the second lamp lit: {lit:?}");
    // A minute on with a hero still below: nothing comes back on top of them.
    run(&mut s, 60.0);
    assert!(s.slain("boss"), "not while someone's below");
    // Gone up: the run starts over, the lamps as they were.
    let save = s.leave(id).unwrap();
    let _ = save;
    s.step(DT);
    assert!(!s.slain("boss"), "the boss stands again");
    assert!(s.mobs.values().any(|m| s.content.mobs[m.def].key == "warlord" && m.health > 0));
    assert!(s.objs.values().any(|o| matches!(o.kind, ObjKind::Strongbox { .. })), "the strongbox full again");
    assert_eq!(s.below.as_ref().unwrap().lit, lit, "the lamps stay lit");
    assert!(s.below.as_ref().unwrap().run_at.is_none(), "a new run");
}

#[test]
fn the_deadmines_start_over_with_the_camp() {
    let mut s = below();
    let id = s.join("ayla", "Ayla", None);
    let lamp = obj(&s, |k| matches!(k, ObjKind::Lamp { key } if key == "second"));
    s.move_player(id, [60.0, 0.0, 0.0], 0.0, Moving::default());
    s.interact(id, Some(lamp));
    run(&mut s, 5.0);
    let boss = *s.mobs.keys().find(|m| s.content.mobs[s.mobs[*m].def].key == "warlord").unwrap();
    s.hurt_mob(boss, id, 1_000_000);
    s.step(DT);
    s.reset_below(true);
    let b = s.below.as_ref().unwrap();
    assert!(b.lit.is_empty() && b.open.is_empty() && b.slain.is_empty(), "as on the first day");
    assert!(dist2d(s.players[&id].pos, [0.0, 0.0, 0.0]) < 1.0, "set down at the first lamp: {:?}", s.players[&id].pos);
    assert!(s.mobs.values().any(|m| s.content.mobs[m.def].key == "warlord" && m.health > 0));
}

#[test]
fn a_boss_leaves_one_of_its_pieces_for_certain_and_the_rest_by_chance() {
    let mut c = content();
    let w = c.mob("warlord").unwrap();
    c.mobs[w].gear = vec!["cookie_tenderizer".into(), "cookie_trousers".into()];
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    let k = s.content.packs.iter().position(|p| p.key == "boss").unwrap();
    let boss = *s.mobs.keys().find(|m| s.mobs[*m].pack == Some(k)).unwrap();
    s.mobs.remove(&boss);
    let (axe, legs) = (s.content.item("cookie_tenderizer").unwrap(), s.content.item("cookie_trousers").unwrap());
    let (mut seen_axe, mut seen_legs, mut total) = (0, 0, 0);
    for _ in 0..60 {
        let before: Vec<Id> = s.objs.keys().copied().collect();
        s.below_kill(w, [100.0, 0.0, 0.0], [100.0, 0.0, 0.0], Some(k), false, false);
        let drops: Vec<usize> = s.objs.iter().filter(|(id, _)| !before.contains(id)).filter_map(|(_, o)| if let ObjKind::Loot { item, .. } = o.kind { Some(item) } else { None }).collect();
        assert!(!drops.is_empty(), "one piece for certain");
        seen_axe += drops.iter().filter(|i| **i == axe).count();
        seen_legs += drops.iter().filter(|i| **i == legs).count();
        total += drops.len();
    }
    assert!(seen_axe > 0 && seen_legs > 0, "the weapon and the armour both fall: {seen_axe} {seen_legs}");
    assert!(total < 100, "not everything every time: {total} pieces from 60 kills");
}

#[test]
fn below_the_journal_is_the_camps() {
    let mut s = below();
    let _id = s.join("ayla", "Ayla", None);
    let counts = BTreeMap::from([("delve:book".to_string(), 1)]);
    s.mirror_journal(5, &counts);
    let ev = run(&mut s, 0.5);
    assert!(ev.iter().any(|e| matches!(e, Event::Goal)), "shown again");
    assert_eq!(s.camp.goal, 5, "the camp's goal, not the first");
    // Done or not, the journal moves on at camp only.
    let g = s.content.goals[5].clone();
    s.camp.counts.insert(format!("{}:{}", g.kind, g.what), g.count);
    run(&mut s, 0.5);
    assert_eq!(s.camp.goal, 5);
}

#[test]
fn a_boss_turns_at_each_of_its_phases_harder_and_quicker_each_time() {
    use crate::content::EnrageDef;
    let mut c = content();
    let w = c.mob("warlord").unwrap();
    let phase = |at: f32, speed: f32, might: f32| EnrageDef { at, speed, pattern: String::new(), text: format!("at {at}"), summon: Vec::new(), might, kit: 0, look: 0, sound: 0, shove: 0.0 };
    c.mobs[w].enrage = None;
    c.mobs[w].posture = 0;
    c.mobs[w].phases = vec![phase(0.75, 0.9, 1.1), phase(0.5, 0.8, 1.2), phase(0.25, 0.7, 1.4)];
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    let id = s.join("ayla", "Ayla", None);
    s.move_player(id, [95.0, 0.0, 0.0], 0.0, Moving::default());
    let boss = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "warlord").unwrap().0;
    s.engage(boss, mobs::Foe::Hero(id));
    let base = s.attack_now(boss);
    let max = s.mobs[&boss].max_health;
    let mut seen = Vec::new();
    for share in [0.8f32, 0.7, 0.45, 0.2] {
        let to = (share * max as f32) as u32;
        let now = s.mobs[&boss].health;
        s.hurt_mob(boss, id, now - to);
        s.mobs.get_mut(&boss).unwrap().pattern = None;
        seen.push((s.mobs[&boss].phase, s.attack_now(boss)));
    }
    assert_eq!(seen.iter().map(|(p, _)| *p).collect::<Vec<_>>(), vec![0, 1, 2, 3], "one phase per threshold crossed");
    let last = &seen[3].1;
    assert!(last.windup < base.windup * 0.75 && last.damage > base.damage, "{} vs {}", last.windup, base.windup);
    assert!(seen[2].1.damage > seen[1].1.damage, "harder each turn");
}

#[test]
fn a_secret_form_rises_from_a_fallen_boss_only_once_its_secret_is_found() {
    let mut c = content();
    let w = c.mob("warlord").unwrap();
    c.mobs[w].then = "dm_sneed".into();
    c.mobs[w].then_after = 3.0;
    c.mobs[w].then_needs = "shortcut".into();
    c.mobs[w].then_text = "Not yet!".into();
    let fight = |c: Content, opened: bool| {
        let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
        if opened {
            s.below.as_mut().unwrap().open.insert("shortcut".into());
        }
        let id = s.join("ayla", "Ayla", None);
        s.move_player(id, [95.0, 0.0, 0.0], 0.0, Moving::default());
        let boss = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "warlord").unwrap().0;
        s.engage(boss, mobs::Foe::Hero(id));
        s.hurt_mob(boss, id, 1_000_000);
        let ev = run(&mut s, 0.5);
        (s, id, ev)
    };
    // Not found: the boss is simply dead.
    let (s, _, _) = fight(c.clone(), false);
    assert!(s.slain("boss"));
    assert!(!s.mobs.values().any(|m| s.content.mobs[m.def].key == "dm_sneed"));
    // Found: the fight seems won, the fog holds, and nothing is to be seen...
    let (mut s, id, ev) = fight(c, true);
    assert!(!s.slain("boss") && !s.door_is_open("fog"), "the pack stands while its secret form waits");
    let sneed = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "dm_sneed").unwrap().0;
    assert!(s.mobs[&sneed].hidden, "unseen while it waits");
    assert!(!ev.iter().any(|e| matches!(e, Event::MobSpawned { id } if *id == sneed)));
    // ...until it rises, on its killer.
    let ev = run(&mut s, 3.0);
    assert!(!s.mobs[&sneed].hidden && s.mobs[&sneed].rise_at.is_none());
    assert!(ev.iter().any(|e| matches!(e, Event::MobSpawned { id } if *id == sneed)));
    assert!(ev.iter().any(|e| matches!(e, Event::Announce { text } if text.contains("Not yet!"))));
    assert!(!matches!(s.mobs[&sneed].state, MobState::Idle { .. }), "straight at its killer");
    s.hurt_mob(sneed, id, 1_000_000);
    s.step(DT);
    assert!(s.slain("boss"), "beaten both ways: gone for good");
}

#[test]
fn a_false_wall_gives_way_and_a_hidden_spot_is_found_each_laying_out_what_it_kept() {
    let mut c = content();
    let d = c.dungeon.as_mut().unwrap();
    d.packs.push(pack("grunt", "rare", [10.0, 20.0], true));
    d.boxes.push(crate::content::BoxDef { name: String::new(), key: "stash".into(), at: [12.0, 20.0, 0.0], facing: 0.0, loot: BTreeMap::from([("plunder".to_string(), 5)]) });
    d.doors.push(DoorDef { key: "wall".into(), name: "A hollow wall".into(), kind: "illusory".into(), display: 1, at: [10.0, 15.0, 0.0], scale: 1.0, bars: Some([[5.0, 15.0], [15.0, 15.0]]), reveals: vec!["stash".into()], page: "Behind the wall, a stash and a page of sums.{n}{n}Someone counted twice.".into(), ..Default::default() });
    d.doors.push(DoorDef { key: "nook".into(), name: "A nook".into(), kind: "hidden".into(), at: [-20.0, -20.0, 0.0], find: 3.0, display: 2890, look: "A Heap of Sacks".into(), reveals: vec!["rare".into(), "confession".into()], ..Default::default() });
    d.notes.push(crate::content::NoteDef { hints: vec![], clue: String::new(), key: "confession".into(), name: "A confession".into(), at: [-21.0, -20.0, 0.0], facing: 0.0, display: 0, scale: 1.0, text: "It was me.".into() });
    d.notes.push(crate::content::NoteDef { hints: vec!["wall".into()], clue: "The wall by the stash.".into(), key: "diary".into(), name: "A diary".into(), at: [2.0, 0.0, 0.0], facing: 0.0, display: 0, scale: 1.0, text: "The wall by the stash sounds hollow.".into() });
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    let id = s.join("ayla", "Ayla", None);
    let has = |s: &Survival, f: &dyn Fn(&ObjKind) -> bool| s.objs.values().any(|o| f(&o.kind));
    let rare = |s: &Survival| s.mobs.values().any(|m| m.pack.is_some_and(|k| s.content.packs[k].key == "rare"));
    assert!(!has(&s, &|k| matches!(k, ObjKind::Strongbox { key } if key == "stash")), "kept back");
    assert!(!rare(&s), "kept back");
    assert!(!has(&s, &|k| matches!(k, ObjKind::Note { key } if key == "confession")), "a note the nook keeps isn't there to read yet");
    // The note says what it says.
    let note = obj(&s, |k| matches!(k, ObjKind::Note { key } if key == "diary"));
    s.move_player(id, [1.0, 0.0, 0.0], 0.0, Moving::default());
    s.interact(id, Some(note));
    let ev = s.step(DT);
    // A page to read at leisure, with its clue while the wall still stands.
    assert!(ev.iter().any(|e| matches!(e, Event::Letter { kind, text, clue, .. } if kind == "note" && text.contains("hollow") && clue == "The wall by the stash.")), "{ev:?}");
    assert!(s.ever("read", "diary"), "the camp remembers it read");
    let lore = |s: &mut Survival| {
        s.lore_page(id, "").unwrap();
        s.step(DT).into_iter().find_map(|e| match e {
            Event::Letter { kind, text, links, place, .. } if kind == "lore" => Some((text, links, place)),
            _ => None,
        })
    };
    let (text, links, place) = lore(&mut s).expect("the book's page");
    assert!(text.contains("The wall by the stash. (A diary)") && place.starts_with("1 of ") && place.ends_with(" notes read") && !text.contains("Secrets found") && !place.contains("secret"), "{text} / {place}");
    assert_eq!(links, vec![(".es note diary".to_string(), "A diary".to_string())]);
    // The wall, used: it's gone, and the stash is there.
    let wall = obj(&s, |k| matches!(k, ObjKind::Door { key } if key == "wall"));
    assert!(s.object(wall).is_some(), "a wall to see");
    assert!(s.through_shut_door([10.0, 10.0, 0.0], [10.0, 20.0, 0.0]), "and it bars the way");
    s.move_player(id, [10.0, 13.0, 0.0], 0.0, Moving::default());
    s.interact(id, Some(wall));
    let ev = s.step(DT);
    assert!(!ev.iter().any(|e| matches!(e, Event::Banner { .. })), "no banner and horn for everyone");
    assert!(
        ev.iter().any(|e| matches!(e, Event::Letter { player, kind, text, .. } if *player == id && kind == "secret" && text.starts_with("Behind the wall, a stash"))),
        "the finder's page opens, with the secret's page"
    );
    assert!(
        ev.iter().any(|e| matches!(e, Event::Letter { kind, text, .. } if kind == "secret" && !text.contains("{n}") && text.contains("sums.\n\nSomeone"))),
        "its breaks are real newlines, for chat without the add-on as much as for the add-on"
    );
    assert!(s.object(wall).is_none() && !s.through_shut_door([10.0, 10.0, 0.0], [10.0, 20.0, 0.0]));
    assert!(has(&s, &|k| matches!(k, ObjKind::Strongbox { key } if key == "stash")));
    // Found, the clue is spent; read again, the note has none.
    assert!(s.reread(id, "diary").is_ok());
    assert!(s.step(DT).iter().any(|e| matches!(e, Event::Letter { clue, place, .. } if clue.is_empty() && place.starts_with("From the book"))));
    assert!(s.reread(id, "confession").is_err(), "not read yet");
    // The nook: walked up to, its marker catches the eye; searched, the rare one steps out of it.
    let sacks = obj(&s, |k| matches!(k, ObjKind::Door { key } if key == "nook"));
    assert_eq!(s.object(sacks).map(|v| (v.display, v.name, v.usable)), Some((2890, "A Heap of Sacks".to_string(), true)));
    s.move_player(id, [-19.0, -19.0, 0.0], 0.0, Moving::default());
    let ev = run(&mut s, 0.2);
    assert!(ev.iter().any(|e| matches!(e, Event::Tell { text, .. } if text.contains("heap of sacks"))), "{ev:?}");
    assert!(!s.below.as_ref().unwrap().open.contains("nook"), "noticed, not found");
    assert_eq!(s.use_prompt(id).map(|(_, n, v)| (n, v)), Some(("A Heap of Sacks".to_string(), "Search")));
    s.interact(id, Some(sacks));
    let ev = s.step(DT);
    assert!(ev.iter().any(|e| matches!(e, Event::Letter { kind, title, place, .. } if kind == "secret" && title == "A nook" && place.starts_with("A secret of ") && !place.contains("found"))), "no tally: {ev:?}");
    assert!(s.below.as_ref().unwrap().open.contains("nook") && rare(&s));
    assert!(s.object(sacks).is_none(), "the sacks pulled aside");
    assert!(has(&s, &|k| matches!(k, ObjKind::Note { key } if key == "confession")), "and its note with it");
}

#[test]
fn every_dungeon_is_a_world_of_its_own_with_its_secrets_kept_back() {
    let c = Content::builtin();
    let keys: Vec<String> = c.dungeons().map(|d| d.key.clone()).collect();
    assert!(keys.len() >= 2, "{keys:?}");
    for key in keys {
        let s = Survival::new_dungeon(c.for_dungeon_of(&key), Box::new(FlatLand::default()), 3, 1_000_000.0);
        let d = s.content.dungeon.clone().unwrap();
        assert_eq!(d.key, key);
        assert!(d.lamps.len() <= 3, "{key}: few lamps ({})", d.lamps.len());
        let lamps = s.objs.values().filter(|o| matches!(o.kind, ObjKind::Lamp { .. })).count();
        assert_eq!(lamps, d.lamps.len());
        // What a secret keeps isn't there yet.
        for dr in d.doors.iter().filter(|x| x.secret()) {
            for r in &dr.reveals {
                assert!(!s.mobs.values().any(|m| m.pack.is_some_and(|k| &s.content.packs[k].key == r)), "{key}: {r} kept back");
                assert!(!s.objs.values().any(|o| matches!(&o.kind, ObjKind::Strongbox { key } if key == r)), "{key}: {r} kept back");
                assert!(!s.objs.values().any(|o| matches!(&o.kind, ObjKind::Note { key } if key == r)), "{key}: {r} kept back");
            }
        }
        // A boss's secret form waits on a secret whose note is only read once it's found (so
        // reading it means the secret is found).
        for p in &d.packs {
            let m = &s.content.mobs[s.content.mob(&p.mob).unwrap()];
            if let Some(dr) = d.doors.iter().find(|x| x.secret() && x.key == m.then_needs) {
                for n in d.notes.iter().filter(|n| dist2d(n.at, dr.at) < 8.0) {
                    assert!(dr.reveals.contains(&n.key), "{key}: {}'s note {} beside {} is readable before it's found", m.key, n.key, dr.key);
                }
            }
        }
        // Every boss turns at least once.
        for p in &d.packs {
            let m = &s.content.mobs[s.content.mob(&p.mob).unwrap()];
            if m.boss {
                assert!(!m.phases.is_empty(), "{key}: {} has no phases", m.key);
            }
        }
    }
}

#[test]
fn a_common_foe_with_a_second_form_comes_back_once_at_a_rest() {
    let mut c = content();
    c.dungeon.as_mut().unwrap().packs[0].mob = "wc_druid".into();
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    let id = s.join("ayla", "Ayla", None);
    let count = |s: &Survival, key: &str| s.mobs.values().filter(|m| m.health > 0 && s.content.mobs[m.def].key == key).count();
    let druid = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "wc_druid").unwrap().0;
    s.engage(druid, mobs::Foe::Hero(id));
    s.hurt_mob(druid, id, 1_000_000);
    s.step(DT);
    assert_eq!(count(&s, "wc_fang_serpent"), 1, "the serpent crawls out");
    let serpent = *s.mobs.iter().find(|(_, m)| m.health > 0 && s.content.mobs[m.def].key == "wc_fang_serpent").unwrap().0;
    s.hurt_mob(serpent, id, 1_000_000);
    run(&mut s, 5.0);
    let lamp = obj(&s, |k| matches!(k, ObjKind::Lamp { key } if key == "first"));
    s.move_player(id, [0.0, 0.0, 0.0], 0.0, Moving::default());
    s.interact(id, Some(lamp));
    run(&mut s, 5.0);
    s.rest(id).unwrap();
    assert_eq!((count(&s, "wc_druid"), count(&s, "wc_fang_serpent")), (1, 0), "the druid is back, once; no serpent of its own");
}

#[test]
fn whitemane_comes_when_mograine_falls_raises_him_at_half_and_the_ashbringer_waits_for_its_shard() {
    let fight = |shard: bool| {
        let c = Content::builtin().for_dungeon_of("cathedral");
        let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
        if shard {
            s.below.as_mut().unwrap().open.insert("scarlet_reliquary".into());
        }
        let id = s.join("ayla", "Ayla", None);
        s.players.get_mut(&id).unwrap().health = 1_000_000;
        let find = |s: &Survival, key: &str| s.mobs.iter().find(|(_, m)| m.health > 0 && s.content.mobs[m.def].key == key).map(|(id, _)| *id);
        let mograine = find(&s, "sc_mograine").expect("Mograine at the altar");
        let at = s.mobs[&mograine].pos;
        // (Set down beside the altar: too far to walk there in one move.)
        s.players.get_mut(&id).unwrap().pos = [at[0] - 3.0, at[1], at[2]];
        s.engage(mograine, mobs::Foe::Hero(id));
        s.hurt_mob(mograine, id, 1_000_000);
        run(&mut s, 3.0);
        let whitemane = find(&s, "sc_whitemane").expect("Whitemane comes");
        assert!(!s.mobs[&whitemane].hidden);
        assert!(!matches!(s.mobs[&whitemane].state, MobState::Idle { .. }), "on her champion's killer: {:?}", s.mobs[&whitemane].state);
        let max = s.mobs[&whitemane].max_health;
        s.hurt_mob(whitemane, id, max / 2 + 5);
        // (A blow that big breaks her stance first: her turn comes once she stands again.)
        run(&mut s, 4.0);
        assert!(find(&s, "sc_mograine_risen").is_some(), "Arise, my champion!");
        s.hurt_mob(whitemane, id, 1_000_000);
        run(&mut s, 4.5);
        (find(&s, "sc_ashen").is_some(), s.slain("mograine"))
    };
    assert_eq!(fight(false), (false, true), "without the shard: done");
    assert_eq!(fight(true), (true, false), "with it: the Ashbringer's shadow, and the altar's pack stands");
}

#[test]
fn a_lobbed_throw_is_marked_from_its_wind_up_and_takes_its_time_even_point_blank() {
    let mut c = content();
    let w = c.mob("warlord").unwrap();
    c.mobs[w].patterns.clear();
    c.mobs[w].special = None;
    c.mobs[w].phases.clear();
    c.mobs[w].enrage = None;
    let a = &mut c.mobs[w].attack;
    a.ranged = true;
    a.windup = 1.0;
    a.radius = 3.0;
    a.speed = 12.0;
    a.range = 24.0;
    a.damage = 30;
    a.blockable = false;
    // A fireball: a 3-yard splash, so warned by the drawn circle at its true size, not Heated
    // Ground (which covers far more than 3 yards).
    a.spell = 133;
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    let id = s.join("ayla", "Ayla", None);
    let boss = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "warlord").unwrap().0;
    let bp = s.mobs[&boss].pos;
    // Point blank: two yards off.
    let here = [bp[0] - 2.0, bp[1], bp[2]];
    s.move_player(id, here, 0.0, Moving::default());
    s.windup(boss, mobs::Foe::Hero(id), here);
    let marked = s.events.iter().any(|e| matches!(e, Event::Mark { pos, radius, .. } if dist2d(*pos, here) < 0.5 && (*radius - 3.0).abs() < 0.01));
    assert!(marked, "its ring is on the ground where the hero stood, from the wind-up's start");
    assert!(s.events.iter().any(|e| matches!(e, Event::Mark { ground: 0, radius, .. } if (*radius - 3.0).abs() < 0.01)), "drawn as its circle");
    assert!(!s.events.iter().any(|e| matches!(e, Event::Mark { ground: 22191, .. })), "not laid as Heated Ground");
    let mark = s.mobs[&boss].lob.expect("the lob remembered").1;
    // Still there at the moment it's let go (the wind-up done), and a while after.
    run(&mut s, 1.05);
    assert!(s.mark_left(mark) > mobs::LOB_MIN_FLY - 0.2, "the ring stays through the flight: {}", s.mark_left(mark));
    assert!(!s.missiles.is_empty(), "in the air, not landed at once");
    // Stepping out in that time is enough.
    let full = s.players[&id].health;
    s.players.get_mut(&id).unwrap().pos = [here[0] - 6.0, here[1], here[2]];
    run(&mut s, 1.0);
    assert!(s.missiles.is_empty(), "down by now");
    assert_eq!(s.players[&id].health, full, "missed: the hero stepped out of the ring");
}

#[test]
fn gear_gives_its_perks_while_on_and_a_set_more_for_its_pieces() {
    use crate::content::{PerkDef, SetBonusDef, SetDef};
    let mut c = content();
    let jerkin = c.item("defias_jerkin").unwrap();
    let legs = c.item("defias_leggings").unwrap();
    c.items[jerkin].perks = vec![PerkDef { kind: "might".into(), amount: 0.05 }];
    // (The content's own sets and perks off these two, a set of the test's own on them.)
    c.items[jerkin].set = "test_set".into();
    c.items[legs].set = "test_set".into();
    c.items[legs].perks.clear();
    c.sets.push(SetDef { key: "test_set".into(), name: "The Brotherhood's Garb".into(), bonus: vec![SetBonusDef { pieces: 2, kind: "stamina".into(), amount: 0.1 }] });
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    let id = s.join("ayla", "Ayla", None);
    let fx = |s: &Survival, k: &str| s.effect_of(&s.players[&id], k);
    assert_eq!(fx(&s, "might"), 0.0);
    s.players.get_mut(&id).unwrap().armor.insert("chest".into(), jerkin);
    assert!((fx(&s, "might") - 0.05).abs() < 1e-6, "the jerkin's perk, worn");
    assert_eq!(fx(&s, "stamina"), 0.0, "one piece of the set: no bonus");
    s.players.get_mut(&id).unwrap().armor.insert("legs".into(), legs);
    assert!((fx(&s, "stamina") - 0.1).abs() < 1e-6, "two pieces: the set's bonus");
    let words = s.gear_words(jerkin);
    assert!(words.contains("+5% damage") && words.contains("The Brotherhood's Garb") && words.contains("2 pieces"), "{words}");
    s.players.get_mut(&id).unwrap().armor.remove("chest");
    assert_eq!(fx(&s, "might"), 0.0, "taken off: gone");
    assert_eq!(fx(&s, "stamina"), 0.0);
}

#[test]
fn a_strongbox_can_have_guards_waiting_unseen_for_whoever_opens_it() {
    let mut c = content();
    let d = c.dungeon.as_mut().unwrap();
    d.boxes.push(crate::content::BoxDef { name: String::new(), key: "bait".into(), at: [30.0, 30.0, 0.0], facing: 0.0, loot: BTreeMap::from([("plunder".to_string(), 5)]) });
    let mut guards = pack("grunt", "", [33.0, 30.0], false);
    guards.count = 2;
    guards.radius = 1.5;
    guards.wake = "bait".into();
    d.packs.push(guards);
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    let id = s.join("ayla", "Ayla", None);
    let at_box = |s: &Survival| s.mobs.values().filter(|m| m.health > 0 && dist2d(m.pos, [33.0, 30.0, 0.0]) < 4.0).collect::<Vec<_>>().len();
    let shown = |s: &Survival| s.mobs.values().filter(|m| m.health > 0 && !m.hidden && dist2d(m.pos, [33.0, 30.0, 0.0]) < 4.0).count();
    assert_eq!(at_box(&s), 2, "laid out");
    assert_eq!(shown(&s), 0, "but unseen");
    // Standing by them wakes nothing: only the box does.
    s.move_player(id, [31.0, 30.0, 0.0], 0.0, Moving::default());
    run(&mut s, 0.5);
    assert_eq!(shown(&s), 0, "still unseen beside them");
    let bx = obj(&s, |k| matches!(k, ObjKind::Strongbox { key } if key == "bait"));
    s.interact(id, Some(bx));
    let ev = s.step(DT);
    assert!(ev.iter().any(|e| matches!(e, Event::Banner { title, .. } if title == "Ambush!")));
    assert_eq!(shown(&s), 2, "out of the dark");
    assert!(s.mobs.values().filter(|m| dist2d(m.pos, [33.0, 30.0, 0.0]) < 4.0).all(|m| m.state.target() == Some(id)), "on the opener");
}

#[test]
fn a_wager_runs_against_the_clock_raises_the_stakes_and_gives_the_dungeon_back() {
    use crate::sim::wager::{WAGER_DAMAGE, WAGER_HEALTH};
    let mut s = below();
    let grunt = |s: &Survival| *s.mobs.iter().find(|(_, m)| m.pack.is_some_and(|k| s.content.packs[k].mob == "grunt")).unwrap().0;
    let plain = s.mobs[&grunt(&s)].max_health as f32;
    s.below.as_mut().unwrap().open.insert("short".into());
    assert_eq!(s.wager_unlocked("deadmines"), 1);
    s.start_wager(3);
    // Fresh, and tougher: the shortcut shut, the grunt a level's worth (and Hardened) stronger.
    assert!(!s.door_is_open("short"), "shut for the wager");
    // What it has to beat, for the tracker: the boss, by name, still up.
    let boss_name = s.content.mobs.iter().find(|m| m.key == s.content.dungeon.as_ref().unwrap().packs.iter().find(|p| p.key == "boss").unwrap().mob).unwrap().name.clone();
    assert_eq!(s.wager_progress(), vec![(boss_name.clone(), false)]);
    let tough = s.mobs[&grunt(&s)].max_health as f32;
    let want = plain * (1.0 + WAGER_HEALTH).powi(2) * 1.2;
    assert!((tough - want).abs() <= want * 0.02 + 1.0, "{tough} vs {want}");
    let g = grunt(&s);
    let hit = s.attack_now(g).damage as f32;
    let def = s.mobs[&g].def;
    let base = s.content.mobs[def].attack.damage as f32;
    assert!((hit - base * (1.0 + WAGER_DAMAGE).powi(2) * 1.16).abs() <= 1.0, "{hit} vs {base}");
    // A hero comes down: the slip, and the clock waits till they leave the lamp.
    let id = s.join("ayla", "Ayla", None);
    let ev = run(&mut s, 1.0);
    assert!(ev.iter().any(|e| matches!(e, Event::Banner { title, .. } if title == "The Bookmaker's Wager: +3")), "{ev:?}");
    assert!(s.wager().unwrap().started.is_none());
    s.wager_fall();
    assert_eq!(s.wager().unwrap().deaths, 0, "a fall before the clock costs nothing");
    s.move_player(id, [20.0, 0.0, 0.0], 0.0, Moving::default());
    let ev = run(&mut s, 0.2);
    assert!(ev.iter().any(|e| matches!(e, Event::Banner { title, .. } if title.contains("clock is running"))));
    s.wager_fall();
    assert_eq!(s.wager().unwrap().deaths, 1);
    assert!(s.wager_elapsed() >= 10.0, "a fall's ten seconds");
    // Foes chase farther during a wager.
    use crate::sim::wager::WAGER_LEASH;
    assert_eq!(s.leash_of(def), s.content.mobs[def].leash * WAGER_LEASH);
    // Its boss down first: not won till enough of its foes are down too.
    assert_eq!(s.wager_forces(), Some((0, 1, 1)), "one grunt standing, one needed");
    let boss = *s.mobs.iter().find(|(_, m)| m.pack.is_some_and(|k| s.content.packs[k].key == "boss")).unwrap().0;
    s.hurt_mob(boss, id, 10_000_000);
    let ev = run(&mut s, 0.2);
    assert!(!ev.iter().any(|e| matches!(e, Event::WagerDone(_))), "the count isn't made");
    assert!(ev.iter().any(|e| matches!(e, Event::Banner { title, sub, .. } if title == "The bosses are down" && sub.contains("1 more foe"))), "{ev:?}");
    // The grunt down, quickly: won in time, three levels up.
    let g = grunt(&s);
    s.hurt_mob(g, id, 10_000_000);
    let ev = run(&mut s, 0.2);
    assert_eq!(s.wager_forces().map(|f| f.0), Some(1));
    let done = ev.iter().find_map(|e| if let Event::WagerDone(r) = e { Some(r.clone()) } else { None }).expect("won");
    assert_eq!(s.wager_progress(), vec![(boss_name, true)], "down");
    assert!(done.timed && done.level == 3 && done.deaths == 1 && done.party == vec![("Ayla".to_string(), s.players[&id].class.clone())], "{done:?}");
    assert_eq!(s.wager_unlocked("deadmines"), 6, "three up for a quick one");
    assert!(ev.iter().any(|e| matches!(e, Event::ToCamp { counts, .. } if counts.iter().any(|(k, n)| k == "wager:deadmines" && *n == 5))), "the camp told");
    let plunder = s.content.item("plunder").unwrap();
    assert!(s.players[&id].bag.get(&plunder).copied().unwrap_or(0) >= 15, "paid");
    // Everyone gone: the dungeon as it was, the shortcut open again, the leash its own.
    s.leave(id);
    run(&mut s, 0.2);
    assert!(s.wager().is_none());
    assert!(s.door_is_open("short"), "given back");
    assert_eq!(s.leash_of(def), s.content.mobs[def].leash);
}

#[test]
fn a_wager_above_what_the_camp_has_won_is_refused() {
    let mut c = content();
    c.dungeon.as_mut().unwrap().book = [0.0, 0.0];
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    s.debug = true;
    let id = s.join("ayla", "Ayla", None);
    let err = s.travel_wager(id, "deadmines", 2).unwrap_err();
    assert!(err.contains("up to +1"), "{err}");
    assert!(s.travel_wager(id, "nowhere", 1).is_err());
}

#[test]
fn a_ground_look_is_only_laid_for_an_area_as_big_as_it_looks() {
    let mut s = below();
    // Under the threshold: the drawn circle, at its true size.
    s.mark_ground([0.0, 0.0, 0.0], crate::sim::GROUND_LOOK_MIN - 0.1, 1.0, 22191);
    // At it or over: the spell's own look.
    s.mark_ground([5.0, 0.0, 0.0], crate::sim::GROUND_LOOK_MIN, 1.0, 22191);
    let marks: Vec<(f32, u32)> = s.events.iter().filter_map(|e| if let Event::Mark { radius, ground, .. } = e { Some((*radius, *ground)) } else { None }).collect();
    assert!(marks.iter().any(|(r, g)| *r < crate::sim::GROUND_LOOK_MIN && *g == 0), "{marks:?}");
    assert!(marks.iter().any(|(r, g)| *r >= crate::sim::GROUND_LOOK_MIN && *g == 22191), "{marks:?}");
}

#[test]
fn every_dungeon_has_a_count_of_foes_for_its_wager() {
    use crate::sim::wager::{wager_forces_needed, wager_forces_total, WAGER_FORCES};
    let c = Content::builtin();
    for d in c.dungeons() {
        let (total, needed) = (wager_forces_total(d, &c), wager_forces_needed(d, &c));
        assert!(total >= 20, "{}: {total} foes to count", d.key);
        assert!(needed < total && needed as f32 >= total as f32 * WAGER_FORCES, "{}: {needed} of {total}", d.key);
    }
}

#[test]
fn a_wagers_clock_waits_till_everyone_is_ready_then_counts_down() {
    use crate::sim::wager::WAGER_COUNTDOWN;
    let mut s = below();
    s.start_wager(1);
    let a = s.join("ayla", "Ayla", None);
    let b = s.join("bren", "Bren", None);
    run(&mut s, 1.0);
    assert_eq!(s.wager_readiness(), Some((0, 2, None)));
    assert!(s.wager_ready(a).is_ok());
    assert!(s.wager_ready(a).is_err(), "once");
    assert_eq!(s.wager_readiness(), Some((1, 2, None)), "waiting on Bren");
    run(&mut s, 2.0);
    assert!(s.wager().unwrap().started.is_none(), "no clock without everyone");
    assert!(s.wager_ready(b).is_ok());
    assert!(s.wager_readiness().unwrap().2.is_some(), "the count runs");
    // Someone arriving mid-count stops it till they're ready too.
    let c = s.join("cael", "Cael", None);
    assert_eq!(s.wager_readiness(), Some((2, 3, None)), "stopped for Cael");
    assert!(s.wager_ready(c).is_ok());
    let ev = run(&mut s, WAGER_COUNTDOWN as f32 + 0.3);
    let called: Vec<String> = ev.iter().filter_map(|e| if let Event::Banner { title, .. } = e { Some(title.clone()) } else { None }).collect();
    assert!(["3", "2", "1"].iter().all(|n| called.iter().any(|t| t == n)) && called.iter().any(|t| t.contains("clock is running")), "{called:?}");
    assert!(s.wager().unwrap().started.is_some());
    assert!(s.wager_readiness().is_none(), "past it");
    assert!(s.wager_ready(a).is_err(), "too late");
}

#[test]
fn stepping_off_from_the_lamp_before_everyone_is_ready_starts_the_clock_at_once() {
    let mut s = below();
    s.start_wager(1);
    let a = s.join("ayla", "Ayla", None);
    let _b = s.join("bren", "Bren", None);
    run(&mut s, 1.0);
    s.move_player(a, [20.0, 0.0, 0.0], 0.0, Moving::default());
    let ev = run(&mut s, 0.2);
    assert!(ev.iter().any(|e| matches!(e, Event::Banner { title, sub, .. } if title.contains("clock is running") && sub.contains("Ayla stepped off"))), "{ev:?}");
    assert!(s.wager().unwrap().started.is_some(), "no head start for free");
}

#[test]
fn every_piece_the_dungeons_foes_carry_has_honed_copies_better_by_their_tier() {
    use crate::content::{hone_tier, HONE_TIERS};
    let c = Content::builtin();
    assert_eq!((hone_tier(3), hone_tier(4), hone_tier(8), hone_tier(11), hone_tier(12), hone_tier(20)), (0, 1, 2, 2, 3, 3));
    let honed: Vec<&crate::content::ItemDef> = c.items.iter().filter(|i| i.hone > 0).collect();
    assert!(honed.len() >= 60 * HONE_TIERS.len(), "{} honed", honed.len());
    for h in &honed {
        let base = &c.items[c.item(&h.honed_from).unwrap()];
        let by = HONE_TIERS[h.hone as usize - 1].2;
        assert_eq!(c.honed(c.item(&base.key).unwrap(), h.hone), c.item(&h.key).unwrap());
        assert!(h.name.starts_with(&base.name) && h.display == base.display && h.icon == base.icon, "{}", h.key);
        if base.reduce > 0.0 {
            assert!(h.reduce > base.reduce, "{}: armour better", h.key);
        }
        for (p, q) in base.perks.iter().zip(&h.perks) {
            assert!((q.amount - p.amount * by).abs() < 1e-4, "{}: perk", h.key);
        }
        if let (Some(w), Some(hw)) = (c.weapon(&base.key), c.weapon(&h.key)) {
            assert!(hw.damage[1] > w.damage[1] && hw.two_handed == w.two_handed, "{}: damage", h.key);
        }
    }
}

#[test]
fn a_wager_won_at_eight_pays_keen_gear_into_the_camps_chest() {
    use crate::sim::wager::{wager_bosses, wager_forces_needed};
    let c = Content::builtin().for_dungeon_of("stockade");
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    s.start_wager(8);
    let id = s.join("ayla", "Ayla", None);
    run(&mut s, 0.2);
    let d = s.content.dungeon.clone().unwrap();
    let needed = wager_forces_needed(&d, &s.content);
    let wall = s.wall();
    let bosses = wager_bosses(&d, &s.content);
    let b = s.below.as_mut().unwrap();
    b.slain.extend(bosses);
    let w = b.wager.as_mut().unwrap();
    w.started = Some(wall);
    w.forces = needed;
    let ev = run(&mut s, 0.2);
    assert!(ev.iter().any(|e| matches!(e, Event::WagerDone(r) if r.timed && r.level == 8)), "won");
    let paid: Vec<usize> = ev.iter().filter_map(|e| if let Event::ToCamp { items, .. } = e { Some(items.keys().copied().collect::<Vec<_>>()) } else { None }).flatten().collect();
    let gear: Vec<&crate::content::ItemDef> = paid.iter().map(|i| &s.content.items[*i]).filter(|i| i.kind != crate::content::ItemKind::Resource).collect();
    assert!(!gear.is_empty() && gear.iter().all(|i| i.hone == 2), "keen: {:?}", gear.iter().map(|i| &i.key).collect::<Vec<_>>());
    let _ = id;
}

#[test]
fn a_harder_dungeons_foes_are_tougher_by_its_tier_and_the_book_lists_the_easiest_first() {
    use crate::content::{TIER_DAMAGE, TIER_HEALTH};
    let c = Content::builtin();
    for (key, tier) in [("ragefire", 1u32), ("stockade", 2), ("cathedral", 3)] {
        assert_eq!(c.dungeons().find(|d| d.key == key).unwrap().tier, tier);
        let s = Survival::new_dungeon(c.for_dungeon_of(key), Box::new(FlatLand::default()), 3, 1_000_000.0);
        let (id, m) = s.mobs.iter().find(|(_, m)| !s.content.mobs[m.def].boss && !s.content.mobs[m.def].critter).map(|(id, m)| (*id, m)).unwrap();
        let d = &s.content.mobs[m.def];
        let past = (tier - 1) as f32;
        let want = (d.health as f32 * s.content.combat.foe_health * (1.0 + TIER_HEALTH * past)).round();
        assert!((m.max_health as f32 - want).abs() <= 1.0, "{key}: {} vs {want}", m.max_health);
        let base = s.attack_now(id).damage as f32;
        let plain = d.attack.damage as f32 * (1.0 + TIER_DAMAGE * past);
        assert!((base - plain.round()).abs() <= 1.0, "{key}: {base} vs {plain}");
    }
    let s = Survival::new(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    let tiers: Vec<u32> = s.book_dungeons().iter().map(|b| b.tier).collect();
    assert!(tiers.windows(2).all(|w| w[0] <= w[1]) && tiers.first() == Some(&1) && tiers.last() == Some(&3), "{tiers:?}");
}

#[test]
fn a_sets_words_count_its_pieces_not_their_honed_copies() {
    let c = Content::builtin();
    let s = Survival::new(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    let mut seen = 0;
    for set in &s.content.sets {
        let pieces = s.content.items.iter().filter(|i| i.set == set.key && i.hone == 0).count();
        for (k, i) in s.content.items.iter().enumerate().filter(|(_, i)| i.set == set.key) {
            let words = s.gear_words(k);
            assert!(words.contains(&format!("({pieces} pieces)")), "{}: {words}", i.key);
            seen += 1;
        }
    }
    assert!(seen > 0, "some set pieces, honed copies among them");
    assert!(s.content.items.iter().any(|i| !i.set.is_empty() && i.hone > 0), "a honed set piece");
}

#[test]
fn a_wager_keeps_each_bosses_split_as_it_falls() {
    let mut s = below();
    s.start_wager(1);
    let id = s.join("ayla", "Ayla", None);
    run(&mut s, 0.2);
    s.move_player(id, [20.0, 0.0, 0.0], 0.0, Moving::default());
    run(&mut s, 0.2);
    assert_eq!(s.wager_splits(), vec![None], "the boss still up");
    run(&mut s, 2.0);
    let boss = *s.mobs.iter().find(|(_, m)| m.pack.is_some_and(|k| s.content.packs[k].key == "boss")).unwrap().0;
    s.hurt_mob(boss, id, 10_000_000);
    run(&mut s, 0.2);
    let split = s.wager_splits()[0].expect("its split");
    assert!((2.0..4.0).contains(&split), "{split}");
    // On with the count: the run's splits go with it.
    let g = *s.mobs.iter().find(|(_, m)| m.pack.is_some_and(|k| s.content.packs[k].mob == "grunt")).unwrap().0;
    run(&mut s, 1.0);
    s.hurt_mob(g, id, 10_000_000);
    let ev = run(&mut s, 0.2);
    let done = ev.iter().find_map(|e| if let Event::WagerDone(r) = e { Some(r.clone()) } else { None }).expect("won");
    assert_eq!(done.splits.len(), 1);
    assert_eq!(done.splits[0].1, (split * 1000.0).round() as u64, "the split as it fell, not the end");
    assert!(done.ms > done.splits[0].1);
}
