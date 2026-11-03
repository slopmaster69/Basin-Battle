//! Health between pulls and in them (#118): the out-of-combat regeneration everywhere, food as a
//! quick bite whose healing comes after it, and eating and drinking that look like it.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::*;

const DT: f32 = 0.05;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];
/// EmoteEat in the 1.12 client's AnimationData: its /eat (and its /drink).
const EMOTE_EAT: u16 = 61;

fn world() -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 1, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

fn hero(s: &mut Survival, name: &str, pos: Vec3) -> Id {
    let id = s.join(&name.to_lowercase(), name, None);
    s.move_player(id, pos, 0.0, Moving::default());
    let _ = s.step(0.0);
    id
}

/// The Deadmines with no foes and no hazards: a hero at the first lamp.
fn below() -> (Survival, Id) {
    let mut c = Content::dev();
    c.pvp = Default::default();
    c.dungeon.as_mut().unwrap().hazards.clear();
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    let p = s.join("ayla", "Ayla", None);
    let _ = s.step(0.0);
    (s, p)
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        out.extend(s.step(DT));
    }
    out
}

fn give(s: &mut Survival, p: Id, key: &str, n: u32) {
    let i = s.content.item(key).unwrap();
    *s.players.get_mut(&p).unwrap().bag.entry(i).or_default() += n;
}

fn count(s: &Survival, p: Id, key: &str) -> u32 {
    let i = s.content.item(key).unwrap();
    s.players[&p].bag.get(&i).copied().unwrap_or(0)
}

fn health(s: &Survival, p: Id) -> u32 {
    s.players[&p].health
}

/// Hurt and in the fight: down to `share` of max health, a blow taken just now.
fn wounded(s: &mut Survival, p: Id, share: f32) -> u32 {
    let max = s.players[&p].max_health;
    s.players.get_mut(&p).unwrap().health = (max as f32 * share) as u32 + 1;
    s.players.get_mut(&p).unwrap().heal_bank = 0.0;
    s.hurt_player(p, 1);
    assert!(s.effect_of(&s.players[&p], "regen") == 0.0, "no regeneration from gear here");
    health(s, p)
}

/// The clips played on a hero, and the spells cast by them.
fn clips(ev: &[Event], p: Id) -> Vec<(u16, f32)> {
    ev.iter().filter_map(|e| match e {
        Event::Clip { id, clip, secs } if *id == p => Some((*clip, *secs)),
        _ => None,
    }).collect()
}

fn casts(ev: &[Event], p: Id) -> Vec<u32> {
    ev.iter().filter_map(|e| match e {
        Event::Cast { caster, spell, .. } if *caster == p => Some(*spell),
        _ => None,
    }).collect()
}

#[test]
fn the_trial_numbers_are_two_percent_a_second_from_five_seconds() {
    let c = Content::builtin();
    assert_eq!((c.player.field_regen, c.player.field_regen_after), (0.02, 5.0));
    assert_eq!(c.player.combat_secs, 5.0, "out of regeneration's wait is out of combat");
}

#[test]
fn health_comes_back_two_percent_a_second_once_five_seconds_pass_without_a_blow() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN);
    let max = s.players[&p].max_health as f32;
    let h = wounded(&mut s, p, 0.3);
    // Not before 5 sec.
    run(&mut s, 4.5);
    assert_eq!(health(&s, p), h, "nothing back in the first 5 sec");
    // Then 2% a second: about 1 sec of it by 6 sec.
    run(&mut s, 1.5);
    let one = (health(&s, p) - h) as f32;
    assert!(one >= max * 0.02 * 0.75 && one <= max * 0.02 * 1.25 + 1.0, "about 2% in the first second: {one} of {max}");
    // And 5 sec more: 10% more.
    let before = health(&s, p);
    run(&mut s, 5.0);
    let five = (health(&s, p) - before) as f32;
    assert!((five - max * 0.1).abs() <= 2.0, "2% a second: {five} in 5 sec of {max}");
}

#[test]
fn no_health_comes_back_in_a_fight() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN);
    let h = wounded(&mut s, p, 0.5);
    // A blow every 4 sec for 20 sec: never 5 sec out of combat.
    for _ in 0..5 {
        run(&mut s, 4.0);
        s.hurt_player(p, 1);
    }
    assert_eq!(health(&s, p), h - 5, "only the blows: no regeneration between them");
    // Swinging keeps a hero in the fight too.
    let h = health(&s, p);
    for _ in 0..5 {
        run(&mut s, 4.0);
        s.input(p, Input::Attack);
        s.input(p, Input::AttackUp);
    }
    run(&mut s, 1.0);
    assert_eq!(health(&s, p), h, "a swing every 4 sec: still in the fight");
}

#[test]
fn health_comes_back_out_of_combat_below_too() {
    let (mut s, p) = below();
    assert!(s.is_below());
    let max = s.players[&p].max_health as f32;
    let h = wounded(&mut s, p, 0.3);
    run(&mut s, 4.5);
    assert_eq!(health(&s, p), h, "not before 5 sec, below as anywhere");
    run(&mut s, 5.5);
    let gained = (health(&s, p) - h) as f32;
    assert!((gained - max * 0.1).abs() <= 2.0, "2% a second below: {gained} in 5 sec of {max}");
}

#[test]
fn a_bite_takes_a_second_a_hit_doesnt_spoil_it_and_its_healing_comes_after() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN);
    give(&mut s, p, "roast", 2);
    let roast = s.content.items[s.content.item("roast").unwrap()].clone();
    assert!(roast.steady && roast.heal_secs > 0.0 && (roast.use_secs - 1.0).abs() < 0.01, "a quick bite: {} sec", roast.use_secs);
    let max = s.players[&p].max_health as f32;
    let h = wounded(&mut s, p, 0.25);
    let bag = count(&s, p, "roast");
    s.input(p, Input::Use);
    run(&mut s, 0.5);
    assert!(matches!(s.players[&p].action, Action::Eat { .. }), "eating");
    // Struck mid-bite: it goes on.
    s.hurt_player(p, 5);
    assert!(matches!(s.players[&p].action, Action::Eat { .. }), "a hit doesn't spoil the bite");
    assert_eq!(count(&s, p, "roast"), bag, "not eaten yet");
    // About a second in all.
    run(&mut s, 0.6);
    assert_eq!(s.players[&p].action, Action::Ready, "done in about a second");
    assert_eq!(count(&s, p, "roast"), bag - 1, "eaten");
    let after = health(&s, p);
    assert!(((after + 5 - h) as f32) < max * roast.heal * 0.25, "not healed at once: {h} -> {after}");
    assert!(s.players[&p].effects.iter().any(|e| e.kind == MEAL && e.name == roast.name), "its healing on the hero's effects");
    // Struck again while it heals: the healing goes on, all of it.
    run(&mut s, 2.0);
    assert!(health(&s, p) > after, "healing as it goes");
    s.hurt_player(p, 5);
    run(&mut s, roast.heal_secs - 1.0);
    let gained = (health(&s, p) + 5 + 5) as f32 - h as f32;
    assert!((gained - max * roast.heal).abs() <= 3.0, "{}% of {max} over {} sec, blows or not: {gained}", roast.heal * 100.0, roast.heal_secs);
    assert!(!s.players[&p].effects.iter().any(|e| e.kind == MEAL), "and then it's over");
}

#[test]
fn a_blow_that_staggers_mid_bite_doesnt_waste_the_food() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN);
    give(&mut s, p, "roast", 1);
    wounded(&mut s, p, 0.25);
    let bag = count(&s, p, "roast");
    s.input(p, Input::Use);
    run(&mut s, 0.3);
    let until = s.clock + 0.5;
    s.set_action(p, Action::Stagger { until });
    assert_eq!(count(&s, p, "roast"), bag - 1, "eaten all the same");
    assert!(s.players[&p].effects.iter().any(|e| e.kind == MEAL), "its healing to come");
}

#[test]
fn a_salve_is_still_spoiled_by_a_hit() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN);
    give(&mut s, p, "salve", 1);
    wounded(&mut s, p, 0.25);
    let salve = s.content.item("salve").unwrap();
    s.use_item(p, salve);
    run(&mut s, 0.3);
    s.hurt_player(p, 5);
    assert_eq!(s.players[&p].action, Action::Ready, "interrupted");
    assert_eq!(count(&s, p, "salve"), 1, "nothing used");
}

#[test]
fn every_meal_is_a_steady_bite_and_every_drink_drinks() {
    let c = Content::builtin();
    for key in ["roast", "stew", "grilled_fish", "fish_feast", "pumpkin_pie"] {
        let d = &c.items[c.item(key).unwrap()];
        assert_eq!(d.anim, "eat", "{key}");
        assert!(d.steady && d.heal > 0.0 && d.heal_secs >= 3.0 && d.heal_secs <= 8.0 && d.use_secs <= 1.0, "{key}: a bite, healing after");
    }
    for key in ["draught", "tonic", "clarity"] {
        assert_eq!(c.items[c.item(key).unwrap()].anim, "drink", "{key}");
    }
    assert_eq!((c.player.eat_clip, c.player.drink_clip), (EMOTE_EAT, EMOTE_EAT), "EmoteEat: 1.12 has no drinking clip of its own");
}

#[test]
fn eating_plays_the_eat_clip_not_a_cast() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN);
    give(&mut s, p, "roast", 1);
    let roast = s.content.item("roast").unwrap();
    s.use_item(p, roast);
    let ev = run(&mut s, 0.2);
    let eat = s.content.player.eat_clip;
    assert_eq!(clips(&ev, p), vec![(eat, s.content.items[roast].use_secs)], "the eat clip, over the bite");
    assert!(!casts(&ev, p).contains(&EAT_SPELL), "not the Food spell's cast");
}

#[test]
fn drinking_plays_the_drink_clip() {
    // A tonic at camp.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN);
    give(&mut s, p, "tonic", 1);
    let tonic = s.content.item("tonic").unwrap();
    s.use_item(p, tonic);
    let ev = run(&mut s, 0.2);
    let drink = s.content.player.drink_clip;
    assert_eq!(clips(&ev, p), vec![(drink, s.content.items[tonic].use_secs)]);
    assert!(!casts(&ev, p).contains(&EAT_SPELL));
    // A Miner's Draught below.
    let (mut s, p) = below();
    assert_eq!(s.quick_item(&s.players[&p]), s.content.item("draught"));
    s.input(p, Input::Use);
    let ev = run(&mut s, 0.2);
    let d = &s.content.items[s.content.item("draught").unwrap()];
    assert_eq!(clips(&ev, p), vec![(drink, d.use_secs)], "the Draught drinks");
}

#[test]
fn a_salve_still_has_the_food_spells_look() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN);
    give(&mut s, p, "salve", 1);
    let salve = s.content.item("salve").unwrap();
    s.use_item(p, salve);
    let ev = run(&mut s, 0.2);
    assert!(clips(&ev, p).is_empty());
    assert!(casts(&ev, p).contains(&EAT_SPELL));
}
