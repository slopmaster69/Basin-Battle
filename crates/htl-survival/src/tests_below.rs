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
fn rest_pressed_again_and_again_is_one_rest() {
    let mut s = below();
    let id = s.join("ayla", "Ayla", None);
    let lamp = obj(&s, |k| matches!(k, ObjKind::Lamp { key } if key == "first"));
    s.interact(id, Some(lamp));
    s.step(DT);
    let rests = |ev: &[Event]| {
        ev.iter()
            .filter(|e| matches!(e, Event::Announce { text } if text.contains("rested at the lamp")))
            .count()
    };
    for _ in 0..3 {
        s.rest(id).unwrap();
    }
    assert_eq!(rests(&s.step(DT)), 1, "Nov 7: thirty presses, thirty lines in chat");
    assert_eq!(rests(&run(&mut s, 1.0)), 0);
    s.rest(id).unwrap();
    assert_eq!(rests(&s.step(DT)), 0, "still the same rest");
    run(&mut s, 3.0);
    s.rest(id).unwrap();
    assert_eq!(rests(&s.step(DT)), 1, "a rest later on is a rest");
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
    // (The first tier's: the harder ones open as the camp clears the easier, the ladder.)
    let mut more: Vec<&crate::content::DungeonDef> = s.content.more_dungeons.iter().filter(|d| d.tier <= 1).collect();
    more.sort_by_key(|d| d.tier);
    let firsts: Vec<String> = more.iter().map(|d| d.lamps[0].key.clone()).collect();
    assert_eq!(pages[2..].to_vec(), firsts);
    let books = s.book_dungeons();
    assert_eq!(books[0].key, "deadmines");
    assert_eq!(books.len(), 1 + s.content.more_dungeons.len());
    assert!(books.iter().all(|b| b.open == (b.tier <= 1)));
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
    // Return from the lamp for the next pull; an abandoned driver's fight resets immediately.
    let at = s.mobs[&back].pos;
    s.move_player(id, [at[0] + 2.5, at[1], at[2]], std::f32::consts::PI, Moving::default());
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
fn a_hero_come_down_from_the_camp_starts_whole_with_their_draughts_as_they_were() {
    let mut s = below();
    let id = s.join("ayla", "Ayla", None);
    let draught = s.content.item(&s.content.dungeon.as_ref().unwrap().draught).unwrap();
    let p = s.players.get_mut(&id).unwrap();
    p.health = 7;
    p.stamina = 1.0;
    p.bag.insert(draught, 1);
    s.come_down(id);
    let p = &s.players[&id];
    assert_eq!(p.health, p.max_health, "whole");
    assert_eq!(p.stamina, s.content.player.stamina, "rested");
    assert_eq!(p.bag.get(&draught).copied(), Some(1), "the Draughts as they were");
    // One fallen (their body below) isn't stood up by it.
    s.players.get_mut(&id).unwrap().health = 0;
    s.come_down(id);
    assert_eq!(s.players[&id].health, 0);
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
    let phase = |at: f32, speed: f32, might: f32| EnrageDef { at, speed, pattern: String::new(), text: format!("at {at}"), summon: Vec::new(), might, kit: 0, look: 0, sound: 0, shove: 0.0, ..Default::default() };
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

/// A boss's words are yelled from where it stands, never told as a server line: as it turns on a
/// hero, at a phase, over a hero it fells, and as it falls.
#[test]
fn a_bosss_words_are_yelled_by_the_boss_as_it_turns_at_each_phase_over_its_kill_and_as_it_falls() {
    use crate::content::EnrageDef;
    let mut c = content();
    let w = c.mob("warlord").unwrap();
    c.mobs[w].enrage = None;
    c.mobs[w].posture = 0;
    c.mobs[w].aggro_text = "Who goes there?".into();
    c.mobs[w].slay_text = "One less.".into();
    c.mobs[w].death_text = "Impossible...".into();
    c.mobs[w].phases = vec![EnrageDef { at: 0.5, speed: 1.0, pattern: String::new(), text: "Enough!".into(), summon: Vec::new(), might: 1.0, kit: 0, look: 0, sound: 0, shove: 0.0, ..Default::default() }];
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    let a = s.join("ayla", "Ayla", None);
    let b = s.join("bram", "Bram", None);
    s.move_player(a, [95.0, 0.0, 0.0], 0.0, Moving::default());
    s.move_player(b, [96.0, 0.0, 0.0], 0.0, Moving::default());
    let boss = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "warlord").unwrap().0;
    let yelled = |s: &Survival, line: &str| s.events.iter().any(|e| matches!(e, Event::Yell { id, text } if *id == boss && text == line));
    let told = |s: &Survival, line: &str| s.events.iter().any(|e| matches!(e, Event::Announce { text } if text.contains(line)));
    s.events.clear();
    s.engage(boss, mobs::Foe::Hero(a));
    assert!(yelled(&s, "Who goes there?"), "its words as it turns on them");
    s.events.clear();
    let max = s.mobs[&boss].max_health;
    s.hurt_mob(boss, a, max / 2 + 1);
    assert!(yelled(&s, "Enough!") && !told(&s, "Enough!"), "a phase's words, yelled, not a server line");
    s.events.clear();
    // The boss's blow is the one that fells Bram (what a creature's strike records as it lands).
    let now = s.clock;
    let p = s.players.get_mut(&b).unwrap();
    p.last_hit_by = Some(boss);
    p.combat_at = now;
    let hp = p.health;
    s.hurt_player_by(b, hp + 10_000, boss, "a blow");
    assert!(yelled(&s, "One less."), "over the hero it felled: {:?}", s.events);
    s.events.clear();
    let left = s.mobs[&boss].health;
    s.hurt_mob(boss, a, left);
    assert!(yelled(&s, "Impossible..."), "as it falls");
}

#[test]
fn a_bosss_adds_go_when_its_fight_ends_with_its_heroes_fallen_but_not_when_it_falls() {
    use crate::content::EnrageDef;
    let mut c = content();
    let w = c.mob("warlord").unwrap();
    c.mobs[w].enrage = None;
    c.mobs[w].posture = 0;
    let summon = vec!["grunt".to_string(), "grunt".to_string()];
    c.mobs[w].phases = vec![EnrageDef { at: 0.75, speed: 1.0, pattern: String::new(), text: String::new(), summon, might: 1.0, kit: 0, look: 0, sound: 0, shove: 0.0, ..Default::default() }];
    // The boss at its first turn, its two adds called up on the hero.
    let fight = |c: Content| {
        let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
        let id = s.join("ayla", "Ayla", None);
        s.move_player(id, [95.0, 0.0, 0.0], 0.0, Moving::default());
        let boss = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "warlord").unwrap().0;
        s.engage(boss, mobs::Foe::Hero(id));
        let max = s.mobs[&boss].max_health;
        s.hurt_mob(boss, id, max - (0.7 * max as f32) as u32);
        let adds: Vec<Id> = s.mobs.iter().filter(|(_, m)| m.summoned_by == Some(boss)).map(|(a, _)| *a).collect();
        assert_eq!(adds.len(), 2, "two called up");
        (s, id, boss, adds)
    };
    let (mut s, id, boss, adds) = fight(c.clone());
    let others: Vec<Id> = s.mobs.keys().copied().filter(|m| *m != boss && !adds.contains(m)).collect();
    assert!(!others.is_empty(), "the grunt on the way in");
    // The hero falls: the fight's over, and its adds go at once, not home to where the boss stood.
    s.hurt_player(id, 10_000);
    let ev = run(&mut s, 1.0);
    for a in &adds {
        assert!(!s.mobs.contains_key(a), "an add gone");
        assert!(ev.iter().any(|e| matches!(e, Event::MobGone { id } if id == a)), "and gone from the client");
    }
    for o in &others {
        assert!(s.mobs.contains_key(o), "the dungeon's own stay");
    }
    // The boss itself goes home and stands whole, its turn to come again.
    run(&mut s, 30.0);
    let b = &s.mobs[&boss];
    assert!(matches!(b.state, MobState::Idle { .. }), "{:?}", b.state);
    assert_eq!((b.health, b.phase), (b.max_health, 0));
    assert!(!s.mobs.values().any(|m| m.summoned_by == Some(boss)), "none left");

    // The boss felled with its adds still up: they fight on.
    let (mut s, id, boss, adds) = fight(c);
    s.hurt_mob(boss, id, 1_000_000);
    run(&mut s, 1.0);
    for a in &adds {
        let m = &s.mobs[a];
        assert!(m.health > 0 && m.state.target() == Some(id), "{:?}", m.state);
    }
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
    assert!(ev.iter().any(|e| matches!(e, Event::Yell { id, text } if *id == sneed && text == "Not yet!")), "yelled by the form that rose");
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
        // Every boss turns at least once, by health, workforce or a second form.
        for p in &d.packs {
            let m = &s.content.mobs[s.content.mob(&p.mob).unwrap()];
            if m.boss {
                assert!(!m.phases.is_empty() || m.perch_crew.is_some() || !m.then.is_empty(), "{key}: {} has no encounter stages", m.key);
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
    assert_eq!(count(&s, "wc_fang_serpent"), 1, "the dream raptor tears out");
    let raptor = *s.mobs.iter().find(|(_, m)| m.health > 0 && s.content.mobs[m.def].key == "wc_fang_serpent").unwrap().0;
    s.hurt_mob(raptor, id, 1_000_000);
    run(&mut s, 5.0);
    let lamp = obj(&s, |k| matches!(k, ObjKind::Lamp { key } if key == "first"));
    s.move_player(id, [0.0, 0.0, 0.0], 0.0, Moving::default());
    s.interact(id, Some(lamp));
    run(&mut s, 5.0);
    s.rest(id).unwrap();
    assert_eq!((count(&s, "wc_druid"), count(&s, "wc_fang_serpent")), (1, 0), "the druid is back, once; no raptor of its own");
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
    // Fresh, and tougher: the shortcut shut, the grunt two levels' worth stronger (the week's
    // terms change the fight, not its numbers).
    assert!(!s.door_is_open("short"), "shut for the wager");
    // What it has to beat, for the tracker: the boss, by name, still up.
    let boss_name = s.content.mobs.iter().find(|m| m.key == s.content.dungeon.as_ref().unwrap().packs.iter().find(|p| p.key == "boss").unwrap().mob).unwrap().name.clone();
    assert_eq!(s.wager_progress(), vec![(boss_name.clone(), false)]);
    let tough = s.mobs[&grunt(&s)].max_health as f32;
    let want = plain * (1.0 + WAGER_HEALTH).powi(2);
    assert!((tough - want).abs() <= want * 0.02 + 1.0, "{tough} vs {want}");
    let g = grunt(&s);
    let hit = s.attack_now(g).damage as f32;
    let def = s.mobs[&g].def;
    let base = s.content.mobs[def].attack.damage as f32;
    assert!((hit - base * (1.0 + WAGER_DAMAGE).powi(2)).abs() <= 1.0, "{hit} vs {base}");
    // A hero comes down: the slip, and the clock waits till they leave the lamp.
    let id = s.join("ayla", "Ayla", None);
    let ev = run(&mut s, 1.0);
    assert!(ev.iter().any(|e| matches!(e, Event::Banner { title, .. } if title == "Bookmaker's Wager +3")), "{ev:?}");
    assert!(s.wager().unwrap().started.is_none());
    s.wager_fall();
    assert_eq!(s.wager().unwrap().deaths, 0, "a fall before the clock costs nothing");
    s.move_player(id, [20.0, 0.0, 0.0], 0.0, Moving::default());
    let ev = run(&mut s, 0.2);
    assert!(ev.iter().any(|e| matches!(e, Event::Banner { title, .. } if title.contains("Clock Running"))));
    s.wager_fall();
    assert_eq!(s.wager().unwrap().deaths, 1);
    assert!(s.wager_elapsed() >= 10.0, "a fall's ten seconds");
    // Foes chase farther during a wager.
    use crate::sim::wager::WAGER_LEASH;
    assert_eq!(s.leash_of(def), s.content.mobs[def].leash * WAGER_LEASH.max(s.content.ai.leash_scale));
    // Its boss down first: not won till enough of its foes are down too.
    assert_eq!(s.wager_forces(), Some((0, 1, 1)), "one grunt standing, one needed");
    let boss = *s.mobs.iter().find(|(_, m)| m.pack.is_some_and(|k| s.content.packs[k].key == "boss")).unwrap().0;
    s.hurt_mob(boss, id, 10_000_000);
    let ev = run(&mut s, 0.2);
    assert!(!ev.iter().any(|e| matches!(e, Event::WagerDone(_))), "the count isn't made");
    assert!(ev.iter().any(|e| matches!(e, Event::Banner { title, sub, .. } if title == "Bosses Down" && sub.contains("1 more enemy"))), "{ev:?}");
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
    assert_eq!(s.leash_of(def), s.content.mobs[def].leash * s.content.ai.leash_scale);
}

#[test]
fn a_wager_above_what_the_camp_has_won_is_refused() {
    let mut c = content();
    c.dungeon.as_mut().unwrap().book = [0.0, 0.0];
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    s.debug = true;
    let id = s.join("ayla", "Ayla", None);
    let err = s.travel_wager(id, "deadmines", 2).unwrap_err();
    assert!(err.contains("highest open is +1"), "{err}");
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
fn below_a_foes_currency_goes_to_every_hero_in_the_fight() {
    let c = Content::builtin().for_dungeon_of("deadmines");
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    let a = s.join("ayla", "Ayla", None);
    let b = s.join("bren", "Bren", None);
    let far = s.join("cass", "Cass", None);
    run(&mut s, 0.2);
    let pirate = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "dm_pirate").unwrap().0;
    let at = s.mobs[&pirate].pos;
    s.players.get_mut(&a).unwrap().pos = [at[0] + 2.0, at[1], at[2]];
    s.players.get_mut(&b).unwrap().pos = [at[0] - 6.0, at[1], at[2]];
    s.players.get_mut(&far).unwrap().pos = [at[0] + 200.0, at[1], at[2]];
    run(&mut s, 0.05);
    let plunder = s.content.item("plunder").unwrap();
    let have = |s: &Survival, h: Id| s.players[&h].bag.get(&plunder).copied().unwrap_or(0);
    let (a0, b0, f0) = (have(&s, a), have(&s, b), have(&s, far));
    s.hurt_mob(pirate, a, 1_000_000);
    run(&mut s, 0.05);
    assert!(have(&s, a) > a0, "the killer's");
    assert!(have(&s, b) > b0, "and the hero beside them, their own share");
    assert_eq!(have(&s, far), f0, "not one far off, out of the fight");
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
fn the_weeks_best_wager_pays_the_bookmakers_chest_when_the_week_turns() {
    use crate::sim::wager::week_of;
    let c = Content::builtin();
    let mut camp = Survival::new(c, Box::new(FlatLand::default()), 1, 1_800_000_000.0);
    let now = week_of(camp.wall());
    // Last week: +5 in the Stockade and +3 in the Deadmines; this week, +2 in Ragefire.
    camp.camp.counts.insert(format!("wagerweek:{}:stockade:5", now - 1), 1);
    camp.camp.counts.insert(format!("wagerweek:{}:deadmines:3", now - 1), 1);
    camp.camp.counts.insert(format!("wagerweek:{now}:ragefire:2"), 1);
    assert_eq!(camp.week_best_here(), Some((2, "ragefire".to_string())));
    let before: u32 = camp.storage.values().sum();
    let ev = camp.step(DT);
    let after: u32 = camp.storage.values().sum();
    assert_eq!(after - before, 2, "the best, +5: two pieces (one, and one more at +4)");
    assert!(camp.storage.keys().any(|i| camp.content.items[*i].hone == 1), "honed, as a +5 wager pays");
    for i in camp.storage.keys().filter(|i| camp.content.items[**i].hone > 0) {
        assert!(camp.ever("found", camp.content.base_key(*i)), "found, so the rack keeps it once it's out of the chest");
    }
    assert!(ev.iter().any(|e| matches!(e, Event::Banner { title, sub, .. } if title == "The Bookmaker's Chest" && sub.contains("+5") && sub.contains("Stockade"))));
    // Once a week; this week's waits for it to turn. Last week's wins are gone, its mark kept.
    camp.step(DT);
    assert_eq!(camp.storage.values().sum::<u32>(), after);
    assert!(!camp.camp.counts.keys().any(|k| k.starts_with(&format!("wagerweek:{}:", now - 1))));
    assert_eq!(camp.camp.counts.get(&format!("weekchest:{}", now - 1)), Some(&5));
    // A win for that week arriving late: not paid again, and let go.
    camp.camp.counts.insert(format!("wagerweek:{}:stockade:9", now - 1), 1);
    camp.step(DT);
    assert_eq!(camp.storage.values().sum::<u32>(), after);
    assert!(!camp.camp.counts.contains_key(&format!("wagerweek:{}:stockade:9", now - 1)));
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
    assert!(["3", "2", "1"].iter().all(|n| called.iter().any(|t| t == n)) && called.iter().any(|t| t.contains("Clock Running")), "{called:?}");
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
    assert!(ev.iter().any(|e| matches!(e, Event::Banner { title, sub, .. } if title.contains("Clock Running") && sub.contains("Ayla left the lamp"))), "{ev:?}");
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

#[test]
fn the_wagers_terms_turn_weekly_and_a_run_keeps_the_ones_it_was_taken_with() {
    use crate::sim::wager::{terms_at, week_of, week_terms, ALL_TERMS, FRENZY_MIGHT, ROTATION};
    // Weeks start on Monday: 1970-01-05 was one.
    assert_eq!((week_of(4.0 * 86_400.0 - 1.0), week_of(4.0 * 86_400.0)), (0, 1));
    // Four weeks round, every term in them.
    for t in &ALL_TERMS {
        assert!(ROTATION.iter().any(|w| w.contains(&t.key)), "{} is never on", t.key);
    }
    let day = 86_400.0;
    let monday = 4.0 * day;
    let names = |wall: f64| week_terms(wall).iter().map(|t| t.key).collect::<Vec<_>>();
    assert_eq!(names(monday), ROTATION[1].to_vec());
    assert_eq!(names(monday + 7.0 * day), ROTATION[2].to_vec());
    assert_eq!(terms_at(3, monday).len(), 1);
    assert_eq!(terms_at(7, monday).len(), 3);
    // A +7 taken in week 1: Quickening, Frenzied, Eruptions.
    let mut s = Survival::new_dungeon(content(), Box::new(FlatLand::default()), 3, monday + day);
    s.start_wager(7);
    assert!(s.wager_has("quickening") && s.wager_has("frenzied") && s.wager_has("eruptions") && !s.wager_has("calling"));
    let keep = s.wager().unwrap().terms.clone();
    // A foe low on health hits a third harder.
    let g = *s.mobs.iter().find(|(_, m)| m.pack.is_some_and(|k| s.content.packs[k].mob == "grunt")).unwrap().0;
    let full = s.attack_now(g).damage as f32;
    let m = s.mobs.get_mut(&g).unwrap();
    m.health = m.max_health / 4;
    let low = s.attack_now(g).damage as f32;
    assert!((low - (full * FRENZY_MIGHT).round()).abs() <= 1.0, "{full} -> {low}");
    // In a fight, the ground under a hero erupts: marked first, then it burns.
    let id = s.join("ayla", "Ayla", None);
    s.move_player(id, [28.0, 0.0, 0.0], 0.0, Moving::default());
    let ev = run(&mut s, 1.0);
    assert!(s.wager().unwrap().started.is_some(), "stepped off: the clock runs");
    let w = s.wager().unwrap();
    let e = w.embers.first().expect("an eruption under the hero");
    assert!(dist2d(e.0, s.players[&id].pos) < 1.0 && e.2 - s.clock > 0.0 && e.2 - s.clock <= 1.5, "marked first, burning {:.1} s on", e.2 - s.clock);
    assert!(w.next_eruption > s.clock + 10.0, "not again for a while");
    // The week turns: the run keeps its terms.
    s.wall_start += 7.0 * day;
    run(&mut s, 0.1);
    assert_eq!(s.wager().unwrap().terms, keep);
}

#[test]
fn a_hero_who_never_readies_holds_the_count_only_so_long() {
    use crate::sim::wager::WAGER_READY_WAIT;
    let mut s = below();
    s.start_wager(1);
    let a = s.join("ayla", "Ayla", None);
    let _away = s.join("bren", "Bren", None);
    run(&mut s, 0.5);
    assert!(s.wager_ready(a).is_ok());
    run(&mut s, 5.0);
    assert!(s.wager_readiness().unwrap().2.is_none(), "waiting on Bren a while");
    s.wall_start += WAGER_READY_WAIT;
    run(&mut s, 0.2);
    assert!(s.wager_readiness().unwrap().2.is_some(), "then the count starts anyway");
}

#[test]
fn a_harder_dungeons_won_wager_pays_more_plunder_by_its_tier() {
    use crate::content::TIER_PLUNDER;
    use crate::sim::wager::{wager_bosses, wager_forces_needed};
    let paid = |key: &str| -> u32 {
        let c = Content::builtin().for_dungeon_of(key);
        let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
        s.start_wager(1);
        let id = s.join("ayla", "Ayla", None);
        run(&mut s, 0.2);
        let d = s.content.dungeon.clone().unwrap();
        let plunder = s.content.item(&d.plunder).unwrap();
        let before = s.players[&id].bag.get(&plunder).copied().unwrap_or(0);
        let (needed, bosses, wall) = (wager_forces_needed(&d, &s.content), wager_bosses(&d, &s.content), s.wall());
        let b = s.below.as_mut().unwrap();
        b.slain.extend(bosses);
        let w = b.wager.as_mut().unwrap();
        w.started = Some(wall);
        w.forces = needed;
        run(&mut s, 0.2);
        s.players[&id].bag.get(&plunder).copied().unwrap_or(0) - before
    };
    let (first, third) = (paid("ragefire"), paid("cathedral"));
    assert_eq!(first, 9, "6 + 3 at +1");
    assert_eq!(third, (9.0 * (1.0 + 2.0 * TIER_PLUNDER)).round() as u32, "a tier-3 dungeon pays half again");
}

#[test]
fn a_first_clear_below_gifts_the_camp_turns_the_books_page_and_only_once() {
    let mut s = below();
    let id = s.join("ayla", "Ayla", None);
    s.move_player(id, [95.0, 0.0, 0.0], 0.0, Moving::default());
    let boss = *s.mobs.keys().find(|m| s.content.mobs[s.mobs[*m].def].key == "warlord").unwrap();
    s.engage(boss, mobs::Foe::Hero(id));
    s.hurt_mob(boss, id, 1_000_000);
    let ev = s.step(DT);
    // Below: every boss down, so the camp hears it's cleared (with the kill).
    let up: Vec<(String, u32)> = ev
        .iter()
        .filter_map(|e| match e {
            Event::ToCamp { counts, .. } => Some(counts.clone()),
            _ => None,
        })
        .flatten()
        .collect();
    assert!(up.iter().any(|(k, n)| k == "cleared:deadmines" && *n == 1), "{up:?}");
    // ...and the heroes below hear it's a first, there and then.
    assert!(ev.iter().any(|e| matches!(e, Event::Banner { player: None, title, sub } if title.contains("Cleared") && sub.contains("first for the camp"))), "told below");

    // At camp: a gift in the chest, a banner, and the book points onward.
    let mut c = content();
    c.dungeon.as_mut().unwrap().first_clear = BTreeMap::from([("plunder".to_string(), 3)]);
    let mut camp = Survival::new(c, Box::new(FlatLand::default()), 1, 0.0);
    let plunder = camp.content.item("plunder").unwrap();
    let before = camp.storage.get(&plunder).copied().unwrap_or(0);
    let next_before = camp.next_dungeon().map(|d| d.key.clone());
    assert_eq!(next_before.as_deref(), Some("deadmines"), "the easiest uncleared first");
    camp.from_below(&up, &BTreeMap::new());
    let ev = camp.step(DT);
    assert_eq!(camp.storage.get(&plunder).copied().unwrap_or(0), before + 3, "the first clear's gift");
    assert!(ev.iter().any(|e| matches!(e, Event::Banner { player: None, title, .. } if title.contains("Cleared"))), "a banner at camp");
    let books = camp.book_dungeons();
    let dm = books.iter().find(|b| b.key == "deadmines").unwrap();
    assert!(dm.cleared && !dm.next, "cleared, and the book has turned its page");
    assert!(books.iter().filter(|b| b.next).count() <= 1, "one next page at most");
    // A second clear: no second gift.
    camp.from_below(&up, &BTreeMap::new());
    camp.step(DT);
    assert_eq!(camp.storage.get(&plunder).copied().unwrap_or(0), before + 3, "only the first");
}

#[test]
fn the_call_brings_the_nearest_pack_at_rest_after_a_few_seconds_and_only_once() {
    use crate::sim::wager::CALL_AFTER;
    let mut c = content();
    c.dungeon.as_mut().unwrap().packs.push(pack("grunt", "", [50.0, 0.0], false));
    // Week 0 of the rotation: The Call.
    let day = 86_400.0;
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, day);
    s.start_wager(2);
    assert!(s.wager_has("calling"));
    s.below.as_mut().unwrap().wager.as_mut().unwrap().started = Some(day);
    let id = s.join("ayla", "Ayla", None);
    let p = s.players.get_mut(&id).unwrap();
    p.max_health = 1_000_000;
    p.health = 1_000_000;
    // Near enough that the caller fights within the Call's 30 yd of the resting pack, wherever it
    // stands to swing.
    s.move_player(id, [25.0, 0.0, 0.0], 0.0, Moving::default());
    let grunt_at = |s: &Survival, x: f32| *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "grunt" && (m.home[0] - x).abs() < 1.0).unwrap().0;
    let (a, b) = (grunt_at(&s, 30.0), grunt_at(&s, 50.0));
    let resting = |s: &Survival, m: Id| matches!(s.mobs[&m].state, mobs::MobState::Idle { .. });
    s.engage(a, mobs::Foe::Hero(id));
    run(&mut s, CALL_AFTER - 1.0);
    assert!(resting(&s, b), "not yet");
    let ev = run(&mut s, 1.5);
    assert!(!resting(&s, b), "called");
    let _ = ev;
    let w = s.wager().unwrap();
    assert!(w.called_packs.len() == 2, "the caller and the called, each once: {:?}", w.called_packs);
}

#[test]
fn quickening_makes_a_boss_hit_harder_the_longer_its_fight_lasts() {
    use crate::sim::wager::{QUICKEN_MIGHT, QUICKEN_STEP};
    // Week 1 of the rotation: Quickening.
    let day = 86_400.0;
    let mut s = Survival::new_dungeon(content(), Box::new(FlatLand::default()), 3, 4.0 * day + day);
    s.start_wager(2);
    assert!(s.wager_has("quickening"));
    s.below.as_mut().unwrap().wager.as_mut().unwrap().started = Some(5.0 * day);
    let id = s.join("ayla", "Ayla", None);
    let p = s.players.get_mut(&id).unwrap();
    p.max_health = 10_000_000;
    p.health = 10_000_000;
    s.move_player(id, [95.0, 0.0, 0.0], 0.0, Moving::default());
    let boss = *s.mobs.keys().find(|m| s.content.mobs[s.mobs[*m].def].key == "warlord").unwrap();
    assert_eq!(s.quickening(boss), 1.0, "at rest");
    s.engage(boss, mobs::Foe::Hero(id));
    run(&mut s, QUICKEN_STEP * 2.0 + 0.5);
    assert!((s.quickening(boss) - (1.0 + 2.0 * QUICKEN_MIGHT)).abs() < 1e-4, "two steps in: {}", s.quickening(boss));
    // A boss that goes home starts over.
    s.mobs.get_mut(&boss).unwrap().state = mobs::MobState::Return;
    run(&mut s, 0.2);
    assert_eq!(s.quickening(boss), 1.0, "home again");
}

#[test]
fn the_call_wakes_only_the_called_pack_not_what_rests_beside_it() {
    use crate::sim::wager::CALL_AFTER;
    let mut c = content();
    // The pack it calls is linked to a third that rests right beside it.
    let mut called = pack("grunt", "", [50.0, 0.0], false);
    called.link = 15.0;
    c.dungeon.as_mut().unwrap().packs.push(called);
    c.dungeon.as_mut().unwrap().packs.push(pack("grunt", "", [58.0, 0.0], false));
    let day = 86_400.0;
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, day);
    s.start_wager(2);
    s.below.as_mut().unwrap().wager.as_mut().unwrap().started = Some(day);
    let id = s.join("ayla", "Ayla", None);
    let p = s.players.get_mut(&id).unwrap();
    p.max_health = 1_000_000;
    p.health = 1_000_000;
    s.move_player(id, [20.0, 0.0, 0.0], 0.0, Moving::default());
    let grunt_at = |s: &Survival, x: f32| *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "grunt" && (m.home[0] - x).abs() < 1.0).unwrap().0;
    let (a, b, third) = (grunt_at(&s, 30.0), grunt_at(&s, 50.0), grunt_at(&s, 58.0));
    let resting = |s: &Survival, m: Id| matches!(s.mobs[&m].state, mobs::MobState::Idle { .. });
    s.engage(a, mobs::Foe::Hero(id));
    run(&mut s, CALL_AFTER + 0.5);
    assert!(!resting(&s, b), "the nearest pack comes");
    assert!(resting(&s, third), "and nothing it's linked to");
}

#[test]
fn a_pack_grows_with_the_heroes_below_and_shares_its_health_over_them() {
    let mut c = content();
    let d = c.dungeon.as_mut().unwrap();
    d.hero_foes = 0.5;
    d.hero_health = 0.6;
    let mut two = pack("grunt", "", [30.0, 10.0], false);
    two.count = 2;
    two.radius = 3.0;
    d.packs.push(two);
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    let k = s.content.packs.iter().position(|p| p.count == 2).unwrap();
    let def = s.content.mob("grunt").unwrap();
    let in_pack = |s: &Survival| s.mobs.values().filter(|m| m.pack == Some(k) && m.health > 0).count();
    let names = ["ayla", "bren", "cass"];
    let ids: Vec<Id> = names.iter().map(|n| s.join(n, n, None)).collect();
    run(&mut s, 0.2);
    // Three heroes: two more foes' worth (2 x (1 + 0.5 x 2) = 4), the pack's health (2 x 2.2)
    // over the four of them.
    assert_eq!(in_pack(&s), 4, "grown for three");
    let base = s.base_health(def) as f32;
    let each = s.mobs.values().find(|m| m.pack == Some(k)).unwrap().max_health as f32;
    assert!((each - base * 2.2 * 2.0 / 4.0).abs() <= 1.0, "{each} of {base}");
    // A lone grunt doesn't grow; it takes the whole health.
    let lone = s.mobs.values().filter(|m| m.pack.is_some_and(|p| s.content.packs[p].count == 1 && s.content.packs[p].mob == "grunt")).count();
    assert_eq!(lone, 1);
    // Two go home: the pack at rest gives back what it grew for them.
    s.leave(ids[1]);
    s.leave(ids[2]);
    run(&mut s, 0.2);
    assert_eq!(in_pack(&s), 2, "back to its own two");
    // A pack in a fight isn't changed under it.
    let b = s.join("bren", "Bren", None);
    run(&mut s, 0.2);
    assert_eq!(in_pack(&s), 3, "grown for two");
    let one = *s.mobs.iter().find(|(_, m)| m.pack == Some(k)).unwrap().0;
    s.engage(one, mobs::Foe::Hero(b));
    let _c = s.join("cass", "Cass", None);
    run(&mut s, 0.1);
    assert_eq!(in_pack(&s), 3, "left as it is mid-fight");
    // ...and its unhurt foes keep the whole share for three heroes (3 bodies of its own 2: the
    // pack's 2 x 2.2 over 3), not the share of a pack grown to four.
    let other = s.mobs.iter().find(|(id, m)| **id != one && m.pack == Some(k) && m.health == m.max_health).map(|(_, m)| m.max_health as f32).unwrap();
    assert!((other - base * 2.2 * 2.0 / 3.0).abs() <= 1.0, "{other} of {base}");
}

#[test]
fn a_wagers_count_of_foes_grows_with_the_band() {
    use crate::sim::dungeon::pack_count;
    use crate::sim::wager::{wager_forces_total, wager_forces_total_for};
    let c = Content::builtin();
    for d in c.dungeons() {
        let (one, four) = (wager_forces_total(d, &c), wager_forces_total_for(d, &c, 4));
        assert!(four as f32 >= one as f32 * 1.5, "{}: {one} alone, {four} for four", d.key);
        // Bosses and elites stay alone.
        for p in &d.packs {
            if p.elite || c.mob(&p.mob).is_some_and(|m| c.mobs[m].boss) {
                assert_eq!(pack_count(p, d, &c, 4), p.count, "{}: {}", d.key, p.mob);
            }
        }
    }
}

#[test]
fn every_dungeon_pack_is_a_foe_that_exists_and_every_dungeon_lays_out() {
    let c = Content::builtin();
    for d in c.dungeons() {
        for p in &d.packs {
            assert!(c.mob(&p.mob).is_some(), "{}: no foe {}", d.key, p.mob);
        }
        let s = Survival::new_dungeon(c.for_dungeon_of(&d.key), Box::new(FlatLand::default()), 5, 1_000_000.0);
        assert!(s.mobs.len() >= d.packs.iter().filter(|p| p.hidden <= 0.0 && p.wake.is_empty()).count() / 2, "{}: laid out", d.key);
    }
}

#[test]
fn the_way_out_says_cleared_only_with_every_boss_down_and_counts_this_dungeons_exit() {
    let mut s = below();
    let id = s.join("ayla", "Ayla", None);
    run(&mut s, 0.1);
    // The way out open (its keeper down) but another boss still up: out, not cleared.
    let mut c = content();
    c.dungeon.as_mut().unwrap().packs.push(pack("warlord", "boss2", [200.0, 0.0], false));
    let mut s2 = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    let id2 = s2.join("ayla", "Ayla", None);
    s2.below.as_mut().unwrap().slain.insert("boss".into());
    s2.move_player(id2, [120.0, 0.0, 0.0], 0.0, Moving::default());
    let ev = run(&mut s2, 0.2);
    assert!(ev.iter().any(|e| matches!(e, Event::Banner { title, .. } if title.starts_with("Out of"))), "out, not cleared: {:?}", ev.iter().filter(|e| matches!(e, Event::Banner { .. })).collect::<Vec<_>>());
    let up: Vec<String> = ev.iter().filter_map(|e| if let Event::ToCamp { counts, .. } = e { Some(counts.iter().map(|c| c.0.clone()).collect::<Vec<_>>()) } else { None }).flatten().collect();
    assert!(up.contains(&"delve:end:deadmines".to_string()) && up.contains(&"delve:end".to_string()), "{up:?}");
    // Every boss down: cleared.
    s.below.as_mut().unwrap().slain.insert("boss".into());
    s.move_player(id, [120.0, 0.0, 0.0], 0.0, Moving::default());
    let ev = run(&mut s, 0.2);
    assert!(ev.iter().any(|e| matches!(e, Event::Banner { title, .. } if title.ends_with("Cleared"))));
}

#[test]
fn the_camp_hears_where_its_chests_news_comes_from() {
    let mut camp = Survival::new(content(), Box::new(FlatLand::default()), 1, 0.0);
    let plunder = camp.content.item("plunder").unwrap();
    camp.from_below_at(Some("The Stockade"), &[], &BTreeMap::from([(plunder, 2)]));
    let ev = camp.step(DT);
    assert!(ev.iter().any(|e| matches!(e, Event::Announce { text } if text.contains("from The Stockade"))), "{:?}", ev.iter().filter(|e| matches!(e, Event::Announce { .. })).collect::<Vec<_>>());
}

#[test]
fn a_hero_leaving_a_run_gets_its_tally() {
    let mut s = below();
    let id = s.join("ayla", "Ayla", None);
    run(&mut s, 0.2);
    // Plunder picked up below, a foe felled, a fall.
    let plunder = s.content.item("plunder").unwrap();
    *s.players.get_mut(&id).unwrap().bag.entry(plunder).or_default() += 7;
    let grunt = *s.mobs.keys().find(|m| s.content.mobs[s.mobs[*m].def].key == "grunt").unwrap();
    s.hurt_mob(grunt, id, 1_000_000);
    s.run_fall(id);
    // Home by a lamp: the page, with what they did.
    let lamp = obj(&s, |k| matches!(k, ObjKind::Lamp { key } if key == "first"));
    s.move_player(id, [0.0, 0.0, 0.0], 0.0, Moving::default());
    s.interact(id, Some(lamp));
    run(&mut s, 0.1);
    s.travel(id, "camp").unwrap();
    let ev = run(&mut s, 0.05);
    let page = ev.iter().find_map(|e| if let Event::Letter { kind, place, text, .. } = e { (kind == "run").then(|| (place.clone(), text.clone())) } else { None }).expect("the run's page");
    assert_eq!(page.0, "Home by a lamp");
    assert!(page.1.contains("Enemies killed: 1") && page.1.contains("You fell once") && page.1.contains("7 "), "{}", page.1);
    assert!(page.1.contains("Still standing") || page.1.contains("No boss down yet"), "{}", page.1);
}

#[test]
fn a_second_trip_down_tallies_only_itself() {
    let mut s = below();
    let id = s.join("ayla", "Ayla", None);
    run(&mut s, 0.2);
    let sword = s.content.item("sword").unwrap();
    s.players.get_mut(&id).unwrap().bag.insert(sword, 1);
    s.below.as_mut().unwrap().log.gear.entry("ayla".into()).or_default().push(sword);
    // Home, and down again: the sword went home on the first trip.
    s.leave(id);
    let id = s.join("ayla", "Ayla", None);
    run(&mut s, 0.2);
    s.run_page(id, "Home by a lamp");
    let ev = run(&mut s, 0.05);
    let text = ev.iter().find_map(|e| if let Event::Letter { kind, text, .. } = e { (kind == "run").then(|| text.clone()) } else { None }).unwrap();
    assert!(!text.contains(&s.content.items[sword].name), "{text}");
}

#[test]
fn a_new_band_down_starts_its_own_tally_and_the_bosses_are_the_dungeons() {
    let mut s = below();
    let id = s.join("ayla", "Ayla", None);
    run(&mut s, 0.2);
    let grunt = *s.mobs.keys().find(|m| s.content.mobs[s.mobs[*m].def].key == "grunt").unwrap();
    s.hurt_mob(grunt, id, 1_000_000);
    s.run_fall(id);
    s.below.as_mut().unwrap().log.first = true;
    s.leave(id);
    run(&mut s, 120.0);
    // Back down alone, and home at once: none of the last trip's.
    let id = s.join("ayla", "Ayla", None);
    run(&mut s, 0.2);
    s.run_page(id, "Home by a lamp");
    let ev = run(&mut s, 0.05);
    let text = ev.iter().find_map(|e| if let Event::Letter { kind, text, .. } = e { (kind == "run").then(|| text.clone()) } else { None }).unwrap();
    assert!(text.contains("Enemies killed: 0") && text.contains("didn't fall") && !text.contains("A first") && !text.contains("Time below: 2"), "{text}");
    // A boss the dungeon has down (kept over a restart, as its slain are) is on the page.
    let d = s.dungeon_def().cloned().unwrap();
    let boss = crate::sim::wager::wager_bosses(&d, &s.content)[0].clone();
    s.below.as_mut().unwrap().slain.insert(boss);
    s.below.as_mut().unwrap().log.paged.clear();
    s.run_page(id, "Home by a lamp");
    let ev = run(&mut s, 0.05);
    let text = ev.iter().find_map(|e| if let Event::Letter { kind, text, .. } = e { (kind == "run").then(|| text.clone()) } else { None }).unwrap();
    assert!(text.contains("Bosses down: "), "{text}");
}

#[test]
fn a_run_page_comes_once_a_trip_and_its_wager_link_is_a_command() {
    let mut s = below();
    let id = s.join("ayla", "Ayla", None);
    run(&mut s, 0.2);
    s.run_page(id, "Wager +2, in time: 9:00");
    let ev = run(&mut s, 0.05);
    let links = ev.iter().find_map(|e| if let Event::Letter { kind, links, .. } = e { (kind == "run").then(|| links.clone()) } else { None }).unwrap();
    assert!(links.iter().all(|(cmd, _)| cmd.starts_with(".es ")), "{links:?}");
    // Home by a lamp after the won wager: no second page over the first.
    s.run_page(id, "Home by a lamp");
    let ev = run(&mut s, 0.05);
    assert!(!ev.iter().any(|e| matches!(e, Event::Letter { kind, .. } if kind == "run")));
}

#[test]
fn a_run_page_names_the_gear_carried_out_honed() {
    let c = Content::builtin().for_dungeon_of("deadmines");
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    let id = s.join("ayla", "Ayla", None);
    run(&mut s, 0.2);
    let base = (0..s.content.items.len()).find(|i| s.content.honed(*i, 1) != *i && s.content.is_dungeon_gear(&s.content.items[*i].key)).expect("a dungeon piece with a honed copy");
    let honed = s.content.honed(base, 1);
    s.below.as_mut().unwrap().log.gear.entry("ayla".into()).or_default().push(base);
    s.players.get_mut(&id).unwrap().bag.insert(honed, 1);
    s.run_page(id, "Home by a lamp");
    let ev = run(&mut s, 0.05);
    let text = ev.iter().find_map(|e| if let Event::Letter { kind, text, .. } = e { (kind == "run").then(|| text.clone()) } else { None }).unwrap();
    assert!(text.contains(&s.content.items[honed].name), "{text}");
}

#[test]
fn a_dungeons_own_world_knows_the_book_s_next_dungeon() {
    let all = Content::builtin();
    let c = all.for_dungeon_of("ragefire");
    assert_eq!(c.ladder().count(), all.dungeons().count());
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    s.camp.counts.insert("cleared:ragefire".into(), 1);
    let next = s.next_dungeon().map(|d| d.key.clone());
    assert!(next.as_deref().is_some_and(|k| k != "ragefire"), "{next:?}");
}

#[test]
fn the_quartermaster_hones_the_dungeons_own_gear_and_sells_a_flask_for_its_currency() {
    let c = Content::builtin().for_dungeon_of("deadmines");
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    let id = s.join("ayla", "Ayla", None);
    run(&mut s, 0.2);
    let cutlass = s.content.item("pirate_cutlass").unwrap();
    let plunder = s.content.item("plunder").unwrap();
    s.players.get_mut(&id).unwrap().bag.insert(cutlass, 1);
    let worth = s.run_worth();
    assert!(worth > 100, "a run's worth of plunder: {worth}");
    // Offered, a tier on, at a share of a run; too dear with none.
    let offer = s.hone_offers(id).into_iter().find(|o| o.0 == "pirate_cutlass").expect("offered");
    assert_eq!(offer.2, "Honed");
    assert!(!offer.4, "can't pay yet");
    assert!(s.hone(id, "pirate_cutlass").unwrap_err().contains("Defias Plunder"));
    assert!(s.buy(id, "whetstone").unwrap_err().contains("Defias Plunder"), "the ware's price in the dungeon's own currency");
    // Paid: the cutlass in the bags is its Honed copy, the price gone.
    s.players.get_mut(&id).unwrap().bag.insert(plunder, offer.3 + 5);
    s.hone(id, "pirate_cutlass").unwrap();
    let honed = s.content.honed(cutlass, 1);
    assert_ne!(honed, cutlass);
    assert_eq!(s.players[&id].bag.get(&honed), Some(&1));
    assert!(!s.players[&id].bag.contains_key(&cutlass));
    assert_eq!(s.players[&id].bag.get(&plunder), Some(&5));
    // Next it offers Keen, dearer; a camp's own piece is never offered.
    let next = s.hone_offers(id).into_iter().find(|o| o.0 == s.content.items[honed].key).expect("again");
    assert!(next.2 == "Keen" && next.3 > offer.3);
    let sword = s.content.item("sword").unwrap();
    s.players.get_mut(&id).unwrap().bag.insert(sword, 1);
    assert!(!s.hone_offers(id).iter().any(|o| o.0 == "sword"));
    // A flask: one more Draught for good.
    let d = s.dungeon_def().unwrap().clone();
    let price = s.trades(id).into_iter().find(|t| t.0 == d.flask).expect("a flask on offer").2;
    s.players.get_mut(&id).unwrap().bag.insert(plunder, price);
    s.buy(id, &d.flask).unwrap();
    assert_eq!(s.below.as_ref().unwrap().heroes["ayla"].flasks, 1);
    // A dungeon whose foes drop none of its currency: nothing honed, no flask, at any price.
    let mut c = Content::builtin().for_dungeon_of("deadmines");
    for m in &mut c.mobs {
        m.drops.clear();
    }
    c.dungeon.as_mut().unwrap().boxes.clear();
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    let id = s.join("ayla", "Ayla", None);
    run(&mut s, 0.2);
    let cutlass = s.content.item("pirate_cutlass").unwrap();
    s.players.get_mut(&id).unwrap().bag.insert(cutlass, 1);
    assert_eq!(s.run_worth(), 0);
    assert!(s.hone_offers(id).is_empty());
    let flask = s.dungeon_def().unwrap().flask.clone();
    assert!(!s.trades(id).iter().any(|t| t.0 == flask));
}

#[test]
fn a_harder_tier_opens_once_two_of_the_tier_below_are_cleared() {
    let c = Content::builtin();
    let mut camp = Survival::new(c, Box::new(FlatLand::default()), 1, 0.0);
    let open = |s: &Survival, k: &str| s.book_dungeons().into_iter().find(|b| b.key == k).map(|b| (b.open, b.locked)).unwrap();
    assert!(open(&camp, "deadmines").0 && open(&camp, "ragefire").0, "the first tier from the start");
    let (shut, words) = open(&camp, "stockade");
    assert!(!shut && words.contains("clear 2 more Tier I"), "{words}");
    assert!(!open(&camp, "cathedral").0);
    assert!(!camp.book_pages().iter().any(|(_, n)| n.contains("Stockade")), "no page to go down by");
    // One cleared: one more to go. Two: the second tier reads.
    camp.camp.counts.insert("cleared:deadmines".into(), 1);
    assert!(open(&camp, "stockade").1.contains("clear 1 more"));
    camp.camp.counts.insert("cleared:wailing".into(), 1);
    assert!(open(&camp, "stockade").0 && open(&camp, "razorfen").0);
    assert!(!open(&camp, "graveyard").0, "the third waits on the second");
    // A dungeon the camp has been in already stays open (an old camp isn't locked out).
    camp.camp.counts.insert("kill:gy_vishas".into(), 1);
    assert!(open(&camp, "graveyard").0);
}

#[test]
fn the_journal_walks_the_camp_through_every_dungeon_by_their_clears() {
    let c = Content::builtin();
    let keys: Vec<String> = c.dungeons().map(|d| d.key.clone()).collect();
    for k in keys.iter().filter(|k| *k != "deadmines") {
        assert!(c.goals.iter().any(|g| g.kind == "cleared" && &g.what == k), "a journal step for {k}");
    }
    // In the book's order: no step for a harder tier before every one of an easier tier's.
    let tier = |k: &str| c.dungeons().find(|d| d.key == k).unwrap().tier;
    let steps: Vec<u32> = c.goals.iter().filter(|g| g.kind == "cleared").map(|g| tier(&g.what)).collect();
    assert!(steps.windows(2).all(|w| w[0] <= w[1]), "{steps:?}");
    let camp = Survival::new(c.clone(), Box::new(FlatLand::default()), 1, 0.0);
    let g = c.goals.iter().find(|g| g.kind == "cleared" && g.what == "ragefire").unwrap();
    assert_eq!(camp.goal_progress(g), (0, 1));
}

#[test]
fn the_books_record_of_a_dungeon_lists_its_bosses_their_gear_and_the_camps_kills() {
    let c = Content::builtin();
    let mut camp = Survival::new(c, Box::new(FlatLand::default()), 1, 0.0);
    let id = camp.join("ayla", "Ayla", None);
    camp.camp.counts.insert("kill:dm_smite".into(), 2);
    camp.lore_page(id, "deadmines").unwrap();
    let ev = camp.step(DT);
    let text = ev.iter().find_map(|e| if let Event::Letter { kind, text, .. } = e { (kind == "lore").then(|| text.clone()) } else { None }).unwrap();
    assert!(text.contains("Defeat every boss"), "{text}");
    assert!(text.contains("Mr. Smite: felled 2 times") && text.contains("Smite's Mighty Hammer"), "{text}");
    assert!(text.contains("Cookie: not felled yet"), "{text}");
    // A trophy is the camp's the first time; after, it's in the chest.
    let line = |who: &str| text.lines().find(|l| l.starts_with(&format!("- {who}:"))).unwrap_or("").to_string();
    assert!(line("Mr. Smite").contains("in storage") && line("Cookie").contains("sent to storage the first time"), "{text}");
}

/// A boss whose phase has a route (`run_to`) breaks off and runs it, heeding nobody and taking up
/// no other phase on the way; at its end it waits for a hero to come near, yells, and fights there
/// (its home from then); a fight it then loses puts it back where its fight began, whole.
#[test]
fn a_boss_runs_its_route_waits_at_its_end_for_a_hero_and_goes_back_where_it_began_if_its_fight_is_lost() {
    use crate::content::EnrageDef;
    let mut c = content();
    let w = c.mob("warlord").unwrap();
    c.mobs[w].enrage = None;
    c.mobs[w].posture = 0;
    c.mobs[w].phases = vec![
        EnrageDef { at: 0.6, speed: 1.0, might: 1.0, run_to: vec![[130.0, 0.0, 0.0], [130.0, 30.0, 0.0]], wait_for: 10.0, arrive_text: "Mine now.".into(), ..Default::default() },
        EnrageDef { at: 0.3, speed: 1.0, might: 1.0, text: "Later.".into(), ..Default::default() },
    ];
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    let id = s.join("ayla", "Ayla", None);
    s.move_player(id, [95.0, 0.0, 0.0], 0.0, Moving::default());
    let boss = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "warlord").unwrap().0;
    // Where its fight begins: its home (a pack member stands a little off its pack's spot).
    let start = s.mobs[&boss].home;
    s.engage(boss, mobs::Foe::Hero(id));
    let max = s.mobs[&boss].max_health;
    s.hurt_mob(boss, id, max / 2);
    assert!(s.boss_running(boss), "off on its route at its phase");
    // Struck past its next phase on the run: that waits.
    s.hurt_mob(boss, id, max / 4);
    assert_eq!(s.mobs[&boss].phase, 1, "no second phase while it runs");
    run(&mut s, 20.0);
    let at = s.mobs[&boss].pos;
    assert!(dist2d(at, [130.0, 30.0, 0.0]) < 2.0, "at its route's end: {at:?}");
    assert!(s.boss_running(boss), "waiting there for a hero, not fighting from afar");
    // A hero comes: it yells, and fights there.
    s.move_player(id, [128.0, 26.0, 0.0], 0.0, Moving::default());
    let ev = run(&mut s, 0.3);
    assert!(!s.boss_running(boss));
    assert!(ev.iter().any(|e| matches!(e, Event::Yell { id: b, text } if *b == boss && text == "Mine now.")));
    assert!(dist2d(s.mobs[&boss].home, [130.0, 30.0, 0.0]) < 0.5, "its home from then");
    // The fight lost: its hero falls. Home, whole, and back where its fight began.
    let hp = s.players[&id].health;
    s.hurt_player_by(id, hp + 10_000, boss, "a blow");
    run(&mut s, 40.0);
    let m = &s.mobs[&boss];
    // (Resting at home, it wanders a step or two round it.)
    assert!(dist2d(m.pos, start) < 4.0 && dist2d(m.home, start) < 0.5, "put back where it began: at {:?}, home {:?}, began {start:?}", m.pos, m.home);
    assert_eq!((m.phase, m.health), (0, m.max_health), "whole again");
    assert!(!s.boss_running(boss));
}

/// A heated boss's heavy blow leaves burning ground that bites a hero standing in it, then burns
/// out; a blow that lodges holds the boss where it is, open.
#[test]
fn a_heated_bosss_heavy_blow_leaves_burning_ground_and_a_lodged_blow_holds_it() {
    use crate::content::{BurnDef, EnrageDef, PatternDef, SpecialDef};
    let mut c = content();
    let w = c.mob("warlord").unwrap();
    c.mobs[w].enrage = None;
    c.mobs[w].posture = 0;
    c.mobs[w].attack.damage = 0;
    c.mobs[w].special = None;
    c.mobs[w].phases = vec![EnrageDef { at: 0.9, speed: 1.0, might: 1.0, burn: Some(BurnDef { radius: 3.0, damage: 6, secs: 3.0, look: 0 }), ..Default::default() }];
    let lodged: SpecialDef = toml::from_str("lodge = 2.0").unwrap();
    c.mobs[w].patterns = vec![PatternDef { name: "slam".into(), weight: 0.0, min_range: 0.0, max_range: 40.0, below: 1.0, steps: vec![lodged] }];
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    let id = s.join("ayla", "Ayla", None);
    s.move_player(id, [60.0, 0.0, 0.0], 0.0, Moving::default());
    let boss = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "warlord").unwrap().0;
    s.engage(boss, mobs::Foe::Hero(id));
    let max = s.mobs[&boss].max_health;
    s.hurt_mob(boss, id, max / 5);
    let pos = s.mobs[&boss].pos;
    s.boss_blow_lands(boss, pos, 0.0, 3.0, 90.0, true);
    let fire = s.objs.iter().find(|(_, o)| matches!(o.kind, ObjKind::Fire { .. }) && dist2d(o.pos, [pos[0] + 2.1, pos[1], 0.0]) < 0.5).map(|(f, _)| *f);
    assert!(fire.is_some(), "burning where the blow landed");
    // A light blow leaves nothing.
    let fires = s.objs.values().filter(|o| matches!(o.kind, ObjKind::Fire { .. })).count();
    s.boss_blow_lands(boss, pos, 1.5, 3.0, 90.0, false);
    assert_eq!(s.objs.values().filter(|o| matches!(o.kind, ObjKind::Fire { .. })).count(), fires);
    // A hero standing in it is bitten each second.
    s.move_player(id, [pos[0] + 2.1, pos[1], 0.0], 0.0, Moving::default());
    let hp = s.players[&id].health;
    run(&mut s, 1.1);
    assert!(s.players[&id].health < hp, "bitten by the burning iron");
    run(&mut s, 3.0);
    assert!(!s.objs.contains_key(&fire.unwrap()), "burned out");
    // The lodged blow: it hauls at its weapon, open.
    let now = s.clock;
    s.mobs.get_mut(&boss).unwrap().pattern = Some((0, 0));
    s.boss_blow_lands(boss, pos, 0.0, 3.0, 90.0, false);
    assert!(s.mobs[&boss].stagger_until >= now + 1.9, "held, open, as long as the blow says");
}

/// Debris: dust over each point first, then the rocks, a heavy blow there.
#[test]
fn a_bosss_debris_comes_down_over_its_points_after_the_dust() {
    use crate::content::EnrageDef;
    let mut c = content();
    let w = c.mob("warlord").unwrap();
    c.mobs[w].enrage = None;
    c.mobs[w].posture = 0;
    c.mobs[w].phases = vec![EnrageDef { at: 0.9, speed: 1.0, might: 1.0, debris: vec![[105.0, 5.0, 0.0], [105.0, -5.0, 0.0]], ..Default::default() }];
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    let id = s.join("ayla", "Ayla", None);
    s.move_player(id, [105.0, 5.0, 0.0], 0.0, Moving::default());
    let boss = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "warlord").unwrap().0;
    s.engage(boss, mobs::Foe::Hero(id));
    s.events.clear();
    let max = s.mobs[&boss].max_health;
    s.hurt_mob(boss, id, max / 5);
    let dust = s.events.iter().filter(|e| matches!(e, Event::Missile { spell: 7272, .. })).count();
    assert_eq!(dust, 2, "dust over each point first");
    assert!(s.events.iter().any(|e| matches!(e, Event::Danger { player, .. } if *player == id)), "the hero under it is warned");
    let rocks: Vec<_> = s.missiles.iter().filter(|m| m.owner == boss && m.heavy && m.radius >= 3.0).collect();
    assert_eq!(rocks.len(), 2, "and the rocks to come");
}

/// A boss whose heroes all fall while it's on its run gives the run up and goes home whole.
#[test]
fn a_bosss_run_ends_when_its_fight_is_lost_on_the_way_and_it_goes_home_whole() {
    use crate::content::EnrageDef;
    let mut c = content();
    let w = c.mob("warlord").unwrap();
    c.mobs[w].enrage = None;
    c.mobs[w].posture = 0;
    c.mobs[w].phases = vec![EnrageDef { at: 0.6, speed: 1.0, might: 1.0, run_to: vec![[160.0, 0.0, 0.0], [160.0, 60.0, 0.0]], ..Default::default() }];
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    let id = s.join("ayla", "Ayla", None);
    s.move_player(id, [95.0, 0.0, 0.0], 0.0, Moving::default());
    let boss = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "warlord").unwrap().0;
    let start = s.mobs[&boss].home;
    s.engage(boss, mobs::Foe::Hero(id));
    let max = s.mobs[&boss].max_health;
    s.hurt_mob(boss, id, max / 2);
    run(&mut s, 2.0);
    assert!(s.boss_running(boss));
    let hp = s.players[&id].health;
    s.hurt_player_by(id, hp + 10_000, boss, "a blow");
    run(&mut s, 40.0);
    assert!(!s.boss_running(boss), "the run given up");
    let m = &s.mobs[&boss];
    assert!(dist2d(m.pos, start) < 4.0, "home again: {:?}", m.pos);
    assert_eq!(m.health, m.max_health, "whole");
}

/// A collapse: its front runs the route; a hero behind it is crushed, one close ahead is warned,
/// one far ahead is left alone; at the route's end the boss yells and it stops.
#[test]
fn a_collapse_runs_its_route_crushing_the_buried_and_warning_those_just_ahead() {
    use crate::content::{CollapseDef, EnrageDef};
    let mut c = content();
    let w = c.mob("warlord").unwrap();
    c.mobs[w].enrage = None;
    c.mobs[w].posture = 0;
    c.mobs[w].attack.damage = 0;
    c.mobs[w].patterns.clear();
    c.mobs[w].special = None;
    let route = vec![[100.0, 0.0, 0.0], [160.0, 0.0, 0.0], [160.0, 60.0, 0.0]];
    c.mobs[w].phases = vec![EnrageDef { at: 0.9, speed: 1.0, might: 1.0, collapse: Some(CollapseDef { route, speed: 10.0, lead: 10.0, crush: 0.2, hold: 0.0, end_text: "At the ship.".into(), held_text: String::new() }), ..Default::default() }];
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    let buried = s.join("ayla", "Ayla", None);
    let warned = s.join("bram", "Bram", None);
    let clear = s.join("cora", "Cora", None);
    s.move_player(buried, [102.0, 0.0, 0.0], 0.0, Moving::default());
    s.move_player(warned, [132.0, 0.0, 0.0], 0.0, Moving::default());
    s.move_player(clear, [160.0, 50.0, 0.0], 0.0, Moving::default());
    let boss = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "warlord").unwrap().0;
    // Its fight is with the hero in the band; the one far ahead is a bystander.
    s.engage(boss, mobs::Foe::Hero(warned));
    let max = s.mobs[&boss].max_health;
    s.hurt_mob(boss, warned, max / 5);
    assert_eq!(s.collapse_front(), Some((0.0, false)), "the collapse has begun");
    let hp = |s: &Survival, h: Id| s.players[&h].health;
    let (b0, c0) = (hp(&s, buried), hp(&s, clear));
    let ev = run(&mut s, 3.0);
    let front = s.collapse_front().unwrap().0;
    assert!((front - 30.0).abs() < 1.0, "the front at 10 yd/s: {front}");
    assert!(hp(&s, buried) < b0, "the buried hero crushed");
    assert!(ev.iter().any(|e| matches!(e, Event::Danger { player, .. } if *player == warned)), "the hero just ahead warned");
    assert_eq!(hp(&s, clear), c0, "the hero far ahead untouched");
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { spell: 7272, .. })), "dust and rock where it comes down");
    // The hero in the fight retreats ahead of the front, as one must.
    s.move_player(warned, [160.0, 45.0, 0.0], 0.0, Moving::default());
    let ev = run(&mut s, 10.0);
    assert_eq!(s.collapse_front(), Some((120.0, true)), "at its end");
    assert!(ev.iter().any(|e| matches!(e, Event::Yell { id, text } if *id == boss && text == "At the ship.")));
}

/// A boss can't be brought below its collapse's hold until the front reaches the end; then it can.
#[test]
fn a_boss_is_held_above_its_last_stand_until_its_collapse_reaches_the_end() {
    use crate::content::{CollapseDef, EnrageDef};
    let mut c = content();
    let w = c.mob("warlord").unwrap();
    c.mobs[w].enrage = None;
    c.mobs[w].posture = 0;
    let route = vec![[100.0, 0.0, 0.0], [140.0, 0.0, 0.0]];
    c.mobs[w].phases = vec![EnrageDef { at: 0.9, speed: 1.0, might: 1.0, collapse: Some(CollapseDef { route, speed: 20.0, lead: 0.0, crush: 0.0, hold: 0.3, end_text: String::new(), held_text: "Not here!".into() }), ..Default::default() }];
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    let id = s.join("ayla", "Ayla", None);
    s.move_player(id, [150.0, 0.0, 0.0], 0.0, Moving::default());
    let boss = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "warlord").unwrap().0;
    s.engage(boss, mobs::Foe::Hero(id));
    let max = s.mobs[&boss].max_health;
    s.hurt_mob(boss, id, max / 5);
    s.events.clear();
    s.hurt_mob(boss, id, max);
    let floor = (0.3 * max as f32).round() as u32;
    assert_eq!(s.mobs[&boss].health, floor, "held at its last stand");
    assert!(s.events.iter().any(|e| matches!(e, Event::Yell { text, .. } if text == "Not here!")));
    run(&mut s, 3.0);
    assert_eq!(s.collapse_front().map(|f| f.1), Some(true));
    let left = s.mobs[&boss].health;
    s.hurt_mob(boss, id, left);
    assert_eq!(s.mobs[&boss].health, 0, "and at the end it can fall");
}

/// A fight lost while the collapse comes down ends it, and the boss is back where it began, whole.
#[test]
fn a_collapse_ends_when_its_fight_is_lost_and_the_boss_goes_back_whole() {
    use crate::content::{CollapseDef, EnrageDef};
    let mut c = content();
    let w = c.mob("warlord").unwrap();
    c.mobs[w].enrage = None;
    c.mobs[w].posture = 0;
    let route = vec![[100.0, 0.0, 0.0], [300.0, 0.0, 0.0]];
    c.mobs[w].phases = vec![EnrageDef { at: 0.9, speed: 1.0, might: 1.0, collapse: Some(CollapseDef { route, speed: 3.0, lead: 10.0, crush: 0.2, hold: 0.25, end_text: String::new(), held_text: String::new() }), ..Default::default() }];
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    let id = s.join("ayla", "Ayla", None);
    s.move_player(id, [110.0, 0.0, 0.0], 0.0, Moving::default());
    let boss = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "warlord").unwrap().0;
    let start = s.mobs[&boss].home;
    s.engage(boss, mobs::Foe::Hero(id));
    let max = s.mobs[&boss].max_health;
    s.hurt_mob(boss, id, max / 5);
    run(&mut s, 2.0);
    let hp = s.players[&id].health;
    s.hurt_player_by(id, hp + 10_000, boss, "a blow");
    run(&mut s, 20.0);
    assert_eq!(s.collapse_front(), None, "the collapse is over");
    let m = &s.mobs[&boss];
    assert!(dist2d(m.pos, start) < 4.0 && dist2d(m.home, start) < 0.5, "back where it began: {:?}", m.pos);
    assert_eq!(m.health, m.max_health, "whole");
}

/// A dungeon's other ways out (`more_exits`) end the run as the front door does, once its boss is down.
#[test]
fn another_way_out_ends_the_run_once_the_exit_boss_is_down() {
    let mut c = content();
    c.dungeon.as_mut().unwrap().more_exits = vec![[0.0, -40.0, 0.0]];
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    let id = s.join("ayla", "Ayla", None);
    s.move_player(id, [95.0, 0.0, 0.0], 0.0, Moving::default());
    let boss = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "warlord").unwrap().0;
    s.engage(boss, mobs::Foe::Hero(id));
    s.hurt_mob(boss, id, 1_000_000);
    run(&mut s, 0.5);
    s.move_player(id, [0.0, -40.0, 0.0], 0.0, Moving::default());
    let ev = run(&mut s, 0.5);
    assert!(ev.iter().any(|e| matches!(e, Event::Travel { player, to: None } if *player == id)), "out by the other way");
}

/// A boss whose collapse is coming down doesn't chase a hero who ran off ahead of the fight: it
/// holds the line, walking the route with the front, and hurls rock down the tunnel at them. The
/// fight (and the collapse) goes on; the boss and the front set where it is, not the hero.
#[test]
fn a_boss_holds_the_collapse_line_and_hurls_rock_at_a_hero_who_runs_off_ahead() {
    use crate::content::{CollapseDef, EnrageDef};
    let mut c = content();
    let w = c.mob("warlord").unwrap();
    c.mobs[w].enrage = None;
    c.mobs[w].posture = 0;
    let route = vec![[100.0, 0.0, 0.0], [300.0, 0.0, 0.0]];
    c.mobs[w].phases = vec![EnrageDef { at: 0.9, speed: 1.0, might: 1.0, collapse: Some(CollapseDef { route, speed: 2.0, lead: 10.0, crush: 0.2, hold: 0.25, end_text: String::new(), held_text: String::new() }), ..Default::default() }];
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    let id = s.join("ayla", "Ayla", None);
    s.move_player(id, [105.0, 0.0, 0.0], 0.0, Moving::default());
    let boss = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "warlord").unwrap().0;
    s.engage(boss, mobs::Foe::Hero(id));
    let max = s.mobs[&boss].max_health;
    s.hurt_mob(boss, id, max / 5);
    // The hero runs far down the route, well past the fight.
    s.move_player(id, [220.0, 0.0, 0.0], 0.0, Moving::default());
    let ev = run(&mut s, 8.0);
    let (front, done) = s.collapse_front().unwrap();
    assert!(!done, "the collapse still coming down");
    assert!(matches!(s.mobs[&boss].state.foe(), Some(mobs::Foe::Hero(h)) if h == id), "still in the fight with the hero");
    let x = s.mobs[&boss].pos[0];
    assert!(x > 100.0 + front - 2.0 && x < 100.0 + front + 10.0 + 6.0, "on the line, with the front ({front}): {x}");
    let hurled = ev.iter().any(|e| matches!(e, Event::Danger { player, .. } if *player == id));
    assert!(hurled, "rock hurled at the hero who ran");
    // The hero comes back to the fight: the boss fights them as ever.
    let at = s.mobs[&boss].pos;
    s.move_player(id, [at[0] + 3.0, 0.0, 0.0], 0.0, Moving::default());
    run(&mut s, 0.5);
    assert!(!s.step_boss_holdline(boss), "in reach of the fight: no holding the line");
}

/// A coarse chord crosses rock, while the real walking route bends around it. The hero on
/// another floor cannot lure the boss onto that floor, and every actual stride stays grounded.
#[test]
fn the_collapse_and_its_boss_follow_native_bends_without_resetting() {
    use crate::content::{CollapseDef, EnrageDef};
    use crate::land::Land;
    struct Bend;
    impl Land for Bend {
        fn height(&self, x: f32, y: f32) -> f32 {
            if x > 110.0 && x < 130.0 && y < 10.0 { 300.0 } else { 0.0 }
        }
        fn plans(&self) -> bool { true }
        fn walk_clear(&self, a: Vec3, b: Vec3) -> bool {
            let steps = (dist2d(a, b) / 0.1).ceil().max(1.0) as usize;
            (0..=steps).all(|i| {
                let t = i as f32 / steps as f32;
                self.footing([a[0] + (b[0]-a[0])*t, a[1] + (b[1]-a[1])*t, a[2] + (b[2]-a[2])*t]).is_some()
            })
        }
        fn path(&self, a: Vec3, b: Vec3) -> Option<Vec<Vec3>> {
            if a[0] <= 110.0 && b[0] >= 130.0 && b[2] == 0.0 {
                Some(vec![[110.0, 12.0, 0.0], [130.0, 12.0, 0.0], b])
            } else { None }
        }
    }
    let mut c = content();
    let w = c.mob("warlord").unwrap();
    c.mobs[w].enrage = None;
    c.mobs[w].posture = 0;
    c.mobs[w].phases = vec![EnrageDef { at: 0.9, speed: 1.0, might: 1.0, collapse: Some(CollapseDef {
        route: vec![[100.0, 0.0, 0.0], [140.0, 0.0, 0.0]], speed: 2.0, lead: 4.0, crush: 0.2, hold: 0.26,
        end_text: String::new(), held_text: String::new(),
    }), ..Default::default() }];
    let mut s = Survival::new_dungeon(c, Box::new(Bend), 3, 1_000_000.0);
    let id = s.join("ayla", "Ayla", None);
    s.move_player(id, [140.0, 0.0, 50.0], 0.0, Moving::default());
    let boss = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "warlord").unwrap().0;
    s.engage(boss, mobs::Foe::Hero(id));
    let max = s.mobs[&boss].max_health;
    s.hurt_mob(boss, id, max / 5);
    let mut previous = s.mobs[&boss].pos;
    let mut max_y = 0.0_f32;
    for _ in 0..500 {
        let ev = s.step(DT);
        let at = s.mobs[&boss].pos;
        assert!(s.land().footing(at).is_some(), "grounded at {at:?}");
        assert!(dist2d(previous, at) <= s.content.mobs[w].run_speed * DT + 0.01, "no teleport: {previous:?} -> {at:?}");
        assert!(!ev.iter().any(|e| matches!(e, Event::Log(t) if t.contains("put back"))));
        previous = at;
        max_y = max_y.max(at[1]);
        s.players.get_mut(&id).unwrap().health = s.players[&id].max_health;
    }
    assert!(max_y >= 11.9, "walked the bend: {max_y}");
    assert!(s.mobs[&boss].pos[0] >= 135.0, "past both corners: {:?}", s.mobs[&boss].pos);
    assert!(s.collapse_front().unwrap().0 > 40.0, "native distance exceeds the authored chord");
}

#[test]
fn a_collapse_passes_a_repeated_hairpin_point_in_order() {
    use crate::content::{CollapseDef, EnrageDef};
    let mut c = content();
    let w = c.mob("warlord").unwrap();
    c.mobs[w].enrage = None;
    c.mobs[w].posture = 0;
    c.mobs[w].phases = vec![EnrageDef { at: 0.9, speed: 1.0, might: 1.0, collapse: Some(CollapseDef {
        route: vec![[100.0, 0.0, 0.0], [110.0, 0.0, 0.0], [110.2, 0.0, 0.0], [110.0, 0.0, 0.0], [110.0, 10.0, 0.0], [140.0, 10.0, 0.0]],
        speed: 2.0, lead: 4.0, crush: 0.0, hold: 0.26, end_text: String::new(), held_text: String::new(),
    }), ..Default::default() }];
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    let id = s.join("ayla", "Ayla", None);
    s.move_player(id, [140.0, 10.0, 50.0], 0.0, Moving::default());
    let boss = *s.mobs.iter().find(|(_, m)| m.def == w).unwrap().0;
    s.engage(boss, mobs::Foe::Hero(id));
    let max = s.mobs[&boss].max_health;
    s.hurt_mob(boss, id, max / 5);
    for _ in 0..500 {
        let p = s.players.get_mut(&id).unwrap();
        p.health = p.max_health;
        let ev = s.step(DT);
        assert!(!ev.iter().any(|e| matches!(e, Event::Log(t) if t.contains("put back"))));
    }
    assert!(s.mobs[&boss].pos[0] > 136.0, "passed the repeated point instead of cycling there: {:?}", s.mobs[&boss].pos);
}

#[test]
fn collapse_route_samples_the_floor_between_corners_and_rejects_wet_targets() {
    use crate::land::{Land, walking_route};
    struct Hill;
    impl Land for Hill {
        fn height(&self, x: f32, _y: f32) -> f32 { (x / 50.0 * std::f32::consts::PI).sin() * 4.0 }
        fn walk_clear(&self, _a: Vec3, _b: Vec3) -> bool { true }
        fn wet_near(&self, _x: f32, y: f32, _z: f32) -> bool { y > 5.0 }
    }
    let route = walking_route(&Hill, &[[0.0, 0.0, 0.0], [50.0, 0.0, 0.0]]).unwrap();
    assert!(route.iter().any(|p| p[2] > 3.9));
    assert!(route.iter().all(|p| Hill.footing(*p).is_some()));
    assert!(walking_route(&Hill, &[[0.0, 0.0, 0.0], [50.0, 6.0, 0.0]]).is_none());
}
