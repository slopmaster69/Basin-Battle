//! The Rogue's Venom tree (docs/rogue-roles.md, build plan entry 3): Venom and its trait, Coated
//! Blade; Fan of Knives' knives; Eviscerate and its EX Rupture; Garrote and its EX Feint; Leeching
//! Venom and its EX Choking Cloud. Death Mark is kept as it was (its own tests).

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::{Foe, MobState};
use crate::sim::rogue::VENOM;
use crate::sim::*;

const DT: f32 = 0.02;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];
const PI: f32 = std::f32::consts::PI;

fn world_with(full_tree: bool) -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    c.dev.full_tree = full_tree;
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 7, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

fn world() -> Survival {
    world_with(true)
}

/// A hero of this class, made as the creation screen makes one, at `at` facing `f`.
fn hero_of(s: &mut Survival, name: &str, class: &str, at: Vec3, f: f32) -> Id {
    let account = name.to_lowercase();
    let save = s.fresh_save(&account, name, class, HeroLook::default());
    let id = s.join(&account, name, Some(&save));
    s.move_player(id, at, f, Moving::default());
    let _ = s.step(0.0);
    id
}

/// Venom's five cards on these keys.
const VENOM_KEYS: [(&str, &str); 5] = [("ranged", "fan_of_knives"), ("close", "eviscerate"), ("utility", "garrote"), ("defense", "leeching_venom"), ("ultimate", "death_mark")];

fn on_keys(s: &mut Survival, p: Id, keys: &[(&str, &str)]) {
    for (slot, k) in keys {
        s.slot_ability(p, slot, k).unwrap();
    }
}

/// A Rogue at the origin facing +x with all five of Venom's cards on the keys (its trait on).
fn rogue(s: &mut Survival) -> Id {
    let p = hero_of(s, "Ayla", "rogue", ORIGIN, 0.0);
    on_keys(s, p, &VENOM_KEYS);
    assert!(s.rogue_trait(&s.players[&p], VENOM));
    p
}

/// A creature at `at` facing `f`, at rest, that strikes nothing on its own.
fn mob(s: &mut Survival, key: &str, at: Vec3, f: f32) -> Id {
    let def = s.content.mob(key).unwrap();
    let id = s.spawn_mob(def, at, None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.pos = at;
    m.facing = f;
    m.ready_at = 1e9;
    m.health = 100_000;
    m.max_health = 100_000;
    id
}

/// A creature with no shield (a grunt's blocks from the front once it fights) 2 yd in front of
/// the Rogue, facing her.
fn foe(s: &mut Survival) -> Id {
    mob(s, "dm_woodcutter", [2.0, 0.0, 0.0], PI)
}

/// The creature `foe` put, back where it was, at rest and facing the Rogue (a blow draws it into a
/// fight, and it steps round her).
fn pin(s: &mut Survival, m: Id) {
    let x = s.mobs.get_mut(&m).unwrap();
    x.state = MobState::Idle { wander_at: 1e9 };
    x.pos = [2.0, 0.0, 0.0];
    x.facing = PI;
    x.path.clear();
    x.sidle = None;
}

/// In a fight with hero `p`, where it stands.
fn fighting(s: &mut Survival, m: Id, p: Id) {
    s.mobs.get_mut(&m).unwrap().state = MobState::Chase { foe: Foe::Hero(p) };
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        out.extend(s.step(DT));
    }
    out
}

fn idx(s: &Survival, key: &str) -> usize {
    s.content.ability(key).unwrap()
}

fn key_of(slot: &str) -> u8 {
    crate::content::SLOTS.iter().position(|s| *s == slot).unwrap() as u8
}

/// Its key pressed (`ex`: its EX version, with Valor for it), an aimed one let go on a tap of the
/// click.
fn press(s: &mut Survival, p: Id, slot: &str, ex: bool) -> Vec<Event> {
    s.set_valor(p, 100.0);
    s.players.get_mut(&p).unwrap().stamina = 100.0;
    s.input(p, Input::Ability(key_of(slot), ex));
    let mut ev = s.step(DT);
    if matches!(s.players[&p].action, Action::Aim { .. }) {
        s.input(p, Input::Attack);
        ev.extend(s.step(DT));
        s.input(p, Input::AttackUp);
        ev.extend(s.step(DT));
    }
    ev
}

/// A blow of this hand (`Attack` the main hand, `Guard` the off hand on a Rogue), run until late
/// in its recovery, where the next of a run is pressed.
fn blow(s: &mut Survival, p: Id, input: Input) -> Vec<Event> {
    s.players.get_mut(&p).unwrap().stamina = 100.0;
    s.input(p, input);
    let mut ev = Vec::new();
    // (Pressed late in the last blow's recovery, it waits for that blow to end: on to the new one.)
    for _ in 0..40 {
        if matches!(&s.players[&p].action, Action::Swing { t, strike, .. } if *t < strike.windup) {
            break;
        }
        ev.extend(s.step(DT));
    }
    for _ in 0..80 {
        ev.extend(s.step(DT));
        if matches!(&s.players[&p].action, Action::Swing { t, strike, .. } if *t >= strike.windup + strike.active + strike.recovery * 0.75) {
            break;
        }
    }
    s.input(p, if input == Input::Guard { Input::GuardUp } else { Input::AttackUp });
    ev
}

/// The blows that struck `v` (not burns and Venom's ticks).
fn hits_on(ev: &[Event], v: Id) -> Vec<u32> {
    ev.iter()
        .filter_map(|e| match e {
            Event::Swing { victim: Some(x), damage, outcome: Outcome::Hit, style, .. } if *x == v && !matches!(style, Blow::Dot) => Some(*damage),
            _ => None,
        })
        .collect()
}

/// Venom's ticks on `v`.
fn ticks_on(ev: &[Event], v: Id, by: Id) -> Vec<u32> {
    ev.iter()
        .filter_map(|e| match e {
            Event::Swing { attacker, victim: Some(x), damage, style: Blow::Dot, .. } if *x == v && *attacker == by => Some(*damage),
            _ => None,
        })
        .collect()
}

fn flashed(ev: &[Event], p: Id, text: &str) -> bool {
    ev.iter().any(|e| matches!(e, Event::Flash { player, text: t } if *player == p && t.contains(text)))
}

fn casting(s: &Survival, m: Id) -> bool {
    matches!(s.mobs[&m].state, MobState::Windup { .. })
}

/// Every weapon blow a dagger lands for exactly `n` (its roll fixed), so a share of it is exact.
fn fixed_dagger(s: &mut Survival, n: u32) {
    let k = s.content.weapons.iter().position(|w| w.key == "dagger").unwrap();
    s.content.weapons[k].damage = [n, n];
}

// --- Venom and Coated Blade --------------------------------------------------------------------------

#[test]
fn each_off_hand_hit_lays_1_venom_and_a_main_hand_hit_none() {
    let mut s = world();
    let p = rogue(&mut s);
    let m = foe(&mut s);
    let hit = |s: &mut Survival, input: Input| {
        pin(s, m);
        let ev = blow(s, p, input);
        assert!(!hits_on(&ev, m).is_empty(), "{input:?} lands");
    };
    hit(&mut s, Input::Attack);
    assert_eq!(s.venom_on(m), 0, "the main hand never coats");
    hit(&mut s, Input::Guard);
    assert_eq!(s.venom_on(m), 1, "the off hand does");
    hit(&mut s, Input::Attack);
    assert_eq!(s.venom_on(m), 1);
    // The run's 4th, on the off hand's button: the finisher, both blades.
    hit(&mut s, Input::Guard);
    assert_eq!(s.venom_on(m), 2, "a second off-hand hit");
    hit(&mut s, Input::Attack);
    assert_eq!(s.venom_on(m), 2, "a fresh run's main hand: none");
    // Guard (the off hand) then the main hand as the run's 4th: the finisher with both blades.
    let mut s = world();
    let p = rogue(&mut s);
    let m = foe(&mut s);
    for input in [Input::Guard, Input::Attack, Input::Guard] {
        pin(&mut s, m);
        blow(&mut s, p, input);
    }
    assert_eq!(s.venom_on(m), 2);
    pin(&mut s, m);
    s.players.get_mut(&p).unwrap().stamina = 100.0;
    s.input(p, Input::Attack);
    assert!(matches!(&s.players[&p].action, Action::Swing { strike, .. } if strike.finish), "the finisher");
    run(&mut s, 0.5);
    assert_eq!(s.venom_on(m), 3, "the two-blade finisher coats, on either button");
}

#[test]
fn the_venom_trait_is_on_with_3_venom_cards_and_only_for_a_rogue() {
    let mut s = world();
    let p = rogue(&mut s);
    on_keys(&mut s, p, &[("ranged", "distract"), ("close", "backstab")]);
    assert!(s.rogue_trait(&s.players[&p], VENOM), "3 of 5");
    let m = foe(&mut s);
    blow(&mut s, p, Input::Guard);
    assert_eq!(s.venom_on(m), 1);
    on_keys(&mut s, p, &[("utility", "shadowstep")]);
    assert!(!s.rogue_trait(&s.players[&p], VENOM), "2 of 5");
    s.cleanse(m, &["venom"]);
    run(&mut s, 1.0);
    blow(&mut s, p, Input::Guard);
    assert_eq!(s.venom_on(m), 0, "no trait, no coat");
    // A Warrior with two blades and all five of Venom's cards: no trait, no coat.
    let mut s = world();
    let w = hero_of(&mut s, "Bren", "warrior", ORIGIN, 0.0);
    on_keys(&mut s, w, &VENOM_KEYS);
    let knife = s.content.item("dagger");
    let h = s.players.get_mut(&w).unwrap();
    h.weapon = knife.unwrap();
    h.off_weapon = knife;
    assert!(!s.rogue_trait(&s.players[&w], VENOM), "the trait is the Rogue class's");
    let m = foe(&mut s);
    for input in [Input::Attack, Input::Guard, Input::Attack, Input::Guard] {
        blow(&mut s, w, input);
    }
    assert_eq!(s.venom_on(m), 0);
}

#[test]
fn venom_stacks_to_5_for_8_sec_and_ticks_1_5_a_stack() {
    let mut s = world();
    let p = rogue(&mut s);
    let m = foe(&mut s);
    let r = s.content.rogue.clone();
    assert_eq!((r.venom_most, r.venom_secs, r.venom_dps), (5, 8.0, 1.5));
    // One stack: 1.5 a second, the half carried, so 4 sec is 6.
    s.lay_venom(p, m, 1);
    let ev = run(&mut s, 4.01);
    let ticks = ticks_on(&ev, m, p);
    assert_eq!(ticks.len(), 4, "a tick a second: {ticks:?}");
    assert_eq!(ticks.iter().sum::<u32>(), 6, "1.5 a second: {ticks:?}");
    // Up to 5, never more.
    s.lay_venom(p, m, 3);
    assert_eq!(s.venom_on(m), 4);
    s.lay_venom(p, m, 3);
    assert_eq!(s.venom_on(m), 5, "5 at most");
    let ev = run(&mut s, 2.0);
    let ticks = ticks_on(&ev, m, p);
    assert_eq!(ticks.iter().sum::<u32>(), 15, "7.5 a second at 5: {ticks:?}");
    // 8 sec from the latest.
    s.lay_venom(p, m, 1);
    run(&mut s, 7.9);
    assert_eq!(s.venom_on(m), 5, "still on at 7.9 sec");
    run(&mut s, 0.2);
    assert_eq!(s.venom_on(m), 0, "gone at 8 sec");
}

#[test]
fn venom_shows_ripe_at_5() {
    let mut s = world();
    let p = rogue(&mut s);
    let m = foe(&mut s);
    let r = s.content.rogue.clone();
    s.lay_venom(p, m, 4);
    let looks = s.auras_of(m);
    assert!(looks.contains(&r.venom_aura) && !looks.contains(&r.ripe_aura), "the poison's green: {looks:?}");
    let _ = s.step(DT);
    s.lay_venom(p, m, 1);
    let ev = s.step(DT);
    let looks = s.auras_of(m);
    assert!(looks.contains(&r.ripe_aura) && !looks.contains(&r.venom_aura), "ripe: the deeper look in its place: {looks:?}");
    assert!(ev.iter().any(|e| matches!(e, Event::Visual { id, kit, .. } if *id == m && *kit == r.venom_drip)), "a burst of it as it turns");
    assert!(ev.iter().any(|e| matches!(e, Event::Auras { id } if *id == m)), "the clients told");
    // A tick: its number and a drip on the body, with no floor marker.
    let ev = run(&mut s, 1.0);
    assert!(!ticks_on(&ev, m, p).is_empty());
    assert!(ev.iter().any(|e| matches!(e, Event::Visual { id, kit, .. } if *id == m && *kit == r.venom_drip)), "the drip");
    assert!(!ev.iter().any(|e| matches!(e, Event::Mark { .. })), "nothing drawn on the ground");
}

#[test]
fn venom_sits_beside_a_daggers_own_poison_and_eviscerate_spends_only_venom() {
    let mut s = world();
    let p = rogue(&mut s);
    let shiv = s.content.item("bazil_shiv").unwrap();
    s.players.get_mut(&p).unwrap().off_weapon = Some(shiv);
    let m = foe(&mut s);
    blow(&mut s, p, Input::Guard);
    let poison = |s: &Survival| s.cc_of(m).iter().any(|c| c.kind == "poison" && c.until > s.clock);
    assert!(poison(&s), "Bazil's Shiv's own poison");
    assert_eq!(s.venom_on(m), 1, "and a Venom beside it");
    run(&mut s, 1.0);
    pin(&mut s, m);
    s.set_cooldowns(p, true);
    press(&mut s, p, "close", false);
    run(&mut s, 0.5);
    assert_eq!(s.venom_on(m), 0, "Eviscerate spent the Venom");
    assert!(poison(&s), "and left the dagger's poison be");
}

// --- Fan of Knives -------------------------------------------------------------------------------------

#[test]
fn fan_of_knives_lays_venom_only_with_the_trait() {
    for (trait_on, laid) in [(true, 3), (false, 0)] {
        let mut s = world();
        let p = rogue(&mut s);
        if !trait_on {
            on_keys(&mut s, p, &[("close", "backstab"), ("utility", "shadowstep"), ("defense", "smoke_bomb")]);
        }
        assert_eq!(s.rogue_trait(&s.players[&p], VENOM), trait_on);
        // Point blank: all three knives on one.
        let m = mob(&mut s, "grunt", [2.5, 0.0, 0.0], PI);
        s.set_aim_line(p, 0.0, 0.0);
        press(&mut s, p, "ranged", false);
        run(&mut s, 0.6);
        assert!(s.has_cc(m, "burn"), "the knives' own poison either way");
        assert_eq!(s.venom_on(m), laid, "trait {trait_on}");
    }
}

// --- Eviscerate and Rupture ----------------------------------------------------------------------------

#[test]
fn eviscerate_deals_80_percent_and_35_per_venom_and_uses_it_up() {
    let mut s = world();
    let p = rogue(&mut s);
    fixed_dagger(&mut s, 20);
    let m = foe(&mut s);
    let d = s.content.abilities[idx(&s, "eviscerate")].clone();
    assert_eq!((d.weapon, d.per_venom, d.windup, d.cooldown, d.choice, d.slot.as_str()), (0.8, 0.35, 0.3, 6.0, VENOM, "close"));
    // None on it: 80%, from the front.
    let ev = press(&mut s, p, "close", false);
    let mut ev = ev;
    ev.extend(run(&mut s, 0.5));
    assert_eq!(hits_on(&ev, m), vec![16], "80% of 20");
    // 5 on it: 80% + 5 × 35% = 255%, and they're gone, ripe look and all.
    run(&mut s, 1.0);
    pin(&mut s, m);
    s.set_cooldowns(p, true);
    s.lay_venom(p, m, 5);
    let mut ev = press(&mut s, p, "close", false);
    ev.extend(run(&mut s, 0.5));
    assert_eq!(hits_on(&ev, m), vec![51], "255% of 20");
    assert_eq!(s.venom_on(m), 0, "used up");
    let r = &s.content.rogue;
    assert!(!s.auras_of(m).contains(&r.ripe_aura) && !s.auras_of(m).contains(&r.venom_aura), "its look gone");
    assert!(ticks_on(&run(&mut s, 2.0), m, p).is_empty(), "no more ticks");
    // A blow that finds nothing (a creature gone home past its leash slips it) spends none.
    pin(&mut s, m);
    s.set_cooldowns(p, true);
    s.lay_venom(p, m, 3);
    let x = s.mobs.get_mut(&m).unwrap();
    x.state = MobState::Return;
    x.home = [-200.0, 0.0, 0.0];
    assert!(s.evading(m, p));
    let mut ev = press(&mut s, p, "close", false);
    ev.extend(run(&mut s, 0.4));
    assert!(hits_on(&ev, m).is_empty(), "it found nothing");
    assert_eq!(s.venom_on(m), 3, "a missed Eviscerate spends nothing");
}

#[test]
fn eviscerate_requires_two_blades() {
    let mut s = world();
    let p = rogue(&mut s);
    let m = foe(&mut s);
    let h = s.players.get_mut(&p).unwrap();
    h.off_weapon = None;
    h.off_empty = true;
    s.lay_venom(p, m, 5);
    let ev = press(&mut s, p, "close", false);
    assert!(flashed(&ev, p, "Eviscerate requires a weapon in each hand"), "refused");
    run(&mut s, 0.5);
    assert_eq!(s.venom_on(m), 5, "nothing spent");
    assert!(s.content.abilities[idx(&s, "eviscerate")].needs_two_blades);
    assert!(!s.content.abilities[idx(&s, "rupture")].needs_two_blades, "Rupture is a cut with one");
}

#[test]
fn rupture_lets_every_heros_blow_lay_venom_for_8_sec() {
    let mut s = world();
    let p = rogue(&mut s);
    let m = foe(&mut s);
    // A friend at the grunt's back, a Warrior (no trait of his own).
    let b = hero_of(&mut s, "Bren", "warrior", [4.0, 0.0, 0.0], PI);
    let mut ev = press(&mut s, p, "close", true);
    ev.extend(run(&mut s, 0.6));
    assert!(!hits_on(&ev, m).is_empty(), "the cut lands");
    assert!(s.has_cc(m, "rupture"));
    assert_eq!(s.venom_on(m), 0, "the cut itself lays none, nor spends any");
    let left = s.cc_of(m).iter().find(|c| c.kind == "rupture").unwrap().until - s.clock;
    assert!(left > 7.0 && left <= 8.0, "8 sec: {left}");
    // It bleeds while it lasts: Rend's blood burst on the body.
    let ev = run(&mut s, 1.6);
    assert!(ev.iter().any(|e| matches!(e, Event::Visual { id, kit: 220, .. } if *id == m)), "the bleed shows");
    // His blow coats it, as the Rogue's.
    pin(&mut s, m);
    blow(&mut s, b, Input::Attack);
    assert_eq!(s.venom_on(m), 1, "a friend's blow lays 1");
    assert_eq!(s.cc_of(m).iter().find(|c| c.kind == "venom").unwrap().from, p, "the Rogue's Venom");
    // Her main hand too, and her off hand only 1, not 2.
    pin(&mut s, m);
    blow(&mut s, p, Input::Attack);
    assert_eq!(s.venom_on(m), 2);
    pin(&mut s, m);
    blow(&mut s, p, Input::Guard);
    assert_eq!(s.venom_on(m), 3, "one a blow at most");
    // Over: his blows coat nothing.
    run(&mut s, 8.5);
    assert!(!s.has_cc(m, "rupture"));
    s.cleanse(m, &["venom"]);
    pin(&mut s, m);
    blow(&mut s, b, Input::Attack);
    assert_eq!(s.venom_on(m), 0, "after 8 sec, none");
}

// --- Garrote and Feint -------------------------------------------------------------------------------

/// A Kobold Geomancer 1.8 yd in front of the Rogue with its back to her, casting at a friend 8 yd
/// beyond it.
fn caster_with_back_turned(s: &mut Survival, key: &str) -> (Id, Id) {
    let b = hero_of(s, "Bren", "warrior", [10.0, 0.0, 0.0], PI);
    let m = mob(s, key, [1.8, 0.0, 0.0], 0.0);
    let def = s.mobs[&m].def;
    s.content.mobs[def].attack.ranged = true;
    let at = s.players[&b].pos;
    s.mobs.get_mut(&m).unwrap().state = MobState::Chase { foe: Foe::Hero(b) };
    s.windup(m, Foe::Hero(b), at);
    assert!(casting(s, m), "casting");
    (m, b)
}

#[test]
fn garrote_from_behind_breaks_the_cast_silences_and_bleeds() {
    let mut s = world();
    let p = rogue(&mut s);
    let (m, _) = caster_with_back_turned(&mut s, "kobold_geomancer");
    assert!(s.from_behind(m, s.players[&p].pos));
    let hp = s.mobs[&m].health;
    let mut ev = press(&mut s, p, "utility", false);
    ev.extend(run(&mut s, 0.4));
    assert!(s.mobs[&m].health < hp, "the cut lands");
    assert!(ev.iter().any(|e| matches!(e, Event::CastEnd { caster } if *caster == m)), "the cast broken");
    assert!(!casting(&s, m) && s.mobs[&m].stagger_until > s.clock, "it reels");
    assert!(s.has_cc(m, "silence") && s.garroted(m), "silenced");
    let burn = s.cc_of(m).iter().find(|c| c.kind == "burn").cloned().expect("bleeding");
    assert_eq!(burn.amount, 4.0);
    assert!(burn.until - s.clock > 5.0, "for 6 sec");
    assert_eq!(s.venom_on(m), 2, "2 Venom");
    // For 3 sec it starts no cast, ready and with its foe in sight.
    let m2 = s.mobs.get_mut(&m).unwrap();
    m2.ready_at = 0.0;
    let mut cast = false;
    for _ in 0..(2.4 / DT) as usize {
        let _ = s.step(DT);
        cast |= casting(&s, m);
    }
    assert!(!cast, "no cast while silenced");
    let mut cast = false;
    for _ in 0..(3.0 / DT) as usize {
        let _ = s.step(DT);
        s.mobs.get_mut(&m).unwrap().ready_at = 0.0;
        cast |= casting(&s, m);
        if cast {
            break;
        }
    }
    assert!(cast, "and casts again once it's over");
}

#[test]
fn garrote_from_the_front_is_only_a_cut() {
    let mut s = world();
    let p = rogue(&mut s);
    let m = mob(&mut s, "kobold_geomancer", [1.8, 0.0, 0.0], PI);
    s.windup(m, Foe::Hero(p), ORIGIN);
    assert!(casting(&s, m));
    let hp = s.mobs[&m].health;
    let mut ev = press(&mut s, p, "utility", false);
    ev.extend(run(&mut s, 0.4));
    assert!(s.mobs[&m].health < hp, "the cut lands");
    assert!(!ev.iter().any(|e| matches!(e, Event::CastEnd { caster } if *caster == m)), "the cast goes on");
    assert!(!s.has_cc(m, "silence") && !s.garroted(m) && !s.has_cc(m, "burn"), "nothing more");
    assert_eq!(s.venom_on(m), 0);
}

#[test]
fn garrote_on_a_boss_breaks_only_the_cast() {
    let mut s = world();
    let p = rogue(&mut s);
    let (m, _) = caster_with_back_turned(&mut s, "morgan");
    assert!(s.content.mobs[s.mobs[&m].def].boss);
    let mut ev = press(&mut s, p, "utility", false);
    ev.extend(run(&mut s, 0.4));
    assert!(ev.iter().any(|e| matches!(e, Event::CastEnd { caster } if *caster == m)), "the cast broken");
    assert!(!s.has_cc(m, "silence") && !s.garroted(m), "a boss isn't silenced");
    assert!(s.has_cc(m, "burn") && s.venom_on(m) == 2, "it still bleeds and takes the Venom");
}

#[test]
fn feint_hops_back_and_enemies_on_you_lose_interest_for_2_sec() {
    let mut s = world();
    let p = rogue(&mut s);
    let a = mob(&mut s, "grunt", [2.0, 0.0, 0.0], PI);
    let b = mob(&mut s, "grunt", [0.0, 3.0, 0.0], -PI / 2.0);
    let far = mob(&mut s, "grunt", [11.0, 0.0, 0.0], PI);
    // One winding up a blow at her: it finishes the blow first.
    let w = mob(&mut s, "grunt", [-1.5, 1.5, 0.0], 0.0);
    for m in [a, b, far, w] {
        fighting(&mut s, m, p);
    }
    s.windup(w, Foe::Hero(p), ORIGIN);
    let lift = s.content.abilities[idx(&s, "feint")].lift;
    let mut ev = press(&mut s, p, "utility", true);
    ev.extend(run(&mut s, 0.1));
    let hop = ev.iter().find_map(|e| match e {
        Event::Hop { player, dir, speed, .. } if *player == p => Some((*dir, *speed)),
        _ => None,
    });
    let (dir, speed) = hop.expect("a hop");
    assert!(dir.cos() < -0.99, "straight back: {dir}");
    let yards = speed * crate::sim::moves::airtime(lift);
    assert!((yards - 4.0).abs() < 0.2, "4 yd: {yards}");
    for m in [a, b] {
        assert!(matches!(s.mobs[&m].state, MobState::Idle { .. }), "it looks away");
        assert!(s.lulled(m, p), "and takes no notice of her");
    }
    assert!(matches!(s.mobs[&far].state, MobState::Chase { foe: Foe::Hero(h) } if h == p), "11 yd off: still on her");
    assert!(s.mobs[&w].rogue.feint_from.is_some() || s.lulled(w, p), "the one winding up is told");
    // Its blow done, it looks away too.
    run(&mut s, 1.5);
    assert!(s.lulled(w, p) && s.mobs[&w].state.target() != Some(p), "after its blow: {:?}", s.mobs[&w].state);
    // 2 sec, and they may notice her again.
    run(&mut s, 0.5);
    assert!(!s.lulled(a, p), "over at 2 sec");
}

#[test]
fn a_feinted_enemy_with_another_hero_near_turns_to_them() {
    let mut s = world();
    let p = rogue(&mut s);
    let b = hero_of(&mut s, "Bren", "warrior", [4.0, 3.0, 0.0], PI);
    let m = mob(&mut s, "grunt", [2.0, 0.0, 0.0], PI);
    fighting(&mut s, m, p);
    press(&mut s, p, "utility", true);
    assert!(matches!(s.mobs[&m].state, MobState::Chase { foe: Foe::Hero(h) } if h == b), "it turns on her friend: {:?}", s.mobs[&m].state);
    assert!(s.lulled(m, p));
    // A boss halves her threat instead, and stays on her.
    let mut s = world();
    let p = rogue(&mut s);
    let boss = mob(&mut s, "morgan", [2.0, 0.0, 0.0], PI);
    fighting(&mut s, boss, p);
    let now = s.clock;
    s.mobs.get_mut(&boss).unwrap().threat = vec![(p, 0.4, now)];
    press(&mut s, p, "utility", true);
    let t = s.threat_on(&s.mobs[&boss], p);
    assert!((t - 0.2).abs() < 0.01, "halved: {t}");
    assert!(matches!(s.mobs[&boss].state, MobState::Chase { foe: Foe::Hero(h) } if h == p), "still on her");
    assert!(!s.lulled(boss, p));
}

// --- Leeching Venom and Choking Cloud -------------------------------------------------------------

#[test]
fn leeching_venom_heals_what_your_venom_deals_for_6_sec() {
    // Over 3 sec at 5 Venom on one grunt: what she gained with it, less what she gained without.
    let gain = |leech: bool| -> (u32, u32) {
        let mut s = world();
        let p = rogue(&mut s);
        let m = foe(&mut s);
        s.players.get_mut(&p).unwrap().health = 20;
        if leech {
            press(&mut s, p, "defense", false);
            assert!(s.has_cc(p, "leech"), "on");
            let left = s.cc_of(p).iter().find(|c| c.kind == "leech").unwrap().until - s.clock;
            assert!(left > 5.9 && left <= 6.0, "6 sec, a boon's whole time: {left}");
            assert!(s.auras_of(p).contains(&s.content.rogue.venom_aura), "the poison's green on her");
        }
        let (h0, m0) = (s.players[&p].health, s.mobs[&m].health);
        s.lay_venom(p, m, 5);
        run(&mut s, 3.01);
        (s.players[&p].health - h0, m0 - s.mobs[&m].health)
    };
    let (with, dealt) = gain(true);
    let (without, _) = gain(false);
    assert_eq!(dealt, 22, "7.5 a second for 3 sec: 7, 8, 7, the half carried to the next");
    // (Her own mending, out of a fight, is the most the rest can be.)
    assert!(with >= dealt && with <= dealt + without, "healed as much as it dealt: {with} for {dealt} ({without} without)");
    // Over after 6 sec.
    let mut s = world();
    let p = rogue(&mut s);
    let m = foe(&mut s);
    press(&mut s, p, "defense", false);
    run(&mut s, 6.1);
    assert!(!s.has_cc(p, "leech"));
    s.players.get_mut(&p).unwrap().health = 20;
    s.lay_venom(p, m, 5);
    let ev = run(&mut s, 2.01);
    let dealt: u32 = ticks_on(&ev, m, p).iter().sum();
    let gained = s.players[&p].health - 20;
    assert!(dealt == 15 && gained < dealt / 2, "no more healing: {gained} of {dealt}");
}

#[test]
fn choking_cloud_slows_wind_ups_40_percent_and_lays_venom_each_sec() {
    let mut s = world();
    let p = rogue(&mut s);
    let m = mob(&mut s, "grunt", [2.0, 0.0, 0.0], PI);
    let out = mob(&mut s, "grunt", [8.0, 0.0, 0.0], PI);
    let boss = mob(&mut s, "morgan", [0.0, 3.0, 0.0], -PI / 2.0);
    let before = s.attack_now(m).windup;
    let mut ev = press(&mut s, p, "defense", true);
    ev.extend(run(&mut s, 0.1));
    assert!(ev.iter().any(|e| matches!(e, Event::AreaLook { spell: 28158, .. })), "the green cloud on the ground");
    assert!(!ev.iter().any(|e| matches!(e, Event::Mark { tele: Some(_), .. })), "no ring drawn: the cloud is its own look");
    assert!((s.choked_slow(m) - 0.4).abs() < 1.0e-4, "40% slower");
    assert!(s.attack_now(m).windup > before, "its wind-up longer");
    assert_eq!(s.venom_on(m), 1, "1 as it lands");
    assert_eq!(s.choked_slow(boss), 0.0, "a boss isn't slowed");
    assert_eq!(s.venom_on(boss), 1, "but takes the Venom");
    assert_eq!((s.venom_on(out), s.choked_slow(out)), (0, 0.0), "8 yd off: nothing");
    run(&mut s, 3.0);
    assert_eq!(s.venom_on(m), 4, "and 1 a second: 4 over its 4 sec");
    run(&mut s, 2.0);
    assert_eq!(s.choked_slow(m), 0.0, "the slow over with it");
    assert_eq!(s.venom_on(m), 4, "no more laid");
}

// --- the tree ----------------------------------------------------------------------------------------

#[test]
fn venom_is_the_rogues_second_choice_on_every_key_and_each_is_learned_somewhere() {
    let s = world();
    let card = |k: &str| &s.content.abilities[idx(&s, k)];
    for (slot, k) in VENOM_KEYS {
        let d = card(k);
        assert_eq!((d.school.as_str(), d.slot.as_str(), d.choice), ("rogue", slot, VENOM), "{k}");
        assert!(d.on_board() && !s.learned_where(k).is_empty(), "{k} is on the board and learned somewhere");
    }
    for (ex, base) in [("kidney_shot", "fan_of_knives"), ("rupture", "eviscerate"), ("feint", "garrote"), ("choking_cloud", "leeching_venom")] {
        assert_eq!(card(ex).ex_of, base);
        assert_eq!(card(ex).valor, 25, "{ex}: one segment");
    }
    // A new Rogue knows Eviscerate (a class's whole Q and E rows), Shadow on its keys still.
    let mut s = world_with(false);
    let p = hero_of(&mut s, "Ayla", "rogue", ORIGIN, 0.0);
    let h = &s.players[&p];
    assert!(h.owned.contains("eviscerate"));
    assert_eq!(h.loadout.get("close").map(String::as_str), Some("backstab"));
    let stash = s.content.chests.iter().find(|c| c.key == "relic_rogue").unwrap();
    assert!(stash.teaches.iter().any(|k| k == "garrote") && stash.teaches.iter().any(|k| k == "leeching_venom"));
}

#[test]
fn venom_cards_words_say_what_the_data_does() {
    let s = world();
    let r = &s.content.rogue;
    let said = |k: &str, words: &[String]| {
        let d = &s.content.abilities[idx(&s, k)];
        for w in words {
            assert!(d.text.contains(w.as_str()), "{k}: {w:?} in {}", d.text);
        }
    };
    let pct = |x: f32| format!("{}%", (x * 100.0).round());
    let d = &s.content.abilities[idx(&s, "eviscerate")];
    said("eviscerate", &[format!("{} sec windup", d.windup), format!("{} weapon damage", pct(d.weapon)), format!("plus {} for each Venom", pct(d.per_venom)), "Uses up all its Venom".into(), "Requires a weapon in each hand".into(), format!("With {} Venom abilities on your keys, each off-hand hit lays 1 Venom", s.content.rogue.trait_keys), format!("up to {} and deals {} damage per sec each for {} sec", r.venom_most, r.venom_dps, r.venom_secs)]);
    let d = &s.content.abilities[idx(&s, "rupture")];
    let e = &d.effects[0];
    assert_eq!(e.kind, "rupture");
    said("rupture", &[format!("{} sec windup", d.windup), format!("for {} weapon damage", pct(d.weapon)), format!("For {} sec, every melee hit on it by any hero lays 1 Venom", e.secs[1]), "Uses no Venom".into()]);
    let d = &s.content.abilities[idx(&s, "garrote")];
    let (sil, bleed, ven) = (&d.behind_effects[0], &d.behind_effects[1], &d.behind_effects[2]);
    assert_eq!((sil.kind.as_str(), bleed.kind.as_str(), ven.kind.as_str()), ("garrote", "burn", "venom"));
    assert!(d.breaks_cast > 0.0 && d.cooldown == 14.0);
    assert!(sil.hero_secs <= 1.5, "at most 1.5 sec on a hero");
    said("garrote", &[format!("{} sec windup", d.windup), format!("for {} weapon damage", pct(d.weapon)), "From behind, it also interrupts the spell".into(), format!("silences it for {} sec ({} sec against heroes)", sil.secs[1], sil.hero_secs), format!("bleed for {} damage per sec for {} sec", bleed.amount, bleed.secs[1]), format!("lays {} Venom", ven.amount), "Bosses aren't silenced".into()]);
    let d = &s.content.abilities[idx(&s, "feint")];
    let e = &d.self_effects[0];
    assert_eq!(e.kind, "feint");
    assert!(d.back && d.kind == "leap");
    said("feint", &[format!("Hops {} yd backward", d.range[1]), format!("within {} yd attacking you loses interest in you for {} sec", e.amount, e.secs[1]), format!("another hero within {} yd of it", r.feint_turn), "Bosses halve your threat instead".into()]);
    let d = &s.content.abilities[idx(&s, "leeching_venom")];
    let e = &d.effects[0];
    assert_eq!(e.kind, "leech");
    assert_eq!(d.cooldown, 18.0);
    said("leeching_venom", &[format!("For {} sec, every tick of your Venom on any enemy heals you", e.secs[1])]);
    let d = &s.content.abilities[idx(&s, "choking_cloud")];
    let (slow, ven) = (&d.effects[0], &d.effects[1]);
    assert_eq!((slow.kind.as_str(), ven.kind.as_str()), ("choked", "venom"));
    said("choking_cloud", &[format!("Fills {} yd around you", d.radius[1]), format!("for {} sec", d.linger), format!("wind up their attacks {} slower", pct(slow.amount)), format!("Each sec, enemies inside gain {} Venom", ven.amount), "Bosses aren't slowed".into()]);
    let d = &s.content.abilities[idx(&s, "fan_of_knives")];
    assert_eq!(d.coats, 1.0);
    said("fan_of_knives", &[format!("With {} Venom abilities on your keys, each knife also lays {} Venom", s.content.rogue.trait_keys, d.coats)]);
}


// --- Garrote after a Shadowstep, as the visual check plays it -----------------------------------------

/// The visual check's steps through the same inputs the server's key path sends: a Kobold
/// Geomancer 12 yd ahead that comes at her (`.es target`), Shadowstep on the utility key aimed at
/// it (the click carries the reticle's line), the key put back to Garrote 0.1 sec later, and the key
/// pressed with no new line (`in ab 2 0`) 0.2 sec after the landing, then twice more. The line the
/// click left looks the way she faced before the step, away from the back she was put behind: it
/// used to turn the blow away from the kobold (a miss that spent the 14 sec), the next two presses
/// finding Garrote cooling.
#[test]
fn garrote_pressed_just_after_a_shadowstep_lands_in_the_back_it_was_put_behind() {
    let mut s = world();
    let p = rogue(&mut s);
    s.slot_ability(p, "utility", "shadowstep").unwrap();
    s.spawn_target(p, "kobold_geomancer", 12.0, false, false, 1.0).unwrap();
    let m = *s.mobs.keys().next().unwrap();
    run(&mut s, 1.2);
    assert!(casting(&s, m), "it casts at her");
    s.set_valor(p, 100.0);
    s.input(p, Input::Ability(2, false));
    run(&mut s, 0.2);
    let f = facing(s.players[&p].pos, s.mobs[&m].pos);
    s.set_aim_line(p, f, -0.15);
    s.input(p, Input::Attack);
    run(&mut s, 0.1);
    s.input(p, Input::AttackUp);
    assert!(dist2d(s.players[&p].pos, [13.3, 0.0, 0.0]) < 0.6, "behind it: {:?}", s.players[&p].pos);
    assert!(s.from_behind(m, s.players[&p].pos), "in its back");
    s.slot_ability(p, "utility", "garrote").unwrap();
    let hp = s.mobs[&m].health;
    let mut ev = Vec::new();
    for wait in [0.1f32, 0.4, 0.4] {
        ev.extend(run(&mut s, wait));
        s.set_valor(p, 100.0);
        s.players.get_mut(&p).unwrap().stamina = 100.0;
        s.input(p, Input::Ability(2, false));
        ev.extend(run(&mut s, 0.02));
    }
    ev.extend(run(&mut s, 0.5));
    assert!(s.mobs[&m].health < hp, "the cut lands");
    assert!(s.has_cc(m, "silence") && s.garroted(m), "silenced");
    assert!(s.has_cc(m, "burn"), "bleeding");
    assert_eq!(s.venom_on(m), 2, "2 Venom");
    // (Its cast was already loosed by the time she landed: only the blow's own effects are asked.)
    assert!(ev.iter().any(|e| matches!(e, Event::Swing { attacker, outcome: Outcome::Hit, .. } if *attacker == p)), "a hit");
}

/// A Rogue's step (here Shadowstep) drops the reticle's old line and its point: till a new one
/// comes she faces where she was put. A blink that isn't a step behind keeps its line.
#[test]
fn a_rogues_step_drops_the_reticles_old_line() {
    let mut s = world();
    let p = rogue(&mut s);
    s.slot_ability(p, "utility", "shadowstep").unwrap();
    mob(&mut s, "grunt", [12.0, 0.0, 0.0], PI);
    s.set_aim_line(p, 0.0, -(1.9f32).atan2(12.0));
    press(&mut s, p, "utility", false);
    assert!(s.players[&p].pos[0] > 12.5, "behind it");
    assert!(s.players[&p].aim_line.is_none() && s.players[&p].aim_at.is_none());
    assert!((s.aim_of(p).0 - PI).abs() < 0.3, "her facing, till a new line comes: {}", s.aim_of(p).0);
}
