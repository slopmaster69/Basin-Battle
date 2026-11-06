//! The Light Priest, draft 3 (docs/priest-draft3.md): Penance's rings and their timing, Holy Nova
//! and Surge of Light (stronger for every hero inside), Angelic Feather and Body and Soul (the road
//! out, taken by choice), and Holy Word: Salvation (the group's beat, the caster aloft).

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.01;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];
const EAST: f32 = 0.0;
/// The Mage's Arcane Explosion (`[checks] perfect_kit`), which no Priest perfect may show.
const MAGE_KIT: u32 = 1004;

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

/// A creature standing idle where it's put (`ready_at` far off: it never swings).
fn foe(s: &mut Survival, key: &str, pos: Vec3) -> Id {
    let def = s.content.mob(key).unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.ready_at = 1e9;
    m.pos = pos;
    id
}

/// A grunt of 1000 health that stands and takes it (a training dummy heals back up as soon as
/// no one has it as their foe).
fn post(s: &mut Survival, pos: Vec3) -> Id {
    let g = foe(s, "grunt", pos);
    let m = s.mobs.get_mut(&g).unwrap();
    m.max_health = 1000;
    m.health = 1000;
    g
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
    if ex {
        s.set_valor(p, 100.0);
    }
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

fn press(s: &mut Survival, p: Id, slot: &str) -> Vec<Event> {
    s.input(p, Input::Ability(key_of(slot), false));
    s.step(0.0)
}

fn perfects(ev: &[Event], p: Id) -> usize {
    ev.iter().filter(|e| matches!(e, Event::Perfect { player, .. } if *player == p)).count()
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

fn halve(s: &mut Survival, p: Id) -> u32 {
    let h = s.players.get_mut(&p).unwrap();
    h.health = h.max_health / 2;
    h.health
}

fn hp(s: &Survival, m: Id) -> u32 {
    s.mobs[&m].health
}

fn has_haste(s: &Survival, p: Id, amount: f32) -> bool {
    s.cc_of(p).iter().any(|c| c.kind == "haste" && (c.amount - amount).abs() < 1.0e-3 && c.until > s.clock)
}

// --- Penance ----------------------------------------------------------------------------------------

/// The rings of `p`'s Penance under way: (closing at, on whom, a friend).
fn rings(s: &Survival, p: Id) -> Vec<(f32, Id, bool)> {
    s.light.penances.iter().find(|x| x.owner == p).map(|x| x.rings.iter().map(|r| (r.at, r.on, r.friend)).collect()).unwrap_or_default()
}

/// Penance's key pressed `off` seconds from each ring's closing in turn (`None`: that ring let
/// pass).
fn keep_time(s: &mut Survival, p: Id, offs: &[Option<f32>], pins: &[(Id, Vec3)]) -> Vec<Event> {
    let mut ev = Vec::new();
    let all = rings(s, p);
    for (k, off) in offs.iter().enumerate() {
        let Some(off) = off else { continue };
        ev.extend(until(s, all[k].0 + off, pins));
        ev.extend(press(s, p, "ranged"));
    }
    ev.extend(run(s, 0.5, pins));
    ev
}

/// A Priest at the origin looking east, a dummy 10 yd ahead and a hurt friend a little off the
/// reticle 12 yd out.
fn penance_set_up(s: &mut Survival) -> (Id, Id, Id) {
    let p = hero(s, "Ayla", ORIGIN, EAST);
    s.set_aim_line(p, EAST, 0.0);
    let d = foe(s, "training_dummy", [10.0, 0.0, 0.0]);
    let f = hero(s, "Bren", [12.0, 3.0, 0.0], EAST);
    halve(s, f);
    (p, d, f)
}

#[test]
fn penance_rings_friend_and_foe_in_turn_and_q_on_each_ring_sends_its_bolt() {
    let mut s = world();
    let (p, d, f) = penance_set_up(&mut s);
    let (hd, hf) = (hp(&s, d), s.players[&f].health);
    let start = s.clock;
    let mut ev = cast(&mut s, p, "penance", false);
    let r = rings(&s, p);
    assert_eq!(r.len(), 3, "three rings");
    // Nearest the reticle first (the dummy), then turn about: Bren, the dummy again.
    assert_eq!(r.iter().map(|x| (x.1, x.2)).collect::<Vec<_>>(), vec![(d, false), (f, true), (d, false)], "foe, friend, foe");
    assert!((r[0].0 - start - 0.6).abs() < 0.02 && (r[1].0 - r[0].0 - 0.55).abs() < 0.02 && (r[2].0 - r[1].0 - 0.55).abs() < 0.02, "0.6 sec, then 0.55 apart: {r:?}");
    let looks = ev.iter().filter(|e| matches!(e, Event::PriestLook { player, kind: "penance", .. } if *player == p)).count();
    assert_eq!(looks, 3, "each ring told to the caster's screen");
    assert!(!matches!(s.players[&p].action, Action::Channel { .. } | Action::Rhythm { .. } | Action::Stance { .. }), "no channel, no stance: {:?}", s.players[&p].action);
    let valor = s.players[&p].valor;
    // Each dead on the beat: 18.
    ev.extend(keep_time(&mut s, p, &[Some(0.0), Some(0.0), Some(0.0)], &[]));
    assert_eq!(hd - hp(&s, d), 36, "two bolts on the dummy, 18 each");
    assert_eq!(s.players[&f].health, hf + 18, "one on Bren");
    assert_eq!(perfects(&ev, p), 1, "every ring landed: the perfect");
    assert!(s.players[&p].valor >= valor + s.content.checks.perfect_valor, "its Valor");
    assert!(priest_burst(&s, &ev, p, "penance"), "the Priest's holy burst");
    assert!(!mage_burst(&ev), "never Arcane Explosion's");
    assert!(s.light.penances.is_empty(), "over");
    let cd = s.content.abilities[idx(&s, "penance")].cooldown;
    assert!((cd - 9.0).abs() < 1.0e-4, "9 sec");
    assert!(s.cooldown(p, idx(&s, "penance")).charges == 0, "on its cooldown");
}

#[test]
fn a_press_in_the_window_but_off_the_beat_is_14_and_still_counts() {
    let mut s = world();
    let (p, d, f) = penance_set_up(&mut s);
    let (hd, hf) = (hp(&s, d), s.players[&f].health);
    cast(&mut s, p, "penance", false);
    // 0.1 sec off: inside the 0.16 window, outside its dead-on middle (0.3 of it).
    let ev = keep_time(&mut s, p, &[Some(0.1), Some(-0.1), Some(0.1)], &[]);
    assert_eq!(hd - hp(&s, d), 28, "14 a bolt");
    assert_eq!(s.players[&f].health, hf + 14);
    assert_eq!(perfects(&ev, p), 1, "every ring landed is still the perfect");
}

#[test]
fn an_early_press_loses_its_ring_and_a_ring_let_pass_is_lost_but_the_rest_still_come() {
    let mut s = world();
    let (p, d, f) = penance_set_up(&mut s);
    let (hd, hf) = (hp(&s, d), s.players[&f].health);
    cast(&mut s, p, "penance", false);
    // A press long before the first ring is near is nothing at all.
    let first = rings(&s, p)[0].0;
    until(&mut s, first - 0.58, &[]);
    press(&mut s, p, "ranged");
    assert!(s.light.penances[0].rings[0].landed.is_none(), "a stray press, no penalty");
    // 0.3 sec early on the first: lost. The second let pass. The third on time.
    let ev = keep_time(&mut s, p, &[Some(-0.3), None, Some(0.0)], &[]);
    assert_eq!(hd - hp(&s, d), 18, "only the third's bolt on the dummy");
    assert_eq!(s.players[&f].health, hf, "Bren's ring let pass: nothing");
    assert_eq!(perfects(&ev, p), 0, "no perfect");
    assert!(!priest_burst(&s, &ev, p, "penance"));
    assert!(s.light.penances.is_empty(), "over all the same");
}

#[test]
fn a_ring_on_a_laggy_priests_screen_waits_out_their_round_trip() {
    let mut s = world();
    let (p, d, _) = penance_set_up(&mut s);
    s.set_rtt(p, 0.2);
    cast(&mut s, p, "penance", false);
    let first = rings(&s, p)[0].0;
    // Past the window but within the window and the round trip: not yet lost.
    until(&mut s, first + 0.3, &[]);
    assert!(s.light.penances[0].rings[0].landed.is_none(), "waiting on her round trip");
    run(&mut s, 0.1, &[]);
    assert_eq!(s.light.penances[0].rings[0].landed, Some(false), "then lost");
    let _ = d;
}

#[test]
fn penance_picks_only_hurt_friends_and_bodies_in_front_within_30_yd() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    s.set_aim_line(p, EAST, 0.0);
    let d = foe(&mut s, "training_dummy", [10.0, 0.0, 0.0]);
    let whole = hero(&mut s, "Bren", [8.0, 1.0, 0.0], EAST);
    let _behind = foe(&mut s, "training_dummy", [-6.0, 0.0, 0.0]);
    let _wide = foe(&mut s, "training_dummy", [5.0, 6.0, 0.0]);
    let _far = foe(&mut s, "training_dummy", [33.0, 0.0, 0.0]);
    cast(&mut s, p, "penance", false);
    let r = rings(&s, p);
    assert_eq!(r.len(), 3);
    assert!(r.iter().all(|x| x.1 == d), "only the dummy in front: not Bren at full health, nor the dummies behind, wide (50°) or 33 yd off: {r:?}");
    let _ = whole;
}

#[test]
fn penance_with_no_one_in_front_is_refused_and_costs_nothing() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    s.set_aim_line(p, EAST, 0.0);
    let _behind = foe(&mut s, "training_dummy", [-10.0, 0.0, 0.0]);
    let ev = cast(&mut s, p, "penance", false);
    assert!(s.light.penances.is_empty(), "no rings");
    assert!(ev.iter().any(|e| matches!(e, Event::Flash { player, text } if *player == p && text.contains("No one"))), "told why");
    assert_eq!(s.cooldown(p, idx(&s, "penance")).charges, 1, "cooldown given back");
    // Greater Penance: its Valor given back too.
    let ev = cast(&mut s, p, "penance", true);
    assert!(s.light.penances.is_empty());
    assert!((s.players[&p].valor - 100.0).abs() < 1.0e-3, "Valor given back: {}", s.players[&p].valor);
    assert_eq!(s.cooldown(p, idx(&s, "penance")).charges, 1);
    let _ = ev;
}

#[test]
fn the_priest_moves_strikes_and_rolls_while_the_rings_close() {
    let mut s = world();
    let (p, d, _) = penance_set_up(&mut s);
    let hd = hp(&s, d);
    cast(&mut s, p, "penance", false);
    run(&mut s, 0.4, &[]);
    assert!(matches!(s.players[&p].action, Action::Ready), "free between the rings: {:?}", s.players[&p].action);
    // A roll.
    s.input(p, Input::Dodge);
    s.step(0.0);
    assert!(matches!(s.players[&p].action, Action::Dodge { .. }), "rolling: {:?}", s.players[&p].action);
    // Still running: each ring still lands on its beat.
    let ev = keep_time(&mut s, p, &[Some(0.0), Some(0.0), Some(0.0)], &[]);
    assert_eq!(hd - hp(&s, d), 36, "rolled, and still kept time");
    assert_eq!(perfects(&ev, p), 1);
    // Walking meanwhile changes nothing: the rings are on the bodies, not the camera.
    let mut s = world();
    let (p, d, _) = penance_set_up(&mut s);
    let hd = hp(&s, d);
    cast(&mut s, p, "penance", false);
    run(&mut s, 0.3, &[]);
    s.move_player(p, [0.0, -2.0, 0.0], std::f32::consts::FRAC_PI_2, Moving { forward: true, ..Default::default() });
    s.set_aim_line(p, std::f32::consts::PI, 0.0);
    keep_time(&mut s, p, &[Some(0.0), Some(0.0), Some(0.0)], &[]);
    assert_eq!(hd - hp(&s, d), 36, "the camera swung away, the bolts still go");
}

#[test]
fn greater_penance_has_five_rings_closing_quicker() {
    let mut s = world();
    let (p, d, f) = penance_set_up(&mut s);
    let (hd, hf) = (hp(&s, d), s.players[&f].health);
    let ev = cast(&mut s, p, "penance", true);
    let r = rings(&s, p);
    assert_eq!(r.len(), 5, "five rings");
    assert!((r[1].0 - r[0].0 - 0.45).abs() < 0.02, "0.45 apart");
    assert_eq!(r.iter().filter(|x| x.2).count(), 2, "friend and foe in turn: {r:?}");
    let mut ev = ev;
    ev.extend(keep_time(&mut s, p, &[Some(0.0); 5], &[]));
    assert_eq!(hd - hp(&s, d), 54, "three on the dummy");
    assert_eq!(s.players[&f].health, hf + 36, "two on Bren");
    assert_eq!(perfects(&ev, p), 1);
    assert!(priest_burst(&s, &ev, p, "greater_penance") && !mage_burst(&ev));
}

// --- Holy Nova and Surge of Light -------------------------------------------------------------------

#[test]
fn holy_nova_alone_deals_16_and_heals_the_priest_16() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let hp0 = halve(&mut s, p);
    let d = post(&mut s, [5.0, 0.0, 0.0]);
    let out = post(&mut s, [9.5, 0.0, 0.0]);
    let (hd, hout) = (hp(&s, d), hp(&s, out));
    let ev = cast(&mut s, p, "holy_nova", false);
    assert_eq!(hd - hp(&s, d), 16, "16 to the foe within 8 yd");
    assert_eq!(hp(&s, out), hout, "not past 8 yd");
    assert_eq!(s.players[&p].health, hp0 + 16, "and 16 to herself");
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { spell: 15237, .. })), "its burst");
    assert!(perfects(&ev, p) == 0, "no timing check");
}

#[test]
fn holy_nova_is_25_percent_stronger_for_each_other_hero_inside_up_to_75() {
    // Two others inside: 1.5 times.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let a = hero(&mut s, "Bren", [3.0, 0.0, 0.0], EAST);
    let b = hero(&mut s, "Cara", [-3.0, 2.0, 0.0], EAST);
    let far = hero(&mut s, "Dane", [12.0, 0.0, 0.0], EAST);
    let (ha, hb, hfar) = (halve(&mut s, a), halve(&mut s, b), halve(&mut s, far));
    let d = post(&mut s, [5.0, 2.0, 0.0]);
    let hd = hp(&s, d);
    cast(&mut s, p, "holy_nova", false);
    assert_eq!(hd - hp(&s, d), 24, "16 x 1.5");
    assert_eq!((s.players[&a].health, s.players[&b].health), (ha + 24, hb + 24), "24 healing each");
    assert_eq!(s.players[&far].health, hfar, "Dane outside: neither counted nor healed");
    // Four others inside: capped at 75%.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    for (k, at) in [[2.0, 0.0, 0.0], [-2.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, -2.0, 0.0]].into_iter().enumerate() {
        hero(&mut s, &format!("H{k}"), at, EAST);
    }
    let d = post(&mut s, [5.0, 2.0, 0.0]);
    let hd = hp(&s, d);
    cast(&mut s, p, "holy_nova", false);
    assert_eq!(hd - hp(&s, d), 28, "16 x 1.75, no more");
}

#[test]
fn surge_of_light_is_bigger_and_its_friends_run_faster() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let a = hero(&mut s, "Bren", [9.0, 0.0, 0.0], EAST);
    let ha = halve(&mut s, a);
    let d = post(&mut s, [0.0, 9.5, 0.0]);
    let hd = hp(&s, d);
    cast(&mut s, p, "holy_nova", true);
    assert_eq!(hd - hp(&s, d), 30, "24 x 1.25 within 10 yd");
    assert_eq!(s.players[&a].health, ha + 30);
    assert!(has_haste(&s, a, 0.3), "Bren runs 30% faster");
    // (25 Valor spent, a little back for the hits.)
    assert!(s.players[&p].valor < 90.0, "25 Valor: {}", s.players[&p].valor);
}

// --- Angelic Feather and Body and Soul --------------------------------------------------------------

/// A feather laid along a level reticle (20 yd out): where it lies.
fn feather(s: &mut Survival, p: Id) -> Vec3 {
    let n = s.light.feathers.len();
    cast(s, p, "angelic_feather", false);
    assert_eq!(s.light.feathers.len(), n + 1, "laid");
    s.light.feathers.last().unwrap().at
}

#[test]
fn angelic_feather_lies_at_the_reticle_and_frees_and_quickens_the_first_hero_to_touch_it() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    s.set_aim_line(p, EAST, 0.0);
    let f = hero(&mut s, "Bren", [5.0, 5.0, 0.0], EAST);
    let at = feather(&mut s, p);
    assert!((at[0] - 20.0).abs() < 0.6 && at[1].abs() < 0.3, "20 yd along the reticle: {at:?}");
    assert_eq!(s.cooldown(p, idx(&s, "angelic_feather")).charges, 1, "one of its 2 charges left");
    run(&mut s, 0.5, &[]);
    assert_eq!(s.light.feathers.len(), 1, "no one on it yet");
    // Bren, rooted and slowed, steps on it.
    let m = foe(&mut s, "training_dummy", [30.0, 0.0, 0.0]);
    s.apply_cc(f, m, "root", 5.0, 0.0, 0.0, "test");
    s.apply_cc(f, m, "snare", 5.0, 0.5, 0.0, "test");
    s.move_player(f, [at[0] + 0.5, at[1], 0.0], EAST, Moving::default());
    let ev = run(&mut s, 0.1, &[]);
    assert!(s.light.feathers.is_empty(), "taken");
    assert!(!s.has_cc(f, "root") && !s.has_cc(f, "snare"), "freed of root and slow");
    assert!(has_haste(&s, f, 0.4), "40% faster");
    let left = s.cc_of(f).iter().find(|c| c.kind == "haste").map(|c| c.until - s.clock).unwrap();
    assert!(left > 2.7 && left <= 3.0 + 1.0e-3, "for 3 sec: {left}");
    assert!(ev.iter().any(|e| matches!(e, Event::PriestGround { kind: "feather", radius, .. } if *radius == 0.0)), "its look off the ground");
    // The Priest walking there after: nothing left to take.
    s.move_player(p, at, EAST, Moving::default());
    run(&mut s, 0.1, &[]);
    assert!(!has_haste(&s, p, 0.4), "only the first");
}

#[test]
fn the_priest_can_take_her_own_feather() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    s.set_aim_line(p, EAST, 0.0);
    let at = feather(&mut s, p);
    s.move_player(p, at, EAST, Moving::default());
    run(&mut s, 0.1, &[]);
    assert!(s.light.feathers.is_empty() && has_haste(&s, p, 0.4), "her own road out");
}

#[test]
fn a_foe_cant_take_a_feather_and_it_fades_after_15_sec() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    s.set_aim_line(p, EAST, 0.0);
    let at = feather(&mut s, p);
    let g = foe(&mut s, "grunt", at);
    run(&mut s, 1.0, &[(g, at)]);
    assert_eq!(s.light.feathers.len(), 1, "a creature on it takes nothing");
    assert!(!has_haste(&s, g, 0.4) && !s.has_cc(g, "haste"));
    run(&mut s, 13.8, &[(g, at)]);
    assert_eq!(s.light.feathers.len(), 1, "still there just before 15 sec");
    run(&mut s, 0.3, &[(g, at)]);
    assert!(s.light.feathers.is_empty(), "faded at 15 sec");
}

#[test]
fn two_feathers_lie_at_once_the_oldest_going_for_a_third() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    s.set_cooldowns(p, true);
    let mut laid = Vec::new();
    for yaw in [0.0f32, 0.6, 1.2] {
        s.set_aim_line(p, yaw, 0.0);
        cast(&mut s, p, "angelic_feather", false);
        laid.push(s.light.feathers.last().expect("laid").at);
        run(&mut s, 0.6, &[]);
    }
    assert_eq!(s.light.feathers.len(), 2, "two at most");
    let still: Vec<Vec3> = s.light.feathers.iter().map(|f| f.at).collect();
    assert!(!still.iter().any(|a| dist2d(*a, laid[0]) < 0.1), "the oldest gone");
    assert!(still.iter().any(|a| dist2d(*a, laid[2]) < 0.1), "the newest there");
}

#[test]
fn body_and_soul_frees_and_quickens_everyone_near_at_once() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let a = hero(&mut s, "Bren", [6.0, 0.0, 0.0], EAST);
    let far = hero(&mut s, "Cara", [14.0, 0.0, 0.0], EAST);
    let m = foe(&mut s, "training_dummy", [30.0, 0.0, 0.0]);
    for h in [p, a, far] {
        s.apply_cc(h, m, "snare", 5.0, 0.5, 0.0, "test");
    }
    s.apply_cc(a, m, "root", 5.0, 0.0, 0.0, "test");
    cast(&mut s, p, "angelic_feather", true);
    for h in [p, a] {
        assert!(!s.has_cc(h, "snare") && !s.has_cc(h, "root"), "freed");
        assert!(has_haste(&s, h, 0.4), "40% faster");
        let left = s.cc_of(h).iter().find(|c| c.kind == "haste").map(|c| c.until - s.clock).unwrap();
        assert!(left > 3.8, "for 4 sec: {left}");
    }
    assert!(s.has_cc(far, "snare") && !has_haste(&s, far, 0.4), "not past 10 yd");
    assert!(s.light.feathers.is_empty(), "no feather: at once");
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
fn salvations_ring_runs_out_from_the_priest_to_the_rim_on_each_beat() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let (mut ev, beats) = salvation(&mut s, p);
    let start = s.clock;
    assert!((beats[0] - start - 2.0).abs() < 0.02 && (beats[1] - beats[0] - 1.5).abs() < 0.02 && (beats[2] - beats[1] - 1.0).abs() < 0.02, "2, 1.5, 1 sec: {beats:?}");
    ev.extend(until(&mut s, beats[0] - 0.05, &[]));
    let rims: Vec<f32> = ev
        .iter()
        .filter_map(|e| match e {
            Event::Mark { radius, .. } if *radius < 9.99 => Some(*radius),
            _ => None,
        })
        .collect();
    assert!(rims.len() >= 5, "the ring in steps: {rims:?}");
    assert!(rims.windows(2).all(|w| w[1] > w[0]), "growing outward, from the Priest to the rim: {rims:?}");
    assert!(rims[0] < 2.5, "from near the centre: {rims:?}");
}

#[test]
fn salvation_rewards_each_hero_for_the_beats_they_land_and_its_pillar_grows_with_them() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let a = hero(&mut s, "Bren", [3.0, 0.0, 0.0], 0.0);
    let b = hero(&mut s, "Cara", [-3.0, 2.0, 0.0], 0.0);
    let out = hero(&mut s, "Dane", [16.0, 0.0, 0.0], 0.0);
    let g = foe(&mut s, "training_dummy", [5.0, 5.0, 0.0]);
    let pins = [(g, [5.0, 5.0, 0.0])];
    let (ha, hb, hout) = (halve(&mut s, a), halve(&mut s, b), halve(&mut s, out));
    let (mut ev, beats) = salvation(&mut s, p);
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
    let before = hp(&s, g);
    ev.extend(run(&mut s, 0.6, &pins));
    assert_eq!(s.holy_stacks(a), 3, "Bren landed all three");
    assert_eq!(s.holy_stacks(b), 2, "Cara missed one");
    assert_eq!(s.holy_stacks(out), 0, "Dane stood outside it");
    assert_eq!(perfects(&ev, a), 3);
    assert_eq!(perfects(&ev, b), 2);
    assert_eq!(perfects(&ev, out), 0);
    assert!(priest_burst(&s, &ev, a, "holy_word_salvation") && priest_burst(&s, &ev, b, "holy_word_salvation"), "each beat landed bursts in the Priest's light");
    assert!(!mage_burst(&ev), "never Arcane Explosion's");
    // The pillar: 30, and 15 for each of the two who landed the third.
    assert_eq!(before - hp(&s, g), 60, "30 + 2 x 15");
    assert_eq!(s.players[&a].health, ha + 20, "healed 20");
    assert_eq!(s.players[&b].health, hb + 20, "healed 20");
    assert_eq!(s.players[&out].health, hout, "not one outside it");
    assert!(s.auras_of(a).contains(&20007), "Holy Light's glow on him");
    assert!(s.light.sigils.is_empty(), "the circle gone");
    // Holy Light: 10% a stack, for 10 sec.
    let might = s.effect_of(&s.players[&a], "might");
    assert!((might - 0.3).abs() < 1.0e-4, "30% more damage: {might}");
    run(&mut s, 10.5, &pins);
    assert_eq!(s.holy_stacks(a), 0, "gone after 10 sec");
}

#[test]
fn salvations_pillar_is_110_at_most() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let heroes: Vec<Id> = (0..6).map(|k| hero(&mut s, &format!("H{k}"), [2.0 * (k as f32 - 2.5), 3.0, 0.0], 0.0)).collect();
    let g = foe(&mut s, "training_dummy", [5.0, -5.0, 0.0]);
    let pins = [(g, [5.0, -5.0, 0.0])];
    let (_, beats) = salvation(&mut s, p);
    until(&mut s, beats[2], &pins);
    for h in &heroes {
        act(&mut s, *h);
    }
    let before = hp(&s, g);
    run(&mut s, 0.6, &pins);
    assert_eq!(before - hp(&s, g), 110, "30 + 6 x 15 = 120, held to 110");
}

#[test]
fn salvation_lifts_its_priest_out_of_reach_rooted_but_still_fighting_then_down() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let rack = s.objs[&s.rack_id()].pos;
    s.players.get_mut(&p).unwrap().pos = rack;
    s.arm(p, "acolyte_staff").unwrap();
    s.players.get_mut(&p).unwrap().pos = ORIGIN;
    s.set_aim_line(p, EAST, 0.0);
    let d = foe(&mut s, "training_dummy", [12.0, 0.0, 0.0]);
    let g = foe(&mut s, "grunt", [0.0, 2.0, 0.0]);
    let (ev, beats) = salvation(&mut s, p);
    assert!(ev.iter().any(|e| matches!(e, Event::Hop { player, lift, .. } if *player == p && *lift > 0.0)), "up on wings of light");
    assert!(s.salvation_aloft(p), "aloft");
    assert!(s.untouchable(p), "nothing reaches her");
    run(&mut s, 1.0, &[(g, [0.0, 2.0, 0.0])]);
    assert!(s.players[&p].rooted, "held up there, unable to move");
    let full = s.players[&p].health;
    s.strike_player(g, p, 20, [0.0, 2.0, 0.0], true, false, true, Blow::Light);
    assert_eq!(s.players[&p].health, full, "no blow lands on her");
    assert!(!s.apply_cc(p, g, "stun", 2.0, 0.0, 0.0, "test"), "no stun takes hold");
    // She still casts from the air: a Smite.
    let hd = hp(&s, d);
    s.input(p, Input::Attack);
    s.input(p, Input::AttackUp);
    run(&mut s, 0.8, &[(g, [0.0, 2.0, 0.0])]);
    assert!(hp(&s, d) < hd, "a Smite from the air");
    // Down after the last beat.
    until(&mut s, beats[2] + 1.0, &[(g, [0.0, 2.0, 0.0])]);
    assert!(!s.salvation_aloft(p), "let go after it");
    assert!(!s.players[&p].rooted, "free to move again");
}

#[test]
fn alone_the_priest_lands_salvations_beats_herself() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let g = foe(&mut s, "training_dummy", [5.0, 5.0, 0.0]);
    let pins = [(g, [5.0, 5.0, 0.0])];
    let (mut ev, beats) = salvation(&mut s, p);
    for at in &beats {
        ev.extend(until(&mut s, *at, &pins));
        act(&mut s, p);
    }
    let before = hp(&s, g);
    ev.extend(run(&mut s, 0.6, &pins));
    assert_eq!(s.holy_stacks(p), 3, "her own beats");
    assert_eq!(perfects(&ev, p), 3);
    assert!(priest_burst(&s, &ev, p, "holy_word_salvation") && !mage_burst(&ev));
    // 30 + 15 for her; her own three stacks of Holy Light (10% each) make her pillar harder too.
    assert_eq!(before - hp(&s, g), (45.0f32 * 1.3).round() as u32 , "30 + 15 for her, x1.3");
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
    let g = foe(&mut s, "training_dummy", [5.0, 5.0, 0.0]);
    let pins = [(g, [5.0, 5.0, 0.0])];
    let (_, beats) = salvation(&mut s, p);
    for at in &beats {
        until(&mut s, at + 0.18, &pins);
        act(&mut s, chain);
        act(&mut s, near);
    }
    let before = hp(&s, g);
    run(&mut s, 0.6, &pins);
    assert_eq!(s.holy_stacks(chain), 3, "Chain landed each on his own screen");
    assert_eq!(s.holy_stacks(near), 0, "Bren's were late");
    assert_eq!(before - hp(&s, g), 45, "the pillar waited on Chain's press");
}

#[test]
fn salvation_and_void_rift_are_the_ultimates() {
    let s = world();
    let c = &s.content;
    let rift = &c.abilities[idx(&s, "void_rift")];
    let sal = &c.abilities[idx(&s, "holy_word_salvation")];
    assert_eq!((rift.choice, rift.school.as_str(), rift.slot.as_str()), (2, "priest", "ultimate"));
    assert_eq!((sal.choice, sal.school.as_str(), sal.slot.as_str()), (1, "priest", "ultimate"));
    assert_eq!((sal.valor, sal.cooldown), (100, 40.0));
    assert_eq!((rift.valor, rift.cooldown), (100, 40.0));
}

#[test]
fn a_new_priest_starts_with_salvation_on_the_3_key() {
    let mut s = world();
    let p = s.join("ayla", "Ayla", None);
    s.players.get_mut(&p).unwrap().class = "priest".into();
    let (loadout, _) = s.fresh_abilities("priest");
    assert_eq!(loadout.get("ultimate").map(String::as_str), Some("holy_word_salvation"));
}

