//! The Priest's core, draft 3 (docs/priest-draft3.md): the three staffs, each a playstyle with a
//! loop (the Acolyte's Smite, Searing Light, Greater Smite and Healing Gust; the Shadowcaller's
//! Shadow Weaving, Empowered bolts and Dark Reckoning; the Hollow Star's Void Tether and living
//! orb), none of which ever guards; Power Word: Shield, the one save, parrying a blow for a friend;
//! every Priest perfect in the Priest's own holy burst; old saves; and the board's budgets.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::{Foe, MobState};
use crate::sim::*;

const DT: f32 = 0.01;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];
const EAST: f32 = 0.0;
const WEST: f32 = std::f32::consts::PI;
/// The Mage's Arcane Explosion (`[checks] perfect_kit`), which no Priest perfect may show.
const MAGE_KIT: u32 = 1004;

fn world() -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    c.dev.full_tree = true;
    // (No mending by itself: what's healed is the spells'.)
    c.player.field_regen = 0.0;
    c.player.rest_regen = 0.0;
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

/// A hero holding `staff` (taken from the rack), back where they stood, the reticle level along
/// their facing.
fn armed(s: &mut Survival, name: &str, pos: Vec3, facing: f32, staff: &str) -> Id {
    let p = hero(s, name, pos, facing);
    let rack = s.objs[&s.rack_id()].pos;
    s.players.get_mut(&p).unwrap().pos = rack;
    s.arm(p, staff).unwrap();
    s.players.get_mut(&p).unwrap().pos = pos;
    s.set_aim_line(p, facing, 0.0);
    p
}

/// A creature standing idle where it's put (a dummy stands still whatever strikes it).
fn mob(s: &mut Survival, key: &str, pos: Vec3) -> Id {
    let def = s.content.mob(key).unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.ready_at = 1e9;
    m.pos = pos;
    id
}

/// A grunt of 1000 health that stands and takes it (a dummy heals back up as soon as no one has
/// it as their foe, as an area's or an orb's damage leaves it).
fn post(s: &mut Survival, pos: Vec3) -> Id {
    let g = mob(s, "grunt", pos);
    let m = s.mobs.get_mut(&g).unwrap();
    m.max_health = 1000;
    m.health = 1000;
    g
}

/// A wolf `at`, its first bite on its way to `on`.
fn wolf_on(s: &mut Survival, on: Id, at: Vec3) -> Id {
    let def = s.content.mob("wolf").unwrap();
    let k = s.spawn_mob(def, at, None, false);
    s.mobs.get_mut(&k).unwrap().state = MobState::Idle { wander_at: 1e9 };
    s.engage(k, Foe::Hero(on));
    k
}

/// Run until the wolf's bite is `before` seconds off landing.
fn until_bite(s: &mut Survival, k: Id, before: f32) -> Vec<Event> {
    let mut ev = Vec::new();
    for _ in 0..1000 {
        if let MobState::Windup { t, .. } = s.mobs[&k].state {
            if t + before >= s.attack_now(k).windup - 1.0e-4 {
                return ev;
            }
        }
        ev.extend(s.step(DT));
    }
    panic!("no bite");
}

fn pin(s: &mut Survival, pins: &[Id]) -> Vec<(Id, Vec3)> {
    pins.iter().filter_map(|m| s.mobs.get(m).map(|x| (*m, x.pos))).collect()
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        out.extend(s.step(DT));
    }
    out
}

/// Run with these creatures held idle where they stand.
fn run_pinned(s: &mut Survival, secs: f32, pins: &[Id]) -> Vec<Event> {
    let at = pin(s, pins);
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        for (m, p) in &at {
            if let Some(x) = s.mobs.get_mut(m) {
                x.state = MobState::Idle { wander_at: 1e9 };
                x.path.clear();
                x.pos = *p;
            }
        }
        out.extend(s.step(DT));
    }
    out
}

fn key_of(slot: &str) -> u8 {
    crate::content::SLOTS.iter().position(|s| *s == slot).unwrap() as u8
}

fn idx(s: &Survival, key: &str) -> usize {
    s.content.ability(key).unwrap()
}

fn missiles(events: &[Event], caster: Id) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Missile { caster: c, spell, .. } if *c == caster => Some(*spell),
            _ => None,
        })
        .collect()
}

fn perfect(events: &[Event], p: Id) -> bool {
    events.iter().any(|e| matches!(e, Event::Perfect { player, .. } if *player == p))
}


/// Whether the events show the burst of a perfect of `key` on `on`: the Priest's own (the
/// spell's, else `[priest] perfect_kit`), never the checks' default, the Mage's Arcane Explosion.
fn priest_burst(s: &Survival, ev: &[Event], on: Id, key: &str) -> bool {
    let kit = s.perfect_kit_of(idx(s, key));
    assert!(kit != 0 && kit != s.content.checks.perfect_kit && kit != MAGE_KIT, "{key}: a Priest's own burst, not {kit}");
    ev.iter().any(|e| matches!(e, Event::Visual { id, kit: k, .. } if *id == on && *k == kit))
}

/// The Mage's Arcane Explosion anywhere.
fn mage_burst(ev: &[Event]) -> bool {
    ev.iter().any(|e| matches!(e, Event::Visual { kit, .. } if *kit == MAGE_KIT))
}

fn outcomes(events: &[Event], on: Id) -> Vec<Outcome> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Swing { victim: Some(v), outcome, .. } if *v == on => Some(*outcome),
            _ => None,
        })
        .collect()
}

fn hurt(s: &mut Survival, p: Id, health: u32) {
    s.players.get_mut(&p).unwrap().health = health;
}

fn health(s: &Survival, p: Id) -> u32 {
    s.players[&p].health
}

fn hp(s: &Survival, m: Id) -> u32 {
    s.mobs[&m].health
}

/// The stacks of a staff's mark `kind` the hero has on a body.
fn marks(s: &Survival, owner: Id, on: Id, kind: &str) -> u32 {
    s.staff_marks.iter().find(|m| m.owner == owner && m.on == on && m.kind == kind && m.until > s.clock).map_or(0, |m| m.n)
}

/// What a hero's shield has left to absorb (0: none).
fn shield_left(s: &Survival, p: Id) -> f32 {
    s.cc_of(p).iter().filter(|c| c.kind == "shield" && c.until > s.clock).map(|c| c.left).fold(0.0, f32::max)
}

/// Power Word: Shield on its key, cast now along the reticle.
fn shield(s: &mut Survival, p: Id) -> Vec<Event> {
    s.input(p, Input::Ability(key_of("defense"), false));
    let mut ev = s.step(0.0);
    if matches!(s.players[&p].action, Action::Aim { .. }) {
        s.input(p, Input::Attack);
        s.input(p, Input::AttackUp);
        ev.extend(s.step(0.0));
    }
    ev
}

/// A click of the left button.
fn click(s: &mut Survival, p: Id) -> Vec<Event> {
    s.input(p, Input::Attack);
    s.input(p, Input::AttackUp);
    s.step(0.0)
}

/// The right button held, a click, the right button let go: Healing Gust.
fn gust(s: &mut Survival, p: Id) -> Vec<Event> {
    s.input(p, Input::Guard);
    let mut ev = s.step(0.0);
    ev.extend(click(s, p));
    s.input(p, Input::GuardUp);
    ev.extend(s.step(0.0));
    ev
}

/// Greater Smite (or Shadow Burst) held `secs` and let go: what went out.
fn heavy_for(s: &mut Survival, p: Id, secs: f32) -> Vec<Event> {
    s.input(p, Input::Heavy);
    let mut ev = s.step(0.0);
    assert!(matches!(s.players[&p].action, Action::Charge { .. }), "charging: {:?}", s.players[&p].action);
    ev.extend(run(s, secs));
    s.input(p, Input::AttackUp);
    ev.extend(s.step(0.0));
    ev
}

/// Held to its flare (this cast's, rolled as it began) and let go on it.
fn heavy_on_flare(s: &mut Survival, p: Id, late: f32) -> Vec<Event> {
    s.input(p, Input::Heavy);
    let mut ev = s.step(0.0);
    let since = match s.players[&p].action {
        Action::Charge { since, .. } => since,
        ref a => panic!("charging: {a:?}"),
    };
    let flare = s.players[&p].check_flare;
    while s.clock + DT * 0.5 < since + flare + late {
        ev.extend(s.step(DT));
    }
    s.input(p, Input::AttackUp);
    ev.extend(s.step(0.0));
    ev
}

// --- the holy staff: Smite, Searing Light, Greater Smite ----------------------------------------

#[test]
fn smite_is_the_holy_staffs_click_9_and_marks_searing_light_for_5_sec() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "acolyte_staff");
    let d = mob(&mut s, "training_dummy", [12.0, 0.0, 0.0]);
    let full = hp(&s, d);
    let ev = click(&mut s, p);
    assert_eq!(missiles(&ev, p), vec![6685], "Smite's bolt");
    run(&mut s, 1.0);
    assert_eq!(full - hp(&s, d), 9, "9 damage");
    assert_eq!(marks(&s, p, d, "sear"), 1, "Searing Light on it");
    // A second Smite on the marked foe doesn't tear it out (only another spell does).
    run(&mut s, 0.5);
    let before = hp(&s, d);
    click(&mut s, p);
    run(&mut s, 1.0);
    assert_eq!(before - hp(&s, d), 9, "still 9");
    assert_eq!(marks(&s, p, d, "sear"), 1, "the mark held, renewed");
    run(&mut s, 4.6);
    assert_eq!(marks(&s, p, d, "sear"), 0, "gone 5 sec after the last Smite");
}

#[test]
fn another_priest_spell_on_a_marked_foe_tears_searing_light_out_for_8_and_hastens_healing_gust() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "acolyte_staff");
    s.players.get_mut(&p).unwrap().owned.insert("penance".into());
    s.slot_ability(p, "ranged", "penance").unwrap();
    let d = mob(&mut s, "training_dummy", [12.0, 0.0, 0.0]);
    // Both Healing Gust charges spent (on no one: they come back to her), recharging.
    let g = idx(&s, "healing_gust");
    gust(&mut s, p);
    run(&mut s, 0.5);
    gust(&mut s, p);
    run(&mut s, 2.0);
    let cd = s.cooldown(p, g);
    assert_eq!(cd.charges, 0, "both spent");
    let back_at = cd.next_at;
    click(&mut s, p);
    run(&mut s, 1.0);
    assert_eq!(marks(&s, p, d, "sear"), 1);
    // Penance's bolt on it: 18 dead on, and 8 more for the mark.
    let before = hp(&s, d);
    s.input(p, Input::Ability(key_of("ranged"), false));
    let mut ev = s.step(0.0);
    let (_, at, on) = s.penance_next(p).expect("rings on it");
    assert_eq!(on, d);
    while s.clock + DT * 0.5 < at {
        ev.extend(s.step(DT));
    }
    s.input(p, Input::Ability(key_of("ranged"), false));
    ev.extend(s.step(0.0));
    assert_eq!(before - hp(&s, d), 26, "18 + 8");
    assert_eq!(marks(&s, p, d, "sear"), 0, "the mark torn out");
    assert!(ev.iter().any(|e| matches!(e, Event::Visual { id, kit: 3153, .. } if *id == d)), "Holy Fire's flash on it");
    let now = s.cooldown(p, g).next_at;
    assert!((back_at - now - 3.0).abs() < 0.05, "Healing Gust 3 sec sooner: {back_at} to {now}");
    // The next ring: no mark left, plain.
    let before = hp(&s, d);
    let (_, at, _) = s.penance_next(p).unwrap();
    while s.clock + DT * 0.5 < at {
        s.step(DT);
    }
    s.input(p, Input::Ability(key_of("ranged"), false));
    s.step(0.0);
    assert_eq!(before - hp(&s, d), 18);
}

#[test]
fn greater_smite_let_go_on_the_flare_is_26_pierces_and_bursts_in_the_priests_light() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "acolyte_staff");
    let a = post(&mut s, [10.0, 0.0, 0.0]);
    let b = post(&mut s, [14.0, 0.0, 0.0]);
    let (ha, hb) = (hp(&s, a), hp(&s, b));
    let valor = s.players[&p].valor;
    let mut ev = heavy_on_flare(&mut s, p, 0.0);
    ev.extend(run_pinned(&mut s, 1.0, &[a, b]));
    assert_eq!(ha - hp(&s, a), 26, "22 x 1.2");
    assert_eq!(hb - hp(&s, b), 26, "and through it, the one behind");
    assert!(perfect(&ev, p), "the perfect");
    assert!(s.players[&p].valor >= valor + s.content.checks.perfect_valor);
    assert!(priest_burst(&s, &ev, p, "acolyte_greater_smite"), "the Priest's holy burst");
    assert!(!mage_burst(&ev), "never Arcane Explosion's");
}

#[test]
fn greater_smite_off_the_flare_goes_as_charged_and_held_long_past_it_falls_to_its_least() {
    // Let go a little late: as charged (no perfect, no pierce).
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "acolyte_staff");
    let a = mob(&mut s, "training_dummy", [10.0, 0.0, 0.0]);
    let b = mob(&mut s, "training_dummy", [14.0, 0.0, 0.0]);
    let (ha, hb) = (hp(&s, a), hp(&s, b));
    let mut ev = heavy_on_flare(&mut s, p, 0.2);
    ev.extend(run(&mut s, 1.0));
    let dealt = ha - hp(&s, a);
    assert!((16..=22).contains(&dealt), "as charged: {dealt}");
    assert_eq!(hp(&s, b), hb, "no pierce");
    assert!(!perfect(&ev, p));
    // Held past the flare, its window and 0.35 sec more: back to 12.
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "acolyte_staff");
    let a = mob(&mut s, "training_dummy", [10.0, 0.0, 0.0]);
    let ha = hp(&s, a);
    let w = s.content.checks.window;
    heavy_on_flare(&mut s, p, w + 0.35 + 0.1);
    run(&mut s, 1.0);
    assert_eq!(ha - hp(&s, a), 12, "overcharged: its least");
    // Held a whole second: 22.
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "acolyte_staff");
    let a = mob(&mut s, "training_dummy", [10.0, 0.0, 0.0]);
    let ha = hp(&s, a);
    // (A flare at 1.0 would make this a perfect: pin it early.)
    s.input(p, Input::Heavy);
    s.step(0.0);
    s.players.get_mut(&p).unwrap().check_flare = 0.75;
    run(&mut s, 1.05);
    s.input(p, Input::AttackUp);
    s.step(0.0);
    run(&mut s, 1.0);
    assert_eq!(ha - hp(&s, a), 22, "fully charged, late by 0.3: 22");
}

#[test]
fn greater_smite_let_go_in_the_air_throws_the_priest_back_once_a_jump() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "acolyte_staff");
    s.set_cooldowns(p, true);
    let thrown = |ev: &[Event]| ev.iter().find_map(|e| match e {
        Event::Hop { player, dir, speed, .. } if *player == p && *speed > 0.0 => Some((*dir, *speed)),
        _ => None,
    });
    // On the ground: no throw.
    let ev = heavy_for(&mut s, p, 0.5);
    assert!(thrown(&ev).is_none(), "on the ground, nothing");
    run(&mut s, 0.6);
    // In the air: thrown back (west), 4 yd over about a third of a second.
    s.jumped(p);
    let ev = heavy_for(&mut s, p, 0.5);
    let (dir, speed) = thrown(&ev).expect("thrown back");
    assert!((dir - WEST).abs() < 0.05, "back, away from where she faces: {dir}");
    assert!((speed * 0.35 - 4.0).abs() < 0.1, "4 yd: {speed}");
    run(&mut s, 0.6);
    // Again in the same jump: not again.
    let ev = heavy_for(&mut s, p, 0.5);
    assert!(thrown(&ev).is_none(), "once a jump");
    s.landed(p);
    run(&mut s, 0.6);
    s.jumped(p);
    let ev = heavy_for(&mut s, p, 0.5);
    assert!(thrown(&ev).is_some(), "a new jump, a new throw");
}

// --- the holy staff: Healing Gust on the right click ----------------------------------------------

#[test]
fn the_holy_staffs_right_click_held_aims_healing_gust_and_never_raises_a_guard() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "acolyte_staff");
    let w = s.weapon_of(&s.players[&p]).clone();
    assert!(w.has_alt() && w.alt == "healing_gust", "its right click is Healing Gust");
    s.input(p, Input::Guard);
    let ev = s.step(0.0);
    let g = idx(&s, "healing_gust");
    assert!(matches!(s.players[&p].action, Action::Aim { ability } if ability == g), "aiming: {:?}", s.players[&p].action);
    assert!(missiles(&ev, p).is_empty(), "nothing goes on the right click alone");
    // Let go without a click: the aim ends, nothing spent.
    s.input(p, Input::GuardUp);
    s.step(0.0);
    assert!(matches!(s.players[&p].action, Action::Ready), "the aim let go: {:?}", s.players[&p].action);
    assert_eq!(s.cooldown(p, g).charges, 2, "no charge spent");

    // A wolf's bite on the staff's holder, right mouse held as it lands: nothing stops it.
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, WEST, "acolyte_staff");
    let k = wolf_on(&mut s, p, [2.0, 0.0, 0.0]);
    let full = health(&s, p);
    until_bite(&mut s, k, 0.05);
    s.input(p, Input::Guard);
    let ev = run(&mut s, 0.3);
    assert!(!matches!(s.players[&p].action, Action::Guard), "still no guard");
    let out = outcomes(&ev, p);
    assert!(out.contains(&Outcome::Hit), "bitten: {out:?}");
    assert!(!out.iter().any(|o| matches!(o, Outcome::Parried | Outcome::Blocked)), "not parried, not blocked: {out:?}");
    assert!(health(&s, p) < full);
}

#[test]
fn healing_gust_heals_every_ally_it_passes_for_10_percent_and_passes_foes_by() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "acolyte_staff");
    let d = mob(&mut s, "training_dummy", [5.0, 0.0, 0.0]);
    let f = hero(&mut s, "Bren", [10.0, 0.0, 0.0], WEST);
    let g = hero(&mut s, "Cara", [16.0, 0.0, 0.0], WEST);
    hurt(&mut s, f, 100);
    hurt(&mut s, g, 100);
    let hd = hp(&s, d);
    let valor = s.players[&p].valor;
    let ev = gust(&mut s, p);
    assert_eq!(missiles(&ev, p), vec![6685], "a wave of light");
    let gi = idx(&s, "healing_gust");
    assert!(s.cooldown(p, gi).charges == 1, "one of its 2 charges spent");
    let ev = run(&mut s, 1.5);
    assert_eq!(hp(&s, d), hd, "through the dummy, untouched");
    assert_eq!(health(&s, f), 120, "Bren: 10% of 200");
    assert_eq!(health(&s, g), 120, "and Cara behind him");
    assert!(missiles(&ev, p).contains(&2061), "Flash Heal's look on them");
    assert!(s.players[&p].valor > valor, "Valor for the healing");
    assert!(!matches!(s.players[&p].action, Action::Guard), "no guard ever");
}

#[test]
fn healing_gust_that_meets_no_ally_heals_the_priest_as_it_fades() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "acolyte_staff");
    hurt(&mut s, p, 100);
    gust(&mut s, p);
    run(&mut s, 2.5);
    assert_eq!(health(&s, p), 115, "7.5% of 200 back to her");
    // One that meets a friend: nothing to her.
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "acolyte_staff");
    let f = hero(&mut s, "Bren", [8.0, 0.0, 0.0], WEST);
    hurt(&mut s, p, 100);
    hurt(&mut s, f, 100);
    gust(&mut s, p);
    run(&mut s, 2.5);
    assert_eq!(health(&s, f), 120);
    assert_eq!(health(&s, p), 100, "it met him: nothing to her");
}

#[test]
fn healing_gust_holds_2_charges_10_sec_each() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "acolyte_staff");
    let g = idx(&s, "healing_gust");
    let ev = gust(&mut s, p);
    assert!(missiles(&ev, p).contains(&6685));
    run(&mut s, 0.6);
    let ev = gust(&mut s, p);
    assert!(missiles(&ev, p).contains(&6685), "the second");
    run(&mut s, 0.6);
    let ev = gust(&mut s, p);
    assert!(!missiles(&ev, p).contains(&6685), "no third");
    assert_eq!(s.cooldown(p, g).charges, 0);
    run(&mut s, 9.0);
    assert_eq!(s.cooldown(p, g).charges, 1, "one back 10 sec after the first");
}

// --- the shadow staff ---------------------------------------------------------------------------

#[test]
fn shadow_bolts_weave_stacks_into_their_foe_each_8_percent_stronger_up_to_5() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "shadowcaller_staff");
    let d = mob(&mut s, "training_dummy", [10.0, 0.0, 0.0]);
    // (The first bolt, after a long while without one, is Empowered: wait that out here.)
    s.light.last_bolt.insert(p, s.clock);
    let mut dealt = Vec::new();
    for _ in 0..6 {
        let before = hp(&s, d);
        let ev = click(&mut s, p);
        assert_eq!(missiles(&ev, p), vec![686], "Shadow Bolt's look");
        run(&mut s, 0.6);
        dealt.push(before - hp(&s, d));
    }
    // 9, then 8% more a stack: 9.72, 10.44, 11.16, 11.88, then 12.6 at five.
    assert_eq!(dealt, vec![9, 10, 10, 11, 12, 13], "stronger with each stack");
    assert_eq!(marks(&s, p, d, "weave"), 5, "5 at most");
    run(&mut s, 6.2);
    assert_eq!(marks(&s, p, d, "weave"), 0, "gone 6 sec after the last");
}

#[test]
fn a_bolt_after_2_sec_of_patience_is_empowered() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "shadowcaller_staff");
    let d = mob(&mut s, "training_dummy", [10.0, 0.0, 0.0]);
    s.light.last_bolt.insert(p, s.clock);
    click(&mut s, p);
    run(&mut s, 0.6);
    // Patience: 2 sec without a bolt, the staff darkens.
    let ev = run(&mut s, 1.2);
    assert!(!ev.iter().any(|e| matches!(e, Event::PriestLook { kind: "empower", n: 1, .. })), "not yet");
    let ev = run(&mut s, 0.3);
    assert!(ev.iter().any(|e| matches!(e, Event::PriestLook { player, kind: "empower", n: 1, .. } if *player == p)), "Empowered, told");
    assert!(s.auras_of(p).contains(&15473), "the dark swirl on her");
    let before = hp(&s, d);
    let valor = s.players[&p].valor;
    click(&mut s, p);
    let pr = s.projectiles.iter().find(|x| x.owner == p).expect("the bolt");
    let base = &s.content.abilities[idx(&s, "shadowcaller_bolt")];
    assert!((pr.speed - base.speed * 1.5).abs() < 0.01, "half again as fast: {}", pr.speed);
    run(&mut s, 0.6);
    // 9 x 1.5, and the one stack it wove: 8% more.
    assert_eq!(before - hp(&s, d), (9.0f32 * 1.5 * 1.08).round() as u32, "half again as hard");
    assert!(s.players[&p].valor >= valor + 4.0, "4 Valor on the hit");
    assert!(!s.auras_of(p).contains(&15473), "the swirl gone with it");
    // Its reach: 10 yd farther.
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "shadowcaller_staff");
    let far = mob(&mut s, "training_dummy", [36.0, 0.0, 0.0]);
    let hf = hp(&s, far);
    s.light.last_bolt.insert(p, s.clock);
    click(&mut s, p);
    run(&mut s, 1.5);
    assert_eq!(hp(&s, far), hf, "a plain bolt falls short of 36 yd");
    run(&mut s, 1.0);
    click(&mut s, p);
    run(&mut s, 1.5);
    assert!(hp(&s, far) < hf, "an Empowered one reaches it");
}

#[test]
fn dark_reckoning_rips_the_weaving_out_for_7_a_stack_and_with_none_fizzles() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "shadowcaller_staff");
    let d = mob(&mut s, "training_dummy", [10.0, 0.0, 0.0]);
    let w = s.weapon_of(&s.players[&p]).clone();
    assert_eq!(w.alt, "dark_reckoning", "its right click");
    s.light.last_bolt.insert(p, s.clock);
    for _ in 0..3 {
        click(&mut s, p);
        run(&mut s, 0.6);
    }
    assert_eq!(marks(&s, p, d, "weave"), 3);
    let before = hp(&s, d);
    s.input(p, Input::Guard);
    let ev = s.step(0.0);
    s.input(p, Input::GuardUp);
    let dr_spell = s.content.abilities[idx(&s, "dark_reckoning")].spell;
    assert!(missiles(&ev, p).contains(&dr_spell), "the reckoning flies");
    assert!(!matches!(s.players[&p].action, Action::Guard | Action::Aim { .. }), "no guard, no aim");
    run(&mut s, 0.6);
    assert_eq!(before - hp(&s, d), 21, "7 x 3");
    assert_eq!(marks(&s, p, d, "weave"), 0, "ripped out");
    let dr = idx(&s, "dark_reckoning");
    assert_eq!(s.cooldown(p, dr).charges, 0, "8 sec cooldown");
    // Again at once: not ready.
    let ev = { s.input(p, Input::Guard); let e = s.step(0.0); s.input(p, Input::GuardUp); e };
    assert!(!missiles(&ev, p).contains(&dr_spell), "not before its 8 sec");
    // Nothing woven: it fizzles, no damage.
    run(&mut s, 8.0);
    let before = hp(&s, d);
    s.input(p, Input::Guard);
    s.step(0.0);
    s.input(p, Input::GuardUp);
    let ev = run(&mut s, 0.6);
    assert_eq!(hp(&s, d), before, "nothing to rip out: nothing dealt");
    let fizzle = s.content.checks.fizzle_kit;
    assert!(ev.iter().any(|e| matches!(e, Event::Visual { id, kit, .. } if *id == d && *kit == fizzle)), "a fizzle on it");
}

#[test]
fn shadow_burst_let_go_on_its_flare_is_a_priest_perfect() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "shadowcaller_staff");
    let d = mob(&mut s, "training_dummy", [10.0, 0.0, 0.0]);
    let before = hp(&s, d);
    let mut ev = heavy_on_flare(&mut s, p, 0.0);
    ev.extend(run(&mut s, 1.0));
    assert!(perfect(&ev, p));
    assert!(priest_burst(&s, &ev, p, "shadowcaller_bolt_heavy") && !mage_burst(&ev), "the Priest's burst");
    assert!(before - hp(&s, d) >= 26, "26 at full, more for its perfect: {}", before - hp(&s, d));
}

// --- the Staff of the Hollow Star -----------------------------------------------------------------

/// The click held: the Void Tether to what's under the reticle.
fn hold_tether(s: &mut Survival, p: Id) {
    s.input(p, Input::Attack);
    s.step(0.0);
}

#[test]
fn the_void_tether_held_on_a_foe_ramps_from_6_to_12_a_sec_and_ends_on_the_let_go() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "hollow_star_staff");
    let d = post(&mut s, [10.0, 0.0, 0.0]);
    let full = hp(&s, d);
    hold_tether(&mut s, p);
    assert!(matches!(s.players[&p].action, Action::Channel { target, .. } if target == d), "bound to it: {:?}", s.players[&p].action);
    run_pinned(&mut s, 1.0, &[d]);
    let first = full - hp(&s, d);
    assert!((6..=10).contains(&first), "the first second, 6 rising: {first}");
    run_pinned(&mut s, 1.0, &[d]);
    let at2 = hp(&s, d);
    run_pinned(&mut s, 1.0, &[d]);
    let third = at2 - hp(&s, d);
    assert!((11..=13).contains(&third), "12 a sec once ramped: {third}");
    s.input(p, Input::AttackUp);
    s.step(0.0);
    assert!(!matches!(s.players[&p].action, Action::Channel { .. }), "let go, it ends");
    let after = hp(&s, d);
    run_pinned(&mut s, 1.0, &[d]);
    assert_eq!(hp(&s, d), after, "nothing more");
}

#[test]
fn the_void_tether_on_a_friend_heals_4_rising_to_8() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "hollow_star_staff");
    let f = hero(&mut s, "Bren", [10.0, 0.0, 0.0], WEST);
    hurt(&mut s, f, 100);
    hold_tether(&mut s, p);
    assert!(matches!(s.players[&p].action, Action::Channel { target, .. } if target == f), "bound to Bren: {:?}", s.players[&p].action);
    run(&mut s, 2.0);
    let ramp = health(&s, f) - 100;
    assert!((10..=14).contains(&ramp), "4 rising to 8 over 2 sec: {ramp}");
    run(&mut s, 1.0);
    let held = health(&s, f) - 100 - ramp;
    assert!((7..=9).contains(&held), "8 a sec: {held}");
}

#[test]
fn the_void_tether_on_your_own_orb_fills_it_and_draws_it_in() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "hollow_star_staff");
    s.input(p, Input::Guard);
    s.step(0.0);
    s.input(p, Input::GuardUp);
    let orb = s.light.hollows.iter().find(|h| h.owner == p).expect("the orb out").mob;
    let at = s.mobs[&orb].pos;
    let before = hp(&s, orb);
    run(&mut s, 0.6);
    // Aim at it (it floats a yard up), and hold.
    let d = dist2d(ORIGIN, at);
    s.set_aim_line(p, EAST, (1.0f32 - 1.9).atan2(d));
    hold_tether(&mut s, p);
    assert!(matches!(s.players[&p].action, Action::Channel { target, .. } if target == orb), "bound to her orb: {:?}", s.players[&p].action);
    run(&mut s, 1.0);
    let filled = hp(&s, orb) as i32 - before as i32;
    assert!((5..=7).contains(&filled), "6 a sec: {filled}");
    assert!(dist2d(ORIGIN, s.mobs[&orb].pos) < d - 2.0, "drawn toward her, 3 yd a sec: {:?}", s.mobs[&orb].pos);
}

#[test]
fn the_hollow_star_is_an_orb_that_hurts_foes_and_heals_friends_and_fills_and_drains_by_it() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "hollow_star_staff");
    let w = s.weapon_of(&s.players[&p]).clone();
    assert_eq!(w.alt, "hollow_star", "its right click");
    s.input(p, Input::Guard);
    let ev = s.step(0.0);
    s.input(p, Input::GuardUp);
    assert!(!matches!(s.players[&p].action, Action::Guard), "no guard");
    let h = s.light.hollows.iter().find(|h| h.owner == p).expect("the orb out").clone();
    let at = s.mobs[&h.mob].pos;
    assert!((dist2d(ORIGIN, at) - 15.0).abs() < 0.6, "15 yd along the reticle: {at:?}");
    assert_eq!((hp(&s, h.mob), s.mobs[&h.mob].max_health), (40, 60), "40 health, 60 at most");
    assert!(ev.iter().any(|e| matches!(e, Event::Grow { id, .. } if *id == h.mob)), "its bubble, sized by its health");
    // A foe 2 yd from it, a hurt friend 2 yd the other side.
    let g = post(&mut s, [at[0], at[1] + 2.0, 0.0]);
    let f = hero(&mut s, "Bren", [at[0], at[1] - 2.0, 0.0], WEST);
    hurt(&mut s, f, 100);
    let hg = hp(&s, g);
    run_pinned(&mut s, 1.05, &[g]);
    assert_eq!(hg - hp(&s, g), 5, "5 to the foe each second");
    assert_eq!(health(&s, f), 105, "5 to the friend");
    assert_eq!(hp(&s, h.mob), 40, "filled 5, drained 5");
    assert!(s.cc_of(f).iter().any(|c| c.kind == "haste" && (c.amount - 0.15).abs() < 1.0e-3), "15% faster standing in it");
    // The friend gone: it fills on what it deals.
    s.move_player(f, [60.0, 0.0, 0.0], WEST, Moving::default());
    run_pinned(&mut s, 1.0, &[g]);
    assert_eq!(hp(&s, h.mob), 45, "filled by the 5 it dealt");
    // It lasts 10 sec.
    run_pinned(&mut s, 7.7, &[g]);
    assert_eq!(s.light.hollows.len(), 1, "still out just before 10 sec");
    run_pinned(&mut s, 0.4, &[g]);
    assert!(s.light.hollows.is_empty(), "gone after 10 sec");
    assert!(!s.mobs.contains_key(&h.mob), "its body gone with it");
}

#[test]
fn a_hollow_stars_reach_grows_from_3_to_5_yd_as_it_fills() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "hollow_star_staff");
    s.input(p, Input::Guard);
    s.step(0.0);
    s.input(p, Input::GuardUp);
    let orb = s.light.hollows[0].mob;
    let at = s.mobs[&orb].pos;
    // At 40 of 60: 3 + 2 x 2/3 = 4.3 yd (and half a grunt's width). A foe at 5.2 is out of it;
    // at full (5 yd) it's in.
    let g = post(&mut s, [at[0] + 5.2, at[1], 0.0]);
    let hg = hp(&s, g);
    run_pinned(&mut s, 1.05, &[g]);
    assert_eq!(hp(&s, g), hg, "out of its reach at 40");
    s.mobs.get_mut(&orb).unwrap().health = 60;
    run_pinned(&mut s, 1.0, &[g]);
    assert_eq!(hg - hp(&s, g), 5, "in it at 60");
}

#[test]
fn the_void_tether_fills_the_orb_6_a_sec_no_more() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "hollow_star_staff");
    s.input(p, Input::Guard);
    s.step(0.0);
    s.input(p, Input::GuardUp);
    let orb = s.light.hollows[0].mob;
    let at = s.mobs[&orb].pos;
    run(&mut s, 0.6);
    s.set_aim_line(p, EAST, (1.0f32 - 1.9).atan2(dist2d(ORIGIN, at)));
    hold_tether(&mut s, p);
    assert!(matches!(s.players[&p].action, Action::Channel { target, .. } if target == orb), "bound to her orb: {:?}", s.players[&p].action);
    let before = hp(&s, orb);
    // Eight ticks of a quarter second: 2 sec, 12.
    let mut ticks = 0;
    let mut last = before;
    let start = s.clock;
    while ticks < 8 && s.clock < start + 3.0 {
        s.step(DT);
        if hp(&s, orb) != last {
            ticks += 1;
            last = hp(&s, orb);
        }
    }
    assert_eq!(hp(&s, orb) - before, 12, "6 a second over 2 sec");
}

#[test]
fn the_void_tether_holds_to_23_yd_and_breaks_past_it() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "hollow_star_staff");
    let d = post(&mut s, [10.0, 0.0, 0.0]);
    hold_tether(&mut s, p);
    assert!(matches!(s.players[&p].action, Action::Channel { target, .. } if target == d));
    s.mobs.get_mut(&d).unwrap().pos = [22.6, 0.0, 0.0];
    run_pinned(&mut s, 0.5, &[d]);
    assert!(matches!(s.players[&p].action, Action::Channel { .. }), "held at 22.6 yd: {:?}", s.players[&p].action);
    s.mobs.get_mut(&d).unwrap().pos = [23.6, 0.0, 0.0];
    run_pinned(&mut s, 0.5, &[d]);
    assert!(!matches!(s.players[&p].action, Action::Channel { .. }), "broken past 23 yd");
}

#[test]
fn a_greater_smite_on_a_marked_foe_tears_searing_light_out_too() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "acolyte_staff");
    let d = post(&mut s, [10.0, 0.0, 0.0]);
    click(&mut s, p);
    run_pinned(&mut s, 1.0, &[d]);
    assert_eq!(marks(&s, p, d, "sear"), 1);
    let before = hp(&s, d);
    s.input(p, Input::Heavy);
    s.step(0.0);
    s.players.get_mut(&p).unwrap().check_flare = 0.75;
    run_pinned(&mut s, 1.05, &[d]);
    s.input(p, Input::AttackUp);
    s.step(0.0);
    run_pinned(&mut s, 1.0, &[d]);
    assert_eq!(before - hp(&s, d), 30, "22 + 8");
    assert_eq!(marks(&s, p, d, "sear"), 0, "torn out");
}

#[test]
fn one_hollow_star_at_a_time_and_a_foe_can_break_it() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "hollow_star_staff");
    s.set_cooldowns(p, true);
    s.input(p, Input::Guard);
    s.step(0.0);
    s.input(p, Input::GuardUp);
    run(&mut s, 0.6);
    let first = s.light.hollows[0].mob;
    s.set_aim_line(p, 1.0, 0.0);
    s.input(p, Input::Guard);
    s.step(0.0);
    s.input(p, Input::GuardUp);
    run(&mut s, 0.1);
    assert_eq!(s.light.hollows.len(), 1, "one at a time");
    assert_ne!(s.light.hollows[0].mob, first, "the new one");
    assert!(!s.mobs.contains_key(&first), "the old one gone");
    // A creature's blows break it.
    let orb = s.light.hollows[0].mob;
    let g = mob(&mut s, "grunt", [30.0, 30.0, 0.0]);
    s.hurt_mob(orb, g, 100);
    let ev = run(&mut s, 0.1);
    assert!(s.light.hollows.is_empty(), "broken");
    assert!(ev.iter().any(|e| matches!(e, Event::OrbGone { .. })), "its look gone");
    assert_eq!(s.content.abilities[idx(&s, "hollow_star")].cooldown, 18.0);
}

#[test]
fn the_priests_staffs_are_on_the_rack_and_none_guards() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let rack: Vec<String> = s.rack_view(p).into_iter().map(|(k, _, _, _, _)| k).collect();
    for staff in ["acolyte_staff", "shadowcaller_staff", "hollow_star_staff"] {
        assert!(rack.contains(&staff.to_string()), "{staff} on the rack");
        let mut s = world();
        let p = armed(&mut s, "Ayla", ORIGIN, EAST, staff);
        s.input(p, Input::Guard);
        s.step(0.0);
        assert!(!matches!(s.players[&p].action, Action::Guard), "{staff}: the right click never guards");
    }
}

// --- Power Word: Shield ------------------------------------------------------------------------

/// A Priest with the Shield on key 2, 8 yd west of Bren (both facing each other along the
/// line), and a wolf 2 yd east of Bren, its bite on its way to him.
fn shield_set_up(s: &mut Survival) -> (Id, Id, Id) {
    let p = hero(s, "Ayla", [-8.0, 0.0, 0.0], EAST);
    s.slot_ability(p, "defense", "power_word_shield").unwrap();
    let f = hero(s, "Bren", ORIGIN, EAST);
    let k = wolf_on(s, f, [2.0, 0.0, 0.0]);
    (p, f, k)
}

#[test]
fn power_word_shield_is_key_2_and_absorbs_30_for_the_ally_under_the_reticle() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    s.slot_ability(p, "defense", "power_word_shield").unwrap();
    let f = hero(&mut s, "Bren", [8.0, 0.0, 0.0], WEST);
    let sh = idx(&s, "power_word_shield");
    assert_eq!(s.content.ex_of(sh), Some(idx(&s, "power_word_barrier")), "the Barrier is its EX");
    let ev = shield(&mut s, p);
    assert!(
        ev.iter().any(|e| matches!(e, Event::Missile { caster, spell: 17, target: Target::Unit(t) } if *caster == p && *t == f)),
        "its bubble on him"
    );
    assert!((shield_left(&s, f) - 30.0).abs() < 0.01, "30 to absorb: {}", shield_left(&s, f));
    assert_eq!(shield_left(&s, p), 0.0, "not her");
    assert_eq!(s.cooldown(p, sh).charges, 1, "one of its 2 charges left");
    run(&mut s, 0.5);
    let ev = shield(&mut s, p);
    assert!(missiles(&ev, p).contains(&17), "the second at once");
    assert_eq!(s.cooldown(p, sh).charges, 0);
    let k = mob(&mut s, "grunt", [10.0, 0.0, 0.0]);
    let full = health(&s, f);
    s.strike_player(k, f, 20, [10.0, 0.0, 0.0], true, false, true, Blow::Light);
    assert_eq!(health(&s, f), full, "absorbed");
    run(&mut s, 6.5);
    s.strike_player(k, f, 20, [10.0, 0.0, 0.0], true, false, true, Blow::Light);
    assert!(health(&s, f) < full, "6 sec on, gone");
    // Alone, on herself.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    s.slot_ability(p, "defense", "power_word_shield").unwrap();
    shield(&mut s, p);
    assert!((shield_left(&s, p) - 30.0).abs() < 0.01, "on herself with no one under the reticle");
}

#[test]
fn a_shield_on_an_ally_as_the_wolfs_bite_lands_parries_it_for_him_in_the_priests_light() {
    let mut s = world();
    let (p, f, k) = shield_set_up(&mut s);
    let full = health(&s, f);
    let mut ev = until_bite(&mut s, k, 0.05);
    let valor = s.players[&p].valor;
    ev.extend(shield(&mut s, p));
    assert!(shield_left(&s, f) > 29.0, "shielded");
    ev.extend(run(&mut s, 0.2));
    assert!(outcomes(&ev, f).contains(&Outcome::Parried), "parried for him: {:?}", outcomes(&ev, f));
    assert_eq!(health(&s, f), full, "nothing landed");
    assert!(shield_left(&s, f) > 29.0, "the shield stays: {}", shield_left(&s, f));
    assert!(s.mobs[&k].stagger_until > s.clock, "the wolf reels");
    assert!(perfect(&ev, p), "the Priest's perfect");
    assert!(!perfect(&ev, f), "not his");
    assert!(s.players[&p].valor >= valor + s.content.valor.counter, "a counter's Valor");
    assert!(priest_burst(&s, &ev, f, "power_word_shield"), "the Priest's holy burst on him, for everyone");
    assert!(!mage_burst(&ev), "never Arcane Explosion's");
}

#[test]
fn a_shield_too_early_or_too_late_only_absorbs() {
    let mut s = world();
    let (p, f, k) = shield_set_up(&mut s);
    let mut ev = until_bite(&mut s, k, 0.3);
    ev.extend(shield(&mut s, p));
    ev.extend(run(&mut s, 0.5));
    assert!(!outcomes(&ev, f).contains(&Outcome::Parried), "not parried");
    assert!(!perfect(&ev, p), "no perfect");
    assert!(shield_left(&s, f) < 29.0 && shield_left(&s, f) > 0.0, "the bite absorbed: {}", shield_left(&s, f));

    let mut s = world();
    let (p, f, k) = shield_set_up(&mut s);
    let full = health(&s, f);
    let mut ev = until_bite(&mut s, k, 0.0);
    ev.extend(run(&mut s, 0.12));
    ev.extend(shield(&mut s, p));
    ev.extend(run(&mut s, 0.2));
    assert!(health(&s, f) < full, "bitten");
    assert!(!perfect(&ev, p), "no perfect");
}

#[test]
fn a_shield_on_yourself_never_parries() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, WEST);
    s.slot_ability(p, "defense", "power_word_shield").unwrap();
    let k = wolf_on(&mut s, p, [2.0, 0.0, 0.0]);
    let full = health(&s, p);
    let mut ev = until_bite(&mut s, k, 0.05);
    ev.extend(shield(&mut s, p));
    assert!(shield_left(&s, p) > 29.0, "on herself");
    ev.extend(run(&mut s, 0.3));
    assert!(!outcomes(&ev, p).contains(&Outcome::Parried), "not parried");
    assert!(!perfect(&ev, p), "no perfect");
    assert_eq!(health(&s, p), full, "only absorbed");
}

#[test]
fn the_shield_is_judged_fairly_with_a_round_trip() {
    let mut s = world();
    let (p, f, k) = shield_set_up(&mut s);
    s.set_rtt(p, 0.2);
    let mut ev = until_bite(&mut s, k, 0.25);
    ev.extend(shield(&mut s, p));
    ev.extend(run(&mut s, 0.4));
    assert!(outcomes(&ev, f).contains(&Outcome::Parried), "parried: {:?}", outcomes(&ev, f));
    assert!(perfect(&ev, p));

    let mut s = world();
    let (p, f, k) = shield_set_up(&mut s);
    s.set_rtt(f, 0.18);
    let full = health(&s, f);
    let mut ev = until_bite(&mut s, k, 0.0);
    ev.extend(s.step(DT));
    ev.extend(run(&mut s, 0.1));
    assert_eq!(health(&s, f), full, "still waiting on him");
    ev.extend(shield(&mut s, p));
    ev.extend(run(&mut s, 0.3));
    assert!(outcomes(&ev, f).contains(&Outcome::Parried), "parried: {:?}", outcomes(&ev, f));
    assert!(perfect(&ev, p));
}

#[test]
fn every_priest_spell_bursts_in_the_priests_light_never_the_mages() {
    let s = world();
    for (k, d) in s.content.abilities.iter().enumerate() {
        let base = s.content.base_of(k);
        if s.content.abilities[base].school == "priest" {
            let kit = s.perfect_kit_of(k);
            assert!(kit != 0 && kit != s.content.checks.perfect_kit && kit != MAGE_KIT, "{}: the Priest's own burst, not {kit}", d.key);
        }
    }
    assert_eq!(s.content.checks.perfect_kit, MAGE_KIT, "(the default every other perfect shows)");
}

// --- saves -------------------------------------------------------------------------------------

#[test]
fn an_old_save_with_the_barrier_on_key_2_loads_with_the_shield_there_and_the_barrier_its_ex() {
    let mut s = world();
    s.content.dev.full_tree = false;
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let mut save = s.leave(p).unwrap();
    save.owned = ["penance", "holy_nova", "power_word_barrier"].into_iter().map(String::from).collect();
    save.loadout = [("ranged", "penance"), ("defense", "power_word_barrier")].into_iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
    let q = s.join("ayla", "Ayla", Some(&save));
    let h = &s.players[&q];
    assert!(h.owned.contains("power_word_shield"), "owns the Shield: {:?}", h.owned);
    assert_eq!(h.loadout.get("defense").map(String::as_str), Some("power_word_shield"));
}

#[test]
fn a_draft_2_save_loads_with_draft_3s_cards() {
    let mut s = world();
    s.content.dev.full_tree = false;
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let mut save = s.leave(p).unwrap();
    save.owned = ["leap_of_faith", "dispersion", "mind_flay", "mind_blast", "fade"].into_iter().map(String::from).collect();
    save.loadout = [("ranged", "mind_flay"), ("close", "mind_blast"), ("utility", "leap_of_faith"), ("defense", "dispersion")]
        .into_iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect();
    let q = s.join("ayla", "Ayla", Some(&save));
    let h = &s.players[&q];
    for k in ["angelic_feather", "shadowguard", "mind_blast", "psychic_scream", "fade"] {
        assert!(h.owned.contains(k), "owns {k}: {:?}", h.owned);
    }
    for k in ["leap_of_faith", "dispersion", "mind_flay"] {
        assert!(!h.owned.contains(k), "{k} is gone: {:?}", h.owned);
    }
    let on = |slot: &str| h.loadout.get(slot).map(String::as_str);
    assert_eq!(on("ranged"), Some("mind_blast"), "Mind Flay on Q is Mind Blast now");
    assert_eq!(on("close"), Some("psychic_scream"), "Mind Blast on E is Psychic Scream now");
    assert_eq!(on("utility"), Some("angelic_feather"));
    assert_eq!(on("defense"), Some("shadowguard"));
}

// --- the board's budgets ------------------------------------------------------------------------

#[test]
fn every_priest_card_keeps_to_the_boards_budgets() {
    use crate::tests_classes::{board, hard_cc, worth, worth_with_spray};
    let s = world();
    let c = &s.content;
    let mut wrong = Vec::new();
    for (k, ex) in board(c) {
        let d = &c.abilities[k];
        if d.school != "priest" {
            continue;
        }
        let per = d.cooldown / d.charges.max(1) as f32 + d.charge[1];
        match d.slot.as_str() {
            "ranged" => {
                if worth(d) > 45.0 || worth(d) / per.max(1.0) > 9.5 {
                    wrong.push(format!("{}: a poke of {} a hit, {:.1} a second", d.key, worth(d), worth(d) / per.max(1.0)));
                }
            }
            "close" if worth(d) > 45.0 => wrong.push(format!("{}: {}", d.key, worth(d))),
            "utility" if worth(d) > 20.0 || per < 10.0 => wrong.push(format!("{}: a utility back every {per:.1} sec", d.key)),
            "defense" if per < 12.0 => wrong.push(format!("{}: a defense back every {per:.1} sec", d.key)),
            "ultimate" if d.valor != 100 || d.cooldown < 40.0 || worth(d) > 110.0 => wrong.push(format!("{}: an ultimate", d.key)),
            _ => {}
        }
        for card in [Some(k), ex].into_iter().flatten().map(|x| &c.abilities[x]) {
            if hard_cc(card) > 1.5 + 1.0e-3 {
                wrong.push(format!("{}: {} s of hard crowd control on a hero", card.key, hard_cc(card)));
            }
            if card.is_ex() && (![25, 50].contains(&card.valor) || worth_with_spray(c, card) > worth(d) * 1.8 + 20.0) {
                wrong.push(format!("{}: an EX out of its league", card.key));
            }
        }
    }
    assert!(wrong.is_empty(), "over budget: {wrong:#?}");
}

