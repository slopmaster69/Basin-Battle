//! The Shadow Priest (the second choice on Q, E, 1 and 2; claude/priest-kit-draft.md): Mind Flay's
//! cord that grows stronger the farther it's stretched, rings when held taut, snaps past its range
//! and snaps into its foe on the key again; Mind Sear's spill; Mind Blast snapping a taut cord, and
//! Shadow Word: Death's judgement; Fade's perfect dodge and Void Shift's swap; Dispersion's cut,
//! mend and perfect, and Vampiric Touch healing only the Priest who gave it.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::{Foe, MobState};
use crate::sim::*;

const DT: f32 = 0.05;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];
const WEST: f32 = std::f32::consts::PI;

fn world() -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    c.dev.full_tree = true;
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 7, 1_000_000.0);
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

/// A grunt standing idle where it's put.
fn grunt(s: &mut Survival, pos: Vec3) -> Id {
    let def = s.content.mob("grunt").unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.pos = pos;
    m.facing = WEST;
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

fn run_pinned(s: &mut Survival, secs: f32, pins: &[(Id, Vec3)]) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        pin(s, pins);
        out.extend(s.step(DT));
    }
    pin(s, pins);
    out
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

fn flaying(s: &Survival, p: Id) -> bool {
    matches!(s.players[&p].action, Action::Channel { .. })
}

fn perfects(events: &[Event], p: Id) -> Vec<usize> {
    events.iter().filter_map(|e| if let Event::Perfect { player, ability } = e { (*player == p).then_some(*ability) } else { None }).collect()
}

// --- Mind Flay -------------------------------------------------------------------------------------

/// What a Mind Flay on a grunt `dist` yd off deals it in its first `secs`.
fn flayed_for(dist: f32, secs: f32) -> u32 {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let m = grunt(&mut s, [dist, 0.0, 0.0]);
    let pins = [(m, [dist, 0.0, 0.0])];
    let before = hp(&s, m);
    cast(&mut s, p, "mind_flay", false);
    assert!(flaying(&s, p), "bound at {dist} yd");
    run_pinned(&mut s, secs, &pins);
    before - hp(&s, m)
}

#[test]
fn mind_flay_grows_stronger_the_farther_you_stand() {
    let near = flayed_for(4.0, 2.0);
    let mid = flayed_for(15.0, 2.0);
    let far = flayed_for(28.0, 2.0);
    assert!(near > 0 && near <= 12, "about 5 a sec up close: {near}");
    assert!(mid > near && far > mid, "stronger as it stretches: {near}, {mid}, {far}");
    assert!(far >= 21 && far % 7 == 0, "14 a sec taut, 7 a tick: {far}");
    assert!(far >= 2 * near, "at the edge, over twice as strong as up close: {near}, {far}");
}

#[test]
fn mind_flay_slows_its_foe_and_lets_the_priest_walk() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let m = grunt(&mut s, [12.0, 0.0, 0.0]);
    let pins = [(m, [12.0, 0.0, 0.0])];
    cast(&mut s, p, "mind_flay", false);
    run_pinned(&mut s, 0.6, &pins);
    assert!(s.cc_of(m).iter().any(|c| c.kind == "snare" && (c.amount - 0.3).abs() < 1.0e-3 && c.until > s.clock), "slowed by 30%");
    // Walking with it: the channel holds as she steps back.
    s.move_player(p, [-2.0, 0.0, 0.0], 0.0, Moving::default());
    run_pinned(&mut s, 0.2, &pins);
    assert!(flaying(&s, p), "still bound");
    run_pinned(&mut s, 2.5, &pins);
    assert!(!flaying(&s, p), "over after 3 sec");
}

#[test]
fn mind_flay_snaps_past_30_yd_and_the_priest_flinches() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let m = grunt(&mut s, [28.0, 0.0, 0.0]);
    cast(&mut s, p, "mind_flay", false);
    run_pinned(&mut s, 0.3, &[(m, [28.0, 0.0, 0.0])]);
    assert!(flaying(&s, p));
    let ev = run_pinned(&mut s, 0.1, &[(m, [31.0, 0.0, 0.0])]);
    assert!(!flaying(&s, p), "snapped");
    assert!(matches!(s.players[&p].action, Action::Stagger { .. }), "she flinches");
    assert!(ev.iter().any(|e| matches!(e, Event::Visual { id, kit: 219, .. } if *id == p)), "the crack of it on her");
    assert!(perfects(&ev, p).is_empty(), "no perfect");
    run_pinned(&mut s, 0.5, &[(m, [31.0, 0.0, 0.0])]);
    assert!(matches!(s.players[&p].action, Action::Ready), "only briefly");
}

#[test]
fn a_whole_taut_second_is_a_perfect_once_a_cast() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let m = grunt(&mut s, [28.0, 0.0, 0.0]);
    let pins = [(m, [28.0, 0.0, 0.0])];
    let mut ev = cast(&mut s, p, "mind_flay", false);
    ev.extend(run_pinned(&mut s, 0.2, &pins));
    assert!(
        ev.iter().any(|e| matches!(e, Event::Channel { caster, spell: 5138, target: Some(t), .. } if *caster == p && *t == m)),
        "the cord tightens: its taut look"
    );
    let valor = s.players[&p].valor;
    ev.extend(run_pinned(&mut s, 0.6, &pins));
    assert!(perfects(&ev, p).is_empty(), "not yet a second");
    ev.extend(run_pinned(&mut s, 0.4, &pins));
    assert_eq!(perfects(&ev, p), vec![idx(&s, "mind_flay")], "a second taut: it rings");
    assert!(s.players[&p].valor >= valor + s.content.checks.perfect_valor, "a perfect's Valor");
    ev.extend(run_pinned(&mut s, 1.5, &pins));
    assert_eq!(perfects(&ev, p).len(), 1, "once a cast");

    // Slack, it never rings.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let m = grunt(&mut s, [12.0, 0.0, 0.0]);
    let mut ev = cast(&mut s, p, "mind_flay", false);
    ev.extend(run_pinned(&mut s, 3.2, &[(m, [12.0, 0.0, 0.0])]));
    assert!(perfects(&ev, p).is_empty(), "slack: no perfect");

    // Taut, slack, taut again: each stretch counts from its start.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let m = grunt(&mut s, [28.0, 0.0, 0.0]);
    let mut ev = cast(&mut s, p, "mind_flay", false);
    ev.extend(run_pinned(&mut s, 0.7, &[(m, [28.0, 0.0, 0.0])]));
    ev.extend(run_pinned(&mut s, 0.2, &[(m, [20.0, 0.0, 0.0])]));
    ev.extend(run_pinned(&mut s, 0.7, &[(m, [28.0, 0.0, 0.0])]));
    assert!(perfects(&ev, p).is_empty(), "two short stretches aren't a second");
}

#[test]
fn pressed_again_while_taut_it_snaps_into_the_foe_for_twice_whats_left() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let m = grunt(&mut s, [28.0, 0.0, 0.0]);
    let pins = [(m, [28.0, 0.0, 0.0])];
    cast(&mut s, p, "mind_flay", false);
    run_pinned(&mut s, 2.0, &pins);
    let Action::Channel { until, .. } = s.players[&p].action else { panic!("bound") };
    let left = until - s.clock;
    let before = hp(&s, m);
    let ev = press(&mut s, p, "ranged", false);
    let dealt = before - hp(&s, m);
    let want = (2.0 * 14.0 * left).round() as u32;
    assert!(dealt + 1 >= want && dealt <= want + 1, "twice what was left ({want}): {dealt}");
    assert!(!flaying(&s, p), "the cord is spent");
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { caster, spell: 27812, target: Target::Unit(t) } if *caster == p && *t == m)), "whipped into it");

    // Early on it would be far more than 45: 45 at most.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let m = grunt(&mut s, [28.0, 0.0, 0.0]);
    let pins = [(m, [28.0, 0.0, 0.0])];
    cast(&mut s, p, "mind_flay", false);
    run_pinned(&mut s, 0.3, &pins);
    let before = hp(&s, m);
    press(&mut s, p, "ranged", false);
    assert_eq!(before - hp(&s, m), 45, "capped at 45");

    // Slack, the key again only lets it go.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let m = grunt(&mut s, [10.0, 0.0, 0.0]);
    let pins = [(m, [10.0, 0.0, 0.0])];
    cast(&mut s, p, "mind_flay", false);
    run_pinned(&mut s, 0.3, &pins);
    let before = hp(&s, m);
    press(&mut s, p, "ranged", false);
    assert!(!flaying(&s, p), "let go");
    assert_eq!(hp(&s, m), before, "and nothing snapped into it");
}

#[test]
fn a_taut_cord_on_the_priests_own_screen_counts() {
    // The cord went slack on the server a moment ago; on Chain's screen, a round trip behind, it
    // was still taut when he pressed.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.set_rtt(p, 0.15);
    let m = grunt(&mut s, [28.0, 0.0, 0.0]);
    cast(&mut s, p, "mind_flay", false);
    run_pinned(&mut s, 1.5, &[(m, [28.0, 0.0, 0.0])]);
    run_pinned(&mut s, 0.1, &[(m, [24.0, 0.0, 0.0])]);
    let before = hp(&s, m);
    press(&mut s, p, "ranged", false);
    assert!(before - hp(&s, m) >= 30, "snapped: {}", before - hp(&s, m));
}

#[test]
fn mind_sear_spills_half_on_the_foes_round_its_foe() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let m = grunt(&mut s, [28.0, 0.0, 0.0]);
    let beside = grunt(&mut s, [29.0, 3.0, 0.0]);
    let away = grunt(&mut s, [28.0, 9.0, 0.0]);
    let pins = [(m, [28.0, 0.0, 0.0]), (beside, [29.0, 3.0, 0.0]), (away, [28.0, 9.0, 0.0])];
    let (h0, b0, a0) = (hp(&s, m), hp(&s, beside), hp(&s, away));
    cast(&mut s, p, "mind_flay", true);
    assert!(flaying(&s, p), "Mind Sear: the same cord");
    assert!((s.players[&p].valor - 75.0).abs() < 0.01, "a segment of Valor");
    run_pinned(&mut s, 2.1, &pins);
    let (on, spilt) = (h0 - hp(&s, m), b0 - hp(&s, beside));
    assert!(on >= 20, "its foe: {on}");
    assert!(spilt * 2 + 6 >= on && spilt * 2 <= on + 6, "half on the one beside it: {spilt} of {on}");
    assert_eq!(hp(&s, away), a0, "none on one 9 yd off");
}

// --- Mind Blast and Shadow Word: Death -------------------------------------------------------------

#[test]
fn mind_blast_on_a_taut_flayed_foe_snaps_it_for_double_a_perfect() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let m = grunt(&mut s, [28.0, 0.0, 0.0]);
    let pins = [(m, [28.0, 0.0, 0.0])];
    learn(&mut s, p, "mind_blast");
    cast(&mut s, p, "mind_flay", false);
    run_pinned(&mut s, 0.4, &pins);
    // She looks away: the cord holds its foe, and the snap goes to it all the same.
    s.move_player(p, ORIGIN, std::f32::consts::FRAC_PI_2, Moving::default());
    run_pinned(&mut s, 0.1, &pins);
    assert!(flaying(&s, p));
    let before = hp(&s, m);
    let valor = s.players[&p].valor;
    let ev = press(&mut s, p, "close", false);
    assert_eq!(before - hp(&s, m), 44, "double: 44");
    assert_eq!(perfects(&ev, p), vec![idx(&s, "mind_blast")], "a perfect");
    assert!(s.players[&p].valor >= valor + s.content.checks.perfect_valor);
    assert!(!flaying(&s, p), "the cord snapped into it");
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { spell: 27812, target: Target::Unit(t), .. } if *t == m)), "black lightning on it");
    assert!(s.cooldown(p, idx(&s, "mind_blast")).charges == 0, "Mind Blast spent");
}

#[test]
fn mind_blast_otherwise_is_a_dark_star_for_22() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let m = grunt(&mut s, [12.0, 0.0, 0.0]);
    let pins = [(m, [12.0, 0.0, 0.0])];
    let before = hp(&s, m);
    let mut ev = cast(&mut s, p, "mind_blast", false);
    ev.extend(run_pinned(&mut s, 0.4, &pins));
    assert_eq!(before - hp(&s, m), 22);
    assert!(perfects(&ev, p).is_empty(), "no perfect");

    // On a slack cord it's the plain spell: the cord let go, the star aimed.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let m = grunt(&mut s, [12.0, 0.0, 0.0]);
    let pins = [(m, [12.0, 0.0, 0.0])];
    learn(&mut s, p, "mind_blast");
    cast(&mut s, p, "mind_flay", false);
    run_pinned(&mut s, 0.3, &pins);
    let ev = press(&mut s, p, "close", false);
    assert!(perfects(&ev, p).is_empty(), "no perfect");
    assert!(matches!(s.players[&p].action, Action::Aim { .. }), "aimed, the cord let go");
}

/// A grunt of 1000 health at `health`, Shadow Word: Death on it: what it took, what she took.
fn death_on(health: u32) -> (u32, u32, bool) {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let m = grunt(&mut s, [12.0, 0.0, 0.0]);
    let pins = [(m, [12.0, 0.0, 0.0])];
    {
        let x = s.mobs.get_mut(&m).unwrap();
        x.max_health = 1000;
        x.health = health;
    }
    let mine = s.players[&p].health;
    cast(&mut s, p, "mind_blast", true);
    run_pinned(&mut s, 0.4, &pins);
    let left = s.mobs.get(&m).map_or(0, |x| x.health);
    (health - left, mine - s.players[&p].health, left == 0)
}

#[test]
fn shadow_word_death_triples_on_a_foe_low_on_health_and_bites_back_if_it_lives() {
    let (took, back, dead) = death_on(1000);
    assert_eq!((took, back, dead), (15, 10, false), "15, and 10 back at her");
    let (took, back, dead) = death_on(150);
    assert_eq!((took, back, dead), (45, 10, false), "under 20%: 45, and it still lives to bite back");
    let (_, back, dead) = death_on(40);
    assert!(dead && back == 0, "a kill: nothing comes back");
}

// --- Fade and Void Shift ---------------------------------------------------------------------------

/// A Priest facing west, a wolf 2 yd east of her, its first bite on its way.
fn wolf_on(s: &mut Survival, slot_key: &str) -> (Id, Id) {
    let p = hero(s, "Ayla", ORIGIN, WEST);
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
        s.step(0.01);
    }
    panic!("no bite");
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / 0.01).round() as usize {
        out.extend(s.step(0.01));
    }
    out
}

fn slowed(s: &Survival, k: Id, amount: f32) -> bool {
    s.cc_of(k).iter().any(|c| c.kind == "snare" && (c.amount - amount).abs() < 1.0e-3 && c.until > s.clock)
}

#[test]
fn fade_just_before_the_bite_slows_the_wolf_and_quickens_the_priest() {
    let mut s = world();
    let (p, k) = wolf_on(&mut s, "fade");
    until_bite(&mut s, k, 0.05);
    let (full, valor) = (s.players[&p].health, s.players[&p].valor);
    let mut ev = press(&mut s, p, "utility", false);
    assert!(s.players[&p].pos[0] < -5.0 && s.players[&p].pos[0] > -7.0, "glided 6 yd west: {:?}", s.players[&p].pos);
    assert!(s.untouchable(p), "nothing touches her for a moment");
    ev.extend(run(&mut s, 0.2));
    assert_eq!(perfects(&ev, p), vec![idx(&s, "fade")], "a perfect");
    assert_eq!(s.players[&p].health, full, "the bite missed");
    assert!(s.players[&p].valor >= valor + s.content.valor.counter, "a counter's Valor");
    assert!(slowed(&s, k, 0.5), "the wolf slowed by 50%");
    assert!(s.cc_of(p).iter().any(|c| c.kind == "haste" && (c.amount - 0.4).abs() < 1.0e-3), "she runs 40% faster");
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { spell: 27812, target: Target::Point(at), .. } if at[0].abs() < 0.5)), "her afterimage where she stood");
    let cd = s.content.abilities[idx(&s, "fade")].cooldown;
    assert!(s.cooldown(p, idx(&s, "fade")).next_at - s.clock > cd - 0.5, "the whole cooldown");
    run(&mut s, 0.5);
    assert!(!s.untouchable(p), "half a second, no more");
}

#[test]
fn fade_long_before_the_bite_is_a_plain_fade() {
    let mut s = world();
    let (p, k) = wolf_on(&mut s, "fade");
    until_bite(&mut s, k, 0.3);
    let mut ev = press(&mut s, p, "utility", false);
    ev.extend(run(&mut s, 0.5));
    assert!(perfects(&ev, p).is_empty(), "no perfect");
    assert!(!slowed(&s, k, 0.5), "the wolf goes on as it was");
    assert!(!s.cc_of(p).iter().any(|c| c.kind == "haste"), "and so does she");
}

#[test]
fn void_shift_trades_places_with_a_foe_or_a_friend() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let m = grunt(&mut s, [12.0, 0.0, 0.0]);
    cast(&mut s, p, "fade", true);
    run(&mut s, 0.5);
    assert!(s.players[&p].pos[0] > 9.0, "she's where it stood: {:?}", s.players[&p].pos);
    assert!(s.mobs[&m].pos[0] < 2.0, "it's where she stood: {:?}", s.mobs[&m].pos);

    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let f = hero(&mut s, "Bren", [10.0, 0.0, 0.0], 0.0);
    cast(&mut s, p, "fade", true);
    run(&mut s, 0.5);
    assert!(s.players[&p].pos[0] > 8.0, "with a friend too: {:?}", s.players[&p].pos);
    assert!(s.players[&f].pos[0] < 2.0, "{:?}", s.players[&f].pos);
}

// --- Dispersion and Vampiric Touch -----------------------------------------------------------------

fn dispersed(s: &Survival, p: Id) -> bool {
    matches!(s.players[&p].action, Action::Stance { .. })
}

#[test]
fn dispersion_cuts_what_lands_and_mends_until_she_comes_out() {
    let mut s = world();
    let (p, k) = wolf_on(&mut s, "dispersion");
    let max = s.players[&p].max_health;
    s.players.get_mut(&p).unwrap().health = max / 2;
    until_bite(&mut s, k, 0.4);
    press(&mut s, p, "defense", false);
    assert!(dispersed(&s, p), "in shadow");
    assert!((s.taken_share(p) - 0.3).abs() < 1.0e-3, "70% less: {}", s.taken_share(p));
    let before = s.players[&p].health;
    let ev = run(&mut s, 0.45);
    assert!(perfects(&ev, p).is_empty(), "entered early: no perfect");
    let bitten = before.saturating_sub(s.players[&p].health);
    assert!((1..=3).contains(&bitten), "the bite landed, cut to 3 at most: {bitten}");
    // Attacking does nothing in it.
    s.input(p, Input::Attack);
    s.input(p, Input::AttackUp);
    s.step(0.0);
    assert!(dispersed(&s, p), "no attacking");
    // A second in: 5% back (the wolf held off, out of it).
    let away = [(k, [60.0, 0.0, 0.0])];
    let at = s.players[&p].health;
    run_pinned(&mut s, 0.6, &away);
    let each = (max as f32 * 0.05).round() as u32;
    assert_eq!(s.players[&p].health, at + each, "5% a second");
    // The key again: out.
    press(&mut s, p, "defense", false);
    assert!(!dispersed(&s, p), "out");
    assert!((s.taken_share(p) - 1.0).abs() < 1.0e-3, "no longer cut");
    let at = s.players[&p].health;
    run_pinned(&mut s, 1.2, &away);
    assert_eq!(s.players[&p].health, at, "no more mending");
    let cd = s.content.abilities[idx(&s, "dispersion")].cooldown;
    assert!(s.cooldown(p, idx(&s, "dispersion")).next_at - s.clock > cd - 2.0, "on its cooldown");
}

#[test]
fn dispersion_entered_as_the_bite_lands_lets_it_pass_and_slows_the_wolf() {
    let mut s = world();
    let (p, k) = wolf_on(&mut s, "dispersion");
    until_bite(&mut s, k, 0.05);
    let full = s.players[&p].health;
    let valor = s.players[&p].valor;
    let mut ev = press(&mut s, p, "defense", false);
    ev.extend(run(&mut s, 0.15));
    assert_eq!(perfects(&ev, p), vec![idx(&s, "dispersion")], "a perfect");
    assert_eq!(s.players[&p].health, full, "the bite passed through her");
    assert!(slowed(&s, k, 0.5), "the wolf slowed by 50%");
    assert!(s.players[&p].valor >= valor + s.content.valor.counter, "a counter's Valor");
}

#[test]
fn vampiric_touch_heals_the_priest_from_her_own_damage_only() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [6.0, 0.0, 0.0], WEST);
    let m = grunt(&mut s, [2.0, 0.0, 0.0]);
    let pins = [(m, [2.0, 0.0, 0.0])];
    for h in [p, b] {
        let max = s.players[&h].max_health;
        let now = s.clock;
        let x = s.players.get_mut(&h).unwrap();
        x.health = max / 2;
        // (In the fight: no mending in the field meanwhile.)
        x.combat_at = now;
    }
    cast(&mut s, p, "dispersion", true);
    assert!(!dispersed(&s, p), "the grip, not the stance");
    assert!(s.cc_of(m).iter().any(|c| c.kind == "vampiric" && c.from == p), "her grip on it");
    assert!(s.auras_of(m).contains(&15286), "and its look");
    run_pinned(&mut s, 0.5, &pins);
    // Her Mind Blast: a fifth of it back to her.
    let (mine, his) = (s.players[&p].health, s.players[&b].health);
    let before = hp(&s, m);
    cast(&mut s, p, "mind_blast", false);
    run_pinned(&mut s, 0.3, &pins);
    let dealt = before - hp(&s, m);
    assert_eq!(dealt, 22);
    assert_eq!(s.players[&p].health, mine + (dealt as f32 * 0.2).round() as u32, "20% of hers back");
    assert_eq!(s.players[&b].health, his, "nothing to him");
    // His blows on it heal no one.
    run_pinned(&mut s, 0.5, &pins);
    let mine = s.players[&p].health;
    let before = hp(&s, m);
    cast(&mut s, b, "mind_blast", false);
    run_pinned(&mut s, 0.3, &pins);
    assert!(hp(&s, m) < before, "he struck it");
    assert_eq!(s.players[&p].health, mine, "not her");
    assert_eq!(s.players[&b].health, his, "nor him");
}

// --- Where it's learned, and saves -----------------------------------------------------------------

#[test]
fn the_abbey_teaches_shadow_beside_the_light() {
    let s = world();
    let teaches = |k: &str| s.content.chests.iter().find(|c| c.key == k).map(|c| c.teaches.clone()).unwrap_or_default();
    let shrine = teaches("shrine_priest");
    assert!(shrine.contains(&"mind_flay".to_string()) && shrine.contains(&"mind_blast".to_string()), "the altar: Q and E, {shrine:?}");
    let relic = teaches("relic_priest");
    assert!(relic.contains(&"fade".to_string()) && relic.contains(&"dispersion".to_string()), "the reliquary: 1 and 2, {relic:?}");
    for k in ["mind_flay", "mind_blast", "fade", "dispersion"] {
        let d = &s.content.abilities[idx(&s, k)];
        assert!(d.school == "priest" && d.choice == 2 && s.content.ex_of(idx(&s, k)).is_some(), "{k}: Shadow's, with its EX");
    }
}

#[test]
fn a_save_with_mind_blast_on_e_keeps_it_there() {
    let mut s = world();
    s.content.dev.full_tree = false;
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let mut save = s.leave(p).unwrap();
    save.owned = ["mind_flay", "mind_blast", "fade"].into_iter().map(String::from).collect();
    save.loadout = [("ranged", "mind_flay"), ("close", "mind_blast"), ("utility", "fade")].into_iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
    let q = s.join("ayla", "Ayla", Some(&save));
    let h = &s.players[&q];
    for k in ["mind_flay", "mind_blast", "fade"] {
        assert!(h.owned.contains(k), "owns {k}: {:?}", h.owned);
    }
    assert_eq!(h.loadout.get("ranged").map(String::as_str), Some("mind_flay"));
    assert_eq!(h.loadout.get("close").map(String::as_str), Some("mind_blast"));
    assert_eq!(h.loadout.get("utility").map(String::as_str), Some("fade"));
}
