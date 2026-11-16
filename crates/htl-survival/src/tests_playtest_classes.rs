//! One test per line of docs/playtest-checklist.md's "Class abilities (the EX reworks)" section, each asserting what
//! that line promises, from the real content. Named after the item. (What can only be seen in the client, the
//! looks and glows and banners, isn't here.)

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.01;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];
const EAST: f32 = 0.0;
const WEST: f32 = std::f32::consts::PI;

/// The built-in rules on flat land, every card owned, no mending by itself; heroes strike each other with `pvp`.
fn world(pvp: bool) -> Survival {
    let mut c = Content::dev();
    if !pvp {
        c.pvp = Default::default();
    }
    c.dev.full_tree = true;
    c.player.field_regen = 0.0;
    c.player.rest_regen = 0.0;
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

/// A creature standing idle where it's put, never swinging.
fn foe(s: &mut Survival, key: &str, pos: Vec3) -> Id {
    let def = s.content.mob(key).unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.ready_at = 1e9;
    m.pos = pos;
    m.facing = WEST;
    id
}

/// A grunt of 1000 health that stands and takes it.
fn post(s: &mut Survival, pos: Vec3) -> Id {
    let g = foe(s, "grunt", pos);
    let m = s.mobs.get_mut(&g).unwrap();
    m.max_health = 1000;
    m.health = 1000;
    g
}

fn boss(s: &mut Survival, pos: Vec3) -> Id {
    let def = s.content.mobs.iter().position(|d| d.boss && !d.totem && !d.inert()).expect("a boss");
    let id = s.spawn_mob(def, pos, None, false);
    s.mobs.get_mut(&id).unwrap().state = MobState::Idle { wander_at: 1e9 };
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

/// A card learned and used (its EX with `ex`: 100 Valor given first); an aimed one goes on a tap of the click.
fn cast(s: &mut Survival, p: Id, key: &str, ex: bool) -> Vec<Event> {
    cast_with(s, p, key, ex, 100.0)
}

fn cast_with(s: &mut Survival, p: Id, key: &str, ex: bool, valor: f32) -> Vec<Event> {
    learn(s, p, key);
    let slot = s.content.abilities[idx(s, key)].slot.clone();
    if ex {
        s.set_valor(p, valor);
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

fn hp(s: &Survival, m: Id) -> u32 {
    s.mobs[&m].health
}

fn halve(s: &mut Survival, p: Id) {
    let h = s.players.get_mut(&p).unwrap();
    h.health = h.max_health / 2;
}

/// How long an effect of `kind` has left on `u`.
fn left(s: &Survival, u: Id, kind: &str) -> f32 {
    s.cc_of(u).iter().find(|c| c.kind == kind && c.until > s.clock).map_or(0.0, |c| c.until - s.clock)
}

fn perfects(ev: &[Event], p: Id) -> usize {
    ev.iter().filter(|e| matches!(e, Event::Perfect { player, .. } if *player == p)).count()
}

fn flashed(ev: &[Event], p: Id, text: &str) -> bool {
    ev.iter().any(|e| matches!(e, Event::Flash { player, text: t } if *player == p && t.contains(text)))
}

/// The damage of each of `p`'s blows that landed on `m`.
fn hits(ev: &[Event], p: Id, m: Id) -> Vec<u32> {
    ev.iter()
        .filter_map(|e| match e {
            Event::Swing { attacker, victim: Some(v), damage, outcome: Outcome::Hit, .. } if *attacker == p && *v == m && *damage > 0 => Some(*damage),
            _ => None,
        })
        .collect()
}

/// The reticle on a spot on the ground.
fn aim_ground(s: &mut Survival, p: Id, at: Vec3) {
    let from = s.players[&p].pos;
    let d = dist2d(from, at).max(0.5);
    s.set_aim_line(p, facing(from, at), -(1.9 + from[2] - at[2]).atan2(d));
}

// --- Priest ------------------------------------------------------------------------------------------------

#[test]
fn penances_ex_divine_star() {
    // 25 Valor, 24 yd out and back, hurting each foe and healing each friend, each way.
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    s.set_aim_line(p, EAST, 0.0);
    let d = post(&mut s, [10.0, 0.0, 0.0]);
    let far = post(&mut s, [26.0, 0.0, 0.0]);
    let f = hero(&mut s, "Bren", [14.0, 0.0, 0.0], WEST);
    halve(&mut s, f);
    let pins = [(d, [10.0, 0.0, 0.0]), (far, [26.0, 0.0, 0.0])];
    let star = idx(&s, "divine_star");
    assert_eq!(s.content.ex_of(idx(&s, "penance")), Some(star), "Penance's EX");
    assert_eq!((s.content.abilities[star].valor, s.content.abilities[star].range[1]), (25, 24.0));
    let (hd, hf) = (hp(&s, d), s.players[&f].health);
    cast(&mut s, p, "penance", true);
    assert!((75.0..=90.0).contains(&s.players[&p].valor), "25 Valor spent: {}", s.players[&p].valor);
    run(&mut s, 0.9, &pins);
    let out_dmg = hd - hp(&s, d);
    let out_heal = s.players[&f].health - hf;
    assert!(out_dmg > 0 && out_heal > 0, "going out: hurts the foe ({out_dmg}), heals the friend ({out_heal})");
    run(&mut s, 2.5, &pins);
    assert_eq!(hd - hp(&s, d), out_dmg * 2, "the foe again coming back");
    assert_eq!(s.players[&f].health - hf, out_heal * 2, "the friend again coming back");
    assert_eq!(hp(&s, far), 1000, "24 yd and no farther");
    assert!(s.projectiles.is_empty(), "home and gone");
    // Press Penance again to turn it early: home well before a whole flight.
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    s.set_aim_line(p, EAST, 0.0);
    let mut ev = cast(&mut s, p, "penance", true);
    ev.extend(run(&mut s, 0.25, &[]));
    assert_eq!(perfects(&ev, p), 0, "still out");
    ev.extend(press(&mut s, p, "ranged"));
    ev.extend(run(&mut s, 0.9, &[]));
    assert!(s.projectiles.is_empty(), "turned and home inside 1.2 sec (a whole flight is 2.4)");
    // Caught coming home: 5 Valor and the perfect (no timing check: it is home, it is caught).
    assert_eq!(perfects(&ev, p), 1, "the perfect");
    assert!((s.players[&p].valor - 80.0).abs() < 0.5, "75 and 5 for the catch: {}", s.players[&p].valor);
}

#[test]
fn psychic_screams_ex_void_tendrils() {
    let mut s = world(true);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let near = post(&mut s, [7.5, 0.0, 0.0]);
    let back = post(&mut s, [-6.0, 4.0, 0.0]);
    let far = post(&mut s, [9.6, 0.0, 0.0]);
    let h = hero(&mut s, "Bren", [0.0, 5.0, 0.0], WEST);
    let b = boss(&mut s, [-5.0, -3.0, 0.0]);
    let ev_def = idx(&s, "void_tendrils");
    assert_eq!(s.content.ex_of(idx(&s, "psychic_scream")), Some(ev_def));
    cast(&mut s, p, "psychic_scream", true);
    assert!((75.0..=90.0).contains(&s.players[&p].valor), "25 Valor spent: {}", s.players[&p].valor);
    for g in [near, back] {
        let c = s.cc_of(g).iter().find(|c| c.kind == "root" && c.until > s.clock).cloned().expect("rooted");
        assert!((2.9..=3.001).contains(&(c.until - s.clock)), "3 sec: {}", c.until - s.clock);
        assert_eq!((c.amount, c.left), (50.0, 50.0), "a tendril of 50 health");
    }
    assert!(!s.has_cc(far, "root"), "not past 8 yd");
    assert!((1.4..=1.501).contains(&left(&s, h, "root")), "heroes 1.5: {}", left(&s, h, "root"));
    assert!(!s.has_cc(b, "root"), "a boss holds its ground");
    // Hit a tendril until it breaks: 49 of 50 holds, the 50th frees.
    s.hurt_mob(near, p, 30);
    s.hurt_mob(near, p, 19);
    assert!(s.has_cc(near, "root"), "49 of 50 still holds");
    s.hurt_mob(near, p, 1);
    assert!(!s.has_cc(near, "root"), "broken at 50");
}

#[test]
fn fades_ex_dominate_mind() {
    let setup = |s: &mut Survival| {
        let p = hero(s, "Ayla", ORIGIN, EAST);
        s.set_aim_line(p, EAST, 0.0);
        let g = foe(s, "grunt", [10.0, 0.0, 0.0]);
        (p, g)
    };
    let dom = |s: &Survival, u: Id| left(s, u, "dominated");
    // 6 sec, on your side, 25 Valor; it strikes the nearest creature within 14 yd.
    let mut s = world(false);
    let (p, g) = setup(&mut s);
    assert_eq!(s.content.ex_of(idx(&s, "fade")), Some(idx(&s, "dominate_mind")));
    let near = post(&mut s, [10.0, 4.0, 0.0]);
    let wide = post(&mut s, [10.0, -12.0, 0.0]);
    let pins = [(near, [10.0, 4.0, 0.0]), (wide, [10.0, -12.0, 0.0])];
    cast(&mut s, p, "fade", true);
    assert!((75.0..=90.0).contains(&s.players[&p].valor), "25 Valor spent: {}", s.players[&p].valor);
    run(&mut s, 0.3, &pins);
    assert!((5.4..=6.001).contains(&dom(&s, g)), "6 sec: {}", dom(&s, g));
    assert!(s.mobs[&g].friendly && s.mobs[&g].thrall.as_ref().is_some_and(|t| t.owner == p), "fights for her");
    run(&mut s, 4.0, &pins);
    assert!(hp(&s, near) < 1000, "it struck the nearest ({})", hp(&s, near));
    assert_eq!(hp(&s, wide), 1000, "not the farther one while a nearer lives");
    run(&mut s, 2.5, &pins);
    assert!(s.mobs[&g].thrall.is_none() && !s.mobs[&g].friendly, "its own again after 6 sec");
    // With none within 14 yd it follows her instead.
    let mut s = world(false);
    let (p, g) = setup(&mut s);
    let faraway = post(&mut s, [10.0, 20.0, 0.0]);
    let pins = [(faraway, [10.0, 20.0, 0.0])];
    cast(&mut s, p, "fade", true);
    run(&mut s, 0.2, &pins);
    s.move_player(p, [-10.0, -6.0, 0.0], EAST, Moving::default());
    run(&mut s, 5.0, &pins);
    let d = dist2d(s.mobs[&g].pos, s.players[&p].pos);
    assert!(d < 7.0, "it came to her side: {d}");
    assert_eq!(hp(&s, faraway), 1000, "and struck no one past 14 yd");
    // Bosses shrug it off.
    let mut s = world(false);
    let (p, _g) = setup(&mut s);
    s.mobs.clear();
    let b = boss(&mut s, [10.0, 0.0, 0.0]);
    cast(&mut s, p, "fade", true);
    run(&mut s, 0.6, &[]);
    assert_eq!(dom(&s, b), 0.0, "a boss is not taken");
    assert!(!s.mobs[&b].friendly && s.mobs[&b].thrall.is_none());
}

#[test]
fn dominate_mind_on_a_hero() {
    let mut s = world(true);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    s.set_aim_line(p, EAST, 0.0);
    let h = hero(&mut s, "Bren", [10.0, 0.0, 0.0], WEST);
    let mut ev = cast(&mut s, p, "fade", true);
    ev.extend(run(&mut s, 0.6, &[]));
    assert!((0.5..=1.501).contains(&left(&s, h, "dominated")), "1.5 sec: {}", left(&s, h, "dominated"));
    assert!(s.disabled(h), "can't act");
    assert!(ev.iter().any(|e| matches!(e, Event::Hop { player, dir, .. } if *player == h && (dir - WEST).abs() < 0.1)), "walked toward her");
    run(&mut s, 1.4, &[]);
    assert!(!s.disabled(h), "free after 1.5 sec");
}

#[test]
fn holy_novas_ex_guardian_spirit() {
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    s.set_aim_line(p, EAST, 0.0);
    // Two allies: Bren on the reticle, Cara nearer the Priest but off it.
    let bren = hero(&mut s, "Bren", [14.0, 0.0, 0.0], WEST);
    let cara = hero(&mut s, "Cara", [3.0, 9.0, 0.0], WEST);
    for h in [bren, cara] {
        let x = s.players.get_mut(&h).unwrap();
        x.max_health = 100;
        x.health = 100;
    }
    assert_eq!(s.content.ex_of(idx(&s, "holy_nova")), Some(idx(&s, "guardian_spirit")));
    cast(&mut s, p, "holy_nova", true);
    assert!((75.0..=90.0).contains(&s.players[&p].valor), "25 Valor spent: {}", s.players[&p].valor);
    assert!(s.has_cc(bren, "cheat_death") && !s.has_cc(cara, "cheat_death") && !s.has_cc(p, "cheat_death"), "the ally nearest the reticle");
    assert!((5.9..=6.001).contains(&left(&s, bren, "cheat_death")), "6 sec: {}", left(&s, bren, "cheat_death"));
    s.hurt_player(bren, 500);
    assert_eq!(s.players[&bren].health, 1, "a killing blow leaves them at 1 health");
    assert!(!s.has_cc(bren, "cheat_death"), "spent");
    run(&mut s, 4.0, &[]);
    let h = s.players[&bren].health;
    assert!((59..=62).contains(&h), "healed 60%: {h}");
    s.hurt_player(bren, 500);
    assert_eq!(s.players[&bren].health, 0, "once: the next blow kills");
    // With no ally it is on her.
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    cast(&mut s, p, "holy_nova", true);
    assert!(s.has_cc(p, "cheat_death"), "or you");
    // And it lapses unspent after 6 sec.
    run(&mut s, 6.2, &[]);
    assert!(!s.has_cc(p, "cheat_death"));
}

#[test]
fn angelic_feathers_ex_leap_of_faith() {
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    s.set_aim_line(p, EAST, 0.0);
    let f = hero(&mut s, "Bren", [29.0, 0.0, 0.0], WEST);
    let other = hero(&mut s, "Cara", [20.0, 25.0, 0.0], WEST);
    assert_eq!(s.content.ex_of(idx(&s, "angelic_feather")), Some(idx(&s, "faithful_leap")));
    assert_eq!(s.content.abilities[idx(&s, "faithful_leap")].range[1], 30.0);
    let ev = cast(&mut s, p, "angelic_feather", true);
    let hops: Vec<Id> = ev.iter().filter_map(|e| if let Event::Hop { player, .. } = e { Some(*player) } else { None }).collect();
    assert_eq!(hops, vec![f], "the ally nearest the reticle, up to 30 yd, is pulled to her side ({other} stays)");
    assert!((75.0..=90.0).contains(&s.players[&p].valor), "25 Valor spent: {}", s.players[&p].valor);
    // No ally within 30 yd: refused, the Valor back.
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    s.set_aim_line(p, EAST, 0.0);
    let _far = hero(&mut s, "Bren", [33.0, 0.0, 0.0], WEST);
    let ev = cast(&mut s, p, "angelic_feather", true);
    assert!(!ev.iter().any(|e| matches!(e, Event::Hop { .. })), "no one pulled");
    assert!(s.players[&p].valor >= 99.9, "refused, the Valor back: {}", s.players[&p].valor);
    // No ally at all: the same.
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    cast(&mut s, p, "angelic_feather", true);
    assert!(s.players[&p].valor >= 99.9, "no ally: refused, the Valor back");
}

#[test]
fn power_word_shields_ex_pain_suppression() {
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    s.set_aim_line(p, EAST, 0.0);
    let f = hero(&mut s, "Bren", [28.0, 0.0, 0.0], WEST);
    assert_eq!(s.content.ex_of(idx(&s, "power_word_shield")), Some(idx(&s, "pain_suppression")));
    cast(&mut s, p, "power_word_shield", true);
    assert!((75.0..=90.0).contains(&s.players[&p].valor), "25 Valor spent: {}", s.players[&p].valor);
    assert!(s.has_cc(f, "hallowed") && !s.has_cc(p, "hallowed"), "the ally under the reticle at 28 yd");
    assert!((s.taken_share(f) - 0.6).abs() < 0.01, "40% less damage: {}", s.taken_share(f));
    assert!(s.unstaggerable(f), "can't be staggered");
    assert!((3.8..=4.001).contains(&left(&s, f, "hallowed")), "4 sec: {}", left(&s, f, "hallowed"));
    run(&mut s, 4.3, &[]);
    assert!(!s.has_cc(f, "hallowed") && !s.unstaggerable(f) && (s.taken_share(f) - 1.0).abs() < 0.01, "worn off");
    // Past 30 yd the ally isn't reached: it falls on her.
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    s.set_aim_line(p, EAST, 0.0);
    let f = hero(&mut s, "Bren", [34.0, 0.0, 0.0], WEST);
    cast(&mut s, p, "power_word_shield", true);
    assert!(s.has_cc(p, "hallowed") && !s.has_cc(f, "hallowed"), "or you");
}

#[test]
fn vampiric_embrace_shadows_key_2() {
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let b = hero(&mut s, "Bren", [0.0, 6.0, 0.0], WEST);
    let c = hero(&mut s, "Cara", [0.0, -6.0, 0.0], WEST);
    let far = hero(&mut s, "Dane", [0.0, 30.0, 0.0], WEST);
    let m = post(&mut s, [2.0, 0.0, 0.0]);
    let pins = [(m, [2.0, 0.0, 0.0])];
    let ve = idx(&s, "vampiric_embrace");
    assert_eq!((s.content.abilities[ve].slot.as_str(), s.content.abilities[ve].cooldown), ("defense", 15.0), "key 2, 15 sec cooldown");
    for h in [p, b, c, far] {
        halve(&mut s, h);
    }
    s.players.get_mut(&b).unwrap().health = s.players[&b].max_health / 4;
    s.players.get_mut(&far).unwrap().health = 1;
    s.set_aim_line(p, EAST, 0.0);
    cast(&mut s, p, "vampiric_embrace", false);
    assert!(s.cooldown(p, ve).next_at - s.clock > 14.0, "15 sec cooldown");
    run(&mut s, 0.5, &[]);
    assert!((5.3..=6.001).contains(&left(&s, p, "embrace")), "6 sec: {}", left(&s, p, "embrace"));
    let (mine, his, hers, dane) = (s.players[&p].health, s.players[&b].health, s.players[&c].health, s.players[&far].health);
    let before = hp(&s, m);
    aim_ground(&mut s, p, [2.0, 0.0, 0.0]);
    cast(&mut s, p, "mind_blast", false);
    run(&mut s, 1.3, &pins);
    let dealt = before - hp(&s, m);
    assert!(dealt > 0, "Mind Blast landed");
    assert_eq!(s.players[&b].health, his + (dealt as f32 * 0.5).round() as u32, "half of the shadow dealt to the lowest-health ally within 12 yd");
    assert_eq!((s.players[&p].health, s.players[&c].health, s.players[&far].health), (mine, hers, dane), "no one else; Dane is past 12 yd");
    // With no ally near: the Priest.
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let m = post(&mut s, [2.0, 0.0, 0.0]);
    halve(&mut s, p);
    s.players.get_mut(&p).unwrap().combat_at = s.clock;
    cast(&mut s, p, "vampiric_embrace", false);
    run(&mut s, 0.5, &[]);
    let mine = s.players[&p].health;
    aim_ground(&mut s, p, [2.0, 0.0, 0.0]);
    cast(&mut s, p, "mind_blast", false);
    run(&mut s, 1.3, &[(m, [2.0, 0.0, 0.0])]);
    assert!(s.players[&p].health > mine, "she is healed with no ally near");
    // A perfect Mind Blast inside it adds 2 sec: a creature on the move in the blast's heart.
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let g = post(&mut s, [12.0, 0.0, 0.0]);
    cast(&mut s, p, "vampiric_embrace", false);
    run(&mut s, 0.5, &[(g, [12.0, 0.0, 0.0])]);
    let until = |s: &Survival| s.cc_of(p).iter().find(|c| c.kind == "embrace").map(|c| c.until).unwrap();
    let t0 = until(&s);
    aim_ground(&mut s, p, [12.0, 0.0, 0.0]);
    let mut ev = cast(&mut s, p, "mind_blast", false);
    for _ in 0..100 {
        if let Some(m) = s.mobs.get_mut(&g) {
            m.state = MobState::Idle { wander_at: 1e9 };
            m.pos = [12.0, 0.0, 0.0];
            m.path = vec![[12.0, 20.0, 0.0]];
            m.speed = m.speed.max(3.0);
        }
        ev.extend(s.step(DT));
    }
    assert_eq!(perfects(&ev, p), 1, "a perfect Mind Blast");
    assert!((until(&s) - t0 - 2.0).abs() < 1.0e-3, "2 sec added: {} from {t0}", until(&s));
}

#[test]
fn vampiric_embraces_ex_psychic_ward() {
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let f = hero(&mut s, "Bren", [6.0, 0.0, 0.0], WEST);
    s.set_aim_line(p, EAST, 0.0);
    let first = post(&mut s, [7.5, 0.0, 0.0]);
    let second = post(&mut s, [7.5, 1.0, 0.0]);
    assert_eq!(s.content.ex_of(idx(&s, "vampiric_embrace")), Some(idx(&s, "psychic_ward")));
    cast(&mut s, p, "vampiric_embrace", true);
    assert!((75.0..=90.0).contains(&s.players[&p].valor), "25 Valor spent: {}", s.players[&p].valor);
    assert!(s.has_cc(f, "ward") && !s.has_cc(p, "ward"), "on the ally under the reticle");
    assert!((5.3..=6.001).contains(&left(&s, f, "ward")), "6 sec: {}", left(&s, f, "ward"));
    let (h1, h2, hf) = (hp(&s, first), hp(&s, second), s.players[&f].health);
    s.strike_fair(first, f, 5, [7.5, 0.0, 0.0], false, false, false, Blow::Light, None, crate::sim::combat::Follow::None);
    assert_eq!(h1 - hp(&s, first), 20, "the first foe to strike takes 20");
    assert!((2.8..=3.001).contains(&left(&s, first, "panic")), "panic of 3 sec: {}", left(&s, first, "panic"));
    assert!(s.players[&f].health < hf, "the blow still lands");
    s.strike_fair(second, f, 5, [7.5, 1.0, 0.0], false, false, false, Blow::Light, None, crate::sim::combat::Follow::None);
    assert_eq!(hp(&s, second), h2, "only the first");
    // A hero is panicked half as long.
    let mut s = world(true);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let f = hero(&mut s, "Bren", [6.0, 0.0, 0.0], WEST);
    let foe_hero = hero(&mut s, "Cara", [7.0, 0.0, 0.0], WEST);
    s.set_aim_line(p, EAST, 0.0);
    cast(&mut s, p, "vampiric_embrace", true);
    s.strike_fair(foe_hero, f, 5, [7.0, 0.0, 0.0], false, false, false, Blow::Light, None, crate::sim::combat::Follow::None);
    assert!((1.3..=1.501).contains(&left(&s, foe_hero, "panic")), "heroes 1.5: {}", left(&s, foe_hero, "panic"));
}

#[test]
fn a_save_with_shadowguard_on_key_2_comes_back_as_vampiric_embrace() {
    let mut s = world(false);
    s.content.dev.full_tree = false;
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let mut save = s.leave(p).unwrap();
    assert_eq!(s.content.abilities[idx(&s, "shadowguard")].slot, "defense", "Shadowguard held key 2");
    save.owned = ["mind_blast", "shadowguard"].into_iter().map(String::from).collect();
    save.loadout = [("ranged", "mind_blast"), ("defense", "shadowguard")].into_iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
    let q = s.join("ayla", "Ayla", Some(&save));
    let h = &s.players[&q];
    assert_eq!(h.loadout.get("defense").map(String::as_str), Some("vampiric_embrace"), "on key 2: {:?}", h.loadout);
    assert!(h.owned.contains("vampiric_embrace") && !h.owned.contains("shadowguard"), "owned: {:?}", h.owned);
}

// --- Mage --------------------------------------------------------------------------------------------------

fn freeze(s: &mut Survival, by: Id, foes: &[Id]) {
    for f in foes {
        assert!(s.apply_cc(*f, by, "petrify", 10.0, 0.0, 0.0, "test"), "frozen");
    }
}

/// Ice Lance (Fireball's EX) aimed along the reticle and loosed, the foes held where they stand.
fn lance(s: &mut Survival, p: Id, pins: &[(Id, Vec3)]) -> Vec<Event> {
    s.set_cooldowns(p, true);
    s.set_valor(p, 100.0);
    let mut ev = cast(s, p, "fireball", true);
    ev.extend(run(s, 1.0, pins));
    ev
}

#[test]
fn fireballs_ex_ice_lance() {
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    learn(&mut s, p, "fireball");
    let lance_def = idx(&s, "pyroblast");
    let d = s.content.abilities[lance_def].clone();
    assert_eq!((d.name.as_str(), d.valor, d.range[1], d.damage[1]), ("Ice Lance", 25, 30.0, 12.0));
    assert_eq!(s.content.ex_of(idx(&s, "fireball")), Some(lance_def));
    let a = foe(&mut s, "boar", [28.0, 0.0, 0.0]);
    let pins = [(a, [28.0, 0.0, 0.0])];
    let h = hp(&s, a);
    let ev = lance(&mut s, p, &pins);
    assert_eq!(hits(&ev, p, a), vec![12], "12 damage unfrozen, reaching 28 yd");
    assert_eq!(hp(&s, a), h - 12);
    assert!(s.players[&p].valor < 100.0, "25 Valor spent");
    // Frozen: 1.5 times that, the freeze ended.
    let h = hp(&s, a);
    freeze(&mut s, p, &[a]);
    let ev = lance(&mut s, p, &pins);
    assert_eq!(hits(&ev, p, a), vec![18], "1.5 times 12 on a frozen foe");
    assert_eq!(hp(&s, a), h - 18);
    assert!(!s.has_cc(a, "petrify"), "the freeze ended");
    // Past 30 yd it doesn't reach.
    let b = foe(&mut s, "boar", [38.0, 0.0, 0.0]);
    let hb = hp(&s, b);
    let ev = lance(&mut s, p, &[(b, [38.0, 0.0, 0.0])]);
    assert!(hits(&ev, p, b).is_empty() && hp(&s, b) == hb, "30 yd and no farther");
}

#[test]
fn ice_lance_shatter_spread() {
    let mut s = world(true);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    learn(&mut s, p, "fireball");
    // The target and two more frozen within 5 yd of it (one frozen by someone else, a hero among them), one frozen
    // foe 10 yd off, an unfrozen one close by.
    let at = [[10.0, 0.0, 0.0], [10.0, 3.0, 0.0], [13.0, -3.0, 0.0], [20.5, 0.0, 0.0], [10.0, -4.0, 0.0]];
    let ids: Vec<Id> = at.iter().map(|a| foe(&mut s, "boar", *a)).collect();
    let (t, b, c, far, cool) = (ids[0], ids[1], ids[2], ids[3], ids[4]);
    let pins: Vec<(Id, Vec3)> = ids.iter().copied().zip(at).collect();
    let rival = hero(&mut s, "Bren", [8.0, 4.0, 0.0], WEST);
    freeze(&mut s, p, &[t, c, far]);
    freeze(&mut s, rival, &[b]);
    assert!(s.apply_cc(rival, t, "petrify", 10.0, 0.0, 0.0, "test"), "a hero frozen in the cluster too");
    let before: Vec<u32> = ids.iter().map(|i| s.mobs[i].health).collect();
    let hr = s.players[&rival].health;
    let ev = lance(&mut s, p, &pins);
    for (name, f) in [("target", t), ("second", b), ("third", c)] {
        assert_eq!(hits(&ev, p, f), vec![18], "{name} shatters once, whoever froze it");
        assert!(!s.has_cc(f, "petrify"), "{name} thawed");
    }
    assert_eq!(s.mobs[&b].health, before[1] - 18, "credited to the caster");
    assert!(!s.has_cc(rival, "petrify") && s.players[&rival].health < hr, "the frozen hero shatters too");
    assert!(hits(&ev, p, far).is_empty() && s.has_cc(far, "petrify") && s.mobs[&far].health == before[3], "10 yd off: untouched");
    assert!(hits(&ev, p, cool).is_empty() && s.mobs[&cool].health == before[4], "an unfrozen foe nearby isn't hit");
    // No chaining: a chain of frozen foes 4 yd apart stops at the target's 5 yd.
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    learn(&mut s, p, "fireball");
    let at = [[10.0, 0.0, 0.0], [10.0, 4.0, 0.0], [10.0, 8.0, 0.0]];
    let ids: Vec<Id> = at.iter().map(|a| foe(&mut s, "boar", *a)).collect();
    let pins: Vec<(Id, Vec3)> = ids.iter().copied().zip(at).collect();
    freeze(&mut s, p, &ids);
    let h2 = s.mobs[&ids[2]].health;
    let ev = lance(&mut s, p, &pins);
    assert!(!s.has_cc(ids[0], "petrify") && !s.has_cc(ids[1], "petrify"), "target and neighbour shattered");
    assert!(s.has_cc(ids[2], "petrify") && s.mobs[&ids[2]].health == h2 && hits(&ev, p, ids[2]).is_empty(), "no chaining");
}

#[test]
fn frost_nova_then_the_fireball_key() {
    // Frost Nova (Cone of Cold's EX), then the Fireball key: a free Ice Lance for 5 sec, the key empowered (it glows).
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    learn(&mut s, p, "fireball");
    learn(&mut s, p, "cone_of_cold");
    s.set_cooldowns(p, true);
    let k = foe(&mut s, "boar", [6.0, 0.0, 0.0]);
    let pins = [(k, [6.0, 0.0, 0.0])];
    s.set_valor(p, 25.0);
    s.input(p, Input::Ability(key_of("close"), true));
    s.step(DT);
    assert!(s.has_cc(k, "petrify"), "the boar frozen");
    let em = s.empowered_keys(p);
    assert_eq!(em.len(), 1, "one key glows: {em:?}");
    assert_eq!(em[0].0, key_of("ranged") as usize, "the Fireball key");
    assert!((em[0].1 - (s.clock + 5.0)).abs() < 0.2, "for 5 sec");
    // Past 5 sec it lapses (the freeze outlasts it on a hero-free boar, so check the key alone).
    let valor = s.players[&p].valor;
    let health = hp(&s, k);
    run(&mut s, 0.3, &pins);
    s.input(p, Input::Ability(key_of("ranged"), false));
    s.step(0.0);
    assert!(matches!(s.players[&p].action, Action::Aim { ability } if ability == idx(&s, "pyroblast")), "the plain key aims Ice Lance");
    s.input(p, Input::Attack);
    s.step(0.0);
    s.input(p, Input::AttackUp);
    s.step(0.0);
    run(&mut s, 0.8, &pins);
    assert!(hp(&s, k) < health, "the lance hit");
    assert!(!s.has_cc(k, "petrify"), "and shattered the freeze");
    assert!(s.players[&p].valor + 0.001 >= valor, "free: no Valor spent ({valor} then {})", s.players[&p].valor);
    assert!(s.empowered_keys(p).is_empty(), "the combo is spent");
    // A second Fireball press without a new nova is a Fireball.
    s.input(p, Input::Ability(key_of("ranged"), false));
    s.step(0.0);
    assert!(matches!(s.players[&p].action, Action::Aim { ability } if ability == idx(&s, "fireball")), "a Fireball again");
    // And an unused nova's gift lapses after 5 sec.
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    learn(&mut s, p, "fireball");
    learn(&mut s, p, "cone_of_cold");
    s.set_cooldowns(p, true);
    s.set_valor(p, 25.0);
    s.input(p, Input::Ability(key_of("close"), true));
    s.step(DT);
    assert_eq!(s.empowered_keys(p).len(), 1);
    run(&mut s, 5.3, &[]);
    assert!(s.empowered_keys(p).is_empty(), "lapsed after 5 sec");
}

#[test]
fn flamestrikes_ex_blast_wave() {
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    learn(&mut s, p, "flamestrike");
    s.set_cooldowns(p, true);
    let near = foe(&mut s, "boar", [3.0, 0.0, 0.0]);
    let edge = foe(&mut s, "boar", [5.8, 0.0, 0.0]);
    let far = foe(&mut s, "boar", [9.0, 0.0, 0.0]);
    let (hn, he, hf) = (hp(&s, near), hp(&s, edge), hp(&s, far));
    s.set_valor(p, 100.0);
    let bw = idx(&s, "flash_flamestrike");
    assert_eq!(s.content.ex_of(idx(&s, "flamestrike")), Some(bw));
    s.input(p, Input::Ability(key_of("ranged"), true));
    let ev = s.step(0.0);
    assert!(!matches!(s.players[&p].action, Action::Aim { .. }), "instant: no reticle");
    assert_eq!(hits(&ev, p, near), vec![12], "12 damage, at once");
    assert_eq!(hits(&ev, p, edge), vec![12], "6 yd reaches 5.8");
    assert!(hits(&ev, p, far).is_empty() && hp(&s, far) == hf, "not past 6 yd");
    assert_eq!((hp(&s, near), hp(&s, edge)), (hn - 12, he - 12));
    assert!(s.players[&p].valor < 100.0, "Valor spent");
    let c = s.cc_of(near).iter().find(|c| c.kind == "snare").cloned().expect("slowed");
    assert!((c.amount - 0.5).abs() < 1e-3 && (c.until - c.since - 3.0).abs() < 0.01, "50% for 3 sec");
    // A 6 yd knockback.
    let mut farthest = 0.0f32;
    let edge_at = s.mobs[&edge].pos;
    for _ in 0..150 {
        run(&mut s, DT, &[(far, [9.0, 0.0, 0.0]), (edge, edge_at)]);
        farthest = farthest.max(dist2d(s.mobs[&near].pos, ORIGIN));
    }
    assert!((8.0..=10.0).contains(&farthest), "thrown 6 yd from 3 yd out: {farthest:.1}");
    // Not on the default board: Blast Wave shows only with Flamestrike slotted.
    let d = Survival::new(Content::dev(), Box::new(FlatLand::default()), 1, 1.0);
    let mut d = d;
    let q = d.join("dev", "Dev", None);
    assert_ne!(d.players[&q].loadout.get("ranged").map(String::as_str), Some("flamestrike"), "the default board doesn't have it slotted");
}

// --- Hunter ------------------------------------------------------------------------------------------------

#[test]
fn bank_shots_ex_scatter_shot() {
    let mut s = world(true);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let d = s.content.abilities[idx(&s, "scatter_shot")].clone();
    let e = d.effects[0].clone();
    assert_eq!((d.range[1], d.valor, e.kind.as_str(), e.secs[1], e.hero_secs, d.damage[1]), (10.0, 25, "disorient", 4.0, 1.5, 0.0));
    assert_eq!(s.content.ex_of(idx(&s, "bank_shot")), Some(idx(&s, "scatter_shot")));
    let a = foe(&mut s, "grunt", [7.0, 0.0, 0.0]);
    let beyond = foe(&mut s, "grunt", [14.0, 3.0, 0.0]);
    let before = (hp(&s, a), hp(&s, beyond));
    s.set_aim_line(p, EAST, 0.0);
    let mut ev = cast(&mut s, p, "bank_shot", true);
    ev.extend(run(&mut s, 0.6, &[(a, [7.0, 0.0, 0.0]), (beyond, [14.0, 3.0, 0.0])]));
    assert!(s.has_cc(a, "disorient"), "disoriented");
    assert!((3.0..=4.001).contains(&left(&s, a, "disorient")), "4 sec less the flight: {}", left(&s, a, "disorient"));
    assert!(!s.has_cc(beyond, "disorient"), "past 10 yd");
    assert_eq!((hp(&s, a), hp(&s, beyond)), before, "no damage");
    assert!(s.disabled(a) && !s.held_still(a), "it can't act, and wanders");
    assert!(s.players[&p].valor < 90.0, "25 Valor: {}", s.players[&p].valor);
    // Any damage ends it.
    s.hurt_mob(a, p, 1);
    s.cc_on_damage(a, 1);
    assert!(!s.has_cc(a, "disorient"), "damage ends it");
    // Heroes 1.5.
    let h = hero(&mut s, "Bren", [8.0, 0.0, 0.0], WEST);
    s.apply_effects(p, h, &d.effects, 1.0, "Scatter Shot");
    assert!((1.3..=1.501).contains(&left(&s, h, "disorient")), "heroes 1.5: {}", left(&s, h, "disorient"));
}

#[test]
fn aimed_shot_multi_shot_and_bank_shot_still_fly_and_land() {
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    // The old names are gone from the rules; the three cards are there.
    for old in ["split_arrow", "piercing_shot"] {
        assert!(s.content.ability(old).is_none(), "{old} is gone");
    }
    // Multi-Shot (Aimed Shot's EX): five arrows in a fan.
    let fan: Vec<Id> = [-2.4f32, -1.2, 0.0, 1.2, 2.4].iter().map(|y| foe(&mut s, "grunt", [12.0, *y, 0.0])).collect();
    let pins: Vec<(Id, Vec3)> = fan.iter().map(|m| (*m, s.mobs[m].pos)).collect();
    for m in &fan {
        s.mobs.get_mut(m).unwrap().health = 9999;
    }
    s.set_aim_line(p, EAST, -(1.9f32 / 12.0).atan());
    let mut ev = cast(&mut s, p, "aimed_arrow", true);
    ev.extend(run(&mut s, 1.0, &pins));
    for m in &fan {
        assert_eq!(hits(&ev, p, *m).len(), 1, "Multi-Shot: an arrow on each foe");
    }
    // Aimed Shot: one arrow, lands.
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let g = foe(&mut s, "grunt", [10.0, 0.0, 0.0]);
    s.mobs.get_mut(&g).unwrap().health = 9999;
    s.set_aim_line(p, EAST, -(1.9f32 / 10.0).atan());
    learn(&mut s, p, "aimed_arrow");
    s.input(p, Input::Ability(key_of("ranged"), false));
    s.step(0.0);
    s.input(p, Input::Attack);
    let mut ev = run(&mut s, 1.0, &[(g, [10.0, 0.0, 0.0])]);
    s.input(p, Input::AttackUp);
    ev.extend(run(&mut s, 1.0, &[(g, [10.0, 0.0, 0.0])]));
    assert!(!hits(&ev, p, g).is_empty(), "Aimed Shot lands");
    // Bank Shot: flies and lands on a foe in the line.
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let g = foe(&mut s, "grunt", [10.0, 0.0, 0.0]);
    s.mobs.get_mut(&g).unwrap().health = 9999;
    s.set_aim_line(p, EAST, -(1.9f32 / 10.0).atan());
    let mut ev = cast(&mut s, p, "bank_shot", false);
    ev.extend(run(&mut s, 1.0, &[(g, [10.0, 0.0, 0.0])]));
    assert!(!hits(&ev, p, g).is_empty(), "Bank Shot lands");
}

// --- Warlock -----------------------------------------------------------------------------------------------

fn rot(s: &mut Survival, from: Id, on: Id) {
    assert!(s.apply_cc_look(on, from, "burn", 4.0, 4.0, 0.0, "Seed of Corruption", 172));
}

#[test]
fn seed_of_corruptions_ex_soul_swap() {
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    learn(&mut s, p, "seed_of_corruption");
    s.set_cooldowns(p, true);
    let src = foe(&mut s, "boar", [14.0, 0.0, 0.0]);
    let near = foe(&mut s, "boar", [14.0, 8.0, 0.0]);
    let edge = foe(&mut s, "boar", [14.0, -9.5, 0.0]);
    let past = foe(&mut s, "boar", [14.0, 14.0, 0.0]);
    let pins = [(src, [14.0, 0.0, 0.0]), (near, [14.0, 8.0, 0.0]), (edge, [14.0, -9.5, 0.0]), (past, [14.0, 14.0, 0.0])];
    assert_eq!(s.content.ex_of(idx(&s, "seed_of_corruption")), Some(idx(&s, "soul_swap")));
    // The wording says two or more rots; the code spreads one or more (every rot of the Warlock's on the foe).
    rot(&mut s, p, src);
    run(&mut s, 1.0, &pins);
    s.set_valor(p, 25.0);
    cast(&mut s, p, "seed_of_corruption", true);
    run(&mut s, 0.1, &pins);
    for m in [near, edge] {
        assert!(s.has_cc(m, "burn"), "the rot reached it");
        assert!((left(&s, m, "burn") - left(&s, src, "burn")).abs() < 0.2, "with the time left: {} and {}", left(&s, m, "burn"), left(&s, src, "burn"));
        assert!(left(&s, m, "burn") < 3.2, "not a fresh 4 sec");
    }
    assert!(!s.has_cc(past, "burn"), "past 10 yd: nothing");
    assert_eq!(s.players[&p].valor, 75.0, "25 Valor: {}", s.players[&p].valor);
    // No rot: nothing is spent.
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let a = foe(&mut s, "boar", [14.0, 0.0, 0.0]);
    let _b = foe(&mut s, "boar", [14.0, 5.0, 0.0]);
    let ev = cast_with(&mut s, p, "seed_of_corruption", true, 25.0);
    assert!(flashed(&ev, p, "Nothing to spread"));
    assert_eq!(s.players[&p].valor, 25.0, "nothing spent");
    // Nobody near: nothing is spent.
    rot(&mut s, p, a);
    let mut s2 = world(false);
    let p2 = hero(&mut s2, "Ayla", ORIGIN, EAST);
    let lone = foe(&mut s2, "boar", [14.0, 0.0, 0.0]);
    rot(&mut s2, p2, lone);
    let ev = cast_with(&mut s2, p2, "seed_of_corruption", true, 25.0);
    assert!(flashed(&ev, p2, "No other enemies near"));
    assert_eq!(s2.players[&p2].valor, 25.0, "nothing spent");
}

#[test]
fn shadowfurys_ex_howl_of_terror() {
    let mut s = world(true);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let near = foe(&mut s, "boar", [3.0, 0.0, 0.0]);
    let behind = foe(&mut s, "boar", [-9.0, 0.0, 0.0]);
    let past = foe(&mut s, "boar", [11.5, 0.0, 0.0]);
    let h = hero(&mut s, "Bren", [0.0, 6.0, 0.0], WEST);
    assert_eq!(s.content.ex_of(idx(&s, "shadowfury")), Some(idx(&s, "howl_of_terror")));
    cast(&mut s, p, "shadowfury", true);
    assert_eq!(s.players[&p].valor, 75.0, "25 Valor: {}", s.players[&p].valor);
    for m in [near, behind] {
        assert!((left(&s, m, "panic") - 4.0).abs() < 0.2, "flees 4 sec: {}", left(&s, m, "panic"));
    }
    assert!((left(&s, h, "panic") - 1.5).abs() < 0.2, "heroes 1.5: {}", left(&s, h, "panic"));
    assert!(!s.has_cc(past, "panic"), "past 10 yd");
    // Damage frees them early, but not in the first second.
    let share = (s.mobs[&near].max_health as f32 * s.content.cc.panic_break).ceil() as u32 + 1;
    s.cc_on_damage(near, share);
    assert!(s.has_cc(near, "panic"), "the first second holds against damage");
    run(&mut s, 1.1, &[]);
    s.cc_on_damage(behind, share);
    assert!(!s.has_cc(behind, "panic"), "after it, enough damage frees them");
}

// --- Warrior and Rogue -------------------------------------------------------------------------------------

#[test]
fn cleaves_ex_piercing_howl() {
    let mut s = world(true);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let near = foe(&mut s, "brave", [4.0, 0.0, 0.0]);
    let edge = foe(&mut s, "brave", [0.0, 9.5, 0.0]);
    let past = foe(&mut s, "brave", [12.0, 0.0, 0.0]);
    let b = boss(&mut s, [-5.0, 0.0, 0.0]);
    let h = hero(&mut s, "Bren", [0.0, -6.0, 0.0], WEST);
    let before: Vec<u32> = [near, edge, past].iter().map(|m| hp(&s, *m)).collect();
    let hh = s.players[&h].health;
    assert_eq!(s.content.ex_of(idx(&s, "cleave")), Some(idx(&s, "piercing_howl")));
    s.set_cooldowns(p, true);
    cast(&mut s, p, "cleave", true);
    assert!((75.0..=90.0).contains(&s.players[&p].valor), "25 Valor spent: {}", s.players[&p].valor);
    for m in [near, edge] {
        assert!((left(&s, m, "snare") - 6.0).abs() < 0.2, "6 sec: {}", left(&s, m, "snare"));
        assert!((s.speed_share(m) - 0.4).abs() < 1e-3, "slowed 60%: {}", s.speed_share(m));
    }
    assert!(!s.has_cc(past, "snare"), "past 10 yd");
    assert!((left(&s, h, "snare") - 3.0).abs() < 0.2, "heroes 3: {}", left(&s, h, "snare"));
    assert!(!s.has_cc(b, "snare"), "a boss holds its ground");
    let after: Vec<u32> = [near, edge, past].iter().map(|m| hp(&s, *m)).collect();
    assert_eq!((before, hh), (after, s.players[&h].health), "no damage");
}

#[test]
fn fan_of_knives_ex_kidney_shot() {
    let mut s = world(true);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let d = s.content.abilities[idx(&s, "kidney_shot")].clone();
    assert_eq!(s.content.ex_of(idx(&s, "fan_of_knives")), Some(idx(&s, "kidney_shot")));
    assert_eq!((d.windup, d.weapon, d.valor, d.effects[0].secs[1], d.marked_more), (0.25, 0.4, 25, 3.0, 1.0));
    let g = foe(&mut s, "grunt", [1.8, 0.0, 0.0]);
    s.mobs.get_mut(&g).unwrap().max_health = 500;
    s.mobs.get_mut(&g).unwrap().health = 500;
    let pins = [(g, [1.8, 0.0, 0.0])];
    // The windup: nothing lands in the first 0.2 sec, the stab by 0.4.
    s.set_valor(p, 100.0);
    s.set_cooldowns(p, true);
    learn(&mut s, p, "fan_of_knives");
    s.input(p, Input::Ability(key_of("ranged"), true));
    let mut ev = s.step(DT);
    ev.extend(run(&mut s, 0.19, &pins));
    assert!(hits(&ev, p, g).is_empty(), "still winding up");
    ev.extend(run(&mut s, 0.3, &pins));
    let dealt = hits(&ev, p, g);
    assert_eq!(dealt.len(), 1, "one stab");
    let w = s.weapon_of(&s.players[&p]).damage;
    assert!(dealt[0] as f32 <= (w[1] as f32 * 0.4).ceil() + 1.0, "40% weapon damage: {} of {:?}", dealt[0], w);
    assert!(left(&s, g, "stun") > 1.9 && left(&s, g, "stun") <= 3.0, "stunned about 3 sec: {}", left(&s, g, "stun"));
    // A Marked foe: 1 sec longer, the Mark used up.
    let m = foe(&mut s, "grunt", [1.8, 0.0, 0.0]);
    s.mobs.get_mut(&m).unwrap().max_health = 500;
    s.mobs.get_mut(&m).unwrap().health = 500;
    s.mobs.get_mut(&g).unwrap().pos = [30.0, 30.0, 0.0];
    run(&mut s, 5.0, &[(m, [1.8, 0.0, 0.0])]);
    assert!(s.apply_cc(m, p, "mark", 6.0, 0.1, 0.0, "Fan of Knives"));
    s.set_valor(p, 100.0);
    s.set_cooldowns(p, true);
    s.input(p, Input::Ability(key_of("ranged"), true));
    s.step(DT);
    run(&mut s, 0.5, &[(m, [1.8, 0.0, 0.0])]);
    assert!(left(&s, m, "stun") > 2.9 && left(&s, m, "stun") <= 4.0, "3 and 1 more: {}", left(&s, m, "stun"));
    assert!(!s.has_cc(m, "mark"), "the Mark used up");
    // Heroes 1.5.
    let mut s = world(true);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    learn(&mut s, p, "fan_of_knives");
    let h = hero(&mut s, "Bren", [1.8, 0.0, 0.0], WEST);
    s.set_valor(p, 100.0);
    s.set_cooldowns(p, true);
    s.input(p, Input::Ability(key_of("ranged"), true));
    s.step(DT);
    run(&mut s, 0.5, &[]);
    assert!(left(&s, h, "stun") > 0.9 && left(&s, h, "stun") <= 1.5, "heroes 1.5: {}", left(&s, h, "stun"));
}

// --- Shaman and Druid --------------------------------------------------------------------------------------

#[test]
fn thunderstorms_ex_capacitor_totem() {
    let mut s = world(true);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    learn(&mut s, p, "thunderstorm");
    s.set_cooldowns(p, true);
    assert_eq!(s.content.ex_of(idx(&s, "thunderstorm")), Some(idx(&s, "thundercrack")));
    // The reticle on a spot 30 yd off: the totem stands 20 yd out at most.
    aim_ground(&mut s, p, [30.0, 0.0, 0.0]);
    cast(&mut s, p, "thunderstorm", true);
    assert_eq!(s.totems.len(), 1, "a totem down");
    let tm = s.totems[0].mob;
    let at = s.mobs[&tm].pos;
    assert!(dist2d(at, ORIGIN) <= 20.01, "up to 20 yd: {}", dist2d(at, ORIGIN));
    assert_eq!(s.mobs[&tm].max_health, 20, "20 health");
    let near = foe(&mut s, "grunt", [at[0] + 3.0, at[1], 0.0]);
    let edge = foe(&mut s, "grunt", [at[0], at[1] + 7.5, 0.0]);
    let far = foe(&mut s, "grunt", [at[0] - 11.0, at[1], 0.0]);
    let h = hero(&mut s, "Bren", [at[0], at[1] - 5.0, 0.0], WEST);
    let pins: Vec<(Id, Vec3)> = [near, edge, far].into_iter().map(|g| (g, s.mobs[&g].pos)).collect();
    run(&mut s, 1.7, &pins);
    assert!(!s.has_cc(near, "stun") && s.totems.len() == 1, "2 sec fuse: nothing yet");
    run(&mut s, 0.6, &pins);
    for g in [near, edge] {
        assert!((left(&s, g, "stun") - 3.0).abs() < 0.7, "3 sec: {}", left(&s, g, "stun"));
    }
    assert!(!s.has_cc(far, "stun"), "past 8 yd");
    assert!(left(&s, h, "stun") > 0.5 && left(&s, h, "stun") <= 1.5, "heroes 1.5: {}", left(&s, h, "stun"));
    assert!(s.totems.is_empty() && !s.mobs.contains_key(&tm), "the totem falls");
    // Destroy it first (20 health): it does nothing.
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    aim_ground(&mut s, p, [10.0, 0.0, 0.0]);
    cast(&mut s, p, "thunderstorm", true);
    let tm = s.totems[0].mob;
    let at = s.mobs[&tm].pos;
    let g = foe(&mut s, "grunt", [at[0] + 3.0, at[1], 0.0]);
    let pins = [(g, s.mobs[&g].pos)];
    run(&mut s, 0.8, &pins);
    s.hurt_mob(tm, g, 19);
    assert!(s.mobs.contains_key(&tm), "19 of 20 doesn't fell it");
    s.hurt_mob(tm, g, 1);
    run(&mut s, 3.0, &pins);
    assert!(!s.has_cc(g, "stun"), "destroyed first: it does nothing");
}

#[test]
fn feral_leaps_ex_prowl() {
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    learn(&mut s, p, "feral_leap");
    s.set_cooldowns(p, true);
    assert_eq!(s.content.ex_of(idx(&s, "feral_leap")), Some(idx(&s, "pounce")));
    let k = foe(&mut s, "boar", [1.5, 0.0, 0.0]);
    s.mobs.get_mut(&k).unwrap().health = 500;
    s.mobs.get_mut(&k).unwrap().max_health = 500;
    let pins = [(k, [1.5, 0.0, 0.0])];
    let most = s.weapon_of(&s.players[&p]).damage[1];
    let bonus = crate::sim::cc::PROWL_BONUS as u32;
    s.set_valor(p, 100.0);
    cast(&mut s, p, "feral_leap", true);
    assert!((75.0..=90.0).contains(&s.players[&p].valor), "25 Valor spent: {}", s.players[&p].valor);
    assert!(s.has_cc(p, "veil") && s.has_cc(p, "prowl"), "stealth");
    let v = s.cc_of(p).iter().find(|c| c.kind == "veil").cloned().unwrap();
    assert!((v.until - v.since - 10.0).abs() < 0.01, "up to 10 sec");
    run(&mut s, 8.0, &pins);
    assert!(s.has_cc(p, "veil"), "still hidden at 8 sec");
    s.input(p, Input::Attack);
    let mut ev = s.step(DT);
    s.input(p, Input::AttackUp);
    ev.extend(run(&mut s, 1.0, &pins));
    let hit = hits(&ev, p, k)[0];
    assert!(hit >= bonus && hit - bonus <= most, "20 more damage: {hit} (weapon tops {most})");
    assert!((left(&s, k, "stun") - 2.0).abs() < 0.2 || (s.cc_of(k).iter().any(|c| c.kind == "stun" && (c.until - c.since - 2.0).abs() < 0.01)), "stuns it 2 sec");
    assert!(!s.has_cc(p, "veil") && !s.has_cc(p, "prowl"), "out of it");
    // Heroes stunned 1 sec.
    let mut s = world(true);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let h = hero(&mut s, "Bren", [1.5, 0.0, 0.0], WEST);
    cast(&mut s, p, "feral_leap", true);
    run(&mut s, 0.5, &[]);
    s.input(p, Input::Attack);
    s.step(DT);
    s.input(p, Input::AttackUp);
    for _ in 0..150 {
        if s.has_cc(h, "stun") {
            break;
        }
        s.step(DT);
    }
    let st = s.cc_of(h).iter().find(|c| c.kind == "stun").cloned().expect("a hero stunned");
    assert!((st.until - st.since - 1.0).abs() < 0.01, "heroes 1 sec: {}", st.until - st.since);
    // Damage on you ends it with no strike.
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let k = foe(&mut s, "boar", [1.5, 0.0, 0.0]);
    s.mobs.get_mut(&k).unwrap().health = 500;
    s.mobs.get_mut(&k).unwrap().max_health = 500;
    cast(&mut s, p, "feral_leap", true);
    s.hurt_player(p, 3);
    assert!(!s.has_cc(p, "veil") && !s.has_cc(p, "prowl"), "damage ends it");
    s.input(p, Input::Attack);
    s.step(DT);
    s.input(p, Input::AttackUp);
    run(&mut s, 1.0, &[(k, [1.5, 0.0, 0.0])]);
    assert!(!s.has_cc(k, "stun"), "no strike");
}
