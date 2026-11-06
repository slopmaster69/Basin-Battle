//! The Shadow Priest, draft 3 (docs/priest-draft3.md): Mind Blast's sigil (lead your target, its
//! heart the perfect) and Shadow Word: Death's judgement; Psychic Scream and Horror; Fade's
//! sidestep; Shadowguard (take it, give it back) and Vampiric Touch; Void Rift (gather, then
//! implode); where Shadow is taught.

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
/// The reticle's eye height above the hero's feet (`abilities::EYE`).
const EYE: f32 = 1.9;

fn world() -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    c.dev.full_tree = true;
    c.player.field_regen = 0.0;
    c.player.rest_regen = 0.0;
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 7, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

/// [`world`] with heroes' blows and spells striking each other (`[pvp] friendly_fire`).
fn world_pvp() -> Survival {
    let mut s = world();
    s.content.pvp = Content::dev().pvp;
    assert!(s.content.pvp.friendly_fire, "the rules' pvp has friendly fire on");
    s
}

fn hero(s: &mut Survival, name: &str, pos: Vec3, facing: f32) -> Id {
    let id = s.join(&name.to_lowercase(), name, None);
    s.move_player(id, pos, facing, Moving::default());
    let _ = s.step(0.0);
    s.players.get_mut(&id).unwrap().cd_off = false;
    id
}

/// A grunt standing idle where it's put, never swinging (1000 health with `big`).
fn grunt(s: &mut Survival, pos: Vec3, big: bool) -> Id {
    let def = s.content.mob("grunt").unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.ready_at = 1e9;
    m.pos = pos;
    m.facing = WEST;
    if big {
        m.max_health = 1000;
        m.health = 1000;
    }
    id
}

/// Creatures held idle where they are (struck, they'd come for the hero).
fn pin(s: &mut Survival, pins: &[(Id, Vec3)]) {
    for (m, at) in pins {
        if let Some(x) = s.mobs.get_mut(m) {
            x.state = MobState::Idle { wander_at: 1e9 };
            x.path.clear();
            x.pos = *at;
        }
    }
}

/// Creatures kept from walking (idle, no path), wherever something drags them.
fn hold(s: &mut Survival, ids: &[Id]) {
    for m in ids {
        if let Some(x) = s.mobs.get_mut(m) {
            x.state = MobState::Idle { wander_at: 1e9 };
            x.path.clear();
        }
    }
}

fn run_pinned(s: &mut Survival, secs: f32, pins: &[(Id, Vec3)]) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        pin(s, pins);
        out.extend(s.step(DT));
    }
    pin(s, pins);
    out
}

fn run_held(s: &mut Survival, secs: f32, ids: &[Id]) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        hold(s, ids);
        out.extend(s.step(DT));
    }
    out
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    run_pinned(s, secs, &[])
}

fn key_of(slot: &str) -> u8 {
    crate::content::SLOTS.iter().position(|s| *s == slot).unwrap() as u8
}

fn idx(s: &Survival, key: &str) -> usize {
    s.content.ability(key).unwrap()
}

/// Own a card and put it on its key.
fn learn(s: &mut Survival, p: Id, key: &str) {
    let slot = s.content.abilities[idx(s, key)].slot.clone();
    s.players.get_mut(&p).unwrap().owned.insert(key.to_string());
    s.slot_ability(p, &slot, key).unwrap();
}

/// A card learned and used (its EX version with `ex`, Valor enough for it); an aimed one goes on
/// a tap of the click.
fn cast(s: &mut Survival, p: Id, key: &str, ex: bool) -> Vec<Event> {
    learn(s, p, key);
    let slot = s.content.abilities[idx(s, key)].slot.clone();
    if ex {
        s.set_valor(p, 100.0);
    }
    s.input(p, Input::Ability(key_of(&slot), ex));
    let mut ev = s.step(0.0);
    if matches!(s.players[&p].action, Action::Aim { .. }) {
        s.input(p, Input::Attack);
        s.input(p, Input::AttackUp);
        ev.extend(s.step(0.0));
    }
    ev
}

fn press(s: &mut Survival, p: Id, slot: &str, ex: bool) -> Vec<Event> {
    s.input(p, Input::Ability(key_of(slot), ex));
    s.step(0.0)
}

fn hp(s: &Survival, m: Id) -> u32 {
    s.mobs[&m].health
}

fn perfects(events: &[Event], p: Id) -> Vec<usize> {
    events.iter().filter_map(|e| if let Event::Perfect { player, ability } = e { (*player == p).then_some(*ability) } else { None }).collect()
}

/// Whether the events show the burst of a perfect of `key` on `on`: the Priest's own (the
/// spell's, else `[priest] perfect_kit`), never the checks' default, the Mage's Arcane Explosion.
fn priest_burst(s: &Survival, ev: &[Event], on: Id, key: &str) -> bool {
    let kit = s.perfect_kit_of(idx(s, key));
    assert!(kit != 0 && kit != s.content.checks.perfect_kit && kit != MAGE_KIT, "{key}: a Priest's own burst, not {kit}");
    ev.iter().any(|e| matches!(e, Event::Visual { id, kit: k, .. } if *id == on && *k == kit))
}

fn mage_burst(ev: &[Event]) -> bool {
    ev.iter().any(|e| matches!(e, Event::Visual { kit, .. } if *kit == MAGE_KIT))
}

/// The reticle on a spot on the ground.
fn aim_ground(s: &mut Survival, p: Id, at: Vec3) {
    let from = s.players[&p].pos;
    let d = dist2d(from, at).max(0.5);
    s.set_aim_line(p, facing(from, at), -(EYE + from[2] - at[2]).atan2(d));
}

fn panicked_for(s: &Survival, u: Id) -> f32 {
    s.cc_of(u).iter().find(|c| c.kind == "panic" && c.until > s.clock).map_or(0.0, |c| c.until - s.clock)
}

// --- Mind Blast -----------------------------------------------------------------------------------

/// A Priest at the origin, Mind Blast on the ground at `at`.
fn blast_at(s: &mut Survival, p: Id, at: Vec3) -> Vec<Event> {
    aim_ground(s, p, at);
    cast(s, p, "mind_blast", false)
}

#[test]
fn mind_blast_marks_the_ground_and_erupts_09_sec_later_for_24_within_25_yd() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let near = grunt(&mut s, [12.0, 2.0, 0.0], true);
    let out = grunt(&mut s, [12.0, -3.8, 0.0], true);
    let pins = [(near, [12.0, 2.0, 0.0]), (out, [12.0, -3.8, 0.0])];
    let (hn, ho) = (hp(&s, near), hp(&s, out));
    let mut ev = blast_at(&mut s, p, [12.0, 0.0, 0.0]);
    assert!(ev.iter().any(|e| matches!(e, Event::PriestGround { kind: "blast", .. })), "the dark sigil drawn");
    ev.extend(run_pinned(&mut s, 0.8, &pins));
    assert_eq!(hp(&s, near), hn, "not before 0.9 sec");
    ev.extend(run_pinned(&mut s, 0.2, &pins));
    assert_eq!(hn - hp(&s, near), 24, "24 within 2.5 yd");
    assert_eq!(hp(&s, out), ho, "not 3.8 yd out");
    assert!(perfects(&ev, p).is_empty(), "nothing in its heart: no perfect");
    assert_eq!(s.content.abilities[idx(&s, "mind_blast")].cooldown, 8.0);
}

#[test]
fn a_foe_standing_in_mind_blasts_heart_takes_30_and_no_perfect() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let g = grunt(&mut s, [12.0, 0.0, 0.0], true);
    let side = grunt(&mut s, [12.0, 2.0, 0.0], true);
    let pins = [(g, [12.0, 0.0, 0.0]), (side, [12.0, 2.0, 0.0])];
    let (hg, hs) = (hp(&s, g), hp(&s, side));
    let mut ev = blast_at(&mut s, p, [12.0, 0.0, 0.0]);
    ev.extend(run_pinned(&mut s, 1.0, &pins));
    assert_eq!(hg - hp(&s, g), 30, "the heart, standing still");
    assert_eq!(hs - hp(&s, side), 24, "the one beside it, at the edge");
    assert!(perfects(&ev, p).is_empty(), "a foe standing still is no read: no perfect");
}

/// Runs `secs` with `g` walking a long path but held at `at`: on the move, in the same place.
fn run_walking(s: &mut Survival, secs: f32, g: Id, at: Vec3) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        if let Some(m) = s.mobs.get_mut(&g) {
            m.state = MobState::Idle { wander_at: 1e9 };
            m.pos = at;
            m.path = vec![[at[0], at[1] + 20.0, at[2]]];
            m.speed = m.speed.max(3.0);
        }
        out.extend(s.step(DT));
    }
    out
}

#[test]
fn a_foe_on_the_move_in_mind_blasts_heart_is_struck_true_36_and_5_valor() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let g = grunt(&mut s, [12.0, 0.0, 0.0], true);
    let hg = hp(&s, g);
    let valor = s.players[&p].valor;
    let mut ev = blast_at(&mut s, p, [12.0, 0.0, 0.0]);
    ev.extend(run_walking(&mut s, 1.0, g, [12.0, 0.0, 0.0]));
    assert_eq!(hg - hp(&s, g), 36, "struck true: the read was right");
    assert_eq!(perfects(&ev, p), vec![idx(&s, "mind_blast")], "the perfect, once");
    assert!(s.players[&p].valor >= valor + 5.0, "5 Valor: {} from {valor}", s.players[&p].valor);
    assert!(priest_burst(&s, &ev, p, "mind_blast"), "the Priest's own burst");
    assert!(!mage_burst(&ev));
}

#[test]
fn mind_blast_is_25_percent_harder_on_a_woven_or_shadow_burnt_foe() {
    // Shadow Weaving from the staff's bolts.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let rack = s.objs[&s.rack_id()].pos;
    s.players.get_mut(&p).unwrap().pos = rack;
    s.arm(p, "shadowcaller_staff").unwrap();
    s.players.get_mut(&p).unwrap().pos = ORIGIN;
    let g = grunt(&mut s, [12.0, 0.0, 0.0], true);
    let side = grunt(&mut s, [12.0, 2.0, 0.0], true);
    let pins = [(g, [12.0, 0.0, 0.0]), (side, [12.0, 2.0, 0.0])];
    s.set_aim_line(p, facing(ORIGIN, [12.0, 2.0, 0.0]), 0.0);
    s.input(p, Input::Attack);
    s.input(p, Input::AttackUp);
    run_pinned(&mut s, 1.0, &pins);
    assert!(s.staff_marks.iter().any(|m| m.on == side && m.kind == "weave"), "woven");
    let (hg, hs) = (hp(&s, g), hp(&s, side));
    blast_at(&mut s, p, [12.0, 0.0, 0.0]);
    run_pinned(&mut s, 1.0, &pins);
    assert_eq!(hs - hp(&s, side), 30, "24 x 1.25 on the woven one");
    assert_eq!(hg - hp(&s, g), 30, "the heart, unwoven and still: 30");
    // Woven, in its heart: 30 x 1.25, 38; on the move, 36 x 1.25, 45.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let rack = s.objs[&s.rack_id()].pos;
    s.players.get_mut(&p).unwrap().pos = rack;
    s.arm(p, "shadowcaller_staff").unwrap();
    s.players.get_mut(&p).unwrap().pos = ORIGIN;
    let g = grunt(&mut s, [12.0, 0.0, 0.0], true);
    let pins = [(g, [12.0, 0.0, 0.0])];
    s.set_aim_line(p, EAST, 0.0);
    s.input(p, Input::Attack);
    s.input(p, Input::AttackUp);
    run_pinned(&mut s, 1.0, &pins);
    let hg = hp(&s, g);
    blast_at(&mut s, p, [12.0, 0.0, 0.0]);
    run_pinned(&mut s, 1.0, &pins);
    assert_eq!(hg - hp(&s, g), 38, "30 x 1.25");
    run_pinned(&mut s, 8.0, &pins);
    s.set_aim_line(p, EAST, 0.0);
    s.input(p, Input::Attack);
    s.input(p, Input::AttackUp);
    run_pinned(&mut s, 1.0, &pins);
    let hg = hp(&s, g);
    blast_at(&mut s, p, [12.0, 0.0, 0.0]);
    run_walking(&mut s, 1.0, g, [12.0, 0.0, 0.0]);
    assert_eq!(hg - hp(&s, g), 45, "36 x 1.25: the board's 45 a hit");
    // A Warlock's shadow burning on it.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let g = grunt(&mut s, [12.0, 2.0, 0.0], true);
    let pins = [(g, [12.0, 2.0, 0.0])];
    let rot = s.content.abilities.iter().find(|a| a.school == "warlock").unwrap().name.clone();
    s.apply_cc(g, p, "burn", 6.0, 0.0001, 0.0, &rot);
    let hg = hp(&s, g);
    blast_at(&mut s, p, [12.0, 0.0, 0.0]);
    run_pinned(&mut s, 1.0, &pins);
    assert_eq!(hg - hp(&s, g), 30, "shadowed by a Warlock's burn: 24 x 1.25");
}

// --- Shadow Word: Death ---------------------------------------------------------------------------

/// A grunt of 1000 health at `health`, Shadow Word: Death on it: what it took, what she took, and
/// whether it died.
fn death_on(health: u32) -> (u32, u32, bool) {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    s.set_aim_line(p, EAST, 0.0);
    let m = grunt(&mut s, [12.0, 0.0, 0.0], true);
    let pins = [(m, [12.0, 0.0, 0.0])];
    s.mobs.get_mut(&m).unwrap().health = health;
    let mine = s.players[&p].health;
    cast(&mut s, p, "mind_blast", true);
    run_pinned(&mut s, 0.4, &pins);
    let left = s.mobs.get(&m).map_or(0, |x| x.health);
    (health - left, mine - s.players[&p].health, left == 0)
}

#[test]
fn shadow_word_death_is_18_or_45_below_20_percent_and_bites_back_12_if_it_doesnt_kill() {
    assert_eq!(death_on(1000), (18, 12, false), "18, and 12 back at her");
    assert_eq!(death_on(150), (45, 12, false), "below 20%: 45, and it still lives to bite back");
    let (_, back, dead) = death_on(40);
    assert!(dead && back == 0, "a kill: nothing comes back");
}

#[test]
fn a_kill_with_shadow_word_death_gives_one_free_death_within_3_sec_and_that_one_gives_no_other() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    s.set_aim_line(p, EAST, 0.0);
    let a = grunt(&mut s, [12.0, 0.0, 0.0], true);
    s.mobs.get_mut(&a).unwrap().health = 40;
    let mb = idx(&s, "mind_blast");
    cast(&mut s, p, "mind_blast", true);
    run(&mut s, 0.4);
    assert!(!s.mobs.contains_key(&a) || hp(&s, a) == 0, "killed");
    assert_eq!(s.cooldown(p, mb).charges, 1, "Mind Blast's cooldown cleared");
    // The free one: on the plain key, with no Valor.
    let b = grunt(&mut s, [12.0, 0.0, 0.0], true);
    s.mobs.get_mut(&b).unwrap().health = 30;
    s.set_valor(p, 0.0);
    press(&mut s, p, "ranged", false);
    let swd = idx(&s, "shadow_word_death");
    assert!(matches!(s.players[&p].action, Action::Aim { ability } if ability == swd), "the plain key aims a free Death: {:?}", s.players[&p].action);
    s.input(p, Input::Attack);
    s.input(p, Input::AttackUp);
    s.step(0.0);
    assert!(s.projectiles.iter().any(|x| x.owner == p && x.ability == swd), "it goes with no Valor");
    run(&mut s, 0.4);
    assert!(!s.mobs.contains_key(&b) || hp(&s, b) == 0, "the free one killed too");
    // That kill gives no other: no free Death on the key (and the free one's cast took Mind
    // Blast's cooldown, as a Death does).
    s.set_valor(p, 0.0);
    let _c = grunt(&mut s, [12.0, 0.0, 0.0], true);
    press(&mut s, p, "ranged", false);
    assert!(!matches!(s.players[&p].action, Action::Aim { ability } if ability == swd), "no third Death: {:?}", s.players[&p].action);
    assert!(s.players[&p].combos.empowered.get(&mb).is_none_or(|t| *t <= s.clock), "nothing armed");
}

#[test]
fn the_free_death_waits_only_3_sec() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    s.set_aim_line(p, EAST, 0.0);
    let a = grunt(&mut s, [12.0, 0.0, 0.0], true);
    s.mobs.get_mut(&a).unwrap().health = 40;
    let mb = idx(&s, "mind_blast");
    cast(&mut s, p, "mind_blast", true);
    // The lance takes 0.3 sec to 12 yd; the 3 sec run from the kill.
    run(&mut s, 3.5);
    s.set_valor(p, 0.0);
    let _b = grunt(&mut s, [12.0, 0.0, 0.0], true);
    press(&mut s, p, "ranged", false);
    assert!(matches!(s.players[&p].action, Action::Aim { ability } if ability == mb), "3 sec on, the plain key is Mind Blast: {:?}", s.players[&p].action);
}

// --- Psychic Scream and Psychic Horror ---------------------------------------------------------------

#[test]
fn psychic_scream_panics_every_foe_within_6_yd_for_3_sec_and_damage_ends_it() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let near = grunt(&mut s, [5.0, 0.0, 0.0], false);
    let back = grunt(&mut s, [-4.0, 2.0, 0.0], false);
    let far = grunt(&mut s, [7.5, 0.0, 0.0], false);
    cast(&mut s, p, "psychic_scream", false);
    for g in [near, back] {
        let left = panicked_for(&s, g);
        assert!(left > 2.9 && left <= 3.0 + 1.0e-3, "fleeing 3 sec: {left}");
    }
    assert_eq!(panicked_for(&s, far), 0.0, "not past 6 yd");
    assert_eq!(s.content.abilities[idx(&s, "psychic_scream")].cooldown, 18.0);
    // Damage (15% of its health) ends it.
    s.hurt_mob(near, p, 20);
    assert_eq!(panicked_for(&s, near), 0.0, "broken by the blow");
    assert!(panicked_for(&s, back) > 0.0, "the other still runs");
}

#[test]
fn psychic_horror_reaches_9_yd_for_3_sec() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let g = grunt(&mut s, [8.5, 0.0, 0.0], false);
    let far = grunt(&mut s, [10.5, 0.0, 0.0], false);
    cast(&mut s, p, "psychic_scream", true);
    let left = panicked_for(&s, g);
    assert!(left > 2.9 && left <= 3.0 + 1.0e-3, "3 sec: {left}");
    assert_eq!(panicked_for(&s, far), 0.0);
}

#[test]
fn a_hero_flees_the_scream_15_sec_and_then_cant_be_scared_again_for_3() {
    let mut s = world_pvp();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let h = hero(&mut s, "Bren", [4.0, 0.0, 0.0], WEST);
    s.set_cooldowns(p, true);
    cast(&mut s, p, "psychic_scream", false);
    let left = panicked_for(&s, h);
    assert!(left > 1.4 && left <= 1.5 + 1.0e-3, "a hero: 1.5 sec: {left}");
    run(&mut s, 1.6);
    assert_eq!(panicked_for(&s, h), 0.0);
    s.move_player(h, [4.0, 0.0, 0.0], WEST, Moving::default());
    cast(&mut s, p, "psychic_scream", false);
    assert_eq!(panicked_for(&s, h), 0.0, "within 3 sec of the last: the grace");
    run(&mut s, 3.1);
    s.move_player(h, [4.0, 0.0, 0.0], WEST, Moving::default());
    cast(&mut s, p, "psychic_scream", false);
    assert!(panicked_for(&s, h) > 1.4, "after it, again");
}

#[test]
fn psychic_horror_on_a_hero_is_held_to_15_sec_too() {
    let mut s = world_pvp();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let h = hero(&mut s, "Bren", [4.0, 0.0, 0.0], WEST);
    cast(&mut s, p, "psychic_scream", true);
    let left = panicked_for(&s, h);
    assert!(left > 1.4 && left <= 1.5 + 1.0e-3, "a hero: 1.5 sec, its words say: {left}");
}

// --- Fade -------------------------------------------------------------------------------------------

/// A Priest facing east, Fade (or what's given) on her key, a wolf 2 yd east of her, its first
/// bite on its way.
fn wolf_on(s: &mut Survival, slot_key: &str) -> (Id, Id) {
    let p = hero(s, "Ayla", ORIGIN, EAST);
    learn(s, p, slot_key);
    let def = s.content.mob("wolf").unwrap();
    let k = s.spawn_mob(def, [2.0, 0.0, 0.0], None, false);
    s.mobs.get_mut(&k).unwrap().state = MobState::Idle { wander_at: 1e9 };
    s.engage(k, Foe::Hero(p));
    (p, k)
}

/// Run until the wolf's bite is `before` seconds off landing.
fn until_bite(s: &mut Survival, k: Id, before: f32) {
    for _ in 0..1000 {
        if let MobState::Windup { t, .. } = s.mobs[&k].state {
            if t + before >= s.attack_now(k).windup - 1.0e-4 {
                return;
            }
        }
        s.step(DT);
    }
    panic!("no bite");
}

/// Fade's key: the sidestep it sends the hero's client (its direction and length); with `moved`,
/// the client carries them along it, telling the server where they are each tick, as a client
/// does (the server runs no dash of its own).
fn fade(s: &mut Survival, p: Id, ex: bool, moved: bool) -> (Vec<Event>, f32, f32) {
    let mut ev = press(s, p, "utility", ex);
    let (dir, len, secs) = ev
        .iter()
        .find_map(|e| match e {
            Event::Dash { player, dir, speed, secs } if *player == p => Some((*dir, speed * secs, *secs)),
            _ => None,
        })
        .expect("a sidestep");
    if moved {
        let from = s.players[&p].pos;
        let f = s.players[&p].facing;
        let start = s.clock;
        while s.clock < start + secs + DT {
            let k = ((s.clock + DT - start) / secs).min(1.0);
            s.move_player(p, [from[0] + dir.cos() * len * k, from[1] + dir.sin() * len * k, from[2]], f, Moving::default());
            ev.extend(s.step(DT));
        }
    }
    (ev, dir, len)
}

fn has_cc(s: &Survival, k: Id, kind: &str, amount: f32) -> bool {
    s.cc_of(k).iter().any(|c| c.kind == kind && (c.amount - amount).abs() < 1.0e-3 && c.until > s.clock)
}

#[test]
fn fade_sidesteps_5_yd_back_when_still_and_the_way_you_move_otherwise() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    learn(&mut s, p, "fade");
    let (ev, dir, len) = fade(&mut s, p, false, false);
    assert!(!matches!(s.players[&p].action, Action::Aim { .. }), "on the key, no aim");
    assert!(ev.iter().any(|e| matches!(e, Event::Hop { player, roll: true, .. } if *player == p)), "the body sways aside");
    assert!((dir - WEST).abs() < 0.05 && (len - 5.0).abs() < 0.2, "5 yd back: {dir} {len}");
    assert!(s.untouchable(p), "nothing touches her for a moment");
    run(&mut s, 0.45);
    assert!(!s.untouchable(p), "0.4 sec, no more");
    assert_eq!(s.content.abilities[idx(&s, "fade")].cooldown, 10.0);
    // Moving left: 5 yd to her left.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    learn(&mut s, p, "fade");
    s.move_player(p, ORIGIN, EAST, Moving { left: true, ..Default::default() });
    let (_, dir, len) = fade(&mut s, p, false, false);
    assert!((dir - std::f32::consts::FRAC_PI_2).abs() < 0.05 && (len - 5.0).abs() < 0.2, "5 yd left: {dir} {len}");
    // Moving on: ahead, through what's there and striking nothing.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    learn(&mut s, p, "fade");
    let g = grunt(&mut s, [2.5, 0.0, 0.0], false);
    let hg = hp(&s, g);
    s.move_player(p, ORIGIN, EAST, Moving { forward: true, ..Default::default() });
    let (_, dir, len) = fade(&mut s, p, false, false);
    assert!(dir.abs() < 0.05 && (len - 5.0).abs() < 0.2, "5 yd on, past the grunt: {dir} {len}");
    run(&mut s, 0.5);
    assert_eq!(hp(&s, g), hg, "nothing struck on the way");
}

#[test]
fn fade_just_before_the_bite_slips_it_slows_the_wolf_and_quickens_the_priest() {
    let mut s = world();
    let (p, k) = wolf_on(&mut s, "fade");
    until_bite(&mut s, k, 0.05);
    let (full, valor) = (s.players[&p].health, s.players[&p].valor);
    let (mut ev, ..) = fade(&mut s, p, false, true);
    ev.extend(run(&mut s, 0.3));
    assert_eq!(perfects(&ev, p), vec![idx(&s, "fade")], "a perfect");
    assert_eq!(s.players[&p].health, full, "the bite missed");
    assert!(s.players[&p].valor >= valor + s.content.valor.counter, "a counter's Valor");
    assert!(has_cc(&s, k, "snare", 0.4), "the wolf slowed by 40%");
    let left = s.cc_of(k).iter().find(|c| c.kind == "snare").map(|c| c.until - s.clock).unwrap();
    assert!(left > 1.5 && left <= 2.0, "for 2 sec: {left}");
    assert!(has_cc(&s, p, "haste", 0.3), "she runs 30% faster");
    assert!(priest_burst(&s, &ev, p, "fade") && !mage_burst(&ev), "the Priest's burst");
    let cd = s.content.abilities[idx(&s, "fade")].cooldown;
    assert!(s.cooldown(p, idx(&s, "fade")).next_at - s.clock > cd - 0.5, "the whole cooldown");
}

#[test]
fn a_bite_that_lands_in_fades_untouchable_moment_misses_because_of_it() {
    // Pressed 0.05 sec before the bite: her client hasn't carried her out of its reach yet, but
    // nothing can touch her. The bite misses because of Fade.
    let mut s = world();
    let (p, k) = wolf_on(&mut s, "fade");
    until_bite(&mut s, k, 0.05);
    let full = s.players[&p].health;
    let (mut ev, ..) = fade(&mut s, p, false, false);
    ev.extend(run(&mut s, 0.2));
    assert_eq!(s.players[&p].health, full, "the bite missed");
    assert_eq!(perfects(&ev, p), vec![idx(&s, "fade")], "a blow that misses because of Fade: the perfect");
}

#[test]
fn fade_long_before_the_bite_is_a_plain_sidestep() {
    let mut s = world();
    let (p, k) = wolf_on(&mut s, "fade");
    until_bite(&mut s, k, 0.6);
    let (mut ev, ..) = fade(&mut s, p, false, true);
    ev.extend(run(&mut s, 0.8));
    assert!(perfects(&ev, p).is_empty(), "no perfect");
    assert!(!has_cc(&s, k, "snare", 0.4), "the wolf goes on as it was");
    assert!(!s.has_cc(p, "haste"), "and so does she");
}

#[test]
fn greater_fade_steps_7_yd_and_weakens_the_striker() {
    let mut s = world();
    let (p, k) = wolf_on(&mut s, "fade");
    s.set_valor(p, 100.0);
    until_bite(&mut s, k, 0.05);
    let (mut ev, _, len) = fade(&mut s, p, true, true);
    ev.extend(run(&mut s, 0.3));
    assert!((len - 7.0).abs() < 0.2, "7 yd: {len}");
    assert_eq!(perfects(&ev, p), vec![idx(&s, "greater_fade")]);
    assert!(has_cc(&s, k, "snare", 0.4), "slowed");
    assert!(has_cc(&s, k, "weaken", 0.2), "and dealing 20% less");
    let left = s.cc_of(k).iter().find(|c| c.kind == "weaken").map(|c| c.until - s.clock).unwrap();
    assert!(left > 2.5, "for 3 sec: {left}");
    assert!(priest_burst(&s, &ev, p, "greater_fade") && !mage_burst(&ev));
}

// --- Shadowguard and Vampiric Touch ---------------------------------------------------------------

fn shelled(s: &Survival, p: Id) -> bool {
    matches!(s.players[&p].action, Action::Stance { .. })
}

/// A Priest with Shadowguard, a grunt of 1000 health 10 yd along her reticle, another beside her
/// to strike her.
fn guard_set_up(s: &mut Survival) -> (Id, Id, Id, [(Id, Vec3); 2]) {
    let p = hero(s, "Ayla", ORIGIN, EAST);
    s.set_aim_line(p, EAST, 0.0);
    learn(s, p, "shadowguard");
    let t = grunt(s, [10.0, 0.0, 0.0], true);
    let k = grunt(s, [0.0, 2.0, 0.0], true);
    (p, t, k, [(t, [10.0, 0.0, 0.0]), (k, [0.0, 2.0, 0.0])])
}

#[test]
fn shadowguard_halves_what_lands_stores_it_and_throws_it_on_the_key_again() {
    let mut s = world();
    let (p, t, k, pins) = guard_set_up(&mut s);
    let ht = hp(&s, t);
    press(&mut s, p, "defense", false);
    assert!(shelled(&s, p), "in the shell, at once on the key");
    assert!((s.taken_share(p) - 0.5).abs() < 1.0e-3, "50% less: {}", s.taken_share(p));
    assert!(s.speed_for(p) <= s.content.player.run_speed * 0.5 + 1.0e-3, "at half speed: {}", s.speed_for(p));
    let full = s.players[&p].health;
    let ev = { s.strike_player(k, p, 20, [0.0, 2.0, 0.0], true, false, true, Blow::Light); s.step(0.0) };
    assert_eq!(full - s.players[&p].health, 10, "half of 20");
    assert!(ev.iter().any(|e| matches!(e, Event::PriestLook { kind: "shell", n: 10, .. })), "10 stored, on her screen");
    // No attacking in it.
    s.input(p, Input::Attack);
    s.input(p, Input::AttackUp);
    s.step(0.0);
    assert!(shelled(&s, p), "no attacking");
    run_pinned(&mut s, 0.5, &pins);
    let ev = press(&mut s, p, "defense", false);
    assert!(!shelled(&s, p), "out");
    let shard = s.content.abilities[idx(&s, "shadowguard_shard")].spell;
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { caster, spell, .. } if *caster == p && *spell == shard)) || s.projectiles.iter().any(|x| x.owner == p), "the stored shadow flies");
    let mut ev = ev;
    ev.extend(run_pinned(&mut s, 0.6, &pins));
    assert_eq!(ht - hp(&s, t), 10, "what it stored");
    assert!(perfects(&ev, p).is_empty(), "let go long after the blow: no perfect");
    assert!((s.taken_share(p) - 1.0).abs() < 1.0e-3, "no longer cut");
    let cd = s.content.abilities[idx(&s, "shadowguard")].cooldown;
    assert!(s.cooldown(p, idx(&s, "shadowguard")).next_at - s.clock > cd - 1.5, "on its 12 sec cooldown");
}

#[test]
fn shadowguard_let_go_just_after_a_blow_is_half_again_as_hard_45_at_most_the_perfect() {
    let mut s = world();
    let (p, t, k, pins) = guard_set_up(&mut s);
    let ht = hp(&s, t);
    press(&mut s, p, "defense", false);
    // 80 lands as 40: 40 stored, its most.
    s.strike_player(k, p, 80, [0.0, 2.0, 0.0], true, false, true, Blow::Light);
    run_pinned(&mut s, 0.1, &pins);
    let valor = s.players[&p].valor;
    let mut ev = press(&mut s, p, "defense", false);
    ev.extend(run_pinned(&mut s, 0.6, &pins));
    assert_eq!(ht - hp(&s, t), 45, "40 x 1.5 = 60, held to 45");
    assert_eq!(perfects(&ev, p), vec![idx(&s, "shadowguard")], "the perfect");
    assert!(s.players[&p].valor >= valor + s.content.checks.perfect_valor);
    assert!(priest_burst(&s, &ev, p, "shadowguard") && !mage_burst(&ev));
    // 20 stored, let go at once: 30.
    let mut s = world();
    let (p, t, k, pins) = guard_set_up(&mut s);
    let ht = hp(&s, t);
    press(&mut s, p, "defense", false);
    s.strike_player(k, p, 40, [0.0, 2.0, 0.0], true, false, true, Blow::Light);
    run_pinned(&mut s, 0.2, &pins);
    press(&mut s, p, "defense", false);
    run_pinned(&mut s, 0.6, &pins);
    assert_eq!(ht - hp(&s, t), 30, "20 x 1.5");
}

#[test]
fn a_heavy_blow_on_shadowguard_neither_fells_nor_staggers_her_so_the_key_again_throws_it_back() {
    // The second film: a Bloodhoof Brave's heavy blow knocked the shelled Priest down, and the
    // key again went nowhere until the shell ran out.
    let mut s = world();
    let (p, t, k, pins) = guard_set_up(&mut s);
    let ht = hp(&s, t);
    press(&mut s, p, "defense", false);
    s.strike_player(k, p, 52, [0.0, 2.0, 0.0], true, true, true, Blow::Light);
    run_pinned(&mut s, 0.1, &pins);
    assert!(shelled(&s, p), "still in the shell, not on the ground: {:?}", s.players[&p].action);
    let mut ev = press(&mut s, p, "defense", false);
    ev.extend(run_pinned(&mut s, 0.6, &pins));
    assert_eq!(ht - hp(&s, t), 39, "26 stored x 1.5");
    assert_eq!(perfects(&ev, p), vec![idx(&s, "shadowguard")], "the perfect");
}

#[test]
fn shadowguard_judges_the_let_go_on_the_priests_own_screen() {
    let mut s = world();
    let (p, t, k, pins) = guard_set_up(&mut s);
    s.set_rtt(p, 0.2);
    let ht = hp(&s, t);
    press(&mut s, p, "defense", false);
    s.strike_player(k, p, 40, [0.0, 2.0, 0.0], true, false, true, Blow::Light);
    // Arriving 0.45 sec after the blow: 0.25 on her screen.
    run_pinned(&mut s, 0.45, &pins);
    let mut ev = press(&mut s, p, "defense", false);
    ev.extend(run_pinned(&mut s, 0.6, &pins));
    assert_eq!(perfects(&ev, p).len(), 1, "a perfect on her screen");
    assert_eq!(ht - hp(&s, t), 30);
}

#[test]
fn shadowguard_with_nothing_stored_throws_nothing_and_left_alone_lets_go_after_2_sec() {
    let mut s = world();
    let (p, t, k, pins) = guard_set_up(&mut s);
    let ht = hp(&s, t);
    press(&mut s, p, "defense", false);
    run_pinned(&mut s, 0.3, &pins);
    let mut ev = press(&mut s, p, "defense", false);
    ev.extend(run_pinned(&mut s, 0.6, &pins));
    assert!(!shelled(&s, p));
    assert!(s.projectiles.iter().all(|x| x.owner != p), "nothing flies");
    assert_eq!(hp(&s, t), ht);
    // Left alone: 2 sec, then what it stored flies.
    let mut s = world();
    let (p, t, k2, pins) = guard_set_up(&mut s);
    let ht = hp(&s, t);
    press(&mut s, p, "defense", false);
    s.strike_player(k2, p, 20, [0.0, 2.0, 0.0], true, false, true, Blow::Light);
    run_pinned(&mut s, 1.85, &pins);
    assert!(shelled(&s, p), "still in it");
    run_pinned(&mut s, 0.2, &pins);
    assert!(!shelled(&s, p), "out at 2 sec");
    run_pinned(&mut s, 0.6, &pins);
    assert_eq!(ht - hp(&s, t), 10, "the 10 stored flew");
    let _ = k;
}

#[test]
fn vampiric_touch_heals_the_priest_from_her_own_damage_only() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let b = hero(&mut s, "Bren", [0.0, 6.0, 0.0], WEST);
    let m = grunt(&mut s, [2.0, 0.0, 0.0], true);
    let pins = [(m, [2.0, 0.0, 0.0])];
    for h in [p, b] {
        let max = s.players[&h].max_health;
        let now = s.clock;
        let x = s.players.get_mut(&h).unwrap();
        x.health = max / 2;
        x.combat_at = now;
    }
    s.set_aim_line(p, EAST, 0.0);
    cast(&mut s, p, "shadowguard", true);
    assert!(!shelled(&s, p), "the grip, not the shell");
    assert!(s.cc_of(m).iter().any(|c| c.kind == "vampiric" && c.from == p), "her grip on it");
    run_pinned(&mut s, 0.5, &pins);
    let (mine, his) = (s.players[&p].health, s.players[&b].health);
    let before = hp(&s, m);
    blast_at(&mut s, p, [2.0, 0.0, 0.0]);
    run_pinned(&mut s, 1.0, &pins);
    let dealt = before - hp(&s, m);
    assert!(dealt >= 26, "her Mind Blast: {dealt}");
    assert_eq!(s.players[&p].health, mine + (dealt as f32 * 0.2).round() as u32, "20% of hers back");
    assert_eq!(s.players[&b].health, his, "nothing to him");
}

// --- Void Rift --------------------------------------------------------------------------------------

/// A Priest at the origin, Void Rift torn open on the ground at `at` (the whole bar paid).
fn rift_at(s: &mut Survival, p: Id, at: Vec3) -> Vec<Event> {
    s.set_valor(p, 100.0);
    aim_ground(s, p, at);
    cast(s, p, "void_rift", false)
}

const HEART: Vec3 = [12.0, 0.0, 0.0];

#[test]
fn void_rift_drags_and_hurts_what_it_holds_and_implodes_on_the_key_again() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let gs: Vec<Id> = [[12.0, 3.0, 0.0], [12.0, -3.0, 0.0], [15.0, 0.0, 0.0]].into_iter().map(|at| grunt(&mut s, at, true)).collect();
    let before: Vec<u32> = gs.iter().map(|g| hp(&s, *g)).collect();
    let mut ev = rift_at(&mut s, p, HEART);
    assert_eq!(s.light.rifts.len(), 1, "torn open");
    let at = s.light.rifts[0].at;
    assert!(dist2d(at, HEART) < 0.8, "at the reticle: {at:?}");
    assert!((s.players[&p].valor).abs() < 1.0, "the whole bar");
    let from: Vec<f32> = gs.iter().map(|g| dist2d(s.mobs[g].pos, at)).collect();
    // (Unheld while it hauls them, the nearest each comes to its heart.)
    let mut nearest = from.clone();
    for _ in 0..110 {
        ev.extend(s.step(DT));
        for (k, g) in gs.iter().enumerate() {
            nearest[k] = nearest[k].min(dist2d(s.mobs[g].pos, at));
        }
    }
    hold(&mut s, &gs);
    for (k, g) in gs.iter().enumerate() {
        assert_eq!(before[k] - hp(&s, *g), 6, "6 a second");
        // (A pull stops 1.6 yd short of where it hauls to.)
        assert!(nearest[k] < from[k] - 1.0 && nearest[k] < 2.0, "dragged toward its heart: {from:?} to {nearest:?}");
    }
    for g in &gs {
        let x = s.mobs.get_mut(g).unwrap();
        x.pos = [at[0] + 1.0, at[1], at[2]];
    }
    assert!(ev.iter().any(|e| matches!(e, Event::Tether { .. })), "its cords");
    let valor = s.players[&p].valor;
    let mid: Vec<u32> = gs.iter().map(|g| hp(&s, *g)).collect();
    let mut ev = press(&mut s, p, "ultimate", false);
    ev.extend(run_held(&mut s, 0.1, &gs));
    assert!(s.light.rifts.is_empty(), "imploded");
    for (k, g) in gs.iter().enumerate() {
        assert_eq!(mid[k] - hp(&s, *g), 64, "40 + 8 x 3");
    }
    assert_eq!(perfects(&ev, p), vec![idx(&s, "void_rift")], "3 caught: the perfect");
    assert!(s.players[&p].valor >= valor + 20.0, "20 Valor back: {} from {valor}", s.players[&p].valor);
    assert!(priest_burst(&s, &ev, p, "void_rift") && !mage_burst(&ev));
}

#[test]
fn void_rift_grows_from_4_to_7_yd_over_2_sec() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let g = grunt(&mut s, [12.0, 6.2, 0.0], true);
    let pins = [(g, [12.0, 6.2, 0.0])];
    let h = hp(&s, g);
    rift_at(&mut s, p, HEART);
    run_pinned(&mut s, 0.55, &pins);
    assert_eq!(hp(&s, g), h, "6 yd out: not at first (4 yd, growing)");
    run_pinned(&mut s, 1.0, &pins);
    assert_eq!(h - hp(&s, g), 6, "caught as it grows");
}

#[test]
fn void_rift_with_one_foe_implodes_for_48_and_is_no_perfect() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let g = grunt(&mut s, [12.0, 2.0, 0.0], true);
    rift_at(&mut s, p, HEART);
    run_held(&mut s, 0.55, &[g]);
    let mid = hp(&s, g);
    let mut ev = press(&mut s, p, "ultimate", false);
    ev.extend(run_held(&mut s, 0.1, &[g]));
    assert_eq!(mid - hp(&s, g), 48, "40 + 8");
    assert!(perfects(&ev, p).is_empty(), "one caught: no perfect");
}

#[test]
fn void_rifts_implosion_is_80_at_most() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    rift_at(&mut s, p, HEART);
    let at = s.light.rifts[0].at;
    // (Put round it once it's open: in the way, one would have taken the reticle's line.)
    let gs: Vec<Id> = (0..6).map(|k| {
        let a = k as f32 * std::f32::consts::TAU / 6.0;
        grunt(&mut s, [at[0] + 2.5 * a.cos(), at[1] + 2.5 * a.sin(), 0.0], true)
    }).collect();
    run_held(&mut s, 0.55, &gs);
    let mid: Vec<u32> = gs.iter().map(|g| hp(&s, *g)).collect();
    press(&mut s, p, "ultimate", false);
    run_held(&mut s, 0.1, &gs);
    for (k, g) in gs.iter().enumerate() {
        assert_eq!(mid[k] - hp(&s, *g), 80, "40 + 8 x 6 = 88, held to 80");
    }
}

#[test]
fn void_rift_left_alone_collapses_after_4_sec_for_20() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let g = grunt(&mut s, [12.0, 2.0, 0.0], true);
    let h = hp(&s, g);
    rift_at(&mut s, p, HEART);
    run_held(&mut s, 3.9, &[g]);
    assert_eq!(s.light.rifts.len(), 1, "still open");
    assert_eq!(h - hp(&s, g), 24, "four seconds' 6");
    let ev = run_held(&mut s, 0.2, &[g]);
    assert!(s.light.rifts.is_empty(), "collapsed");
    assert_eq!(h - hp(&s, g), 44, "and 20 as it goes");
    assert!(perfects(&ev, p).is_empty());
}

// --- where it's learned -------------------------------------------------------------------------

#[test]
fn the_abbey_teaches_shadow_beside_the_light() {
    let s = world();
    let teaches = |k: &str| s.content.chests.iter().find(|c| c.key == k).map(|c| c.teaches.clone()).unwrap_or_default();
    let shrine = teaches("shrine_priest");
    assert!(shrine.contains(&"mind_blast".to_string()) && shrine.contains(&"psychic_scream".to_string()), "the altar: Q and E, {shrine:?}");
    let relic = teaches("relic_priest");
    assert!(relic.contains(&"fade".to_string()) && relic.contains(&"shadowguard".to_string()), "the reliquary: 1 and 2, {relic:?}");
    for k in ["mind_blast", "psychic_scream", "fade", "shadowguard"] {
        let d = &s.content.abilities[idx(&s, k)];
        assert!(d.school == "priest" && d.choice == 2 && s.content.ex_of(idx(&s, k)).is_some(), "{k}: Shadow's, with its EX");
    }
    for k in ["mind_flay", "mind_sear"] {
        if let Some(a) = s.content.ability(k) {
            assert!(!s.content.abilities[a].on_board(), "{k} is benched");
        }
    }
}

