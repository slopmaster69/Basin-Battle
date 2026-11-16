//! The playtest checklist's "Weapons, by family" section (docs/playtest-checklist.md), one test per
//! item, each asserting what its line says in numbers: the part of each item the simulation can
//! check without a client. What only a client shows (a clip's look, a leap's look) is left to the
//! playtest; the tooltip sentence is checked against the numbers it is built from.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::{Foe, MobState};
use crate::sim::*;

// --- the harness (the same shape as tests_weapon_identity's) -------------------------------------------

fn world() -> (Survival, Id) {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 5, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    let p = s.join("ayla", "Ayla", None);
    s.move_player(p, [0.0, 0.0, 0.0], 0.0, Moving::default());
    let _ = s.step(0.0);
    (s, p)
}

fn wield(s: &mut Survival, p: Id, weapon: &str) {
    let i = s.content.item(weapon).unwrap_or_else(|| panic!("no item {weapon}"));
    s.players.get_mut(&p).unwrap().weapon = i;
}

/// A weapon whose every blow rolls this much, so what's read is a share and not the dice.
fn fixed(s: &mut Survival, weapon: &str, n: u32) {
    let i = s.content.weapons.iter().position(|w| w.key == weapon).unwrap_or_else(|| panic!("no weapon {weapon}"));
    s.content.weapons[i].damage = [n, n];
}

/// A creature `at` yd in front of the hero that stands still, never strikes back, and won't die.
fn foe_at(s: &mut Survival, key: &str, at: [f32; 2]) -> Id {
    let def = s.content.mob(key).unwrap_or_else(|| panic!("no mob {key}"));
    // (It takes no notice of the hero until struck: it stands where it is put.)
    s.content.mobs[def].aggro = 0.0;
    let id = s.spawn_mob(def, [at[0], at[1], 0.0], None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1.0e9 };
    m.facing = std::f32::consts::PI;
    m.ready_at = 1.0e9;
    m.health = 1_000_000;
    m.max_health = 1_000_000;
    id
}

fn foe(s: &mut Survival, key: &str) -> Id {
    foe_at(s, key, [2.0, 0.0])
}

/// The foe back where it was put, standing, facing the hero.
fn reset_foe(s: &mut Survival, m: Id) {
    let x = s.mobs.get_mut(&m).unwrap();
    x.pos = [2.0, 0.0, 0.0];
    x.facing = std::f32::consts::PI;
    x.state = MobState::Idle { wander_at: 1.0e9 };
    x.stagger_until = 0.0;
}

/// The foe with its back to the hero, standing in its recovery (nothing to make it turn).
fn turn_away(s: &mut Survival, m: Id) {
    let hero = *s.players.keys().next().unwrap();
    let x = s.mobs.get_mut(&m).unwrap();
    x.facing = 0.0;
    x.state = MobState::Recover { foe: Foe::Hero(hero), until: 1.0e9 };
}

/// Run on until the clock reads `t`.
fn run_to(s: &mut Survival, t: f32) {
    let secs = (t - s.clock).max(0.0);
    run(s, secs);
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut ev = Vec::new();
    for _ in 0..(secs / 0.02).round() as usize {
        ev.extend(s.step(0.02));
    }
    ev
}

/// A hero at full breath (a run of blows never leaves them winded).
fn breathe(s: &mut Survival, p: Id) {
    let most = s.content.player.stamina;
    s.players.get_mut(&p).unwrap().stamina = most;
}

/// One light swing's events, and a moment after.
fn swing(s: &mut Survival, p: Id) -> Vec<Event> {
    breathe(s, p);
    s.move_player(p, [0.0, 0.0, 0.0], 0.0, Moving::default());
    s.input(p, Input::Attack);
    run(s, 0.8)
}

/// One light swing of a slow weapon.
fn slow_swing(s: &mut Survival, p: Id) -> Vec<Event> {
    breathe(s, p);
    s.move_player(p, [0.0, 0.0, 0.0], 0.0, Moving::default());
    s.input(p, Input::Attack);
    run(s, 3.0)
}

/// A heavy swing: the events over `secs`, and when (the clock) it first dealt damage to `m`.
fn heavy(s: &mut Survival, p: Id, m: Id, secs: f32) -> (Vec<Event>, Option<f32>) {
    breathe(s, p);
    s.move_player(p, [0.0, 0.0, 0.0], 0.0, Moving::default());
    s.input(p, Input::Heavy);
    let (mut ev, mut at) = (Vec::new(), None);
    for _ in 0..(secs / 0.02).round() as usize {
        let e = s.step(0.02);
        if at.is_none() && hits_on(&e, m).iter().any(|(d, _)| *d > 0) {
            at = Some(s.clock);
        }
        ev.extend(e);
    }
    (ev, at)
}

/// A light swing: the events over 0.8 sec, and when it first dealt damage to `m`.
fn light(s: &mut Survival, p: Id, m: Id) -> (Vec<Event>, Option<f32>) {
    light_for(s, p, m, 0.8)
}

/// A light swing over `secs`.
fn light_for(s: &mut Survival, p: Id, m: Id, secs: f32) -> (Vec<Event>, Option<f32>) {
    breathe(s, p);
    s.move_player(p, [0.0, 0.0, 0.0], 0.0, Moving::default());
    s.input(p, Input::Attack);
    let (mut ev, mut at) = (Vec::new(), None);
    for _ in 0..(secs / 0.02).round() as usize {
        let e = s.step(0.02);
        if at.is_none() && hits_on(&e, m).iter().any(|(d, _)| *d > 0) {
            at = Some(s.clock);
        }
        ev.extend(e);
    }
    (ev, at)
}

fn hits_on(ev: &[Event], m: Id) -> Vec<(u32, Outcome)> {
    ev.iter()
        .filter_map(|e| match e {
            Event::Swing { victim: Some(v), damage, outcome, style, .. } if *v == m && !matches!(style, Blow::Dot) => Some((*damage, *outcome)),
            _ => None,
        })
        .collect()
}

fn first_hit(ev: &[Event], m: Id) -> u32 {
    hits_on(ev, m).into_iter().find(|(d, o)| *d > 0 && *o == Outcome::Hit).map_or(0, |(d, _)| d)
}

/// When a condition on `id` ends (the clock), if it holds.
fn until(s: &Survival, id: Id, kind: &str) -> Option<f32> {
    s.cc_of(id).iter().find(|c| c.kind == kind && c.until > s.clock).map(|c| c.until)
}

fn near(a: f32, b: f32, tol: f32) -> bool {
    (a - b).abs() <= tol
}

/// A heavy swing at `m`, and how far from the hero it stood when struck, and the nearest and the
/// farthest it got in the second after (a struck foe walks about, so the throw is read from these).
fn heavy_gaps(s: &mut Survival, p: Id, m: Id) -> (bool, f32, f32, f32) {
    breathe(s, p);
    s.move_player(p, [0.0, 0.0, 0.0], 0.0, Moving::default());
    s.input(p, Input::Heavy);
    let (mut hit, mut at, mut near_, mut far) = (false, 0.0f32, f32::MAX, 0.0f32);
    for _ in 0..150 {
        let ev = s.step(0.02);
        let gap = dist2d(s.mobs[&m].pos, s.players[&p].pos);
        if !hit && hits_on(&ev, m).iter().any(|(d, _)| *d > 0) {
            hit = true;
            at = gap;
        }
        if hit {
            near_ = near_.min(gap);
            far = far.max(gap);
        }
    }
    (hit, at, near_, far)
}


// --- All families --------------------------------------------------------------------------------------

#[test]
fn swing_a_1h_axe_a_1h_mace_a_2h_axe_and_a_maul_each_has_its_own_moveset() {
    let c = Content::dev();
    let set = |weapon: &str| c.moveset_of(c.weapon(weapon).unwrap()).clone();
    let (axe, mace, axe2, maul) = (set("axe"), set("mace"), set("guillotine_axe"), set("maul"));
    assert_eq!((axe.key.as_str(), mace.key.as_str(), axe2.key.as_str(), maul.key.as_str()), ("axe_one_hand", "mace_one_hand", "axe_two_hand", "maul"));
    // (Four sets, and none the grip's.)
    let keys = [&axe.key, &mace.key, &axe2.key, &maul.key];
    for (i, a) in keys.iter().enumerate() {
        assert!(!["one_hand", "two_hand"].contains(&a.as_str()));
        assert!(keys.iter().skip(i + 1).all(|b| a != b));
    }
    // Wide hacking cleaves: three chops, the last a two-hander's wide swing (the dwarf-less scourge clips 17, 17, 18).
    assert_eq!(axe.light.iter().map(|m| m.clip).collect::<Vec<_>>(), [17, 17, 18]);
    // Overhead slams, each slower and heavier than the last: every step a beat slower, and harder.
    assert!(mace.light.iter().all(|m| m.clip == 57 && m.windup > 1.2 && m.damage > 1.1));
    assert!(mace.light.windows(2).all(|w| w[1].windup > w[0].windup && w[1].damage > w[0].damage && w[1].recovery > w[0].recovery));
    // Big diagonal cleaves, each wider than the last.
    assert!(axe2.light.windows(2).all(|w| w[1].windup > w[0].windup && w[1].damage >= w[0].damage));
    assert_eq!(axe2.light.iter().map(|m| m.clip).collect::<Vec<_>>(), [18, 19, 18]);
    // One slow overhead slam, every blow: the same clip each step, slow and hard.
    assert!(maul.light.iter().all(|m| m.clip == 19 && m.windup > 1.25 && m.damage > 1.2));
    // Each is slower than its grip's own set, step for step, and the heavy's coil is held longest on the maul.
    let (grip1, grip2) = (c.movesets.iter().find(|m| m.key == "one_hand").unwrap(), c.movesets.iter().find(|m| m.key == "two_hand").unwrap());
    assert!(mace.light.iter().zip(&grip1.light).all(|(a, b)| a.windup > b.windup));
    assert!(maul.light.iter().zip(&grip2.light).all(|(a, b)| a.windup > b.windup));
    assert!(maul.heavy.lunge < axe2.heavy.lunge, "the maul's heavy holds its coil");
    // In the hero's hand (what the clients are told to play).
    let (mut s, p) = world();
    wield(&mut s, p, "mace");
    let mace_hit = s.strike_at(p, StrikeKind::Light, 0);
    wield(&mut s, p, "sword");
    let sword_hit = s.strike_at(p, StrikeKind::Light, 0);
    assert!(mace_hit.windup / s.content.weapon("mace").unwrap().windup > sword_hit.windup / s.content.weapon("sword").unwrap().windup * 1.2);
    // The clips are chosen stock ones, never watched in the client: CLIENT.
}

#[test]
fn start_a_fresh_hero_with_the_footmans_axe_it_still_swings_at_the_grips_old_pace() {
    let (mut s, p) = world();
    assert_eq!(s.players[&p].weapon, s.content.item("axe").unwrap(), "a fresh hero starts with the Footman's Axe");
    let w = s.content.weapon("axe").unwrap().clone();
    assert_eq!(w.moveset_key(), "axe_one_hand");
    let grip = s.content.movesets.iter().find(|m| m.key == "one_hand").unwrap().clone();
    let own = s.content.moveset_of(&w).clone();
    assert_eq!(own.light.len(), grip.light.len());
    for (k, (a, b)) in own.light.iter().zip(&grip.light).enumerate() {
        // The grip's timing, step for step: wind-up, recovery, damage and stamina.
        assert_eq!((a.windup, a.recovery, a.damage, a.stamina), (b.windup, b.recovery, b.damage, b.stamina), "step {k}");
        // And so the seconds a blow takes in the hero's hand are the grip's old ones.
        let strike = s.strike_at(p, StrikeKind::Light, k);
        assert!(near(strike.windup, w.windup * b.windup, 1.0e-5) && near(strike.recovery, w.recovery * b.recovery, 1.0e-5), "step {k}: {} + {}", strike.windup, strike.recovery);
    }
}

// --- Swords --------------------------------------------------------------------------------------------

#[test]
fn silver_edged_longsword_on_a_worgen_or_an_undead_foe_50_percent_more_damage_and_it_stays_down() {
    // 50% more damage, and only on those (a woodcutter takes the plain blow).
    for (key, bane) in [("risen_skeleton", true), ("sf_worg", true), ("dm_woodcutter", false)] {
        let (mut s, p) = world();
        fixed(&mut s, "silver_sword", 20);
        wield(&mut s, p, "silver_sword");
        let m = foe(&mut s, key);
        assert_eq!(first_hit(&swing(&mut s, p), m), if bane { 30 } else { 20 }, "{key}");
    }
    // It stays down once slain: a pack's creature felled under silver isn't put back to respawn;
    // the same creature felled by a plain sword is.
    for (weapon, back) in [("silver_sword", 0), ("sword", 1)] {
        let (mut s, p) = world();
        wield(&mut s, p, weapon);
        let def = s.content.mob("risen_skeleton").unwrap();
        let m = s.spawn_mob(def, [2.0, 0.0, 0.0], Some(0), false);
        {
            let x = s.mobs.get_mut(&m).unwrap();
            x.state = MobState::Idle { wander_at: 1.0e9 };
            x.facing = std::f32::consts::PI;
            x.ready_at = 1.0e9;
            x.health = 1;
        }
        swing(&mut s, p);
        assert_eq!(s.mobs.get(&m).map_or(0, |x| x.health), 0, "{weapon}: slain");
        assert_eq!(s.respawns.len(), back, "{weapon}: a silvered foe stays down, another comes back");
    }
}

#[test]
fn cruel_barb_heavy_the_foe_is_hooked_and_pulled_up_to_6_yd_to_you() {
    assert_eq!(Content::dev().weapon("cruel_barb").unwrap().hook, 6.0);
    let gap = |weapon: &str| {
        let (mut s, p) = world();
        wield(&mut s, p, weapon);
        let m = foe_at(&mut s, "dm_woodcutter", [3.0, 0.0]);
        let (hit, _, near_, _) = heavy_gaps(&mut s, p, m);
        assert!(hit, "{weapon} lands");
        near_
    };
    let hooked = gap("cruel_barb");
    assert!(hooked < 2.0, "the foe is hauled in to the hero's feet: {hooked}");
    assert!(gap("sword") > 2.4, "a sword's heavy leaves it where it stood");
    // The pull itself reaches up to 6 yd: a far foe is hauled all of it, short of the hero.
    let (mut s, p) = world();
    wield(&mut s, p, "cruel_barb");
    let m = foe_at(&mut s, "dm_woodcutter", [12.0, 0.0]);
    let pull = crate::content::AbilityEffect { kind: "pull".into(), amount: 6.0, from_caster: true, ..Default::default() };
    s.displace(p, m, &pull);
    run(&mut s, 1.5);
    let moved = 12.0 - s.mobs[&m].pos[0];
    assert!(near(moved, 6.0, 0.4), "{moved}");
}

#[test]
fn pirates_cutlass_heavy_a_5_yd_boarding_leap_at_the_foe() {
    // The leap itself is the client's (the server tells it how far each blow carries the hero: `Strike::lunge`).
    let (mut s, p) = world();
    wield(&mut s, p, "pirate_cutlass");
    assert_eq!(s.content.weapon("pirate_cutlass").unwrap().boarding, 5.0);
    assert_eq!(s.strike_at(p, StrikeKind::Heavy, 0).lunge, 5.0, "the heavy leaps 5 yd");
    assert!(s.strike_at(p, StrikeKind::Light, 0).lunge < 1.5, "a light blow doesn't");
    wield(&mut s, p, "sword");
    assert!(s.strike_at(p, StrikeKind::Heavy, 0).lunge < 1.5, "nor does a sword's heavy");
    // Struck from the leap's end, the foe takes the heavy blow as any other.
    wield(&mut s, p, "pirate_cutlass");
    let m = foe_at(&mut s, "dm_woodcutter", [3.0, 0.0]);
    assert!(heavy(&mut s, p, m, 1.5).1.is_some());
    // (The leap's look needs the client.)
}

#[test]
fn sneeds_machete_light_hits_each_chops_every_foe_in_a_5_yd_line_and_it_has_no_backstab() {
    let c = Content::dev();
    let w = c.weapon("sneed_machete").unwrap();
    assert!(w.line == 5.0 && w.no_backstab && w.backstab == 0.0);
    let (mut s, p) = world();
    fixed(&mut s, "sneed_machete", 20);
    wield(&mut s, p, "sneed_machete");
    let near_foe = foe_at(&mut s, "dm_woodcutter", [2.0, 0.0]);
    let end = foe_at(&mut s, "dm_woodcutter", [4.6, 0.0]);
    let beyond = foe_at(&mut s, "dm_woodcutter", [6.8, 0.0]);
    let aside = foe_at(&mut s, "dm_woodcutter", [2.0, 3.5]);
    let all = [near_foe, end, beyond, aside];
    let spots = [[2.0, 0.0], [4.6, 0.0], [6.8, 0.0], [2.0, 3.5]];
    // Each light blow of the chain, three in a row (the foes put back where they stood between).
    for k in 0..3 {
        for (m, at) in all.iter().zip(spots) {
            let x = s.mobs.get_mut(m).unwrap();
            x.pos = [at[0], at[1], 0.0];
            x.state = MobState::Idle { wander_at: 1.0e9 };
        }
        breathe(&mut s, p);
        s.input(p, Input::Attack);
        let mut ev = Vec::new();
        for _ in 0..80 {
            ev.extend(s.step(0.02));
            if matches!(&s.players[&p].action, Action::Swing { t, strike, .. } if *t >= strike.windup + strike.active + strike.recovery * s.content.combat.chain_after) {
                break;
            }
        }
        let lost: Vec<bool> = all.iter().map(|m| !hits_on(&ev, *m).is_empty()).collect();
        assert_eq!(lost, [true, true, false, false], "light blow {k}: the line is chopped to its end and no farther, nothing to the side");
    }
    // From behind, no more than from the front (a stabbing blade would do more).
    let blow = |weapon: &str, behind: bool| {
        let (mut s, p) = world();
        fixed(&mut s, weapon, 20);
        wield(&mut s, p, weapon);
        let m = foe(&mut s, "dm_woodcutter");
        if behind {
            turn_away(&mut s, m);
        }
        first_hit(&swing(&mut s, p), m)
    };
    assert_eq!(blow("sneed_machete", true), blow("sneed_machete", false), "no backstab");
    assert!(blow("sword", true) > blow("sword", false), "a sword has one");
}

#[test]
fn cursed_felblade_last_blow_of_the_light_chain_you_step_behind_the_foe() {
    let (mut s, p) = world();
    wield(&mut s, p, "cursed_felblade");
    let m = foe(&mut s, "dm_woodcutter");
    let len = s.content.moveset_of(s.content.weapon("cursed_felblade").unwrap()).light.len();
    assert!(len >= 2);
    let size = s.content.mobs[s.mobs[&m].def].body();
    for step in 0..len {
        reset_foe(&mut s, m);
        let late = {
            let k = s.strike_at(p, StrikeKind::Light, step);
            k.windup + k.active + k.recovery * 0.5
        };
        s.input(p, Input::Attack);
        run(&mut s, late + 0.04);
        let (x, foe_x) = (s.players[&p].pos[0], s.mobs[&m].pos[0]);
        if step + 1 < len {
            assert!(x < foe_x, "blow {step}: still in front ({x} of {foe_x})");
        } else {
            let gap = x - foe_x;
            assert!(gap > 0.5 && gap < size + crate::sim::weapon_fx::BEHIND_GAP + 0.3, "the last carries the hero just behind the foe: {gap} past it (body {size})");
            assert!(s.players[&p].facing.cos() < -0.9, "and turned to face it: {}", s.players[&p].facing);
        }
    }
    // A heavy blow doesn't step (the chain's last light blow alone does).
    let (mut s, p) = world();
    wield(&mut s, p, "cursed_felblade");
    let m = foe(&mut s, "dm_woodcutter");
    heavy(&mut s, p, m, 1.5);
    assert!(s.players[&p].pos[0] < s.mobs[&m].pos[0]);
    // (The fel step's look needs the client.)
}

// --- One-hand axes and polearms ------------------------------------------------------------------------

#[test]
fn bearded_axe_heavy_against_a_shield_bearer_they_cant_block_for_4_sec() {
    assert_eq!(Content::dev().weapon("bearded_axe").unwrap().guard_hook, 4.0);
    let (mut s, p) = world();
    let pirate = s.content.mob("dm_pirate").unwrap();
    s.content.mobs[pirate].block = 1.0;
    wield(&mut s, p, "bearded_axe");
    let m = foe(&mut s, "dm_pirate");
    let hold = |s: &mut Survival| {
        let x = s.mobs.get_mut(&m).unwrap();
        x.pos = [2.0, 0.0, 0.0];
        x.facing = std::f32::consts::PI;
        x.state = MobState::Recover { foe: Foe::Hero(p), until: 1.0e9 };
        x.stagger_until = 0.0;
    };
    hold(&mut s);
    let blocked = |ev: &[Event], m| hits_on(ev, m).iter().any(|(_, o)| *o == Outcome::Blocked);
    let (ev, _) = light(&mut s, p, m);
    assert!(blocked(&ev, m), "the shield takes a light blow");
    run(&mut s, 1.0);
    hold(&mut s);
    let (_, at) = heavy(&mut s, p, m, 1.5);
    let at = at.expect("the heavy lands through the shield");
    assert!(near(s.mobs[&m].fx.guard_down_until - at, 4.0, 0.06), "{}", s.mobs[&m].fx.guard_down_until - at);
    // Just inside the 4 sec a light blow lands; just past them the shield is back.
    run_to(&mut s, at + 3.3);
    hold(&mut s);
    let (ev, _) = light(&mut s, p, m);
    assert!(first_hit(&ev, m) > 0 && !blocked(&ev, m), "3.3 sec on, no shield");
    run_to(&mut s, at + 4.3);
    hold(&mut s);
    let (ev, _) = light(&mut s, p, m);
    assert!(blocked(&ev, m), "past 4 sec, the shield is back");
}

#[test]
fn bloodlust_edge_each_kill_speeds_your_swing_8_percent_up_to_4_times_fading_6_sec_after_the_last() {
    let (mut s, p) = world();
    wield(&mut s, p, "bloodlust_edge");
    let base = s.strike_at(p, StrikeKind::Light, 0).windup;
    let kill = |s: &mut Survival| {
        let m = foe(s, "dm_woodcutter");
        s.mobs.get_mut(&m).unwrap().health = 10;
        s.hurt_mob(m, p, 1_000_000);
        assert_eq!(s.mobs[&m].health, 0);
    };
    // Each kill: 8% quicker, stacking.
    for n in 1..=4u32 {
        kill(&mut s);
        let quick = s.strike_at(p, StrikeKind::Light, 0).windup;
        assert!(near(quick, base / (1.0 + 0.08 * n as f32), 1.0e-4), "{n} kills: {quick} of {base}");
    }
    // Up to 4 times: a fifth and sixth add nothing.
    kill(&mut s);
    kill(&mut s);
    assert!(near(s.strike_at(p, StrikeKind::Light, 0).windup, base / 1.32, 1.0e-4));
    // It fades 6 sec after the last, and each kill pushes that back.
    run(&mut s, 5.0);
    assert!(near(s.strike_at(p, StrikeKind::Light, 0).windup, base / 1.32, 1.0e-4), "5 sec on, still quick");
    kill(&mut s);
    run(&mut s, 5.0);
    assert!(near(s.strike_at(p, StrikeKind::Light, 0).windup, base / 1.32, 1.0e-4), "5 sec after the next kill, still quick");
    run(&mut s, 1.2);
    assert!(near(s.strike_at(p, StrikeKind::Light, 0).windup, base, 1.0e-4), "6 sec after the last, gone");
}

#[test]
fn defias_miners_pick_on_a_golem_a_rockling_or_the_shredder_50_percent_more_damage_and_stances_break_quicker() {
    let (mut s, p) = world();
    fixed(&mut s, "miner_pick", 20);
    wield(&mut s, p, "miner_pick");
    // Which creatures the bane names: the only golem is Stitches, who must carry the tag.
    for key in ["stitches", "rockling", "dm_shredder"] {
        let def = s.content.mob(key).unwrap();
        assert!(s.content.mobs[def].tags.iter().any(|t| ["golem", "stone", "mechanical"].contains(&t.as_str())), "{key} is stone, golem or mechanical");
    }
    for (key, bane) in [("stitches", true), ("rockling", true), ("dm_shredder", true), ("dm_woodcutter", false)] {
        let (mut s, p) = world();
        fixed(&mut s, "miner_pick", 20);
        wield(&mut s, p, "miner_pick");
        let m = foe(&mut s, key);
        assert_eq!(first_hit(&swing(&mut s, p), m), if bane { 30 } else { 20 }, "{key}");
    }
    // Stances break quicker: a champion's posture builds 3 times as fast on the pick as on an axe.
    let built = |weapon: &str| {
        let (mut s, p) = world();
        fixed(&mut s, weapon, 20);
        wield(&mut s, p, weapon);
        let m = foe(&mut s, "dm_woodcutter");
        let def = s.mobs[&m].def;
        s.content.mobs[def].posture = 100_000;
        s.content.mobs[def].tags.clear();
        let d = first_hit(&swing(&mut s, p), m);
        (s.mobs[&m].posture as f32 / d as f32, d)
    };
    let (pick, d1) = built("miner_pick");
    let (axe, d2) = built("axe");
    assert_eq!((d1, d2), (20, 20));
    assert!(near(pick / axe, 3.0, 0.05), "{pick} against {axe} posture a point of damage");
}

#[test]
fn the_horsemans_scythe_a_200_degree_sweep_of_up_to_5_foes_and_each_foe_struck_heals_you() {
    let w = Content::dev().weapon("horsemans_scythe").unwrap().clone();
    assert_eq!((w.arc, w.targets), (200.0, 5));
    let ring = |degrees: &[f32], r: f32| -> Vec<[f32; 2]> { degrees.iter().map(|d| [r * d.to_radians().cos(), r * d.to_radians().sin()]).collect() };
    let strike = |at: Vec<[f32; 2]>| {
        let (mut s, p) = world();
        wield(&mut s, p, "horsemans_scythe");
        let foes: Vec<Id> = at.iter().map(|a| foe_at(&mut s, "dm_woodcutter", *a)).collect();
        let max = s.players[&p].max_health;
        s.players.get_mut(&p).unwrap().health = max / 2;
        let ev = swing(&mut s, p);
        let struck: Vec<bool> = foes.iter().map(|m| hits_on(&ev, *m).iter().any(|(d, _)| *d > 0)).collect();
        (struck, s.players[&p].health - max / 2, max)
    };
    // A sweep reaches round to the sides: 95 degrees off the line is in, 105 is out.
    let (struck, _, _) = strike(ring(&[0.0, 95.0, -95.0, 105.0, -105.0, 180.0], 3.0));
    assert_eq!(struck, [true, true, true, false, false, false]);
    // Up to 5: seven in the sweep, five struck.
    let (struck, healed, max) = strike(ring(&[0.0, 30.0, -30.0, 60.0, -60.0, 90.0, -90.0], 3.0));
    assert_eq!(struck.iter().filter(|s| **s).count(), 5);
    // Each foe struck heals the reaper: 1% of THEIR OWN health (the checklist said "its").
    let each = (max as f32 * w.reap_heal).round() as u32;
    assert_eq!(healed, 5 * each, "{each} a foe");
    assert!(near(each as f32 / max as f32, 0.01, 0.005));
}

#[test]
fn loksey_boar_spear_its_thrust_slows_the_foe_40_percent_for_2_and_a_half_sec() {
    let (mut s, p) = world();
    wield(&mut s, p, "loksey_boar_spear");
    let m = foe_at(&mut s, "dm_woodcutter", [4.0, 0.0]);
    assert!(near(s.speed_share(m), 1.0, 1.0e-4));
    let (_, at) = light(&mut s, p, m);
    let at = at.expect("the thrust lands");
    assert!(near(s.speed_share(m), 0.6, 1.0e-4), "slowed 40%: {}", s.speed_share(m));
    assert!(near(until(&s, m, "snare").unwrap() - at, 2.5, 0.06));
    run_to(&mut s, at + 2.3);
    assert!(s.has_cc(m, "snare"), "still slow at 2.3 sec");
    run(&mut s, 0.4);
    assert!(!s.has_cc(m, "snare") && near(s.speed_share(m), 1.0, 1.0e-4), "free by 2.7 sec");
}

// --- Maces ---------------------------------------------------------------------------------------------

#[test]
fn hoggers_cudgel_heavy_the_foe_is_knocked_5_yd_back() {
    let moved = |weapon: &str| {
        let (mut s, p) = world();
        wield(&mut s, p, weapon);
        let m = foe(&mut s, "dm_woodcutter");
        // (Struck, it stands: only the throw moves it.)
        let def = s.mobs[&m].def;
        s.content.mobs[def].run_speed = 0.0;
        s.content.mobs[def].walk_speed = 0.0;
        let (hit, at, _, far) = heavy_gaps(&mut s, p, m);
        assert!(hit, "{weapon} lands");
        far - at
    };
    let (club, mace) = (moved("hogger_club"), moved("mace"));
    assert!(near(club, 5.0, 0.4), "the club's heavy sends it 5 yd back: {club}");
    // (A struck foe slides a yard or so with the blow, a mace's too; the knock is the 5.)
    assert!(mace < 2.5, "a mace's heavy doesn't knock it: {mace}");
}

#[test]
fn cookies_tenderizer_with_a_friend_the_friends_next_hit_deals_25_percent_more_once_and_not_your_own() {
    let hit_by = |tenderized: bool, wielder_blows_between: u32, ally_blows: u32| {
        let (mut s, p) = world();
        let q = s.join("bren", "Bren", None);
        s.move_player(q, [0.0, 1.0, 0.0], 0.0, Moving::default());
        for k in ["sword", "cookie_tenderizer"] {
            fixed(&mut s, k, 20);
        }
        wield(&mut s, p, "cookie_tenderizer");
        wield(&mut s, q, "sword");
        let m = foe(&mut s, "dm_woodcutter");
        let mut own = Vec::new();
        if tenderized {
            own.push(first_hit(&swing(&mut s, p), m));
            for _ in 0..wielder_blows_between {
                reset_foe(&mut s, m);
                own.push(first_hit(&swing(&mut s, p), m));
            }
        }
        let mut ally = Vec::new();
        for _ in 0..ally_blows {
            reset_foe(&mut s, m);
            ally.push(first_hit(&swing(&mut s, q), m));
        }
        (own, ally)
    };
    // (What the friend's blows do untenderized: the same blows, to read the share off.)
    let (_, plain) = hit_by(false, 0, 2);
    assert!(plain[0] >= 20 && plain[1] >= 20, "{plain:?}");
    let more = (plain[0] as f32 * 1.25).round() as u32;
    // The friend's next hit: 25% more, then no more.
    let (_, ally) = hit_by(true, 0, 2);
    assert_eq!(ally, [more, plain[1]], "25% more, once");
    // Your own next hits get no bonus, and don't use it up for the friend.
    let (own, ally) = hit_by(true, 2, 1);
    assert_eq!(ally, [more], "the friend still gets it after your own blows");
    assert_eq!(own.len(), 3);
    let (mut s, p) = world();
    wield(&mut s, p, "cookie_tenderizer");
    let m = foe(&mut s, "dm_woodcutter");
    light(&mut s, p, m);
    for k in 0..3 {
        reset_foe(&mut s, m);
        light(&mut s, p, m);
        assert_eq!(s.mobs[&m].fx.tender.0, 0.25, "your own blow {k} leaves the mark on it");
    }
}

#[test]
fn the_smelters_spanner_heavy_stuns_a_mechanical_non_champion_foe_for_4_sec() {
    let (stun, shredder) = {
        let (mut s, p) = world();
        let def = s.content.mob("dm_woodcutter").unwrap();
        s.content.mobs[def].tags.push("mechanical".into());
        assert!(!s.content.mobs[def].boss);
        wield(&mut s, p, "gilnid_wrench");
        let m = foe(&mut s, "dm_woodcutter");
        let (_, at) = heavy(&mut s, p, m, 1.5);
        let at = at.expect("lands");
        let stunned_for = until(&s, m, "stun").expect("stunned") - at;
        run_to(&mut s, at + 3.6);
        let still = s.has_cc(m, "stun");
        run(&mut s, 0.6);
        (stunned_for, (still, s.has_cc(m, "stun")))
    };
    assert!(near(stun, 4.0, 0.06), "{stun}");
    assert_eq!(shredder, (true, false), "held at 3.6 sec, free by 4.2");
    // A light blow doesn't; a man (not mechanical) isn't; the Shredder, a champion, shrugs it off.
    let (mut s, p) = world();
    let def = s.content.mob("dm_woodcutter").unwrap();
    s.content.mobs[def].tags.push("mechanical".into());
    wield(&mut s, p, "gilnid_wrench");
    let m = foe(&mut s, "dm_woodcutter");
    light(&mut s, p, m);
    assert!(!s.has_cc(m, "stun"), "a light blow doesn't stun");
    let (mut s, p) = world();
    wield(&mut s, p, "gilnid_wrench");
    let m = foe(&mut s, "dm_woodcutter");
    heavy(&mut s, p, m, 1.5);
    assert!(!s.has_cc(m, "stun"), "not mechanical");
    let (mut s, p) = world();
    wield(&mut s, p, "gilnid_wrench");
    let c = Content::dev();
    assert!(c.mobs[c.mob("dm_shredder").unwrap()].boss && c.mobs[c.mob("dm_shredder").unwrap()].tags.iter().any(|t| t == "mechanical"));
    let m = foe(&mut s, "dm_shredder");
    heavy(&mut s, p, m, 1.5);
    assert!(!s.has_cc(m, "stun"), "a champion shrugs it off");
}

/// A second hero standing `at` from the origin, hurt to `share` of their health.
fn hero_at(s: &mut Survival, name: &str, at: [f32; 2], share: f32) -> Id {
    let q = s.join(&name.to_lowercase(), name, None);
    s.move_player(q, [at[0], at[1], 0.0], 0.0, Moving::default());
    let max = s.players[&q].max_health;
    s.players.get_mut(&q).unwrap().health = ((max as f32 * share).round() as u32).max(1);
    q
}

#[test]
fn hand_of_righteousness_each_hit_heals_the_most_wounded_hero_within_15_yd_3_percent_of_their_health() {
    let w = Content::dev().weapon("hand_of_righteousness").unwrap().clone();
    assert_eq!(w.mend_ally, 0.03);
    assert_eq!(crate::content::MEND_RANGE, 15.0);
    let (mut s, p) = world();
    wield(&mut s, p, "hand_of_righteousness");
    let m = foe(&mut s, "dm_woodcutter");
    let max = s.players[&p].max_health;
    let mid = hero_at(&mut s, "Bren", [0.0, 14.5], 0.5);
    let worst = hero_at(&mut s, "Cal", [0.0, 15.6], 0.05);
    let hurt_less = hero_at(&mut s, "Dev", [0.0, 5.0], 0.8);
    s.players.get_mut(&p).unwrap().health = max - 20;
    let before: Vec<u32> = [p, mid, worst, hurt_less].iter().map(|h| s.players[h].health).collect();
    assert!(first_hit(&swing(&mut s, p), m) > 0);
    let after: Vec<u32> = [p, mid, worst, hurt_less].iter().map(|h| s.players[h].health).collect();
    let gain: Vec<i64> = after.iter().zip(&before).map(|(a, b)| *a as i64 - *b as i64).collect();
    let three = (max as f32 * 0.03).round() as i64;
    // (A little comes back by itself in the swing's second.)
    assert!((three..three + 8).contains(&gain[1]), "the most wounded within 15 yd, 14.5 away, gets 3%: {gain:?}");
    assert!(gain[0] < 8 && gain[3] < 8, "no one else: {gain:?}");
    assert!(gain[2] < 8, "the worst hurt hero 15.6 yd off is out of reach: {gain:?}");
    // The wielder too, when they are the most wounded (3% of their own health).
    let (mut s, p) = world();
    wield(&mut s, p, "hand_of_righteousness");
    let m = foe(&mut s, "dm_woodcutter");
    let q = hero_at(&mut s, "Bren", [0.0, 6.0], 0.9);
    s.players.get_mut(&p).unwrap().health = s.players[&p].max_health / 4;
    let (h0, q0, max) = (s.players[&p].health, s.players[&q].health, s.players[&p].max_health);
    assert!(first_hit(&swing(&mut s, p), m) > 0);
    let three = (max as f32 * 0.03).round() as u32;
    assert!((h0 + three..h0 + three + 8).contains(&s.players[&p].health), "you too: {} of {h0}", s.players[&p].health);
    assert!(s.players[&q].health < q0 + 8);
}

#[test]
fn hand_of_righteousness_passes_over_a_hero_held_in_a_duel() {
    let (mut s, p) = world();
    let q = hero_at(&mut s, "Bren", [0.0, 3.0], 0.5);
    let r = hero_at(&mut s, "Cal", [0.0, -3.0], 1.0);
    s.duel_challenge(q, r).unwrap();
    s.duel_accept(r);
    assert!(s.in_duel(q));
    wield(&mut s, p, "hand_of_righteousness");
    let m = foe(&mut s, "dm_woodcutter");
    let max = s.players[&p].max_health;
    s.players.get_mut(&p).unwrap().health = max - 40;
    let (q0, p0) = (s.players[&q].health, s.players[&p].health);
    assert!(first_hit(&swing(&mut s, p), m) > 0);
    assert!(s.players[&q].health < q0 + 8, "the duellist, the most wounded, is passed over: {}", s.players[&q].health);
    let three = (max as f32 * 0.03).round() as u32;
    assert!(s.players[&p].health >= p0 + three, "and the heal goes to the next: the wielder ({} of {p0})", s.players[&p].health);
}

/// A Siege match of two heroes, one in each camp (the hall, the way, the countdown).
fn siege_pair() -> (Survival, Id, Id) {
    use crate::sim::siege::SiegePhase;
    let mut c = Content::dev();
    c.dev.full_tree = false;
    let mut s = Survival::new_siege(c, Box::new(FlatLand::default()), 1, 1_000_000.0);
    let arrive = |s: &mut Survival, name: &str| {
        let id = s.join(&name.to_lowercase(), name, None);
        s.siege_arrive(id);
        id
    };
    let (a, b) = (arrive(&mut s, "Ayla"), arrive(&mut s, "Bren"));
    s.siege_join(a, "1v1").unwrap();
    s.siege_join(b, "1v1").unwrap();
    if !matches!(s.siege.as_ref().unwrap().phase, SiegePhase::Countdown { .. }) {
        s.siege_start_now(a).unwrap();
    }
    for _ in 0..((Content::dev().siege.unwrap().countdown + 0.2) / 0.05).round() as usize {
        s.step(0.05);
    }
    assert_eq!(s.siege.as_ref().unwrap().phase, SiegePhase::Running);
    (s, a, b)
}

#[test]
fn hand_of_righteousness_passes_over_a_rival_camps_hero_in_siege() {
    let (mut s, a, b) = siege_pair();
    assert!(s.camp_of(a) != s.camp_of(b));
    assert!(s.siege_foes(a, b) && !s.helpable(a, b));
    wield(&mut s, a, "hand_of_righteousness");
    // A creature to strike, in front of Ayla, and the rival right beside her, worse hurt.
    let at = s.players[&a].pos;
    s.move_player(a, at, 0.0, Moving::default());
    let def = s.content.mob("dm_woodcutter").unwrap();
    let m = s.spawn_mob(def, [at[0] + 2.0, at[1], at[2]], None, false);
    {
        let x = s.mobs.get_mut(&m).unwrap();
        x.state = MobState::Idle { wander_at: 1.0e9 };
        x.facing = std::f32::consts::PI;
        x.ready_at = 1.0e9;
        x.health = 1_000_000;
        x.max_health = 1_000_000;
    }
    s.move_player(b, [at[0], at[1] + 6.0, at[2]], 0.0, Moving::default());
    let max = s.players[&a].max_health;
    s.players.get_mut(&b).unwrap().health = max / 4;
    s.players.get_mut(&a).unwrap().health = max / 2;
    let (a0, b0) = (s.players[&a].health, s.players[&b].health);
    s.input(a, Input::Attack);
    let ev = run(&mut s, 0.8);
    assert!(first_hit(&ev, m) > 0, "the blow lands");
    let three = (max as f32 * 0.03).round() as u32;
    assert!(s.players[&b].health < b0 + 8, "the rival isn't mended: {} of {b0}", s.players[&b].health);
    assert!(s.players[&a].health >= a0 + three, "the heal goes to the wielder instead ({} of {a0})", s.players[&a].health);
}

#[test]
fn barons_scepter_a_cursed_foe_that_dies_within_12_sec_bursts_in_shadow_on_foes_within_5_yd() {
    assert_eq!((crate::content::CURSE_SECS, crate::content::CURSE_RADIUS), (12.0, 5.0));
    let burst = ((25 + 33) as f32 / 2.0 * 1.5).round() as u32;
    let run_it = |die_after: f32| {
        let (mut s, p) = world();
        wield(&mut s, p, "barons_scepter");
        let m = foe(&mut s, "dm_woodcutter");
        let inside = foe_at(&mut s, "dm_woodcutter", [2.0, 4.8]);
        let outside = foe_at(&mut s, "dm_woodcutter", [2.0, 5.3]);
        let (_, at) = light(&mut s, p, m);
        let at = at.expect("lands");
        let (_, _, _) = s.mobs[&m].fx.curse.expect("cursed");
        run_to(&mut s, at + die_after);
        reset_foe(&mut s, m);
        let (i0, o0) = (s.mobs[&inside].health, s.mobs[&outside].health);
        s.hurt_mob(m, p, 2_000_000);
        assert_eq!(s.mobs[&m].health, 0);
        (i0 - s.mobs[&inside].health, o0 - s.mobs[&outside].health)
    };
    assert_eq!(run_it(11.5), (burst, 0), "dying at 11.5 sec: the foe 4.8 yd away takes the burst, 5.3 yd doesn't");
    assert_eq!(run_it(12.5), (0, 0), "dying at 12.5 sec: no burst");
    assert_eq!(burst, 44);
}

#[test]
fn ironspines_fist_each_hit_gives_a_5_point_shield_for_8_sec_stacking_to_30() {
    let (mut s, p) = world();
    wield(&mut s, p, "ironspine_fist");
    let m = foe(&mut s, "dm_woodcutter");
    let shield = |s: &Survival| s.cc_of(p).iter().find(|c| c.kind == "shield" && c.until > s.clock).map_or(0.0, |c| c.left);
    let mut seen = Vec::new();
    let mut last = 0.0;
    for _ in 0..8 {
        reset_foe(&mut s, m);
        let (_, at) = light(&mut s, p, m);
        last = at.expect("lands");
        seen.push(shield(&s));
    }
    assert_eq!(seen, [5.0, 10.0, 15.0, 20.0, 25.0, 30.0, 30.0, 30.0], "5 a hit, up to 30");
    assert!(near(until(&s, p, "shield").unwrap() - last, 8.0, 0.06), "8 sec from the last hit");
    run_to(&mut s, last + 7.7);
    assert_eq!(shield(&s), 30.0);
    run(&mut s, 0.6);
    assert_eq!(shield(&s), 0.0, "gone after 8 sec");
}

// --- Daggers -------------------------------------------------------------------------------------------

#[test]
fn hypnotic_blade_a_hit_makes_the_foe_drop_you_for_4_sec_and_keep_stabbing_it_is_not_lulled_twice() {
    assert_eq!(crate::content::LULL_SECS, 4.0);
    let (mut s, p) = world();
    wield(&mut s, p, "hypnotic_blade");
    let m = foe(&mut s, "dm_woodcutter");
    s.mobs.get_mut(&m).unwrap().ready_at = 0.0;
    let (_, at) = light(&mut s, p, m);
    let at = at.expect("lands");
    assert!(s.lulled(m, p), "it has dropped you");
    let lulled_until = s.mobs[&m].fx.lulled.unwrap().1;
    assert!(near(lulled_until - at, 4.0, 0.06), "{}", lulled_until - at);
    assert!(matches!(s.mobs[&m].state, MobState::Idle { .. }), "no longer on you");
    // Stabbing again inside the 4 sec doesn't lull it a second time: it turns on you and hits back.
    reset_foe(&mut s, m);
    let (_, again) = light(&mut s, p, m);
    assert!(again.is_some());
    assert_eq!(s.mobs[&m].fx.lulled.unwrap().1, lulled_until, "the lull isn't renewed");
    assert!(s.mobs[&m].state.foe() == Some(Foe::Hero(p)), "it is on you again: {:?}", s.mobs[&m].state.foe());
    // And it hits back, in the end.
    let mut blows = 0;
    for _ in 0..8 {
        s.mobs.get_mut(&m).unwrap().ready_at = 0.0;
        s.input(p, Input::Attack);
        for _ in 0..40 {
            blows += s.step(0.02).iter().filter(|e| matches!(e, Event::Swing { attacker, victim: Some(v), .. } if *attacker == m && *v == p)).count();
        }
    }
    assert!(blows > 0, "struck back: {blows}");
}

#[test]
fn meteor_shard_a_blow_from_behind_a_fire_burst_for_50_percent_on_foes_within_4_yd_and_it_breaks_ice() {
    assert_eq!((crate::content::BURST_RADIUS, Content::dev().weapon("meteor_shard").unwrap().burst), (4.0, 0.5));
    let (mut s, p) = world();
    fixed(&mut s, "meteor_shard", 20);
    wield(&mut s, p, "meteor_shard");
    let m = foe(&mut s, "dm_woodcutter");
    let inside = foe_at(&mut s, "dm_woodcutter", [2.0, 3.8]);
    let outside = foe_at(&mut s, "dm_woodcutter", [2.0, 4.3]);
    turn_away(&mut s, m);
    let (i0, o0) = (s.mobs[&inside].health, s.mobs[&outside].health);
    let hit = first_hit(&swing(&mut s, p), m);
    assert!(hit >= 60, "from behind: {hit}");
    assert_eq!(i0 - s.mobs[&inside].health, hit / 2, "50% of the blow on the foe 3.8 yd off");
    assert_eq!(s.mobs[&outside].health, o0, "none on the one 4.3 yd off");
    // From the front: no burst.
    let (mut s, p) = world();
    wield(&mut s, p, "meteor_shard");
    let m = foe(&mut s, "dm_woodcutter");
    let inside = foe_at(&mut s, "dm_woodcutter", [2.0, 3.0]);
    let i0 = s.mobs[&inside].health;
    first_hit(&swing(&mut s, p), m);
    assert_eq!(s.mobs[&inside].health, i0);
    // From behind on a frozen foe: the ice breaks, and it hurts more.
    let blow = |frozen: bool| {
        let (mut s, p) = world();
        fixed(&mut s, "meteor_shard", 20);
        wield(&mut s, p, "meteor_shard");
        let m = foe(&mut s, "dm_woodcutter");
        turn_away(&mut s, m);
        if frozen {
            assert!(s.apply_cc(m, p, "petrify", 5.0, 0.0, 0.0, "Test freeze"));
        }
        let hit = first_hit(&swing(&mut s, p), m);
        (hit, s.has_cc(m, "petrify"))
    };
    let (plain, _) = blow(false);
    let (shattered, ice) = blow(true);
    assert!(!ice, "the ice is broken");
    assert_eq!(shattered, (plain as f32 * crate::sim::abilities::SHATTER).round() as u32, "{shattered} against {plain}");
}

#[test]
fn bazils_shiv_each_hit_adds_a_poison_stack_3_a_sec_up_to_5_6_sec_beside_a_fireballs_burn() {
    assert_eq!((crate::content::POISON_STACKS, crate::content::POISON_SECS, Content::dev().weapon("bazil_shiv").unwrap().poison), (5, 6.0, 3.0));
    let (mut s, p) = world();
    wield(&mut s, p, "bazil_shiv");
    let m = foe(&mut s, "dm_woodcutter");
    // A Fireball's burn on it already.
    s.apply_cc(m, p, "burn", 30.0, 40.0, 0.0, "Fireball");
    let poison = |s: &Survival| s.cc_of(m).iter().find(|c| c.kind == "poison" && c.until > s.clock).map_or(0.0, |c| c.amount);
    let mut stacks = Vec::new();
    let mut last = 0.0;
    for _ in 0..7 {
        reset_foe(&mut s, m);
        let (_, at) = light(&mut s, p, m);
        last = at.expect("lands");
        stacks.push(poison(&s));
    }
    assert_eq!(stacks, [3.0, 6.0, 9.0, 12.0, 15.0, 15.0, 15.0], "a stack a hit, 3 a sec each, up to 5");
    assert!(near(until(&s, m, "poison").unwrap() - last, 6.0, 0.06), "6 sec from the last");
    // The burn is as it was, beside it.
    let burn = s.cc_of(m).iter().find(|c| c.kind == "burn" && c.until > s.clock).map(|c| c.amount);
    assert_eq!(burn, Some(40.0));
    // Both tick: five stacks cost about 15 a sec, and the burn comes on top.
    let h = s.mobs[&m].health;
    run(&mut s, 1.0);
    let both = h - s.mobs[&m].health;
    let (mut t, q) = world();
    wield(&mut t, q, "bazil_shiv");
    let n = foe(&mut t, "dm_woodcutter");
    for _ in 0..5 {
        reset_foe(&mut t, n);
        light(&mut t, q, n);
    }
    let h = t.mobs[&n].health;
    run(&mut t, 1.0);
    let poison_alone = h - t.mobs[&n].health;
    assert!((13..=17).contains(&poison_alone), "five stacks: {poison_alone} a sec");
    assert!(both > poison_alone, "the burn adds to it: {both} against {poison_alone}");
}

#[test]
fn the_silent_blade_75_percent_more_damage_to_a_foe_recovering_from_its_own_blow_and_plain_on_any_other() {
    let hit = |recovering: bool| {
        let (mut s, p) = world();
        fixed(&mut s, "silent_blade", 20);
        wield(&mut s, p, "silent_blade");
        let m = foe(&mut s, "dm_woodcutter");
        if recovering {
            s.mobs.get_mut(&m).unwrap().state = MobState::Recover { foe: Foe::Hero(p), until: 1.0e9 };
        }
        first_hit(&swing(&mut s, p), m)
    };
    let (recovering, idle) = (hit(true), hit(false));
    assert!(idle >= 20, "it lands: {idle}");
    assert!((recovering as f32 - idle as f32 * 1.75).abs() <= 1.0, "75% more on a foe in its recovery: {recovering} against {idle}");
    // A foe winding up a blow isn't recovering from one.
    let (mut s, p) = world();
    fixed(&mut s, "silent_blade", 20);
    wield(&mut s, p, "silent_blade");
    let m = foe(&mut s, "dm_woodcutter");
    s.mobs.get_mut(&m).unwrap().state = MobState::Windup { foe: Foe::Hero(p), t: 0.0, mark: None, shown: false };
    assert_eq!(first_hit(&swing(&mut s, p), m), idle, "plain on a foe winding up");
}

#[test]
fn thornspike_a_hit_interrupts_a_casting_foe_and_staggers_it_1_and_a_half_sec_not_a_champion() {
    assert_eq!(Content::dev().weapon("thornspike").unwrap().interrupt, 1.5);
    let casting = |weapon: &str, champion: bool| {
        let (mut s, p) = world();
        wield(&mut s, p, weapon);
        let m = foe(&mut s, "dm_woodcutter");
        let def = s.mobs[&m].def;
        s.content.mobs[def].attack.ranged = true;
        s.content.mobs[def].boss = champion;
        s.mobs.get_mut(&m).unwrap().ready_at = 0.0;
        s.engage(m, Foe::Hero(p));
        for _ in 0..100 {
            s.step(0.02);
            if matches!(s.mobs[&m].state, MobState::Windup { .. }) {
                break;
            }
        }
        assert!(matches!(s.mobs[&m].state, MobState::Windup { .. }), "casting");
        s.input(p, Input::Attack);
        let mut at = None;
        for _ in 0..30 {
            let e = s.step(0.02);
            if at.is_none() && hits_on(&e, m).iter().any(|(d, _)| *d > 0) {
                at = Some(s.clock);
            }
        }
        let at = at.expect("the stab lands");
        let staggered_for = s.mobs[&m].stagger_until - at;
        (staggered_for > 0.0, staggered_for, matches!(s.mobs[&m].state, MobState::Windup { .. }))
    };
    let (staggered, secs, winding) = casting("thornspike", false);
    assert!(staggered && near(secs, 1.5, 0.1) && !winding, "interrupted: {secs} sec");
    let (staggered, _, _) = casting("dagger", false);
    assert!(!staggered, "a plain dagger doesn't");
    let (staggered, _, _) = casting("thornspike", true);
    assert!(!staggered, "a champion's cast goes on");
}

#[test]
fn cobrahns_fang_a_blow_from_behind_sleeps_the_foe_3_sec_damage_wakes_it_and_not_again_for_6_sec() {
    // From the front: no sleep.
    {
        let (mut s, p) = world();
        wield(&mut s, p, "cobrahn_fang");
        let m = foe(&mut s, "dm_woodcutter");
        light(&mut s, p, m);
        assert!(!s.has_cc(m, "incapacitate"));
    }
    assert_eq!((Content::dev().weapon("cobrahn_fang").unwrap().sleep, crate::content::SLEEP_IMMUNE_SECS), (3.0, 6.0));
    let (mut s, p) = world();
    wield(&mut s, p, "cobrahn_fang");
    let m = foe(&mut s, "dm_woodcutter");
    // From behind: asleep 3 sec.
    turn_away(&mut s, m);
    let (_, at) = light(&mut s, p, m);
    let at = at.expect("lands");
    assert!(near(until(&s, m, "incapacitate").expect("asleep") - at, 3.0, 0.06));
    // Left alone, it sleeps out its 3 sec and no more.
    run_to(&mut s, at + 3.2);
    assert!(!s.has_cc(m, "incapacitate"), "awake after 3 sec");
    // From behind again, straight after (inside the 6 sec): damage, no sleep. It wakes and stays awake.
    reset_foe(&mut s, m);
    turn_away(&mut s, m);
    let (_, _) = light(&mut s, p, m);
    assert!(!s.has_cc(m, "incapacitate"), "not put to sleep again within 6 sec of its sleep ending");
    // The 6 sec run from when its sleep ran out (at + 3), not from the blow: 5 sec on, still not.
    run_to(&mut s, at + 3.0 + 5.0);
    reset_foe(&mut s, m);
    turn_away(&mut s, m);
    let (_, again) = light(&mut s, p, m);
    assert!(!s.has_cc(m, "incapacitate") && again.expect("lands") < at + 3.0 + 6.0, "5 sec after waking by itself: not again");
    run_to(&mut s, at + 3.0 + 6.5);
    reset_foe(&mut s, m);
    turn_away(&mut s, m);
    light(&mut s, p, m);
    assert!(s.has_cc(m, "incapacitate"), "asleep again 6.5 sec after waking by itself");
    // Damage wakes a sleeper: put to sleep afresh in a clean world and hit it.
    let (mut s, p) = world();
    wield(&mut s, p, "cobrahn_fang");
    let m = foe(&mut s, "dm_woodcutter");
    turn_away(&mut s, m);
    light(&mut s, p, m);
    assert!(s.has_cc(m, "incapacitate"), "asleep");
    run(&mut s, 1.0);
    s.hurt_mob(m, p, 5);
    assert!(!s.has_cc(m, "incapacitate"), "damage wakes it");
    // The 6 sec run from when it woke (1 sec into the sleep), not from the blow: 5 sec on, still not;
    // 6.5 sec on, asleep again.
    let woke = s.clock;
    run(&mut s, 5.0);
    reset_foe(&mut s, m);
    turn_away(&mut s, m);
    let (_, late) = light(&mut s, p, m);
    assert!(!s.has_cc(m, "incapacitate") && late.expect("lands") < woke + 6.0, "5 sec after waking: not again");
    run_to(&mut s, woke + 6.5);
    reset_foe(&mut s, m);
    turn_away(&mut s, m);
    light(&mut s, p, m);
    assert!(s.has_cc(m, "incapacitate"), "and again 6.5 sec after waking, 7.5 sec after the blow");
}

// --- Two-handers ---------------------------------------------------------------------------------------

#[test]
fn fallen_champions_greatsword_heavy_heroes_within_8_yd_you_too_take_15_percent_less_damage_for_5_sec() {
    let (mut s, p) = world();
    wield(&mut s, p, "champions_greatsword");
    let m = foe(&mut s, "dm_woodcutter");
    let ally = hero_at(&mut s, "Bren", [0.0, 7.6], 1.0);
    let far = hero_at(&mut s, "Cal", [0.0, 8.6], 1.0);
    let (_, at) = heavy(&mut s, p, m, 1.0);
    assert!(at.is_some());
    for (who, rallied) in [(p, true), (ally, true), (far, false)] {
        assert_eq!(s.has_cc(who, "hallowed"), rallied, "{who}");
    }
    assert!(near(s.taken_share(p), 0.85, 1.0e-4) && near(s.taken_share(ally), 0.85, 1.0e-4), "15% less damage");
    assert_eq!(s.taken_share(far), 1.0);
    // For 5 sec from the swing, then no more.
    let ends = until(&s, p, "hallowed").unwrap();
    assert!(ends > s.clock && ends - s.clock <= 5.0 + 1.0e-3);
    run_to(&mut s, ends - 0.1);
    assert!(s.has_cc(p, "hallowed"));
    run(&mut s, 0.2);
    assert!(!s.has_cc(p, "hallowed") && !s.has_cc(ally, "hallowed"));
    assert_eq!(crate::content::RALLY_SECS, 5.0);
    // A light blow rallies no one.
    let (mut s, p) = world();
    wield(&mut s, p, "champions_greatsword");
    let m = foe(&mut s, "dm_woodcutter");
    light(&mut s, p, m);
    assert!(!s.has_cc(p, "hallowed"));
}

#[test]
fn fallen_champions_greatsword_passes_over_a_hero_held_in_a_duel() {
    let (mut s, p) = world();
    let q = hero_at(&mut s, "Bren", [0.0, 3.0], 1.0);
    let r = hero_at(&mut s, "Cal", [0.0, -3.0], 1.0);
    let bystander = hero_at(&mut s, "Dev", [0.0, 5.0], 1.0);
    s.duel_challenge(q, r).unwrap();
    s.duel_accept(r);
    wield(&mut s, p, "champions_greatsword");
    let m = foe(&mut s, "dm_woodcutter");
    heavy(&mut s, p, m, 1.0);
    assert!(s.has_cc(p, "hallowed") && s.has_cc(bystander, "hallowed"));
    assert!(!s.has_cc(q, "hallowed") && !s.has_cc(r, "hallowed"), "not the duellists");
}

#[test]
fn thornroot_maul_heavy_the_foes_hit_are_rooted_2_and_a_half_sec() {
    let (mut s, p) = world();
    wield(&mut s, p, "thornroot_maul");
    let a = foe_at(&mut s, "dm_woodcutter", [2.0, 0.0]);
    let b = foe_at(&mut s, "dm_woodcutter", [2.0, 1.2]);
    let c = foe_at(&mut s, "dm_woodcutter", [2.0, -1.2]);
    let (_, at) = heavy(&mut s, p, a, 1.5);
    let at = at.expect("lands");
    for m in [a, b, c] {
        assert!(near(until(&s, m, "root").expect("rooted") - at, 2.5, 0.1), "{}", until(&s, m, "root").unwrap() - at);
    }
    run_to(&mut s, at + 2.3);
    assert!(s.has_cc(a, "root"));
    run(&mut s, 0.4);
    assert!(!s.has_cc(a, "root"));
    // A light blow roots no one.
    let (mut s, p) = world();
    wield(&mut s, p, "thornroot_maul");
    let m = foe(&mut s, "dm_woodcutter");
    slow_swing(&mut s, p);
    assert!(!s.has_cc(m, "root"));
}

#[test]
fn mograines_might_heavy_a_holy_ring_goes_out_6_yd_for_0_point_8_of_the_blow() {
    assert_eq!(crate::content::SHOCK_RADIUS, 6.0);
    let w = Content::dev().weapon("mograines_might").unwrap().clone();
    assert_eq!(w.shockwave, 0.8);
    let ring = ((w.damage[0] + w.damage[1]) as f32 / 2.0 * 0.8).round() as u32;
    assert_eq!(ring, 60);
    let (mut s, p) = world();
    wield(&mut s, p, "mograines_might");
    let front = foe(&mut s, "dm_woodcutter");
    let inside = foe_at(&mut s, "dm_woodcutter", [-5.0, 0.0]);
    let side = foe_at(&mut s, "dm_woodcutter", [0.0, 5.0]);
    let outside = foe_at(&mut s, "dm_woodcutter", [-7.0, 0.0]);
    let before: Vec<u32> = [inside, side, outside].iter().map(|m| s.mobs[m].health).collect();
    heavy(&mut s, p, front, 2.0);
    let lost: Vec<u32> = [inside, side, outside].iter().zip(&before).map(|(m, b)| b - s.mobs[m].health).collect();
    assert_eq!(lost, [ring, ring, 0], "the ring takes what is within 6 yd all round, behind the hero too, and not 7 yd off");
    // A light blow sends none.
    let (mut s, p) = world();
    wield(&mut s, p, "mograines_might");
    let behind = foe_at(&mut s, "dm_woodcutter", [-4.0, 0.0]);
    slow_swing(&mut s, p);
    assert_eq!(s.mobs[&behind].health, 1_000_000);
}

#[test]
fn smites_mighty_hammer_the_third_hit_in_a_row_on_one_foe_within_3_sec_each_staggers_it_1_and_a_half_sec() {
    assert_eq!((crate::content::BUILD_HITS, crate::content::BUILD_WINDOW, Content::dev().weapon("smite_hammer").unwrap().build), (3, 3.0, 1.5));
    let stagger = |s: &Survival, m: Id, at: f32| s.mobs[&m].stagger_until - at;
    let (mut s, p) = world();
    wield(&mut s, p, "smite_hammer");
    let m = foe(&mut s, "dm_woodcutter");
    // Hits 1 and 2 (light blows) stagger nothing.
    let mut third = None;
    for n in 1..=3 {
        reset_foe(&mut s, m);
        let (_, at) = light_for(&mut s, p, m, 2.0);
        let at = at.expect("lands");
        if n < 3 {
            assert!(s.mobs[&m].stagger_until <= at, "hit {n}: not staggered");
        } else {
            third = Some(at);
        }
        run(&mut s, 0.5);
    }
    let third = third.unwrap();
    assert!(near(stagger(&s, m, third), 1.5, 0.1), "the third staggers it for {} sec", stagger(&s, m, third));
    // Not in a row on one foe: a different foe in between spends the build.
    let (mut s, p) = world();
    wield(&mut s, p, "smite_hammer");
    let a = foe(&mut s, "dm_woodcutter");
    let b = foe_at(&mut s, "dm_woodcutter", [30.0, 0.0]);
    light_for(&mut s, p, a, 2.0);
    reset_foe(&mut s, a);
    light_for(&mut s, p, a, 2.0);
    s.mobs.get_mut(&a).unwrap().pos = [30.0, 0.0, 0.0];
    s.mobs.get_mut(&b).unwrap().pos = [2.0, 0.0, 0.0];
    light_for(&mut s, p, b, 2.0);
    s.mobs.get_mut(&b).unwrap().pos = [30.0, 0.0, 0.0];
    reset_foe(&mut s, a);
    let (_, at) = light_for(&mut s, p, a, 2.0);
    assert!(s.mobs[&a].stagger_until <= at.unwrap(), "a hit on another foe broke the row");
    // Nor within 3 sec each: a pause of over 3 sec between hits starts the count again.
    let (mut s, p) = world();
    wield(&mut s, p, "smite_hammer");
    let m = foe(&mut s, "dm_woodcutter");
    for n in 1..=3 {
        reset_foe(&mut s, m);
        let (_, at) = light_for(&mut s, p, m, 2.0);
        assert!(s.mobs[&m].stagger_until <= at.unwrap(), "hit {n} after pauses: not staggered");
        run(&mut s, 3.3);
    }
}

#[test]
fn rhahkzors_maul_heavy_it_marks_the_spot_and_1_sec_later_rocks_fall_for_0_point_6_of_the_blow_within_2_and_a_half_yd() {
    assert_eq!((crate::content::ROCK_DELAY, crate::content::ROCK_RADIUS, Content::dev().weapon("rhahkzor_maul").unwrap().rockfall), (1.0, 2.5, 0.6));
    let rocks = ((52 + 65) as f32 / 2.0 * 0.6).round() as u32;
    let (mut s, p) = world();
    wield(&mut s, p, "rhahkzor_maul");
    // The struck foe, and two off to the side of the spot (and out of the maul's own swing: beyond its reach).
    let m = foe_at(&mut s, "dm_woodcutter", [3.4, 0.0]);
    let inside = foe_at(&mut s, "dm_woodcutter", [5.5, 0.0]);
    let outside = foe_at(&mut s, "dm_woodcutter", [6.6, 0.0]);
    // Neither is in the swing's arc/reach: check that when the blow lands.
    s.move_player(p, [0.0, 0.0, 0.0], 0.0, Moving::default());
    s.input(p, Input::Heavy);
    let (mut marked_at, mut hit_at, mut rock_at) = (None, None, None);
    let (mut m_before, mut in_before) = (0, 0);
    for _ in 0..200 {
        let (mh, ih) = (s.mobs[&m].health, s.mobs[&inside].health);
        let ev = s.step(0.02);
        if marked_at.is_none() && ev.iter().any(|e| matches!(e, Event::Mark { .. })) {
            marked_at = Some(s.clock);
        }
        if hit_at.is_none() && hits_on(&ev, m).iter().any(|(d, _)| *d > 0) {
            hit_at = Some(s.clock);
            (m_before, in_before) = (s.mobs[&m].health, s.mobs[&inside].health);
        }
        if rock_at.is_none() && hit_at.is_some() && s.mobs[&inside].health < in_before {
            rock_at = Some(s.clock);
        }
        let _ = (mh, ih);
    }
    let (marked, hit, rock) = (marked_at.expect("marked"), hit_at.expect("lands"), rock_at.expect("rocks fell on the neighbour"));
    assert!(near(marked, hit, 0.1), "the mark goes down as the blow lands: {marked} and {hit}");
    assert!(near(rock - hit, 1.0, 0.1), "rocks 1 sec later: {}", rock - hit);
    assert_eq!(in_before - s.mobs[&inside].health, rocks, "a neighbour 2.1 yd from the spot takes 0.6 of the blow");
    assert_eq!(m_before - s.mobs[&m].health, rocks, "and so does the foe on it");
    assert_eq!(s.mobs[&outside].health, 1_000_000, "the one 3.2 yd off takes none");
}

#[test]
fn targorrs_cleaver_the_lower_your_health_the_harder_it_hits_up_to_80_percent_more() {
    let blow = |share: f32| {
        let (mut s, p) = world();
        fixed(&mut s, "targorr_axe", 100);
        wield(&mut s, p, "targorr_axe");
        let m = foe(&mut s, "dm_woodcutter");
        let max = s.players[&p].max_health;
        s.players.get_mut(&p).unwrap().health = ((max as f32 * share).round() as u32).max(1);
        let hp = s.players[&p].health as f32 / max as f32;
        (first_hit(&slow_swing(&mut s, p), m), hp)
    };
    let (whole, _) = blow(1.0);
    assert_eq!(whole, 100);
    let (half, hp) = blow(0.5);
    assert_eq!(half, (100.0 * (1.0 + 0.8 * (1.0 - hp))).round() as u32, "half health: 40% more");
    let (low, hp) = blow(0.0);
    assert_eq!(low, (100.0 * (1.0 + 0.8 * (1.0 - hp))).round() as u32);
    assert!((178..=180).contains(&low), "at the brink: 80% more, {low}");
}

#[test]
fn corpsemaker_its_hits_cow_the_foe_25_percent_less_damage_and_slower_windups_for_4_sec() {
    assert_eq!((Content::dev().weapon("corpsemaker").unwrap().cow, crate::content::COW_SECS), (0.25, 4.0));
    let (mut s, p) = world();
    wield(&mut s, p, "corpsemaker");
    let m = foe(&mut s, "dm_woodcutter");
    let (damage, windup) = (s.attack_now(m).damage, s.attack_now(m).windup);
    let (_, at) = light(&mut s, p, m);
    let at = at.expect("lands");
    let cowed = until(&s, m, "cowed").expect("cowed") - at;
    assert!(near(cowed, 4.0, 0.1), "{cowed}");
    let now = s.attack_now(m);
    assert_eq!(now.damage, (damage as f32 * 0.75).round() as u32, "25% less damage ({} of {damage})", now.damage);
    assert!(now.windup > windup * 1.1, "slower wind-ups: {} of {windup}", now.windup);
    // And it wears off.
    run_to(&mut s, at + 4.3);
    let later = s.attack_now(m);
    assert!(later.damage == damage && near(later.windup, windup, 1.0e-4));
    assert!(!s.has_cc(m, "cowed"));
}

#[test]
fn guillotine_axe_heavy_on_a_stunned_asleep_rooted_frozen_or_staggered_foe_triple_damage() {
    let blow = |how: &str, heavy_blow: bool| {
        let (mut s, p) = world();
        fixed(&mut s, "guillotine_axe", 40);
        wield(&mut s, p, "guillotine_axe");
        let m = foe(&mut s, "dm_woodcutter");
        match how {
            "stun" => assert!(s.apply_cc(m, p, "stun", 30.0, 0.0, 0.0, "Test")),
            "sleep" => assert!(s.apply_cc(m, p, "incapacitate", 30.0, 0.0, 0.0, "Test")),
            "root" => assert!(s.apply_cc(m, p, "root", 30.0, 0.0, 0.0, "Test")),
            "frozen" => assert!(s.apply_cc(m, p, "petrify", 30.0, 0.0, 0.0, "Test")),
            "stagger" => s.stagger_mob(m, 30.0, false),
            _ => {}
        }
        let ev = if heavy_blow { heavy(&mut s, p, m, 1.6).0 } else { slow_swing(&mut s, p) };
        first_hit(&ev, m)
    };
    let plain = blow("none", true);
    assert!(plain > 0);
    for how in ["stun", "sleep", "root", "frozen", "stagger"] {
        let h = blow(how, true);
        assert!(near(h as f32, plain as f32 * 3.0, 1.5), "{how}: {h} against {plain}");
    }
    // A light blow on a stunned foe gets no triple.
    assert!(blow("stun", false) < plain, "light on a stunned foe: {}", blow("stun", false));
}

// --- the moveset swing paces (the checklist's swing-pace claims, with each item above) and the tooltips --------

#[test]
fn every_weapon_here_has_a_tooltip_sentence_built_from_its_numbers() {
    let c = Content::dev();
    let text = |k: &str| c.items[c.item(k).unwrap()].text.clone();
    let has = |k: &str, s: &str| assert!(text(k).contains(s), "{k}: {s:?} is not in {:?}", text(k));
    has("silver_sword", "Deals 50% more damage to undead and worgen.");
    has("silver_sword", "stay down once slain");
    has("cruel_barb", "pull it up to 6 yd");
    has("pirate_cutlass", "leap 5 yd");
    has("sneed_machete", "a line 5 yd long");
    has("sneed_machete", "no extra damage from behind");
    has("cursed_felblade", "last blow of a light chain carries you behind the enemy");
    has("bearded_axe", "can't block for 4 sec");
    has("bloodlust_edge", "speeds your swings by 8% for 6 sec, up to 4 times");
    has("miner_pick", "Deals 50% more damage to stone, golem and mechanical");
    has("miner_pick", "Breaks stances 3 times as fast.");
    has("horsemans_scythe", "Restores 1% of your health for each enemy it strikes");
    has("horsemans_scythe", "hits up to 5 enemies");
    has("loksey_boar_spear", "Slows the enemy by 40% for 2.5 sec");
    has("hogger_club", "knock the enemy back 5 yd");
    has("cookie_tenderizer", "tenderize the enemy for 6 sec. The next hit on it from another ally deals 25% more");
    has("gilnid_wrench", "stun mechanical enemies and machines for 4 sec");
    has("hand_of_righteousness", "most wounded ally within 15 yd, you included, for 3% of their health");
    has("barons_scepter", "curse the enemy for 12 sec");
    has("barons_scepter", "bursts in shadow for 44 damage to enemies within 5 yd");
    has("ironspine_fist", "absorbs 5 damage for 8 sec, stacking up to 30");
    has("hypnotic_blade", "lose interest in you for 4 sec");
    has("meteor_shard", "burst in fire for 50% of their damage on enemies within 4 yd");
    has("meteor_shard", "shatter a frozen enemy's ice");
    has("bazil_shiv", "poison the enemy for 3 damage a sec, stacking up to 5 times, for 6 sec");
    has("silent_blade", "Deals 75% more damage to an enemy recovering from its blow");
    has("thornspike", "interrupt an enemy that is casting and stagger it for 1.5 sec");
    has("cobrahn_fang", "put the enemy to sleep for 3 sec");
    has("cobrahn_fang", "can't be put to sleep again for 6 sec after it wakes.");
    has("champions_greatsword", "rally allies within 8 yd, you included: they take 15% less damage for 5 sec");
    has("thornroot_maul", "root the enemies they hit for 2.5 sec");
    has("mograines_might", "holy shockwave out to 6 yd that deals 60 damage");
    has("smite_hammer", "3rd hit in a row on the same enemy within 3 sec staggers it for 1.5 sec");
    has("rhahkzor_maul", "rocks down on the enemy's spot after 1 sec, dealing 35 damage to enemies within 2.5 yd");
    has("targorr_axe", "up to 80% more damage as your health falls");
    has("corpsemaker", "Cows the enemy for 4 sec: it deals 25% less damage and winds up slower");
    has("guillotine_axe", "stunned, asleep, rooted, frozen or staggered enemy deal 300% damage");
}


