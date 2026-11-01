//! The ability system's rules, one test each: projectiles that fly and can be sidestepped,
//! charges, Valor and the EX versions, counters (and what passes them by), the crowd control
//! rules (half on heroes, the grace), totems, tethers, dashes, blinks and the ring tree.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.05;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];

fn world_with(land: FlatLand, pvp: bool) -> Survival {
    let mut c = Content::dev();
    if !pvp {
        c.pvp = Default::default();
    }
    let mut s = Survival::new(c, Box::new(land), 7, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

fn world() -> Survival {
    world_with(FlatLand::default(), false)
}

fn pvp() -> Survival {
    let s = world_with(FlatLand::default(), true);
    assert!(s.content.pvp.friendly_fire);
    s
}

fn hero(s: &mut Survival, name: &str, pos: Vec3, facing: f32) -> Id {
    let id = s.join(&name.to_lowercase(), name, None);
    s.move_player(id, pos, facing, Moving::default());
    let _ = s.step(0.0);
    s.players.get_mut(&id).unwrap().cd_off = false;
    id
}

/// A creature that stands and waits.
fn mob(s: &mut Survival, key: &str, pos: Vec3) -> Id {
    let def = s.content.mob(key).unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.pos = pos;
    id
}

fn calm(s: &mut Survival, id: Id) {
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.path.clear();
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        out.extend(s.step(DT));
    }
    out
}

fn hits_on(events: &[Event], victim: Id) -> Vec<(u32, Outcome)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Swing { victim: Some(v), damage, outcome, anim: false, .. } if *v == victim => Some((*damage, *outcome)),
            _ => None,
        })
        .collect()
}

fn slot(s: &mut Survival, p: Id, slot: &str, key: &str) {
    s.slot_ability(p, slot, key).unwrap();
}

fn key_of(slot: &str) -> u8 {
    crate::content::SLOTS.iter().position(|s| *s == slot).unwrap() as u8
}

/// Press an ability's key, hold the click `secs` (0: a tap), let go.
fn cast(s: &mut Survival, p: Id, slot: &str, ex: bool, secs: f32) -> Vec<Event> {
    s.input(p, Input::Ability(key_of(slot), ex));
    let mut ev = Vec::new();
    if matches!(s.players[&p].action, Action::Aim { .. }) {
        s.input(p, Input::Attack);
        ev.extend(run(s, secs));
        s.input(p, Input::AttackUp);
    }
    ev
}

#[test]
fn a_projectile_flies_and_a_step_aside_after_the_throw_makes_it_miss() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    slot(&mut s, p, "ranged", "frostbolt");
    let k = mob(&mut s, "grunt", [20.0, 0.0, 0.0]);
    // Standing still: it lands, a while after it goes (it flies at its look's own speed).
    let mut ev = cast(&mut s, p, "ranged", false, 1.2);
    let soon = run(&mut s, 0.3);
    assert!(hits_on(&soon, k).is_empty(), "still in flight");
    ev.extend(soon);
    ev.extend(run(&mut s, 1.0));
    assert_eq!(hits_on(&ev, k).len(), 1);
    assert!(s.has_cc(k, "snare"), "a Frostbolt slows");
    // Stepping aside once it's on its way: it flies on past.
    calm(&mut s, k);
    s.mobs.get_mut(&k).unwrap().pos = [20.0, 0.0, 0.0];
    run(&mut s, 3.0);
    cast(&mut s, p, "ranged", false, 1.2);
    run(&mut s, 0.1);
    s.mobs.get_mut(&k).unwrap().pos = [20.0, 3.0, 0.0];
    let ev = run(&mut s, 1.5);
    assert!(hits_on(&ev, k).is_empty(), "nothing curves after a target");
}

#[test]
fn a_longer_charge_lands_harder_and_a_capstone_waits_for_its_whole_charge() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    slot(&mut s, p, "ranged", "frostbolt");
    s.set_cooldowns(p, true);
    let k = mob(&mut s, "boar", [10.0, 0.0, 0.0]);
    let mut ev = cast(&mut s, p, "ranged", false, 0.4);
    ev.extend(run(&mut s, 1.0));
    let early = hits_on(&ev, k)[0].0;
    calm(&mut s, k);
    let mut ev = cast(&mut s, p, "ranged", false, 1.2);
    ev.extend(run(&mut s, 1.0));
    let late = hits_on(&ev, k)[0].0;
    assert!(late > early, "{early} then {late}");
    // Pyroblast (Fireball's EX) let go before its 1.5 s is lost.
    slot(&mut s, p, "ranged", "fireball");
    s.set_valor(p, 100.0);
    cast(&mut s, p, "ranged", true, 1.0);
    assert!(s.players[&p].last_cast.is_none_or(|(a, _)| s.content.abilities[a].key != "pyroblast"), "never cast");
    assert_eq!(s.players[&p].action, Action::Ready, "dropped");
    assert_eq!(s.players[&p].valor, 100.0, "and nothing spent");
}

#[test]
fn valor_fills_from_fighting_and_pays_for_ex_versions() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    assert_eq!(s.players[&p].valor, s.content.valor.start, "each life starts with some");
    s.set_valor(p, 10.0);
    // Ctrl with Hurled Axe's key: Rending Axe, for 25.
    let ev = cast(&mut s, p, "ranged", true, 0.0);
    assert!(ev.is_empty() && matches!(s.players[&p].action, Action::Ready), "not enough Valor");
    let k = mob(&mut s, "boar", [8.0, 0.0, 0.0]);
    s.set_valor(p, 30.0);
    let mut ev = cast(&mut s, p, "ranged", true, 0.7);
    ev.extend(run(&mut s, 1.0));
    assert_eq!(ev.iter().filter(|e| matches!(e, Event::Missile { caster, .. } if *caster == p)).count(), 1, "one axe");
    assert!(ev.iter().any(|e| matches!(e, Event::MissileFrom { caster, .. } if *caster == p)), "and back");
    let v = s.players[&p].valor;
    assert!(v > 5.0 && v < 30.0, "25 spent, some earned back by the hit: {v}");
    // A light blow and a kill earn more.
    calm(&mut s, k);
    s.mobs.get_mut(&k).unwrap().pos = [1.5, 0.0, 0.0];
    let before = s.players[&p].valor;
    s.input(p, Input::Attack);
    run(&mut s, 1.0);
    assert!(s.players[&p].valor > before);
}

#[test]
fn a_counter_takes_melee_and_projectiles_but_not_ground_spells() {
    let mut s = pvp();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [1.8, 0.0, 0.0], std::f32::consts::PI);
    // Bren's Retaliation: Ayla's swing is blocked, and she reels.
    slot(&mut s, b, "defense", "retaliation");
    cast(&mut s, b, "defense", false, 0.0);
    assert!(matches!(s.players[&b].action, Action::Stance { .. }));
    s.input(a, Input::Attack);
    let ev = run(&mut s, 0.6);
    assert_eq!(hits_on(&ev, b)[0].1, Outcome::Blocked);
    assert!(matches!(s.players[&a].action, Action::Stagger { .. }), "the attacker staggers");
    // A Flamestrike under him passes the stance by.
    run(&mut s, 2.0);
    slot(&mut s, a, "ranged", "flamestrike");
    s.slot_ability(b, "defense", "retaliation").unwrap();
    s.set_cooldowns(b, true);
    cast(&mut s, b, "defense", false, 0.0);
    s.set_aim_line(a, 0.0, -0.6);
    let mut ev = cast(&mut s, a, "ranged", false, 0.0);
    ev.extend(run(&mut s, 1.3));
    let hit = hits_on(&ev, b);
    assert!(hit.iter().any(|h| h.1 == Outcome::Hit && h.0 > 0), "ground spells ignore counters: {hit:?}");
}

#[test]
fn retaliation_that_catches_nothing_comes_back_in_half_its_cooldown_and_one_that_catches_takes_it_all() {
    let mut s = pvp();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [1.8, 0.0, 0.0], std::f32::consts::PI);
    let ret = s.content.ability("retaliation").unwrap();
    let full = s.content.abilities[ret].cooldown;
    slot(&mut s, b, "defense", "retaliation");
    cast(&mut s, b, "defense", false, 0.0);
    run(&mut s, 2.0);
    let left = s.cooldown(b, ret).next_at - s.clock;
    assert!(left < full * 0.5 + 0.1 && left > full * 0.5 - 2.5, "misread: half of it ({left:.1})");
    run(&mut s, full);
    cast(&mut s, b, "defense", false, 0.0);
    s.input(a, Input::Attack);
    run(&mut s, 2.0);
    let left = s.cooldown(b, ret).next_at - s.clock;
    assert!(left > full - 2.5, "caught: the whole of it ({left:.1})");
}

#[test]
fn heroes_get_half_the_duration_and_the_grace_stops_a_second_stun() {
    let mut s = pvp();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [10.0, 0.0, 0.0], std::f32::consts::PI);
    s.set_cooldowns(a, true);
    // A two-hander on Avenger's Shield's key throws Hammer of Justice.
    let maul = s.content.item("maul").unwrap();
    s.players.get_mut(&a).unwrap().weapon = maul;
    slot(&mut s, a, "ranged", "avengers_shield");
    let hammer = |s: &mut Survival| cast(s, a, "ranged", false, 0.0);
    let mut ev = hammer(&mut s);
    ev.extend(run(&mut s, 0.8));
    assert!(s.has_cc(b, "stun"), "Hammer of Justice stuns");
    assert!(s.disabled(b));
    let c = s.cc_of(b).iter().find(|c| c.kind == "stun").unwrap().clone();
    assert!((c.until - c.since - 0.75).abs() < 0.05, "0.75 s on a hero, not 1.5");
    run(&mut s, 1.2);
    assert!(!s.has_cc(b, "stun"));
    hammer(&mut s);
    run(&mut s, 0.8);
    assert!(!s.has_cc(b, "stun"), "the grace");
    assert!(s.cc_log_of(b).iter().any(|l| l.contains("grace")), "{:?}", s.cc_log_of(b));
    // On a creature, the whole 1.5 s.
    let k = mob(&mut s, "grunt", [10.0, 0.0, 0.0]);
    s.players.get_mut(&b).unwrap().pos = [10.0, 20.0, 0.0];
    hammer(&mut s);
    run(&mut s, 0.8);
    let c = s.cc_of(k).iter().find(|c| c.kind == "stun").unwrap().clone();
    assert!((c.until - c.since - 1.5).abs() < 0.05);
}

#[test]
fn a_root_breaks_on_damage_only_after_its_first_second_and_a_blink_slips_it() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "grunt", [8.0, 0.0, 0.0]);
    assert!(s.apply_cc(k, p, "root", 3.0, 0.0, 1.0, "test"));
    run(&mut s, 0.4);
    s.hurt_mob(k, p, 1);
    assert!(s.has_cc(k, "root"), "too soon to break");
    run(&mut s, 0.8);
    s.hurt_mob(k, p, 1);
    assert!(!s.has_cc(k, "root"), "broken by damage");
    // A rooted hero blinks out of it.
    let from = s.players[&p].pos;
    assert!(s.apply_cc(p, k, "root", 4.0, 0.0, 0.0, "test"));
    assert!(s.players[&p].rooted);
    slot(&mut s, p, "utility", "blink");
    s.players.get_mut(&p).unwrap().facing = std::f32::consts::PI;
    cast(&mut s, p, "utility", false, 0.0);
    assert!(!s.has_cc(p, "root") && !s.players[&p].rooted, "free");
    assert!(dist2d(s.players[&p].pos, from) > 10.0, "and away: {:?}", s.players[&p].pos);
}

#[test]
fn a_totem_slows_the_foes_near_it_until_it_is_broken() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    slot(&mut s, p, "utility", "earthbind_totem");
    let k = mob(&mut s, "grunt", [12.0, 2.0, 0.0]);
    s.set_aim_line(p, 0.0, -0.2);
    cast(&mut s, p, "utility", false, 0.0);
    let totem = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "earthbind_totem").map(|(id, _)| id).unwrap();
    run(&mut s, 1.2);
    assert!(s.has_cc(k, "snare"), "slowed near it");
    assert!(s.speed_share(k) < 0.6);
    // Broken: no more slowing.
    let hp = s.mobs[&totem].health;
    s.hurt_mob(totem, p, hp);
    run(&mut s, 3.0);
    assert!(!s.has_cc(k, "snare"));
    assert!(!s.mobs.contains_key(&totem) || s.mobs[&totem].health == 0);
}

#[test]
fn a_drain_holds_while_in_sight_and_snaps_behind_a_wall() {
    let land = FlatLand { walls: vec![([8.0, 6.0], 1.0)], ..Default::default() };
    let mut s = world_with(land, false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    slot(&mut s, p, "defense", "drain_life");
    s.players.get_mut(&p).unwrap().health = 100;
    let k = mob(&mut s, "boar", [12.0, 0.0, 0.0]);
    cast(&mut s, p, "defense", false, 0.0);
    assert!(matches!(s.players[&p].action, Action::Channel { .. }));
    let ev = run(&mut s, 1.2);
    assert!(hits_on(&ev, k).len() >= 2, "ticking");
    assert!(s.players[&p].health > 100, "it heals the caster");
    // Behind the wall: it snaps.
    calm(&mut s, k);
    s.mobs.get_mut(&k).unwrap().pos = [16.0, 12.0, 0.0];
    run(&mut s, 0.6);
    assert!(!matches!(s.players[&p].action, Action::Channel { .. }), "line of sight broken");
}

#[test]
fn a_charge_cannot_be_struck_on_the_way_and_staggers_what_it_meets() {
    let mut s = pvp();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [12.0, 0.0, 0.0], std::f32::consts::PI);
    cast(&mut s, a, "utility", false, 0.0);
    assert!(matches!(s.players[&a].action, Action::Dash { .. }));
    let o = s.strike_player(b, a, 20, [12.0, 0.0, 0.0], true, false, true, Blow::Light);
    assert_eq!(o, Outcome::Dodged, "untouchable on the way");
    run(&mut s, 0.8);
    assert!(matches!(s.players[&b].action, Action::Stagger { .. }) || s.cc_log_of(b).iter().any(|l| l.contains("stagger")), "staggered");
}

#[test]
fn cone_of_cold_freezes_what_is_already_slowed() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    slot(&mut s, p, "close", "cone_of_cold");
    s.set_cooldowns(p, true);
    let k = mob(&mut s, "grunt", [6.0, 0.5, 0.0]);
    cast(&mut s, p, "close", false, 0.0);
    assert!(s.has_cc(k, "snare") && !s.has_cc(k, "petrify"));
    run(&mut s, 0.5);
    calm(&mut s, k);
    s.mobs.get_mut(&k).unwrap().pos = [6.0, 0.5, 0.0];
    cast(&mut s, p, "close", false, 0.0);
    assert!(s.has_cc(k, "petrify"), "frozen");
    // The ice takes the next blows first.
    let hp = s.mobs[&k].health;
    s.hurt_mob(k, p, 10);
    assert_eq!(s.mobs[&k].health, hp, "the ice took it");
}

#[test]
fn fire_ward_sends_a_projectile_back_and_ice_block_lets_nothing_in() {
    let mut s = pvp();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [12.0, 0.0, 0.0], std::f32::consts::PI);
    s.slot_ability(b, "defense", "fire_ward").unwrap();
    cast(&mut s, b, "defense", false, 0.0);
    let mut ev = cast(&mut s, a, "ranged", false, 0.7);
    ev.extend(run(&mut s, 2.5));
    assert_eq!(hits_on(&ev, b)[0].1, Outcome::Blocked, "absorbed");
    assert!(hits_on(&ev, a).iter().any(|h| h.1 == Outcome::Hit), "a Fire Blast comes back: {:?}", hits_on(&ev, a));
    // Ice Block: nothing reaches him, he can't act, and its key ends it.
    s.slot_ability(b, "defense", "ice_block").unwrap();
    s.set_cooldowns(b, true);
    cast(&mut s, b, "defense", false, 0.0);
    assert!(s.immune(b));
    let o = s.strike_player(a, b, 30, ORIGIN, true, false, true, Blow::Light);
    assert_eq!(o, Outcome::Blocked);
    s.input(b, Input::Attack);
    assert!(matches!(s.players[&b].action, Action::Stance { .. }), "can't act");
    s.input(b, Input::Ability(key_of("defense"), false));
    assert!(!s.immune(b), "broken out early");
}

#[test]
fn burning_never_lands_the_killing_blow_on_a_hero() {
    let mut s = pvp();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [5.0, 0.0, 0.0], 0.0);
    s.players.get_mut(&b).unwrap().health = 3;
    s.apply_cc(b, a, "burn", 8.0, 10.0, 0.0, "test");
    run(&mut s, 4.0);
    assert_eq!(s.players[&b].health, 1);
}

#[test]
fn the_board_has_five_rows_of_nine_and_a_dev_build_owns_every_card() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    assert!(s.content.dev.full_tree);
    assert_eq!(s.players[&p].owned, s.all_nodes(), "dev: every card");
    // Nine cards to a row, one a class (the beta page apart).
    for row in crate::content::SLOTS {
        let cards: Vec<&str> = s.content.abilities.iter().filter(|a| a.on_board() && !a.is_beta() && a.slot == row).map(|a| a.school.as_str()).collect();
        assert_eq!(cards.len(), 9, "{row}: {cards:?}");
        for col in crate::content::SCHOOLS {
            assert!(cards.contains(&col), "{row} has a {col} card");
        }
    }
    // Without the dev build: the class's first cards (the Warrior's Q and E); the rest are
    // learned out in Elwynn, and the board says where (taken from it only in a debug world).
    s.set_tree(p, false);
    let owned: Vec<&str> = s.players[&p].owned.iter().map(String::as_str).collect();
    assert_eq!(owned, ["hurled_axe", "shockwave"], "Q and E are free");
    assert!(s.slot_ability(p, "ranged", "seed_of_corruption").is_err(), "not unlocked");
    let why = s.take_node(p, "seed_of_corruption").unwrap_err();
    assert!(why.contains("The Cult's Altar"), "{why}");
    s.debug = true;
    s.take_node(p, "seed_of_corruption").unwrap();
    assert!(s.take_node(p, "frostbolt").is_err(), "the bench isn't on the board");
    // A card only goes on its own row's key.
    assert!(s.slot_ability(p, "close", "seed_of_corruption").is_err());
    assert!(s.slot_ability(p, "ranged", "seed_of_corruption").is_ok());
}

#[test]
fn a_heavy_blow_breaks_a_pyroblast_charge() {
    let mut s = pvp();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [2.0, 0.0, 0.0], std::f32::consts::PI);
    slot(&mut s, a, "ranged", "fireball");
    s.set_valor(a, 100.0);
    s.input(a, Input::Ability(key_of("ranged"), true));
    s.input(a, Input::Attack);
    run(&mut s, 0.5);
    assert!(matches!(s.players[&a].action, Action::Charge { .. }));
    s.strike_player(b, a, 20, [2.0, 0.0, 0.0], true, true, true, Blow::Heavy);
    assert!(!matches!(s.players[&a].action, Action::Charge { .. }), "broken");
    assert_eq!(s.players[&a].valor, 100.0, "nothing spent");
}

#[test]
fn the_loadout_and_valor_are_kept_across_a_save() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    slot(&mut s, p, "ranged", "fireball");
    s.set_valor(p, 60.0);
    let saved = s.leave(p).unwrap();
    let q = s.join("ayla", "Ayla", Some(&saved));
    assert_eq!(s.players[&q].loadout.get("ranged").map(String::as_str), Some("fireball"));
    assert_eq!(s.players[&q].valor, 60.0);
}

#[test]
fn backstab_after_backstab_is_worth_less_and_a_champion_turns_on_its_stabber() {
    let mut s = world();
    let tank = hero(&mut s, "Ayla", [3.0, 0.0, 0.0], std::f32::consts::PI);
    let stabber = hero(&mut s, "Bren", [-1.5, 0.0, 0.0], 0.0);
    let boss = mob(&mut s, "dm_smite", ORIGIN);
    s.mobs.get_mut(&boss).unwrap().facing = 0.0;
    s.engage(boss, mobs::Foe::Hero(tank));
    s.players.get_mut(&tank).unwrap().foe = Some((boss, s.clock));
    let first = s.backstab_bonus_for_test(boss);
    for _ in 0..3 {
        s.mobs.get_mut(&boss).unwrap().backstabs.push(s.clock);
    }
    let later = s.backstab_bonus_for_test(boss);
    assert!(later < first && later >= 1.0, "{first} then {later}");
    // With two heroes on it, a run of backstabs has it turn round.
    let mut turned = false;
    for _ in 0..20 {
        s.mobs.get_mut(&boss).unwrap().state = MobState::Chase { foe: mobs::Foe::Hero(tank) };
        s.mobs.get_mut(&boss).unwrap().facing = 0.0;
        s.players.get_mut(&stabber).unwrap().foe = Some((boss, s.clock));
        s.input(stabber, Input::Attack);
        run(&mut s, 0.9);
        if s.mobs.get(&boss).is_some_and(|m| m.state.target() == Some(stabber)) {
            turned = true;
            break;
        }
    }
    assert!(turned, "it turned on the one at its back");
}

#[test]
fn an_ability_key_held_while_it_is_aimed_turns_it_to_its_ex_version_with_valor_enough() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    slot(&mut s, p, "ranged", "frostbolt");
    let base = s.content.ability("frostbolt").unwrap();
    let ex = s.content.ex_of(base).unwrap();
    s.set_valor(p, 0.0);
    s.input(p, Input::Ability(key_of("ranged"), false));
    assert_eq!(s.players[&p].action, Action::Aim { ability: base });
    // Held, with no Valor: still the base, and the hero told.
    s.input(p, Input::Ability(key_of("ranged"), true));
    let ev = s.step(DT);
    assert_eq!(s.players[&p].action, Action::Aim { ability: base });
    assert!(ev.iter().any(|e| matches!(e, Event::Flash { player, text } if *player == p && text.contains("Valor"))));
    // With Valor: the EX's aim, and a second hold changes nothing.
    s.set_valor(p, 100.0);
    s.input(p, Input::Ability(key_of("ranged"), true));
    assert_eq!(s.players[&p].action, Action::Aim { ability: ex });
    s.input(p, Input::Ability(key_of("ranged"), true));
    assert_eq!(s.players[&p].action, Action::Aim { ability: ex });
    // A tap of its key calls it off.
    s.input(p, Input::Ability(key_of("ranged"), false));
    assert_eq!(s.players[&p].action, Action::Ready);
}

#[test]
fn lingering_ground_wears_its_spells_look_until_it_is_gone() {
    let mut s = world();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let meteor = s.content.ability("meteor").unwrap();
    let d = s.content.abilities[meteor].clone();
    s.linger_at(a, meteor, [5.0, 0.0, 0.0], 4.0);
    let ev = run(&mut s, 0.1);
    let look = ev.iter().find_map(|e| match e {
        Event::AreaLook { id, spell, .. } => Some((*id, *spell)),
        _ => None,
    });
    let (id, spell) = look.expect("its ground look");
    assert_eq!(spell, d.linger_spell, "the burning ground's own look");
    let ev = run(&mut s, d.linger + 0.5);
    assert!(ev.iter().any(|e| matches!(e, Event::MarkGone { id: m } if *m == id)), "gone with its mark");
}

#[test]
fn a_creatures_blow_all_round_lays_its_look_at_its_feet() {
    let mut s = world();
    let h = hero(&mut s, "Ayla", [2.0, 0.0, 0.0], std::f32::consts::PI);
    let m = mob(&mut s, "hogger", ORIGIN);
    let slam = s.content.mobs[s.mobs[&m].def].patterns.iter().position(|p| p.name == "slam").unwrap();
    let look = s.content.mobs[s.mobs[&m].def].patterns[slam].steps[0].look.unwrap();
    s.mobs.get_mut(&m).unwrap().pattern = Some((slam, 0));
    s.windup(m, crate::sim::mobs::Foe::Hero(h), [2.0, 0.0, 0.0]);
    let ev = run(&mut s, 2.0);
    assert!(
        ev.iter().any(|e| matches!(e, Event::Missile { caster, spell, target: Target::Point(at) } if *caster == m && *spell == look && dist2d(*at, ORIGIN) < 1.0)),
        "the slam's shockwave where it stands"
    );
}

#[test]
fn meteors_look_starts_coming_down_before_it_lands_so_its_blow_lands_with_it() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.players.get_mut(&p).unwrap().owned.insert("meteor".into());
    slot(&mut s, p, "ultimate", "meteor");
    s.set_valor(p, 100.0);
    let meteor = s.content.ability("meteor").unwrap();
    let d = s.content.abilities[meteor].clone();
    assert!(d.look_lead > 0.0 && d.look_lead < d.delay);
    let mut ev = cast(&mut s, p, "ultimate", false, 0.0);
    let start = s.clock;
    let mut look_at = None;
    let mut looks = 0;
    for _ in 0..(2.5 / DT).round() as usize {
        for e in s.step(DT) {
            if matches!(e, Event::Missile { spell, .. } if spell == d.spell) {
                looks += 1;
                look_at.get_or_insert(s.clock - start);
            }
            ev.push(e);
        }
    }
    assert_eq!(looks, 1, "shown once");
    let t = look_at.expect("its look");
    let want = d.delay - d.look_lead;
    assert!((t - want).abs() < 0.1, "shown {t:.2} s in, {want:.2} wanted");
}
