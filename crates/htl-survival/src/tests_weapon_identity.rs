//! Each weapon's identity (docs/weapon-identities.md): what sets it apart in a fight.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::{Foe, MobState};
use crate::sim::*;

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

/// A creature 2 yd in front of the hero that stands still and never strikes back.
fn foe(s: &mut Survival, key: &str) -> Id {
    let def = s.content.mob(key).unwrap();
    let id = s.spawn_mob(def, [2.0, 0.0, 0.0], None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1.0e9 };
    m.facing = std::f32::consts::PI;
    m.ready_at = 1.0e9;
    id
}

/// One light swing's events, and a moment after.
fn swing(s: &mut Survival, p: Id) -> Vec<Event> {
    s.move_player(p, [0.0, 0.0, 0.0], 0.0, Moving::default());
    s.input(p, Input::Attack);
    let mut ev = Vec::new();
    for _ in 0..40 {
        ev.extend(s.step(0.02));
    }
    ev
}

fn hits_on(ev: &[Event], m: Id) -> Vec<(u32, Outcome)> {
    ev.iter()
        .filter_map(|e| match e {
            Event::Swing { victim: Some(v), damage, outcome, style, .. } if *v == m && !matches!(style, Blow::Dot) => Some((*damage, *outcome)),
            _ => None,
        })
        .collect()
}

#[test]
fn every_weapon_but_a_staff_has_an_identity() {
    let c = Content::dev();
    for w in c.weapons.iter().filter(|w| !w.guard.contains("ward") && !w.key.ends_with("staff")) {
        let has = w.parry > 0.0 || w.riposte > 0.0 || w.bleed > 0.0 || w.posture > 0.0 || w.backstab > 0.0 || w.through_guard || w.swing_poise || w.execute > 0.0 || w.stagger > 0.0;
        assert!(has, "{} has nothing of its own", w.key);
    }
}

#[test]
fn an_axes_hit_bleeds() {
    let (mut s, p) = world();
    wield(&mut s, p, "axe");
    // (A foe with no shield to take it on.)
    let m = foe(&mut s, "dm_woodcutter");
    let ev = swing(&mut s, p);
    assert!(hits_on(&ev, m).iter().any(|(d, _)| *d > 0), "it hit");
    let after = s.mobs[&m].health;
    let mut ticks = 0;
    for _ in 0..150 {
        for e in s.step(0.02) {
            if matches!(e, Event::Swing { victim: Some(v), style: Blow::Dot, .. } if v == m) {
                ticks += 1;
            }
        }
    }
    assert!(ticks >= 2, "it bled: {ticks} ticks");
    assert!(s.mobs[&m].health < after, "and lost health to it");
}

#[test]
fn a_spear_goes_through_a_raised_shield_a_sword_doesnt() {
    let (mut s, p) = world();
    // A shield-bearer that blocks every light blow it faces.
    let pirate = s.content.mob("dm_pirate").unwrap();
    assert!(s.content.mobs[pirate].shield != 0, "the pirate carries a shield");
    s.content.mobs[pirate].block = 1.0;
    wield(&mut s, p, "sword");
    let m = foe(&mut s, "dm_pirate");
    // Between its blows, shield to the hero, standing where it is.
    let hold = |s: &mut Survival| {
        let x = s.mobs.get_mut(&m).unwrap();
        x.pos = [2.0, 0.0, 0.0];
        x.facing = std::f32::consts::PI;
        x.state = MobState::Recover { foe: Foe::Hero(p), until: 1.0e9 };
    };
    hold(&mut s);
    let ev = swing(&mut s, p);
    assert!(hits_on(&ev, m).iter().all(|(_, o)| *o == Outcome::Blocked), "the sword is blocked: {:?}", hits_on(&ev, m));
    for _ in 0..50 {
        let _ = s.step(0.02);
    }
    wield(&mut s, p, "zanjin_spear");
    hold(&mut s);
    let ev = swing(&mut s, p);
    assert!(hits_on(&ev, m).iter().any(|(d, o)| *o == Outcome::Hit && *d > 0), "the spear goes through: {:?}", hits_on(&ev, m));
}

#[test]
fn the_executioners_axe_hits_twice_as_hard_on_a_foe_near_its_end() {
    let (mut s, p) = world();
    wield(&mut s, p, "warlord_axe");
    let m = foe(&mut s, "dm_woodcutter");
    let full = hits_on(&swing(&mut s, p), m).iter().map(|h| h.0).max().unwrap_or(0);
    for _ in 0..60 {
        let _ = s.step(0.02);
    }
    let max = s.mobs[&m].max_health;
    s.mobs.get_mut(&m).unwrap().health = max / 4;
    s.mobs.get_mut(&m).unwrap().max_health = 100_000.max(max);
    // (A share kept under a third whatever its health was.)
    let low = s.mobs[&m].max_health / 5;
    s.mobs.get_mut(&m).unwrap().health = low;
    let near = hits_on(&swing(&mut s, p), m).iter().map(|h| h.0).max().unwrap_or(0);
    let w = s.content.weapon("warlord_axe").unwrap();
    assert!(near as f32 >= w.damage[0] as f32 * 2.0 - 1.0, "near its end: {near} (fresh {full})");
}

// --- the off hand -----------------------------------------------------------------------------------

#[test]
fn with_two_blades_the_right_click_swings_the_off_hand_instead_of_guarding() {
    let (mut s, p) = world();
    wield(&mut s, p, "sword");
    s.players.get_mut(&p).unwrap().off_weapon = s.content.item("axe");
    let m = foe(&mut s, "dm_woodcutter");
    s.move_player(p, [0.0, 0.0, 0.0], 0.0, Moving::default());
    s.input(p, Input::Guard);
    assert!(matches!(&s.players[&p].action, Action::Swing { strike, .. } if strike.kind == StrikeKind::Off), "an off-hand blow: {:?}", s.players[&p].action);
    let mut ev = Vec::new();
    for _ in 0..40 {
        ev.extend(s.step(0.02));
    }
    assert!(hits_on(&ev, m).iter().any(|(d, o)| *o == Outcome::Hit && *d > 0), "it lands");
    assert!(!matches!(s.players[&p].action, Action::Guard), "and no guard comes up");
}

#[test]
fn left_and_right_alternating_four_times_ends_in_a_finisher() {
    let (mut s, p) = world();
    wield(&mut s, p, "sword");
    s.players.get_mut(&p).unwrap().off_weapon = s.content.item("sword");
    let m = foe(&mut s, "dm_woodcutter");
    s.mobs.get_mut(&m).unwrap().health = 100_000;
    s.mobs.get_mut(&m).unwrap().max_health = 100_000;
    let mut damage = Vec::new();
    for k in 0..4 {
        s.players.get_mut(&p).unwrap().stamina = 100.0;
        s.move_player(p, [0.0, 0.0, 0.0], 0.0, Moving::default());
        s.input(p, if k % 2 == 0 { Input::Attack } else { Input::Guard });
        let mut ev = Vec::new();
        // Each blow pressed again late in the last one's recovery, as a run is.
        for _ in 0..60 {
            ev.extend(s.step(0.02));
            if matches!(&s.players[&p].action, Action::Swing { t, strike, .. } if *t >= strike.windup + strike.active + strike.recovery * 0.75) {
                break;
            }
        }
        damage.push(hits_on(&ev, m).iter().map(|h| h.0).max().unwrap_or(0));
    }
    let w = s.content.weapon("sword").unwrap();
    assert!(damage[..3].iter().all(|d| *d > 0), "each blow lands: {damage:?}");
    assert!(damage[3] as f32 > w.damage[1] as f32 * 1.5, "the fourth, both blades, hits far harder: {damage:?}");
}

#[test]
fn a_weapon_alone_guards_with_its_blade_and_parries_wider() {
    let (mut s, p) = world();
    wield(&mut s, p, "sword");
    let shielded = s.guard_of(&s.players[&p]).key.clone();
    s.players.get_mut(&p).unwrap().off_empty = true;
    assert_eq!(s.guard_of(&s.players[&p]).key, crate::sim::ONE_HAND_GUARD, "its own blade (was {shielded})");
    assert!(s.one_hand_alone(&s.players[&p]));
}

#[test]
fn a_save_keeps_the_off_hand() {
    let (mut s, p) = world();
    wield(&mut s, p, "sword");
    s.players.get_mut(&p).unwrap().off_weapon = s.content.item("axe");
    let saved = crate::save::save_player(&s.content, &s.players[&p]);
    assert_eq!(saved.off_hand, "axe");
    let (mut t, _) = world();
    let q = t.join("bren", "Bren", Some(&saved));
    assert_eq!(t.players[&q].off_weapon, t.content.item("axe"), "the axe is back in the off hand");
}

#[test]
fn a_save_holding_a_retired_weapon_loads_the_familys_basic_one() {
    let c = Content::dev();
    for gone in ["apprentice_staff", "squall_staff", "goblin_axe", "goblin_wrench"] {
        assert!(c.item(gone).is_none() && c.weapon(gone).is_none(), "{gone} is gone");
    }
    let (mut s, p) = world();
    let mut save = s.leave(p).unwrap();
    save.weapon = "goblin_axe".into();
    save.off_hand = "goblin_wrench".into();
    save.bag.insert("squall_staff".into(), 1);
    save.bag.insert("apprentice_staff_honed2".into(), 1);
    let q = s.join("ayla", "Ayla", Some(&save));
    let h = &s.players[&q];
    let key = |i: usize| s.content.items[i].key.clone();
    assert_eq!(key(h.weapon), "axe");
    assert_eq!(h.off_weapon.map(key).as_deref(), Some("mace"));
    let bag: Vec<String> = h.bag.keys().map(|i| key(*i)).collect();
    assert!(bag.contains(&"halberd".to_string()) && bag.contains(&"ember_staff".to_string()), "{bag:?}");
}

/// The families that play their own set (the item subclass of each, the set it swings).
const FAMILY_SETS: [(u32, bool, &str, &str); 4] = [
    (0, false, "axe_one_hand", "one_hand"),
    (4, false, "mace_one_hand", "one_hand"),
    (1, true, "axe_two_hand", "two_hand"),
    (5, true, "maul", "two_hand"),
];

#[test]
fn each_family_resolves_to_its_own_set_and_the_rest_keep_the_grips() {
    let c = Content::dev();
    for w in &c.weapons {
        let want = FAMILY_SETS.iter().find(|(sub, two, _, _)| w.subclass == *sub && w.two_handed == *two && w.bolt.is_empty());
        match want {
            Some((_, _, own, _)) => assert_eq!(w.moveset_key(), *own, "{} swings its family's set", w.key),
            // (Swords, daggers, greatswords, polearms, staffs.)
            None => assert!(["one_hand", "two_hand", "dagger", "polearm"].contains(&w.moveset_key()), "{} keeps its grip's set: {}", w.key, w.moveset_key()),
        }
    }
    let keys: Vec<&str> = FAMILY_SETS.iter().map(|f| f.2).collect();
    for k in &keys {
        assert_eq!(keys.iter().filter(|x| *x == k).count(), 1);
        let set = c.movesets.iter().find(|m| m.key == *k).unwrap_or_else(|| panic!("no set {k}"));
        // (Each plays its own timing, not the grip's copied.)
        for g in ["one_hand", "two_hand", "dagger", "polearm"] {
            let grip = c.movesets.iter().find(|m| m.key == g).unwrap();
            let same = set.light.len() == grip.light.len() && set.light.iter().zip(&grip.light).all(|(a, b)| (a.clip, a.take, a.windup, a.recovery, a.damage) == (b.clip, b.take, b.windup, b.recovery, b.damage));
            assert!(!same, "{k} is not {g}");
        }
    }
}

/// A weapon's light chain: damage a second over the whole chain with this set's steps.
fn chain_dps(w: &crate::content::WeaponDef, set: &crate::content::MovesetDef) -> f32 {
    let mid = (w.damage[0] + w.damage[1]) as f32 / 2.0;
    let (dmg, secs) = set.light.iter().fold((0.0, 0.0), |(d, t), s| (d + mid * s.damage, t + w.windup * s.windup + w.active + w.recovery * s.recovery));
    dmg / secs
}

#[test]
fn a_family_set_keeps_the_familys_damage_a_second() {
    let c = Content::dev();
    for (sub, two, own, grip) in FAMILY_SETS {
        let (own, grip) = (c.movesets.iter().find(|m| m.key == own).unwrap(), c.movesets.iter().find(|m| m.key == grip).unwrap());
        for w in c.weapons.iter().filter(|w| w.subclass == sub && w.two_handed == two && w.bolt.is_empty()) {
            let ratio = chain_dps(w, own) / chain_dps(w, grip);
            assert!((0.95..=1.05).contains(&ratio), "{} does {ratio:.2} times the damage a second it did", w.key);
        }
    }
}

// --- swords: the pass's first batch ----------------------------------------------------------------

fn first_hit(ev: &[Event], m: Id) -> u32 {
    hits_on(ev, m).into_iter().find(|(d, o)| *d > 0 && *o == Outcome::Hit).map_or(0, |(d, _)| d)
}

#[test]
fn silver_hits_undead_and_worgen_half_again_as_hard_and_nothing_else() {
    for (foe_key, bane) in [("risen_skeleton", true), ("sf_worg", true), ("dm_woodcutter", false)] {
        let (mut s, p) = world();
        wield(&mut s, p, "silver_sword");
        let m = foe(&mut s, foe_key);
        s.mobs.get_mut(&m).unwrap().health = 1_000_000;
        s.mobs.get_mut(&m).unwrap().max_health = 1_000_000;
        let d = first_hit(&swing(&mut s, p), m);
        let (lo, hi) = if bane { (28, 38) } else { (19, 25) };
        assert!((lo..=hi).contains(&d), "{foe_key}: {d}");
    }
}

#[test]
fn silver_leaves_a_foe_that_doesnt_mend_and_a_kill_that_stays_down() {
    let (mut s, p) = world();
    wield(&mut s, p, "silver_sword");
    let m = foe(&mut s, "risen_skeleton");
    s.mobs.get_mut(&m).unwrap().health = 1_000_000;
    s.mobs.get_mut(&m).unwrap().max_health = 2_000_000;
    assert!(!s.silvered(m));
    swing(&mut s, p);
    assert!(s.silvered(m), "silvered by the blow");
    // Sent home hurt: a silvered foe comes home as it left.
    let hurt = s.mobs[&m].health;
    {
        let x = s.mobs.get_mut(&m).unwrap();
        x.state = MobState::Return;
        x.pos = x.home;
    }
    for _ in 0..5 {
        s.step(0.05);
    }
    assert_eq!(s.mobs[&m].health, hurt, "it doesn't mend");
    // The same on a foe a plain sword struck: whole again.
    let (mut s, p) = world();
    wield(&mut s, p, "sword");
    let m = foe(&mut s, "risen_skeleton");
    s.mobs.get_mut(&m).unwrap().health = 1_000_000;
    s.mobs.get_mut(&m).unwrap().max_health = 2_000_000;
    swing(&mut s, p);
    assert!(!s.silvered(m));
    {
        let x = s.mobs.get_mut(&m).unwrap();
        x.state = MobState::Return;
        x.pos = x.home;
    }
    for _ in 0..5 {
        s.step(0.05);
    }
    assert_eq!(s.mobs[&m].health, 2_000_000, "an ordinary foe mends");
}

#[test]
fn the_barbs_heavy_hooks_the_foe_in_and_the_swords_doesnt() {
    let (hit, _, near) = heavy_blow_at("cruel_barb", "dm_woodcutter", 3.0);
    assert!(hit && near < 2.0, "the heavy hooked it in: {near}");
    let (hit, _, near) = heavy_blow_at("sword", "dm_woodcutter", 3.0);
    assert!(hit && near > 2.4, "a sword's heavy leaves it where it is: {near}");
}

#[test]
fn the_cutlass_heavy_leaps_the_gap_and_the_swords_doesnt() {
    let (mut s, p) = world();
    wield(&mut s, p, "pirate_cutlass");
    let cutlass = s.strike_at(p, StrikeKind::Heavy, 0).lunge;
    wield(&mut s, p, "sword");
    let sword = s.strike_at(p, StrikeKind::Heavy, 0).lunge;
    assert_eq!(cutlass, s.content.weapon("pirate_cutlass").unwrap().boarding);
    assert!(cutlass >= 4.0 * sword, "{cutlass} against {sword}");
}

#[test]
fn the_machete_chops_every_foe_in_a_line_and_stabs_nobody_in_the_back() {
    let (mut s, p) = world();
    wield(&mut s, p, "sneed_machete");
    let near = foe(&mut s, "dm_woodcutter");
    let far = foe(&mut s, "dm_woodcutter");
    let aside = foe(&mut s, "dm_woodcutter");
    s.mobs.get_mut(&far).unwrap().pos = [4.6, 0.0, 0.0];
    s.mobs.get_mut(&aside).unwrap().pos = [2.0, 3.5, 0.0];
    for m in [near, far, aside] {
        s.mobs.get_mut(&m).unwrap().health = 1_000_000;
        s.mobs.get_mut(&m).unwrap().max_health = 1_000_000;
    }
    let ev = swing(&mut s, p);
    assert!(first_hit(&ev, near) > 0 && first_hit(&ev, far) > 0, "the whole line is chopped");
    assert_eq!(first_hit(&ev, aside), 0, "what's off to the side isn't");
    // From behind: no more than the front (the sword's 2.5 on a stabbing blade).
    let w = s.content.weapon("sneed_machete").unwrap();
    assert!(w.no_backstab && w.backstab == 0.0);
    let (mut s, p) = world();
    wield(&mut s, p, "sneed_machete");
    let m = foe(&mut s, "dm_woodcutter");
    s.mobs.get_mut(&m).unwrap().facing = 0.0; // looking away
    let d = first_hit(&swing(&mut s, p), m);
    assert!((18..=25).contains(&d), "from behind it's an ordinary blow: {d}");
}

#[test]
fn the_felblades_last_blow_carries_the_hero_behind_the_foe() {
    let (mut s, p) = world();
    wield(&mut s, p, "cursed_felblade");
    let m = foe(&mut s, "dm_woodcutter");
    s.mobs.get_mut(&m).unwrap().health = 1_000_000;
    s.mobs.get_mut(&m).unwrap().max_health = 1_000_000;
    // Two blows of the chain stay where they stand; the third steps round.
    for step in 0..3 {
        let late = {
            let k = s.strike_at(p, StrikeKind::Light, step);
            k.windup + k.active + k.recovery * 0.5
        };
        s.input(p, Input::Attack);
        for _ in 0..((late / 0.02) as usize + 1) {
            s.step(0.02);
        }
        let x = s.players[&p].pos[0];
        if step < 2 {
            assert!(x < 1.5, "blow {step}: still in front ({x})");
        }
    }
    let x = s.players[&p].pos[0];
    assert!(x > s.mobs[&m].pos[0], "the third carried the hero past the foe: hero {x}, foe {:?}", s.mobs[&m].pos);
}

#[test]
fn a_swords_tooltip_says_its_mechanic_from_its_numbers() {
    let c = Content::dev();
    let text = |k: &str| c.items[c.item(k).unwrap()].text.clone();
    let silver = c.weapon("silver_sword").unwrap();
    assert!(text("silver_sword").contains(&format!("Deals {}% more damage to undead and worgen.", (silver.bane_damage * 100.0).round())), "{}", text("silver_sword"));
    assert!(text("silver_sword").contains(&format!("silvered for {} sec", crate::content::SILVER_SECS)));
    assert!(text("cruel_barb").contains(&format!("pull it up to {} yd", c.weapon("cruel_barb").unwrap().hook)));
    assert!(text("pirate_cutlass").contains(&format!("leap {} yd", c.weapon("pirate_cutlass").unwrap().boarding)));
    assert!(text("sneed_machete").contains(&format!("a line {} yd long", c.weapon("sneed_machete").unwrap().line)));
    assert!(text("cursed_felblade").contains("carries you behind the enemy"));
    for k in ["silver_sword", "cruel_barb", "pirate_cutlass", "sneed_machete", "cursed_felblade"] {
        assert!(!text(k).contains("{fx}"), "{k}: the mark is filled");
    }
}

// --- axes and polearms: the pass's second batch ------------------------------------------------------

fn sturdy_foe(s: &mut Survival, key: &str) -> Id {
    let m = foe(s, key);
    s.mobs.get_mut(&m).unwrap().health = 1_000_000;
    s.mobs.get_mut(&m).unwrap().max_health = 1_000_000;
    m
}

#[test]
fn the_bearded_axes_heavy_hooks_a_guard_aside_for_a_while() {
    let (mut s, p) = world();
    let pirate = s.content.mob("dm_pirate").unwrap();
    s.content.mobs[pirate].block = 1.0;
    wield(&mut s, p, "bearded_axe");
    let m = sturdy_foe(&mut s, "dm_pirate");
    let hold = |s: &mut Survival| {
        let x = s.mobs.get_mut(&m).unwrap();
        x.pos = [2.0, 0.0, 0.0];
        x.facing = std::f32::consts::PI;
        x.state = MobState::Recover { foe: Foe::Hero(p), until: 1.0e9 };
    };
    hold(&mut s);
    let ev = swing(&mut s, p);
    assert!(hits_on(&ev, m).iter().all(|(_, o)| *o == Outcome::Blocked), "the shield takes a light blow: {:?}", hits_on(&ev, m));
    for _ in 0..80 {
        s.step(0.02);
    }
    hold(&mut s);
    s.input(p, Input::Heavy);
    let mut heavy = Vec::new();
    for _ in 0..90 {
        heavy.extend(s.step(0.02));
    }
    assert!(hits_on(&heavy, m).iter().any(|(d, _)| *d > 0), "the heavy lands");
    for _ in 0..40 {
        s.step(0.02);
    }
    hold(&mut s);
    let ev = swing(&mut s, p);
    assert!(hits_on(&ev, m).iter().any(|(d, o)| *d > 0 && *o == Outcome::Hit), "the guard stays aside: {:?}", hits_on(&ev, m));
}

#[test]
fn each_kill_with_the_bloodlust_edge_quickens_the_swing_and_it_fades() {
    let (mut s, p) = world();
    wield(&mut s, p, "bloodlust_edge");
    let base = s.strike_at(p, StrikeKind::Light, 0).windup;
    let lust = s.content.weapon("bloodlust_edge").unwrap().bloodlust;
    let kill = |s: &mut Survival| {
        let m = foe(s, "dm_woodcutter");
        s.hurt_mob(m, p, 1_000_000);
    };
    kill(&mut s);
    let one = s.strike_at(p, StrikeKind::Light, 0).windup;
    assert!((one - base / (1.0 + lust)).abs() < 1.0e-4, "one kill: {one} of {base}");
    for _ in 0..6 {
        kill(&mut s);
    }
    let many = s.strike_at(p, StrikeKind::Light, 0).windup;
    let cap = crate::content::BLOODLUST_STACKS as f32;
    assert!((many - base / (1.0 + lust * cap)).abs() < 1.0e-4, "it stacks to {cap} and no more: {many}");
    for _ in 0..((crate::content::BLOODLUST_SECS + 1.0) / 0.05) as usize {
        s.step(0.05);
    }
    assert!((s.strike_at(p, StrikeKind::Light, 0).windup - base).abs() < 1.0e-4, "it fades");
    // Another weapon builds nothing.
    wield(&mut s, p, "sword");
    let plain = s.strike_at(p, StrikeKind::Light, 0).windup;
    kill(&mut s);
    assert_eq!(s.strike_at(p, StrikeKind::Light, 0).windup, plain);
}

#[test]
fn the_miners_pick_breaks_stances_and_hits_stone_and_machines_harder() {
    let w = Content::dev().weapon("miner_pick").unwrap().clone();
    assert!(w.posture >= 3.0, "a posture breaker");
    for (key, bane) in [("rockling", true), ("dm_shredder", true), ("dm_woodcutter", false)] {
        let (mut s, p) = world();
        wield(&mut s, p, "miner_pick");
        let m = sturdy_foe(&mut s, key);
        let d = first_hit(&swing(&mut s, p), m);
        let (lo, hi) = if bane { (33, 44) } else { (22, 29) };
        assert!((lo..=hi).contains(&d), "{key}: {d}");
    }
}

#[test]
fn the_scythes_sweep_heals_the_reaper_for_each_foe_it_strikes() {
    let (mut s, p) = world();
    wield(&mut s, p, "horsemans_scythe");
    let foes: Vec<Id> = (0..3).map(|_| sturdy_foe(&mut s, "dm_woodcutter")).collect();
    s.mobs.get_mut(&foes[1]).unwrap().pos = [1.5, 1.5, 0.0];
    s.mobs.get_mut(&foes[2]).unwrap().pos = [1.5, -1.5, 0.0];
    let max = s.players[&p].max_health;
    s.players.get_mut(&p).unwrap().health = max / 2;
    let ev = swing(&mut s, p);
    let struck = foes.iter().filter(|m| hits_on(&ev, **m).iter().any(|(d, _)| *d > 0)).count();
    assert_eq!(struck, 3, "all three are in the sweep");
    let w = s.content.weapon("horsemans_scythe").unwrap();
    let each = (max as f32 * w.reap_heal).round() as u32;
    assert_eq!(s.players[&p].health, max / 2 + 3 * each, "{each} a foe");
}

#[test]
fn the_boar_spear_slows_what_it_hits() {
    let (mut s, p) = world();
    wield(&mut s, p, "loksey_boar_spear");
    let m = sturdy_foe(&mut s, "dm_woodcutter");
    s.mobs.get_mut(&m).unwrap().pos = [4.0, 0.0, 0.0];
    assert!(!s.has_cc(m, "snare"));
    swing(&mut s, p);
    assert!(s.has_cc(m, "snare"), "slowed");
    let keep = 1.0 - s.content.weapon("loksey_boar_spear").unwrap().snare;
    assert!((s.speed_share(m) - keep).abs() < 1.0e-4, "{}", s.speed_share(m));
}

#[test]
fn an_axes_and_polearms_tooltips_say_their_mechanics_from_their_numbers() {
    let c = Content::dev();
    let text = |k: &str| c.items[c.item(k).unwrap()].text.clone();
    let w = |k: &str| c.weapon(k).unwrap();
    assert!(text("bearded_axe").contains(&format!("can't block for {} sec", w("bearded_axe").guard_hook)), "{}", text("bearded_axe"));
    let l = w("bloodlust_edge");
    assert!(text("bloodlust_edge").contains(&format!("speeds your swings by {}% for {} sec, up to {} times", (l.bloodlust * 100.0).round(), crate::content::BLOODLUST_SECS, crate::content::BLOODLUST_STACKS)));
    let m = w("miner_pick");
    assert!(text("miner_pick").contains(&format!("Deals {}% more damage to stone, golem and mechanical", (m.bane_damage * 100.0).round())), "{}", text("miner_pick"));
    assert!(text("miner_pick").contains(&format!("Breaks stances {}% faster", ((m.posture - 1.0) * 100.0).round())));
    assert!(text("horsemans_scythe").contains(&format!("Restores {}% of your health for each enemy", (w("horsemans_scythe").reap_heal * 100.0).round())));
    assert!(text("loksey_boar_spear").contains(&format!("Slows the enemy by {}% for {} sec", (w("loksey_boar_spear").snare * 100.0).round(), crate::content::SNARE_SECS)));
}

#[test]
fn the_blow_that_fells_a_foe_under_silver_lays_it_to_rest_too() {
    let (mut s, p) = world();
    wield(&mut s, p, "silver_sword");
    let m = foe(&mut s, "risen_skeleton");
    s.mobs.get_mut(&m).unwrap().health = 1;
    assert!(!s.silvered(m));
    let ev = swing(&mut s, p);
    assert!(hits_on(&ev, m).iter().any(|(d, _)| *d > 0), "the blow lands");
    assert!(s.mobs[&m].health == 0 && s.silvered(m), "felled by the first blow, and silvered by it");
}

// --- maces: the pass's third batch --------------------------------------------------------------------

/// The foe back where it was put, standing, facing the hero (a blow moves it a little).
fn reset_foe(s: &mut Survival, m: Id) {
    let x = s.mobs.get_mut(&m).unwrap();
    x.pos = [2.0, 0.0, 0.0];
    x.facing = std::f32::consts::PI;
    x.state = MobState::Idle { wander_at: 1.0e9 };
    x.stagger_until = 0.0;
}

fn heavy_swing(s: &mut Survival, p: Id) -> Vec<Event> {
    s.input(p, Input::Heavy);
    let mut ev = Vec::new();
    for _ in 0..90 {
        ev.extend(s.step(0.02));
    }
    ev
}

/// A fresh world, `weapon` in hand, a still foe `at` yd ahead, a heavy blow at it: whether it
/// landed, and the farthest and nearest the foe got to the hero after it did.
fn heavy_blow_at(weapon: &str, foe_key: &str, at: f32) -> (bool, f32, f32) {
    let (mut s, p) = world();
    wield(&mut s, p, weapon);
    let m = sturdy_foe(&mut s, foe_key);
    s.mobs.get_mut(&m).unwrap().pos = [at, 0.0, 0.0];
    s.input(p, Input::Heavy);
    let (mut hit, mut far, mut near) = (false, 0.0f32, f32::MAX);
    for _ in 0..120 {
        let ev = s.step(0.02);
        hit |= hits_on(&ev, m).iter().any(|(d, _)| *d > 0);
        if hit {
            let d = dist2d(s.mobs[&m].pos, [0.0, 0.0, 0.0]);
            far = far.max(d);
            near = near.min(d);
        }
    }
    (hit, far, near)
}

#[test]
fn the_clubs_heavy_knocks_a_foe_back_and_the_maces_doesnt() {
    let (hit, far, _) = heavy_blow_at("hogger_club", "dm_woodcutter", 2.0);
    assert!(hit && far > 4.5, "the club's heavy sends it back: {far}");
    let (hit, far, _) = heavy_blow_at("mace", "dm_woodcutter", 2.0);
    assert!(hit && far < 3.6, "the mace's doesn't: {far}");
}

#[test]
fn a_tenderized_foe_takes_more_from_the_next_hit_of_another_hero_once() {
    let (mut s, p) = world();
    let q = s.join("bren", "Bren", None);
    s.move_player(q, [0.0, 1.0, 0.0], 0.0, Moving::default());
    wield(&mut s, p, "cookie_tenderizer");
    wield(&mut s, q, "sword");
    let m = sturdy_foe(&mut s, "dm_woodcutter");
    let plain = first_hit(&swing(&mut s, q), m);
    assert!((14..=19).contains(&plain), "an untouched foe: {plain}");
    for _ in 0..60 {
        s.step(0.02);
    }
    // Now the mallet tenderizes it, and the sword's next hit is 25% harder.
    reset_foe(&mut s, m);
    swing(&mut s, p);
    assert!(s.mobs[&m].fx.tender.1 > s.clock);
    // The wielder's own next blow doesn't use it up.
    reset_foe(&mut s, m);
    let own = first_hit(&swing(&mut s, p), m);
    assert!(own <= 34, "the wielder's own blow is untouched: {own}");
    assert!(s.mobs[&m].fx.tender.0 > 0.0, "still tender");
    reset_foe(&mut s, m);
    let tender = first_hit(&swing(&mut s, q), m);
    assert!((17..=24).contains(&tender), "the ally's hit does a quarter more: {tender}");
    assert_eq!(s.mobs[&m].fx.tender.0, 0.0, "used up");
    reset_foe(&mut s, m);
    let after = first_hit(&swing(&mut s, q), m);
    // (The sword's third blow of the chain, the finisher, is 35% harder: what's read is no more
    // than that, not a quarter on top of it.)
    assert!((14..=26).contains(&after), "and gone: {after}");
}

#[test]
fn the_wrenchs_heavy_stuns_a_machine_a_long_while_and_a_man_not_at_all() {
    for (tag, stunned) in [(true, true), (false, false)] {
        let (mut s, p) = world();
        let def = s.content.mob("dm_woodcutter").unwrap();
        if tag {
            s.content.mobs[def].tags.push("mechanical".into());
        }
        wield(&mut s, p, "gilnid_wrench");
        let m = sturdy_foe(&mut s, "dm_woodcutter");
        heavy_swing(&mut s, p);
        let _ = m;
        assert_eq!(s.has_cc(m, "stun"), stunned, "tagged mechanical: {tag}");
    }
    // (A champion machine, the Shredder, holds its ground against it: the stall stays its own.)
    assert!(Content::dev().mobs[Content::dev().mob("dm_shredder").unwrap()].tags.iter().any(|t| t == "mechanical"));
}

#[test]
fn the_hands_hits_heal_the_most_wounded_hero_nearby() {
    let (mut s, p) = world();
    let q = s.join("bren", "Bren", None);
    s.move_player(q, [0.0, 6.0, 0.0], 0.0, Moving::default());
    let r = s.join("cal", "Cal", None);
    s.move_player(r, [0.0, 40.0, 0.0], 0.0, Moving::default());
    wield(&mut s, p, "hand_of_righteousness");
    let m = sturdy_foe(&mut s, "dm_woodcutter");
    let max = s.players[&q].max_health;
    s.players.get_mut(&p).unwrap().health = max - 20;
    s.players.get_mut(&q).unwrap().health = max / 2;
    s.players.get_mut(&r).unwrap().health = 1;
    let ev = swing(&mut s, p);
    assert!(first_hit(&ev, m) > 0);
    let each = (max as f32 * s.content.weapon("hand_of_righteousness").unwrap().mend_ally).round() as u32;
    // (A little comes back by itself in the swing's second.)
    assert!((max / 2 + each..max / 2 + each + 8).contains(&s.players[&q].health), "the most wounded within reach: {}", s.players[&q].health);
    assert!(s.players[&p].health < max - 20 + 8, "not the wielder, who is hurt less: {}", s.players[&p].health);
    assert!(s.players[&r].health < 9, "not the one out of reach: {}", s.players[&r].health);
}

#[test]
fn a_foe_cursed_by_the_scepter_bursts_in_shadow_when_it_dies() {
    let (mut s, p) = world();
    wield(&mut s, p, "barons_scepter");
    let m = sturdy_foe(&mut s, "dm_woodcutter");
    let near = sturdy_foe(&mut s, "dm_woodcutter");
    let far = sturdy_foe(&mut s, "dm_woodcutter");
    s.mobs.get_mut(&near).unwrap().pos = [3.0, 3.0, 0.0];
    s.mobs.get_mut(&far).unwrap().pos = [3.0, 12.0, 0.0];
    swing(&mut s, p);
    assert!(s.mobs[&m].fx.curse.is_some_and(|(u, _, _)| u > s.clock), "cursed");
    let (n0, f0) = (s.mobs[&near].health, s.mobs[&far].health);
    s.hurt_mob(m, p, 2_000_000);
    assert_eq!(s.mobs[&m].health, 0);
    let burst = ((25 + 33) as f32 / 2.0 * s.content.weapon("barons_scepter").unwrap().curse).round() as u32;
    assert_eq!(n0 - s.mobs[&near].health, burst, "what stood near took the burst");
    assert_eq!(s.mobs[&far].health, f0, "what stood far off didn't");
}

#[test]
fn the_fists_hits_build_a_shield_up_to_a_cap() {
    let (mut s, p) = world();
    wield(&mut s, p, "ironspine_fist");
    sturdy_foe(&mut s, "dm_woodcutter");
    let w = s.content.weapon("ironspine_fist").unwrap().clone();
    let shield = |s: &Survival| s.cc_of(p).iter().find(|c| c.kind == "shield" && c.until > s.clock).map_or(0.0, |c| c.left);
    assert_eq!(shield(&s), 0.0);
    swing(&mut s, p);
    assert_eq!(shield(&s), w.iron_skin);
    let m = *s.mobs.keys().next().unwrap();
    for _ in 0..12 {
        reset_foe(&mut s, m);
        swing(&mut s, p);
    }
    assert_eq!(shield(&s), crate::content::IRON_SKIN_CAP, "up to the cap, no more");
}

#[test]
fn the_maces_tooltips_say_their_mechanics_from_their_numbers() {
    let c = Content::dev();
    let text = |k: &str| c.items[c.item(k).unwrap()].text.clone();
    let w = |k: &str| c.weapon(k).unwrap();
    assert!(text("hogger_club").contains(&format!("knock the enemy back {} yd", w("hogger_club").knock)), "{}", text("hogger_club"));
    assert!(text("cookie_tenderizer").contains(&format!("tenderize the enemy for {} sec. The next hit on it from another ally deals {}% more", crate::content::TENDER_SECS, (w("cookie_tenderizer").tender * 100.0).round())), "{}", text("cookie_tenderizer"));
    assert!(text("gilnid_wrench").contains(&format!("stun mechanical enemies and machines for {} sec", w("gilnid_wrench").machine_stun)));
    assert!(text("hand_of_righteousness").contains(&format!("most wounded ally within {} yd, you included, for {}% of their health", crate::content::MEND_RANGE, (w("hand_of_righteousness").mend_ally * 100.0).round())));
    assert!(text("barons_scepter").contains(&format!("dies bursts in shadow for 44 damage to enemies within {} yd", crate::content::CURSE_RADIUS)), "{}", text("barons_scepter"));
    assert!(text("ironspine_fist").contains(&format!("absorbs {} damage for {} sec, stacking up to {}", w("ironspine_fist").iron_skin, crate::content::IRON_SKIN_SECS, crate::content::IRON_SKIN_CAP)));
}

// --- daggers: the pass's fourth batch -------------------------------------------------------------------

/// A foe with its back to the hero at the origin, and nothing to make it turn (it stands in its
/// recovery, as a foe between blows does).
fn turn_away(s: &mut Survival, m: Id) {
    let hero = *s.players.keys().next().unwrap();
    let x = s.mobs.get_mut(&m).unwrap();
    x.facing = 0.0;
    x.state = MobState::Recover { foe: Foe::Hero(hero), until: 1.0e9 };
}

#[test]
fn the_hypnotic_blade_makes_a_foe_lose_interest_and_look_elsewhere() {
    let (mut s, p) = world();
    wield(&mut s, p, "hypnotic_blade");
    let m = sturdy_foe(&mut s, "dm_woodcutter");
    swing(&mut s, p);
    assert!(matches!(s.mobs[&m].state, MobState::Idle { .. }), "it dropped the hero: {:?}", s.mobs[&m].state.foe());
    assert!(s.lulled(m, p), "and takes no notice of them for a while");
    // A plain dagger leaves it on the hero.
    let (mut s, p) = world();
    wield(&mut s, p, "dagger");
    let m = sturdy_foe(&mut s, "dm_woodcutter");
    swing(&mut s, p);
    assert!(!matches!(s.mobs[&m].state, MobState::Idle { .. }), "engaged");
    assert!(!s.lulled(m, p));
}

#[test]
fn a_shard_in_the_back_bursts_in_fire_round_the_target_and_shatters_ice() {
    let (mut s, p) = world();
    wield(&mut s, p, "meteor_shard");
    let m = sturdy_foe(&mut s, "dm_woodcutter");
    let near = sturdy_foe(&mut s, "dm_woodcutter");
    let far = sturdy_foe(&mut s, "dm_woodcutter");
    s.mobs.get_mut(&near).unwrap().pos = [3.0, 2.5, 0.0];
    s.mobs.get_mut(&far).unwrap().pos = [3.0, 9.0, 0.0];
    turn_away(&mut s, m);
    let (n0, f0) = (s.mobs[&near].health, s.mobs[&far].health);
    let ev = swing(&mut s, p);
    let hit = first_hit(&ev, m);
    assert!(hit >= 60, "from behind: {hit}");
    let burst = (hit as f32 * s.content.weapon("meteor_shard").unwrap().burst).round() as u32;
    assert_eq!(n0 - s.mobs[&near].health, burst, "the neighbour burns");
    assert_eq!(s.mobs[&far].health, f0, "the far one doesn't");
    // From the front: no burst.
    let (mut s, p) = world();
    wield(&mut s, p, "meteor_shard");
    let m = sturdy_foe(&mut s, "dm_woodcutter");
    let near = sturdy_foe(&mut s, "dm_woodcutter");
    s.mobs.get_mut(&near).unwrap().pos = [3.0, 2.5, 0.0];
    let n0 = s.mobs[&near].health;
    swing(&mut s, p);
    assert!(s.mobs[&m].health < 1_000_000);
    assert_eq!(s.mobs[&near].health, n0, "no burst from the front");
    // A frozen foe struck in the back: the ice shatters for SHATTER times the blow.
    let blow = |frozen: bool| {
        let (mut s, p) = world();
        wield(&mut s, p, "meteor_shard");
        let m = sturdy_foe(&mut s, "dm_woodcutter");
        turn_away(&mut s, m);
        if frozen {
            s.apply_cc(m, p, "petrify", 5.0, 0.0, 0.0, "Test freeze");
        }
        let ev = swing(&mut s, p);
        (first_hit(&ev, m), s.has_cc(m, "petrify"))
    };
    let (plain, _) = blow(false);
    let (shattered, still_frozen) = blow(true);
    assert!(!still_frozen, "the ice is gone");
    assert!(shattered as f32 >= plain as f32 * 1.2, "{shattered} against {plain}");
}

#[test]
fn the_shivs_poison_stacks_with_each_hit_up_to_a_cap() {
    let (mut s, p) = world();
    wield(&mut s, p, "bazil_shiv");
    let m = sturdy_foe(&mut s, "dm_woodcutter");
    let per = s.content.weapon("bazil_shiv").unwrap().poison;
    let poison = |s: &Survival| s.cc_of(m).iter().find(|c| c.kind == "poison" && c.until > s.clock).map_or(0.0, |c| c.amount);
    assert_eq!(poison(&s), 0.0);
    swing(&mut s, p);
    assert_eq!(poison(&s), per);
    reset_foe(&mut s, m);
    swing(&mut s, p);
    assert_eq!(poison(&s), 2.0 * per, "a second stack");
    for _ in 0..8 {
        reset_foe(&mut s, m);
        swing(&mut s, p);
    }
    assert_eq!(poison(&s), crate::content::POISON_STACKS as f32 * per, "up to the cap");
}

#[test]
fn the_thorn_breaks_a_cast_and_a_plain_dagger_doesnt() {
    for (weapon, broken) in [("thornspike", true), ("dagger", false)] {
        let (mut s, p) = world();
        wield(&mut s, p, weapon);
        let m = sturdy_foe(&mut s, "dm_woodcutter");
        // Mid-cast: winding up a ranged attack at the hero.
        let def = s.mobs[&m].def;
        s.content.mobs[def].attack.ranged = true;
        s.mobs.get_mut(&m).unwrap().ready_at = 0.0;
        s.engage(m, Foe::Hero(p));
        for _ in 0..80 {
            s.step(0.02);
            if matches!(s.mobs[&m].state, MobState::Windup { .. }) {
                break;
            }
        }
        assert!(matches!(s.mobs[&m].state, MobState::Windup { .. }), "casting");
        s.input(p, Input::Attack);
        for _ in 0..16 {
            s.step(0.02);
        }
        let staggered = s.mobs[&m].stagger_until > s.clock;
        assert_eq!(staggered, broken, "{weapon}");
    }
}

#[test]
fn the_fangs_blow_from_behind_puts_a_foe_to_sleep_and_damage_wakes_it() {
    let (mut s, p) = world();
    wield(&mut s, p, "cobrahn_fang");
    let m = sturdy_foe(&mut s, "dm_woodcutter");
    turn_away(&mut s, m);
    swing(&mut s, p);
    // (Its own damage first, then the sleep: the second blow wakes it.)
    assert!(s.has_cc(m, "incapacitate"), "asleep");
    s.hurt_mob(m, p, 5);
    assert!(!s.has_cc(m, "incapacitate"), "damage wakes it");
    // From the front it doesn't sleep.
    let (mut s, p) = world();
    wield(&mut s, p, "cobrahn_fang");
    let m = sturdy_foe(&mut s, "dm_woodcutter");
    swing(&mut s, p);
    assert!(!s.has_cc(m, "incapacitate"));
}

#[test]
fn the_daggers_tooltips_say_their_mechanics_from_their_numbers() {
    let c = Content::dev();
    let text = |k: &str| c.items[c.item(k).unwrap()].text.clone();
    let w = |k: &str| c.weapon(k).unwrap();
    assert!(text("hypnotic_blade").contains(&format!("lose interest in you for {} sec", crate::content::LULL_SECS)), "{}", text("hypnotic_blade"));
    let m = w("meteor_shard");
    assert!(text("meteor_shard").contains(&format!("burst in fire for {}% of their damage on enemies within {} yd", (m.burst * 100.0).round(), crate::content::BURST_RADIUS)), "{}", text("meteor_shard"));
    assert!(text("meteor_shard").contains("shatter a frozen enemy's ice"));
    let b = w("bazil_shiv");
    assert!(text("bazil_shiv").contains(&format!("poison the enemy for {} damage a sec, stacking up to {} times, for {} sec", b.poison, crate::content::POISON_STACKS, crate::content::POISON_SECS)), "{}", text("bazil_shiv"));
    assert!(text("thornspike").contains(&format!("stagger it for {} sec", w("thornspike").interrupt)));
    assert!(text("cobrahn_fang").contains(&format!("to sleep for {} sec", w("cobrahn_fang").sleep)));
    assert!(text("cobrahn_fang").contains(&format!("can't be put to sleep again for {} sec", crate::content::SLEEP_IMMUNE_SECS)), "{}", text("cobrahn_fang"));
    assert!(text("cobrahn_fang").contains(&format!("can't be put to sleep again for {} sec", crate::content::SLEEP_IMMUNE_SECS)), "{}", text("cobrahn_fang"));
}

// --- two-handers: the pass's fifth batch ---------------------------------------------------------------

/// One light swing of a slow weapon, and the moment after.
fn slow_swing(s: &mut Survival, p: Id) -> Vec<Event> {
    s.move_player(p, [0.0, 0.0, 0.0], 0.0, Moving::default());
    s.input(p, Input::Attack);
    let mut ev = Vec::new();
    for _ in 0..150 {
        ev.extend(s.step(0.02));
    }
    ev
}

#[test]
fn the_thornroot_mauls_heavy_roots_what_it_strikes() {
    for (weapon, rooted) in [("thornroot_maul", true), ("maul", false)] {
        let (mut s, p) = world();
        wield(&mut s, p, weapon);
        let m = sturdy_foe(&mut s, "dm_woodcutter");
        heavy_swing(&mut s, p);
        assert_eq!(s.has_cc(m, "root"), rooted, "{weapon}");
    }
}

#[test]
fn a_heavy_blow_of_mograines_hammer_sends_a_ring_round_the_hero() {
    let (mut s, p) = world();
    wield(&mut s, p, "mograines_might");
    let near = sturdy_foe(&mut s, "dm_woodcutter");
    let behind = sturdy_foe(&mut s, "dm_woodcutter");
    let far = sturdy_foe(&mut s, "dm_woodcutter");
    s.mobs.get_mut(&behind).unwrap().pos = [-4.0, 0.0, 0.0];
    s.mobs.get_mut(&far).unwrap().pos = [-60.0, 0.0, 0.0];
    let (b0, f0) = (s.mobs[&behind].health, s.mobs[&far].health);
    let _ = near;
    heavy_swing(&mut s, p);
    let w = s.content.weapon("mograines_might").unwrap();
    let ring = ((w.damage[0] + w.damage[1]) as f32 / 2.0 * w.shockwave).round() as u32;
    assert_eq!(b0 - s.mobs[&behind].health, ring, "the one behind takes the ring");
    assert_eq!(s.mobs[&far].health, f0, "the one out of the ring doesn't");
    // A light blow sends none.
    let (mut s, p) = world();
    wield(&mut s, p, "mograines_might");
    let behind = sturdy_foe(&mut s, "dm_woodcutter");
    s.mobs.get_mut(&behind).unwrap().pos = [-4.0, 0.0, 0.0];
    slow_swing(&mut s, p);
    assert_eq!(s.mobs[&behind].health, 1_000_000);
}

#[test]
fn smites_hammer_staggers_on_the_third_hit_in_a_row_on_one_foe() {
    let (mut s, p) = world();
    wield(&mut s, p, "smite_hammer");
    let m = sturdy_foe(&mut s, "dm_woodcutter");
    for n in 1..=3u32 {
        reset_foe(&mut s, m);
        slow_swing(&mut s, p);
        let build = s.players[&p].fx.build;
        if n < 3 {
            assert!(build.is_some_and(|(foe, hits, _)| foe == m && hits == n), "hit {n}: {build:?}");
        } else {
            assert!(build.is_none(), "the third spends it: {build:?}");
        }
    }
    // (The stagger came as the third landed: it was set past the clock then.)
    let hit_third = s.mobs[&m].stagger_until;
    assert!(hit_third > 0.0, "it was staggered");
}

#[test]
fn rhahkzors_heavy_marks_a_spot_and_the_rocks_come_down_on_it_a_moment_later() {
    let (mut s, p) = world();
    wield(&mut s, p, "rhahkzor_maul");
    let m = sturdy_foe(&mut s, "dm_woodcutter");
    s.input(p, Input::Heavy);
    let mut marked = false;
    let mut hit_at = None;
    let before = s.mobs[&m].health;
    let mut at_hit = before;
    for _ in 0..300 {
        let ev = s.step(0.02);
        marked |= ev.iter().any(|e| matches!(e, Event::Mark { .. }));
        if hit_at.is_none() && hits_on(&ev, m).iter().any(|(d, _)| *d > 0) {
            hit_at = Some(s.clock);
            at_hit = s.mobs[&m].health;
        }
    }
    assert!(marked, "the spot is marked first");
    assert!(hit_at.is_some());
    let w = s.content.weapon("rhahkzor_maul").unwrap();
    let rocks = ((w.damage[0] + w.damage[1]) as f32 / 2.0 * w.rockfall).round() as u32;
    assert!(at_hit - s.mobs[&m].health >= rocks, "the rocks came down on it: {} of {rocks}", at_hit - s.mobs[&m].health);
    assert!(s.fx_pending.is_empty(), "and are spent");
}

#[test]
fn targorrs_axe_hits_harder_as_the_hero_falls() {
    let blow = |health_share: f32| {
        let (mut s, p) = world();
        wield(&mut s, p, "targorr_axe");
        let m = sturdy_foe(&mut s, "dm_woodcutter");
        let max = s.players[&p].max_health;
        s.players.get_mut(&p).unwrap().health = ((max as f32 * health_share).round() as u32).max(1);
        first_hit(&slow_swing(&mut s, p), m)
    };
    let whole = blow(1.0);
    let half = blow(0.5);
    let low = blow(0.1);
    assert!((50..=63).contains(&whole), "{whole}");
    assert!(half >= 68 && low >= 80, "half {half}, low {low}");
}

#[test]
fn the_corpsemaker_cows_and_the_guillotine_falls_on_the_helpless_and_the_rally_covers_allies() {
    // Corpsemaker.
    let (mut s, p) = world();
    wield(&mut s, p, "corpsemaker");
    let m = sturdy_foe(&mut s, "dm_woodcutter");
    swing(&mut s, p);
    assert!(s.has_cc(m, "cowed"), "cowed");
    // Guillotine: a heavy on a foe held fast does three times a heavy on one that isn't.
    let heavy = |held: bool| {
        let (mut s, p) = world();
        wield(&mut s, p, "guillotine_axe");
        let m = sturdy_foe(&mut s, "dm_woodcutter");
        s.mobs.get_mut(&m).unwrap().pos = [2.0, 0.0, 0.0];
        if held {
            s.apply_cc(m, p, "root", 20.0, 0.0, 0.0, "Test root");
        }
        let before = s.mobs[&m].health;
        heavy_swing(&mut s, p);
        before - s.mobs[&m].health
    };
    let (free, rooted) = (heavy(false), heavy(true));
    assert!(rooted as f32 >= free as f32 * 2.2, "{rooted} against {free}");
    // The Champion's greatsword.
    let (mut s, p) = world();
    let q = s.join("bren", "Bren", None);
    s.move_player(q, [0.0, 4.0, 0.0], 0.0, Moving::default());
    let r = s.join("cal", "Cal", None);
    s.move_player(r, [0.0, 30.0, 0.0], 0.0, Moving::default());
    wield(&mut s, p, "champions_greatsword");
    sturdy_foe(&mut s, "dm_woodcutter");
    heavy_swing(&mut s, p);
    assert!(s.has_cc(q, "hallowed") && s.has_cc(p, "hallowed"), "rallied");
    assert!(!s.has_cc(r, "hallowed"), "not the one out of reach");
}

#[test]
fn the_two_handers_tooltips_say_their_mechanics_from_their_numbers() {
    let c = Content::dev();
    let text = |k: &str| c.items[c.item(k).unwrap()].text.clone();
    let w = |k: &str| c.weapon(k).unwrap();
    assert!(text("thornroot_maul").contains(&format!("root the enemies they hit for {} sec", w("thornroot_maul").root)), "{}", text("thornroot_maul"));
    let m = w("mograines_might");
    let ring = ((m.damage[0] + m.damage[1]) as f32 / 2.0 * m.shockwave).round();
    assert!(text("mograines_might").contains(&format!("holy shockwave out to {} yd that deals {} damage", crate::content::SHOCK_RADIUS, ring)), "{}", text("mograines_might"));
    assert!(text("smite_hammer").contains(&format!("The {}rd hit in a row on the same enemy within {} sec staggers it for {} sec", crate::content::BUILD_HITS, crate::content::BUILD_WINDOW, w("smite_hammer").build)), "{}", text("smite_hammer"));
    let r = w("rhahkzor_maul");
    let rocks = ((r.damage[0] + r.damage[1]) as f32 / 2.0 * r.rockfall).round();
    assert!(text("rhahkzor_maul").contains(&format!("rocks down on the enemy's spot after {} sec, dealing {} damage to enemies within {} yd", crate::content::ROCK_DELAY, rocks, crate::content::ROCK_RADIUS)), "{}", text("rhahkzor_maul"));
    assert!(text("targorr_axe").contains(&format!("up to {}% more damage as your health falls", (w("targorr_axe").rage * 100.0).round())));
    assert!(text("corpsemaker").contains(&format!("deals {}% less damage", (w("corpsemaker").cow * 100.0).round())), "{}", text("corpsemaker"));
    assert!(text("guillotine_axe").contains(&format!("deal {}% damage", (w("guillotine_axe").guillotine * 100.0).round())), "{}", text("guillotine_axe"));
    assert!(text("champions_greatsword").contains(&format!("take {}% less damage for {} sec", (w("champions_greatsword").rally * 100.0).round(), crate::content::RALLY_SECS)), "{}", text("champions_greatsword"));
}

// --- what review of the daggers found ------------------------------------------------------------------

#[test]
fn a_foe_lulled_by_the_hypnotic_blade_is_not_held_off_for_as_long_as_the_hero_keeps_stabbing() {
    let (mut s, p) = world();
    wield(&mut s, p, "hypnotic_blade");
    let m = sturdy_foe(&mut s, "dm_woodcutter");
    // This one fights back.
    s.mobs.get_mut(&m).unwrap().ready_at = 0.0;
    let mut blows_at_hero = 0;
    for _ in 0..12 {
        s.input(p, Input::Attack);
        for _ in 0..30 {
            let ev = s.step(0.02);
            blows_at_hero += ev.iter().filter(|e| matches!(e, Event::Swing { attacker, victim: Some(v), .. } if *attacker == m && *v == p)).count();
        }
        // (Kept within its reach.)
        let x = s.mobs.get_mut(&m).unwrap();
        x.pos = [2.0, 0.0, 0.0];
    }
    assert!(blows_at_hero > 0, "a hero who stabs on and on is struck back: {blows_at_hero}");
}

#[test]
fn a_foe_woken_by_a_blow_is_not_put_straight_back_to_sleep() {
    let (mut s, p) = world();
    wield(&mut s, p, "cobrahn_fang");
    let m = sturdy_foe(&mut s, "dm_woodcutter");
    turn_away(&mut s, m);
    swing(&mut s, p);
    assert!(s.has_cc(m, "incapacitate"), "asleep");
    s.hurt_mob(m, p, 5);
    assert!(!s.has_cc(m, "incapacitate"), "woken");
    turn_away(&mut s, m);
    swing(&mut s, p);
    assert!(!s.has_cc(m, "incapacitate"), "not again straight away");
    for _ in 0..((3.0 + crate::content::SLEEP_IMMUNE_SECS + 1.0) / 0.05) as usize {
        s.step(0.05);
    }
    turn_away(&mut s, m);
    s.mobs.get_mut(&m).unwrap().pos = [2.0, 0.0, 0.0];
    s.mobs.get_mut(&m).unwrap().stagger_until = 0.0;
    swing(&mut s, p);
    assert!(s.has_cc(m, "incapacitate"), "and again once the moment has passed");
}

#[test]
fn the_shivs_poison_stacks_beside_a_burn_that_is_already_on_the_foe() {
    let (mut s, p) = world();
    wield(&mut s, p, "bazil_shiv");
    let m = sturdy_foe(&mut s, "dm_woodcutter");
    s.apply_cc(m, p, "burn", 20.0, 40.0, 0.0, "Fireball");
    let per = s.content.weapon("bazil_shiv").unwrap().poison;
    let poison = |s: &Survival| s.cc_of(m).iter().find(|c| c.kind == "poison" && c.until > s.clock).map_or(0.0, |c| c.amount);
    swing(&mut s, p);
    reset_foe(&mut s, m);
    swing(&mut s, p);
    assert_eq!(poison(&s), 2.0 * per, "two stacks, a stronger burn or not");
    let burn = s.cc_of(m).iter().find(|c| c.kind == "burn" && c.until > s.clock).map_or(0.0, |c| c.amount);
    assert_eq!(burn, 40.0, "the other burn is as it was");
    // And it ticks.
    let h0 = s.mobs[&m].health;
    for _ in 0..60 {
        s.step(0.05);
    }
    assert!(h0 - s.mobs[&m].health > 0);
}
