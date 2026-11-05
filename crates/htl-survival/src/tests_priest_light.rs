//! The Light Priest (claude/priest-kit-draft.md): Holy Word: Salvation's group check, Penance's
//! flick, Holy Fire's Kindle, Holy Nova and Leap of Faith as counters for others, and old saves.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::{Foe, MobState};
use crate::sim::*;

const DT: f32 = 0.01;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];
const WEST: f32 = std::f32::consts::PI;

fn world() -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    c.dev.full_tree = true;
    // (No mending by itself: the heals counted are the spells'.)
    c.player.field_regen = 0.0;
    c.player.rest_regen = 0.0;
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 1, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

fn hero(s: &mut Survival, name: &str, pos: Vec3, f: f32) -> Id {
    let id = s.join(&name.to_lowercase(), name, None);
    s.move_player(id, pos, f, Moving::default());
    let _ = s.step(0.0);
    s.players.get_mut(&id).unwrap().cd_off = false;
    id
}

/// A creature standing idle where it's put.
fn foe(s: &mut Survival, key: &str, pos: Vec3) -> Id {
    let def = s.content.mob(key).unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.pos = pos;
    id
}

fn pin(s: &mut Survival, pins: &[(Id, Vec3)]) {
    for (m, at) in pins {
        if let Some(x) = s.mobs.get_mut(m) {
            x.state = MobState::Idle { wander_at: 1e9 };
            x.path.clear();
            x.pos = *at;
        }
    }
}

fn run(s: &mut Survival, secs: f32, pins: &[(Id, Vec3)]) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        pin(s, pins);
        out.extend(s.step(DT));
    }
    out
}

/// Run until the clock reaches `t`.
fn until(s: &mut Survival, t: f32, pins: &[(Id, Vec3)]) -> Vec<Event> {
    let mut out = Vec::new();
    while s.clock + DT * 0.5 < t {
        pin(s, pins);
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

fn learn(s: &mut Survival, p: Id, key: &str) {
    let slot = s.content.abilities[idx(s, key)].slot.clone();
    s.players.get_mut(&p).unwrap().owned.insert(key.to_string());
    s.slot_ability(p, &slot, key).unwrap();
}

/// Its key pressed (an aimed one let go on a tap of the click).
fn cast(s: &mut Survival, p: Id, key: &str, ex: bool) -> Vec<Event> {
    learn(s, p, key);
    let slot = s.content.abilities[idx(s, key)].slot.clone();
    s.input(p, Input::Ability(key_of(&slot), ex));
    let mut ev = s.step(0.0);
    if matches!(s.players[&p].action, Action::Aim { .. }) {
        s.input(p, Input::Attack);
        ev.extend(s.step(0.0));
        s.input(p, Input::AttackUp);
        ev.extend(s.step(0.0));
    }
    ev
}

fn perfects(ev: &[Event], p: Id) -> usize {
    ev.iter().filter(|e| matches!(e, Event::Perfect { player, .. } if *player == p)).count()
}

/// The reticle on a spot, level.
fn aim_at(s: &mut Survival, p: Id, at: Vec3) {
    let from = s.players[&p].pos;
    s.set_aim_line(p, facing(from, at), 0.0);
}

fn halve(s: &mut Survival, p: Id) -> u32 {
    let h = s.players.get_mut(&p).unwrap();
    h.health = h.max_health / 2;
    h.health
}

// --- Holy Word: Salvation ---------------------------------------------------------------------------

/// A Priest lays Salvation where she stands; when its beats fall.
fn salvation(s: &mut Survival, p: Id) -> (Vec<Event>, Vec<f32>) {
    s.set_valor(p, 100.0);
    let ev = cast(s, p, "holy_word_salvation", false);
    let beats = s.light.sigils.last().expect("the circle laid").beats.clone();
    (ev, beats)
}

/// A hero acts (raises their guard, and lets it down).
fn act(s: &mut Survival, h: Id) {
    s.input(h, Input::Guard);
    s.input(h, Input::GuardUp);
}

#[test]
fn salvation_rewards_each_hero_for_the_beats_they_land_and_its_pillar_grows_with_them() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let a = hero(&mut s, "Bren", [3.0, 0.0, 0.0], 0.0);
    let b = hero(&mut s, "Cara", [-3.0, 2.0, 0.0], 0.0);
    let out = hero(&mut s, "Dane", [16.0, 0.0, 0.0], 0.0);
    let g = foe(&mut s, "grunt", [5.0, 5.0, 0.0]);
    let pins = [(g, [5.0, 5.0, 0.0])];
    let (ha, hb, hout) = (halve(&mut s, a), halve(&mut s, b), halve(&mut s, out));
    let (mut ev, beats) = salvation(&mut s, p);
    assert_eq!(beats.len(), 3);
    let start = s.clock;
    assert!((beats[0] - start - 2.0).abs() < 0.02 && (beats[1] - beats[0] - 1.5).abs() < 0.02 && (beats[2] - beats[1] - 1.0).abs() < 0.02, "2, 1.5, 1 sec: {beats:?}");
    let ring = ev.iter().find_map(|e| match e {
        Event::Mark { id, radius, .. } if (*radius - 10.0).abs() < 0.01 => Some(*id),
        _ => None,
    });
    let ring = ring.expect("its circle, 10 yd");
    assert!(s.mark_friendly(ring), "a friend's ring, for everyone");
    for (k, at) in beats.iter().enumerate() {
        ev.extend(until(&mut s, *at, &pins));
        act(&mut s, a);
        if k != 1 {
            act(&mut s, b);
        }
        act(&mut s, out);
        if k == 1 {
            // Cara a quarter of a second late: out of its window.
            ev.extend(run(&mut s, 0.25, &pins));
            act(&mut s, b);
        }
    }
    let before = s.mobs[&g].health;
    ev.extend(run(&mut s, 0.6, &pins));
    // The rim closed in rings smaller than the circle, for everyone.
    let rims: Vec<f32> = ev
        .iter()
        .filter_map(|e| match e {
            Event::Mark { radius, .. } if *radius < 9.99 => Some(*radius),
            _ => None,
        })
        .collect();
    assert!(rims.len() >= 9 && rims.iter().any(|r| *r < 2.0), "the rim closing: {rims:?}");
    assert_eq!(s.holy_stacks(a), 3, "Bren landed all three");
    assert_eq!(s.holy_stacks(b), 2, "Cara missed one");
    assert_eq!(s.holy_stacks(out), 0, "Dane stood outside it");
    assert_eq!(perfects(&ev, a), 3);
    assert_eq!(perfects(&ev, b), 2);
    assert_eq!(perfects(&ev, out), 0);
    assert_eq!(perfects(&ev, p), 3, "the Priest's, one a beat a friend landed");
    // The pillar: 30, and 15 for each of the two who landed the third.
    let dealt = before - s.mobs[&g].health;
    let expect = (60.0 * s.taken_share(g)).round() as u32;
    assert_eq!(dealt, expect, "30 + 2 x 15");
    assert_eq!(s.players[&a].health, ha + 20, "healed 20");
    assert_eq!(s.players[&b].health, hb + 20, "healed 20");
    assert_eq!(s.players[&out].health, hout, "not one outside it");
    assert!(s.auras_of(a).contains(&20007), "Holy Light's glow on him");
    assert!(s.light.sigils.is_empty(), "the circle gone");
    assert!(ev.iter().any(|e| matches!(e, Event::MarkGone { id } if *id == ring)), "its ring gone with it");
    // Holy Light: 10% a stack.
    let might = s.effect_of(&s.players[&a], "might");
    assert!((might - 0.3).abs() < 1.0e-4, "30% more damage: {might}");
    run(&mut s, 10.5, &pins);
    assert_eq!(s.holy_stacks(a), 0, "gone after 10 sec");
}

#[test]
fn salvations_pillar_counts_only_the_heroes_who_land_its_last_beat() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let a = hero(&mut s, "Bren", [3.0, 0.0, 0.0], 0.0);
    let b = hero(&mut s, "Cara", [-3.0, 2.0, 0.0], 0.0);
    let g = foe(&mut s, "grunt", [5.0, 5.0, 0.0]);
    let pins = [(g, [5.0, 5.0, 0.0])];
    let (_, beats) = salvation(&mut s, p);
    // Only Bren, and only on the last.
    until(&mut s, beats[2], &pins);
    act(&mut s, a);
    let before = s.mobs[&g].health;
    let ev = run(&mut s, 0.6, &pins);
    let dealt = before - s.mobs[&g].health;
    assert_eq!(dealt, (45.0 * s.taken_share(g)).round() as u32, "30 + 15");
    assert_eq!(s.holy_stacks(a), 1);
    assert_eq!(s.holy_stacks(b), 0);
    assert_eq!(perfects(&ev, p), 1);
}

#[test]
fn mashing_keys_through_a_beat_doesnt_land_it() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let a = hero(&mut s, "Bren", [3.0, 0.0, 0.0], 0.0);
    let (_, beats) = salvation(&mut s, p);
    until(&mut s, beats[0] - 0.25, &[]);
    for _ in 0..30 {
        act(&mut s, a);
        s.step(DT);
    }
    assert_eq!(s.holy_stacks(a), 0, "an early press spends the beat");
}

#[test]
fn salvation_judges_a_laggy_heros_press_on_their_own_screen() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let chain = hero(&mut s, "Chain", [3.0, 0.0, 0.0], 0.0);
    let near = hero(&mut s, "Bren", [-3.0, 0.0, 0.0], 0.0);
    s.set_rtt(chain, 0.2);
    let g = foe(&mut s, "grunt", [5.0, 5.0, 0.0]);
    let pins = [(g, [5.0, 5.0, 0.0])];
    let (_, beats) = salvation(&mut s, p);
    // Chain presses on each beat as he sees it: it reaches the server 0.18 sec after it fell.
    // Bren, with no lag, presses as late by the server's clock: too late.
    for at in &beats {
        until(&mut s, at + 0.18, &pins);
        act(&mut s, chain);
        act(&mut s, near);
    }
    let before = s.mobs[&g].health;
    run(&mut s, 0.6, &pins);
    assert_eq!(s.holy_stacks(chain), 3, "Chain landed each on his own screen");
    assert_eq!(s.holy_stacks(near), 0, "Bren's were late");
    let dealt = before - s.mobs[&g].health;
    assert_eq!(dealt, (45.0 * s.taken_share(g)).round() as u32, "the pillar waited on Chain's press");
}

#[test]
fn void_rift_is_shadows_ultimate_and_salvation_lights() {
    let s = world();
    let c = &s.content;
    let rift = &c.abilities[idx(&s, "void_rift")];
    let sal = &c.abilities[idx(&s, "holy_word_salvation")];
    assert_eq!((rift.choice, rift.school.as_str(), rift.slot.as_str()), (2, "priest", "ultimate"));
    assert_eq!((sal.choice, sal.school.as_str(), sal.slot.as_str()), (1, "priest", "ultimate"));
    assert_eq!((sal.valor, sal.cooldown), (100, 40.0));
    let malach = c.mob("malach").unwrap();
    assert!(c.mobs[malach].teaches.contains(&"holy_word_salvation".to_string()), "Brother Malach teaches it");
}

// --- Penance and Holy Fire --------------------------------------------------------------------------

/// Penance's three bolts, the reticle on each spot in turn as its bolt goes.
fn penance_at(s: &mut Survival, p: Id, spots: [Vec3; 3], pins: &[(Id, Vec3)]) -> Vec<Event> {
    learn(s, p, "penance");
    s.input(p, Input::Ability(key_of("ranged"), false));
    let mut ev = s.step(0.0);
    aim_at(s, p, spots[0]);
    s.input(p, Input::Attack);
    ev.extend(s.step(0.0));
    s.input(p, Input::AttackUp);
    let start = s.clock;
    for (k, spot) in spots.iter().enumerate().skip(1) {
        ev.extend(until(s, start + 0.5 * k as f32 - 0.05, pins));
        aim_at(s, p, *spot);
    }
    ev.extend(run(s, 1.0, pins));
    ev
}

#[test]
fn penance_flicked_between_a_friend_and_a_foe_heals_hurts_and_is_a_perfect() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let f = hero(&mut s, "Bren", [8.0, 4.0, 0.0], 0.0);
    let g = foe(&mut s, "grunt", [12.0, -4.0, 0.0]);
    let pins = [(g, [12.0, -4.0, 0.0])];
    let hf = halve(&mut s, f);
    let hg = s.mobs[&g].health;
    let valor = s.players[&p].valor;
    let ev = penance_at(&mut s, p, [[8.0, 4.0, 1.0], [12.0, -4.0, 1.0], [8.0, 4.0, 1.0]], &pins);
    assert_eq!(s.players[&f].health, hf + 24, "two bolts on Bren: 12 each");
    assert_eq!(hg - s.mobs[&g].health, (12.0 * s.taken_share(g)).round() as u32, "one on the grunt");
    assert_eq!(perfects(&ev, p), 1, "the flick");
    assert!(s.players[&p].valor >= valor + s.content.checks.perfect_valor, "its Valor");
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { caster, spell: 15237, target: Target::Point(_) } if *caster == p)), "the last bolt's burst");
    assert!(!matches!(s.players[&p].action, Action::Rhythm { .. }), "over after the third");
}

#[test]
fn penance_all_on_foes_is_no_perfect() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let g = foe(&mut s, "grunt", [12.0, -4.0, 0.0]);
    let pins = [(g, [12.0, -4.0, 0.0])];
    let hg = s.mobs[&g].health;
    let at = [12.0, -4.0, 1.0];
    let ev = penance_at(&mut s, p, [at, at, at], &pins);
    assert_eq!(hg - s.mobs[&g].health, (36.0 * s.taken_share(g)).round() as u32, "three bolts on it");
    assert_eq!(perfects(&ev, p), 0, "no friend: no perfect");
}

fn burn_of(s: &Survival, on: Id) -> Option<(f32, f32)> {
    s.cc_of(on).iter().find(|c| c.kind == "burn" && c.until > s.clock).map(|c| (c.amount, c.until - s.clock))
}

#[test]
fn holy_fire_burns_and_every_bolt_of_yours_fans_it_up_to_five_more() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.set_cooldowns(p, true);
    let g = foe(&mut s, "grunt", [12.0, 0.0, 0.0]);
    let pins = [(g, [12.0, 0.0, 0.0])];
    // (Enough health for all of it.)
    if let Some(m) = s.mobs.get_mut(&g) {
        m.max_health = 1000;
        m.health = 1000;
    }
    let hg = s.mobs[&g].health;
    s.set_valor(p, 100.0);
    aim_at(&mut s, p, [12.0, 0.0, 1.0]);
    let ev = cast(&mut s, p, "penance", true);
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { spell: 14914, .. })), "its holy fire");
    assert!(!ev.iter().any(|e| matches!(e, Event::Mark { .. })), "no ring under it");
    assert_eq!(hg - s.mobs[&g].health, (20.0 * s.taken_share(g)).round() as u32, "20 at once");
    let (dps, left) = burn_of(&s, g).expect("burning");
    assert!((dps - 3.0).abs() < 1.0e-4 && left > 5.5, "3 a sec for 6 sec: {dps} {left}");
    run(&mut s, 2.0, &pins);
    // Penance's three bolts on it: three more a second, and it starts over.
    let at = [12.0, 0.0, 1.0];
    penance_at(&mut s, p, [at, at, at], &pins);
    let (dps, left) = burn_of(&s, g).expect("still burning");
    assert!((dps - 6.0).abs() < 1.0e-4, "fanned three times: {dps}");
    assert!(left > 4.5, "started over: {left}");
    penance_at(&mut s, p, [at, at, at], &pins);
    let (dps, _) = burn_of(&s, g).unwrap();
    assert!((dps - 8.0).abs() < 1.0e-4, "at most 5 more: {dps}");
    let before = s.mobs[&g].health;
    run(&mut s, 2.0, &pins);
    assert!(before - s.mobs[&g].health >= 14, "burning hot: {}", before - s.mobs[&g].health);
}

// --- Holy Nova and Leap of Faith, counters for others -----------------------------------------------

/// A friend (Bren) at the origin facing west, a wolf 2 yd east of him, its first bite on its way;
/// the Priest `away` yd west of him.
fn bitten_friend(s: &mut Survival, away: f32) -> (Id, Id, Id) {
    let f = hero(s, "Bren", ORIGIN, WEST);
    let p = hero(s, "Ayla", [-away, 0.0, 0.0], 0.0);
    let def = s.content.mob("wolf").unwrap();
    let k = s.spawn_mob(def, [2.0, 0.0, 0.0], None, false);
    s.mobs.get_mut(&k).unwrap().state = MobState::Idle { wander_at: 1e9 };
    s.engage(k, Foe::Hero(f));
    (f, p, k)
}

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

fn nova(s: &mut Survival, p: Id) -> Vec<Event> {
    learn(s, p, "holy_nova");
    s.input(p, Input::Ability(key_of("close"), false));
    s.step(0.0)
}

#[test]
fn holy_nova_cast_as_a_bite_lands_on_a_friend_in_it_catches_the_bite() {
    let mut s = world();
    // (The wolf stays outside the nova: struck, it would reel and never bite.)
    let (f, p, k) = bitten_friend(&mut s, 7.5);
    let full = s.players[&f].health;
    until_bite(&mut s, k, 0.05);
    let valor = s.players[&p].valor;
    let mut ev = nova(&mut s, p);
    ev.extend(run(&mut s, 0.3, &[]));
    assert!(ev.iter().any(|e| matches!(e, Event::Swing { attacker, victim: Some(v), outcome: Outcome::Blocked, .. } if *attacker == k && *v == f)), "the bite caught");
    assert!(s.players[&f].health >= full, "nothing landed on Bren");
    assert_eq!(perfects(&ev, p), 1, "the Priest's perfect");
    assert!(s.players[&p].valor >= valor + s.content.valor.counter, "a counter's Valor");
}

#[test]
fn holy_nova_long_before_the_bite_is_a_plain_nova() {
    let mut s = world();
    let (f, p, k) = bitten_friend(&mut s, 7.5);
    let full = s.players[&f].health;
    until_bite(&mut s, k, 0.3);
    let mut ev = nova(&mut s, p);
    ev.extend(run(&mut s, 0.5, &[]));
    assert!(s.players[&f].health < full, "bitten");
    assert_eq!(perfects(&ev, p), 0);
}

#[test]
fn holy_nova_catches_a_bite_waiting_on_a_laggy_friends_round_trip() {
    let mut s = world();
    let (f, p, k) = bitten_friend(&mut s, 7.5);
    s.set_rtt(f, 0.18);
    let full = s.players[&f].health;
    until_bite(&mut s, k, 0.0);
    let mut ev = s.step(DT);
    ev.extend(run(&mut s, 0.08, &[]));
    ev.extend(nova(&mut s, p));
    ev.extend(run(&mut s, 0.3, &[]));
    assert!(s.players[&f].health >= full, "caught while it waited on Bren");
    assert_eq!(perfects(&ev, p), 1);
}

fn leap(s: &mut Survival, p: Id, at: Vec3) -> Vec<Event> {
    learn(s, p, "leap_of_faith");
    s.input(p, Input::Ability(key_of("utility"), false));
    let mut ev = s.step(0.0);
    aim_at(s, p, at);
    s.input(p, Input::Attack);
    ev.extend(s.step(0.0));
    s.input(p, Input::AttackUp);
    ev
}

#[test]
fn leap_of_faith_pulling_a_friend_as_the_bite_lands_pulls_them_out_of_it() {
    let mut s = world();
    let (f, p, k) = bitten_friend(&mut s, 3.0);
    let full = s.players[&f].health;
    // The ray takes about a tenth of a second to reach him.
    until_bite(&mut s, k, 0.12);
    let valor = s.players[&p].valor;
    let mut ev = leap(&mut s, p, [0.0, 0.0, 1.0]);
    ev.extend(run(&mut s, 0.5, &[]));
    // (Thrown by their own client: told to hop her way.)
    assert!(ev.iter().any(|e| matches!(e, Event::Hop { player, dir, .. } if *player == f && (dir - WEST).abs() < 0.1)), "pulled to her");
    assert!(ev.iter().any(|e| matches!(e, Event::Swing { attacker, victim: Some(v), outcome: Outcome::Miss, .. } if *attacker == k && *v == f)), "the bite missed");
    assert!(s.players[&f].health >= full, "nothing landed");
    assert_eq!(perfects(&ev, p), 1, "the Priest's perfect");
    assert!(s.players[&p].valor >= valor + s.content.valor.counter, "a counter's Valor");
}

#[test]
fn leap_of_faith_long_before_the_bite_is_a_plain_pull() {
    let mut s = world();
    let (_, p, k) = bitten_friend(&mut s, 3.0);
    until_bite(&mut s, k, 0.45);
    let mut ev = leap(&mut s, p, [0.0, 0.0, 1.0]);
    ev.extend(run(&mut s, 0.6, &[]));
    assert_eq!(perfects(&ev, p), 0);
}

// --- saves ------------------------------------------------------------------------------------------

#[test]
fn a_save_with_void_rift_and_the_old_light_keys_still_loads_them() {
    let mut s = world();
    s.content.dev.full_tree = false;
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let mut save = s.leave(p).unwrap();
    save.owned = ["penance", "holy_nova", "leap_of_faith", "power_word_barrier", "void_rift"].into_iter().map(String::from).collect();
    save.loadout = [("ranged", "penance"), ("close", "holy_nova"), ("utility", "leap_of_faith"), ("defense", "power_word_barrier"), ("ultimate", "void_rift")]
        .into_iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect();
    let q = s.join("ayla", "Ayla", Some(&save));
    let h = &s.players[&q];
    for (slot, k) in [("ranged", "penance"), ("close", "holy_nova"), ("utility", "leap_of_faith"), ("defense", "power_word_barrier"), ("ultimate", "void_rift")] {
        assert_eq!(h.loadout.get(slot).map(String::as_str), Some(k), "{slot}");
    }
    // And Void Rift still goes off from the 3 key.
    s.set_valor(q, 100.0);
    s.input(q, Input::Ability(key_of("ultimate"), false));
    s.step(0.0);
    assert!(matches!(s.players[&q].action, Action::Aim { .. }), "aimed, as ever");
}

#[test]
fn a_new_priest_starts_with_salvation_on_the_3_key() {
    let mut s = world();
    let p = s.join("ayla", "Ayla", None);
    s.players.get_mut(&p).unwrap().class = "priest".into();
    let (loadout, _) = s.fresh_abilities("priest");
    assert_eq!(loadout.get("ultimate").map(String::as_str), Some("holy_word_salvation"));
}
