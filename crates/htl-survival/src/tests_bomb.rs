//! Living Bomb: a bolt plants a bomb that flares three times and bursts; the key again on a flare
//! sets it off early and harder (a perfect, with a knockback); off a flare, at once and plain.
//! Combustion: every burn on the foe its bolt reaches catches on each foe near it.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.01;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];

fn world() -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    c.dev.full_tree = true;
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 1, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

fn hero(s: &mut Survival, name: &str, pos: Vec3, facing: f32) -> Id {
    let id = s.join(&name.to_lowercase(), name, None);
    s.move_player(id, pos, facing, Moving::default());
    let _ = s.step(0.0);
    s.players.get_mut(&id).unwrap().cd_off = false;
    id
}

fn dummy(s: &mut Survival, pos: Vec3) -> Id {
    let def = s.content.mob("training_dummy").unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    s.mobs.get_mut(&id).unwrap().state = MobState::Idle { wander_at: 1e9 };
    id
}

/// Run on until the clock reads `t`.
fn run_until(s: &mut Survival, t: f32) -> Vec<Event> {
    let left = t - s.clock;
    if left > 0.0 { run(s, left) } else { Vec::new() }
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        out.extend(s.step(DT));
    }
    out
}

fn key_of(slot: &str) -> u8 {
    crate::content::SLOTS.iter().position(|s| *s == slot).unwrap() as u8
}

fn press(s: &mut Survival, p: Id, ex: bool) {
    s.input(p, Input::Ability(key_of("utility"), ex));
    if matches!(s.players[&p].action, Action::Aim { .. }) {
        s.input(p, Input::Attack);
        s.input(p, Input::AttackUp);
    }
}

/// Living Bomb cast at a dummy 10 yd east; run until the bolt has planted it. When it was planted.
fn plant(s: &mut Survival, p: Id) -> f32 {
    s.slot_ability(p, "utility", "living_bomb").unwrap();
    press(s, p, false);
    for _ in 0..200 {
        s.step(DT);
        if !s.bombs.is_empty() {
            return s.bombs[0].since;
        }
    }
    panic!("no bomb planted");
}

fn blows_on(events: &[Event], victim: Id) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Swing { victim: Some(v), damage, anim: false, .. } if *v == victim && *damage > 0 => Some(*damage),
            _ => None,
        })
        .collect()
}

fn perfect_in(events: &[Event], p: Id) -> bool {
    events.iter().any(|e| matches!(e, Event::Perfect { player, .. } if *player == p))
}

#[test]
fn the_bolt_plants_a_bomb_that_flares_three_times_then_bursts_and_burns() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let d = s.content.abilities[s.content.ability("living_bomb").unwrap()].clone();
    let k = dummy(&mut s, [10.0, 0.0, 0.0]);
    let near = dummy(&mut s, [12.0, 3.0, 0.0]);
    let since = plant(&mut s, p);
    let ev = run_until(&mut s, since + d.bomb_secs + 0.2);
    assert!(s.bombs.is_empty(), "burst and gone");
    let flares = ev.iter().filter(|e| matches!(e, Event::Visual { id, kit, .. } if *id == k && *kit == d.bomb_kit)).count();
    assert_eq!(flares, 3, "three flares on the foe");
    assert_eq!(blows_on(&ev, k).into_iter().max(), Some(d.damage[0] as u32), "the plain burst");
    assert_eq!(blows_on(&ev, near).into_iter().max(), Some(d.damage[0] as u32), "on the one 3.6 yd off too");
    assert!(s.has_cc(k, "burn") && s.has_cc(near, "burn"), "and they burn");
    assert!(!perfect_in(&ev, p));
}

#[test]
fn the_key_again_on_a_flare_sets_it_off_early_harder_a_perfect_that_throws_them_back() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let d = s.content.abilities[s.content.ability("living_bomb").unwrap()].clone();
    let k = dummy(&mut s, [10.0, 0.0, 0.0]);
    let since = plant(&mut s, p);
    // The second flare: a third of the way to the end, twice.
    run_until(&mut s, since + d.bomb_secs * 2.0 / 3.0);
    let valor = s.players[&p].valor;
    press(&mut s, p, false);
    let ev = s.step(0.0);
    assert!(s.bombs.is_empty(), "set off");
    assert!(perfect_in(&ev, p), "on a flare: a perfect");
    assert_eq!(blows_on(&ev, k).into_iter().max(), Some(d.damage[1] as u32), "the hard burst");
    assert!(s.players[&p].valor >= valor + s.content.checks.perfect_valor);
    assert!(s.cc_log_of(p).iter().any(|l| l.contains("knockback on")), "and it throws them back: {:?}", s.cc_log_of(p));
}

#[test]
fn off_a_flare_it_goes_at_once_plain_and_a_dead_foe_sets_it_off_where_it_fell() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let d = s.content.abilities[s.content.ability("living_bomb").unwrap()].clone();
    let k = dummy(&mut s, [10.0, 0.0, 0.0]);
    let since = plant(&mut s, p);
    // Halfway between the first and second flares.
    run_until(&mut s, since + d.bomb_secs / 2.0);
    press(&mut s, p, false);
    let ev = s.step(0.0);
    assert!(s.bombs.is_empty() && !perfect_in(&ev, p), "off a flare: at once, no perfect");
    assert_eq!(blows_on(&ev, k).into_iter().max(), Some(d.damage[0] as u32));
    // A foe that dies with it on.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = dummy(&mut s, [10.0, 0.0, 0.0]);
    let other = dummy(&mut s, [11.0, 2.0, 0.0]);
    plant(&mut s, p);
    s.mobs.get_mut(&k).unwrap().health = 0;
    let ev = run(&mut s, 0.05);
    assert!(s.bombs.is_empty(), "it went off where the foe fell");
    assert!(!blows_on(&ev, other).is_empty(), "and caught the one beside it");
}

#[test]
fn combustion_spreads_the_burns_on_its_target_to_everything_near_it() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = dummy(&mut s, [10.0, 0.0, 0.0]);
    let near = dummy(&mut s, [13.0, 4.0, 0.0]);
    let far = dummy(&mut s, [10.0, 12.0, 0.0]);
    // Fire it at a foe that already burns (the bomb's burst).
    let since = plant(&mut s, p);
    let d = s.content.abilities[s.content.ability("living_bomb").unwrap()].clone();
    run_until(&mut s, since + d.bomb_secs + 0.1);
    // Clear the near one's burn, so only Combustion can light it.
    s.cc.remove(&near);
    assert!(s.has_cc(k, "burn") && !s.has_cc(near, "burn"));
    s.players.get_mut(&p).unwrap().valor = s.content.valor.max;
    s.players.get_mut(&p).unwrap().cd_off = true;
    press(&mut s, p, true);
    run(&mut s, 0.6);
    assert!(s.has_cc(near, "burn"), "5 yd off: it catches");
    assert!(!s.has_cc(far, "burn"), "12 yd off: not");
}

#[test]
fn living_bomb_is_the_mages_second_utility_from_the_satchel_with_combustion_its_ex() {
    let s = world();
    let c = &s.content;
    let lb = c.ability("living_bomb").unwrap();
    let d = &c.abilities[lb];
    assert_eq!((d.school.as_str(), d.slot.as_str(), d.choice, d.kind.as_str()), ("mage", "utility", 2, "bomb"));
    assert_eq!(c.ex_of(lb).map(|k| c.abilities[k].key.as_str()), Some("combustion"));
    let relic = c.chests.iter().find(|r| r.key == "relic_mage").unwrap();
    assert!(relic.teaches.contains(&"living_bomb".to_string()));
}
