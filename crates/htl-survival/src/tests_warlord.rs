//! The Warrior's two trees (the designer's layout, Nov 4). The Warlord, the commander who holds
//! the line: War Banner and Intimidating Roar, Cleave and Rising Cut, Tackle and Intervene,
//! Retaliation and Shield Wall, Rallying Cry. The Juggernaut, the berserker: Axe Throw, Shockwave,
//! Charge, Berserker Rage and Blood Fury, Bladestorm, and Momentum.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::{Foe, MobState};
use crate::sim::*;

const DT: f32 = 0.05;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];

fn world_on(pvp: bool, land: FlatLand) -> Survival {
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
    world_on(false, FlatLand::default())
}

fn hero(s: &mut Survival, name: &str, pos: Vec3, facing: f32) -> Id {
    let id = s.join(&name.to_lowercase(), name, None);
    s.move_player(id, pos, facing, Moving::default());
    let _ = s.step(0.0);
    id
}

/// A creature standing still, facing the hero, that won't swing.
fn mob(s: &mut Survival, key: &str, pos: Vec3) -> Id {
    let def = s.content.mob(key).unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.pos = pos;
    m.facing = std::f32::consts::PI;
    m.ready_at = 1e9;
    id
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        out.extend(s.step(DT));
    }
    out
}

fn key_of(slot: &str) -> u8 {
    crate::content::SLOTS.iter().position(|s| *s == slot).unwrap() as u8
}

/// The ability's key (with Ctrl: its EX), cooldowns off.
fn press(s: &mut Survival, p: Id, key: &str, ex: bool) -> Vec<Event> {
    let slot = s.content.abilities[s.content.ability(key).unwrap()].slot.clone();
    s.slot_ability(p, &slot, key).unwrap();
    s.set_cooldowns(p, true);
    s.input(p, Input::Ability(key_of(&slot), ex));
    s.step(DT)
}

/// The key, then a click along `yaw` (an aimed ability goes).
fn cast(s: &mut Survival, p: Id, key: &str, ex: bool, yaw: f32, pitch: f32) -> Vec<Event> {
    let mut ev = press(s, p, key, ex);
    s.set_aim_line(p, yaw, pitch);
    s.input(p, Input::Attack);
    ev.extend(s.step(DT));
    s.input(p, Input::AttackUp);
    ev.extend(s.step(DT));
    ev
}

fn hp(s: &Survival, m: Id) -> u32 {
    s.mobs[&m].health
}

fn struck(ev: &[Event], victim: Id) -> Vec<u32> {
    ev.iter()
        .filter_map(|e| match e {
            Event::Swing { victim: Some(v), damage, outcome: Outcome::Hit, .. } if *v == victim => Some(*damage),
            _ => None,
        })
        .collect()
}

fn flashed(ev: &[Event], p: Id, text: &str) -> bool {
    ev.iter().any(|e| matches!(e, Event::Flash { player, text: t } if *player == p && t.contains(text)))
}

fn walled(at: [f32; 2], r: f32) -> FlatLand {
    FlatLand { walls: vec![(at, r)], ..Default::default() }
}

/// How long an effect of `kind` has left on `u`.
fn left(s: &Survival, u: Id, kind: &str) -> f32 {
    s.cc_of(u).iter().find(|c| c.kind == kind).map_or(0.0, |c| c.until - s.clock)
}

/// Down at the ground `d` yd ahead (the reticle from the hero's eye).
fn at_ground(d: f32) -> f32 {
    -(1.9f32 / d).atan()
}

/// A light attack swung and let land.
fn light(s: &mut Survival, p: Id) -> Vec<Event> {
    s.input(p, Input::Attack);
    s.input(p, Input::AttackUp);
    run(s, 1.0)
}

// --- the board ---------------------------------------------------------------------------------------

#[test]
fn every_warrior_key_has_the_juggernaut_first_and_the_warlord_second() {
    let s = world();
    let rows = [
        ("ranged", "hurled_axe", "war_banner"),
        ("close", "shockwave", "cleave"),
        ("utility", "charge", "tackle"),
        ("defense", "berserker_rage", "retaliation"),
        ("ultimate", "warrior_bladestorm", "rallying_cry"),
    ];
    for (slot, jug, lord) in rows {
        for (key, choice) in [(jug, 1), (lord, 2)] {
            let d = &s.content.abilities[s.content.ability(key).unwrap()];
            assert_eq!((d.school.as_str(), d.slot.as_str(), d.choice), ("warrior", slot, choice), "{key}");
            assert!(d.on_board(), "{key} is a card on the board");
            assert!(!s.learned_where(key).is_empty(), "{key} is learned somewhere");
        }
        let n = s.content.abilities.iter().filter(|a| a.on_board() && a.school == "warrior" && a.slot == slot).count();
        assert_eq!(n, 2, "two Warrior cards on the {slot} key");
    }
    for (ex, base) in [("rending_axe", "hurled_axe"), ("intimidating_roar", "war_banner"), ("sundering_blow", "shockwave"), ("rising_cut", "cleave"), ("heroic_leap", "charge"), ("intervene", "tackle"), ("blood_fury", "berserker_rage"), ("shield_wall", "retaliation")] {
        let x = s.content.ability(ex).unwrap();
        assert_eq!(s.content.abilities[x].ex_of, base);
        // An EX shares its key's cooldown.
        assert_eq!(s.content.base_of(x), s.content.ability(base).unwrap(), "{ex} runs on {base}'s cooldown");
    }
    // An ultimate has no EX.
    for u in ["warrior_bladestorm", "rallying_cry"] {
        assert!(s.content.ex_of(s.content.ability(u).unwrap()).is_none(), "{u}");
    }
}

#[test]
fn the_retired_warrior_abilities_are_gone() {
    let s = world();
    for k in ["ravager", "ravager_recall", "heroic_throw", "thunder_clap", "battle_cry", "onslaught"] {
        assert!(s.content.ability(k).is_none(), "{k} retired");
    }
    assert_eq!(s.content.loadout.get("ultimate").map(String::as_str), Some("warrior_bladestorm"));
    assert_eq!(s.content.loadout.get("defense").map(String::as_str), Some("berserker_rage"));
}

#[test]
fn a_save_naming_retired_warrior_abilities_still_loads() {
    let mut c = Content::dev();
    c.pvp = Default::default();
    c.dev.full_tree = false;
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 7, 1_000_000.0);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let mut save = s.leave(p).unwrap();
    save.class = "warrior".into();
    save.owned = ["ravager", "heroic_throw", "thunder_clap", "rallying_cry", "battle_cry", "onslaught", "ravager_recall"].iter().map(|k| k.to_string()).collect();
    save.loadout = [("ranged", "ravager"), ("close", "thunder_clap"), ("defense", "rallying_cry"), ("ultimate", "onslaught")].iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
    let q = s.join("ayla", "Ayla", Some(&save));
    let h = &s.players[&q];
    assert!(h.owned.iter().all(|k| s.content.ability(k).is_some()), "nothing unknown kept: {:?}", h.owned);
    assert!(h.owned.contains("war_banner") && h.owned.contains("warrior_bladestorm") && h.owned.contains("rallying_cry"), "{:?}", h.owned);
    assert_eq!(h.loadout.get("ranged").map(String::as_str), Some("war_banner"), "Ravager's key holds the War Banner");
    assert_eq!(h.loadout.get("ultimate").map(String::as_str), Some("warrior_bladestorm"), "Onslaught's, Bladestorm");
    assert!(h.loadout.values().all(|k| s.content.ability(k).is_some()), "{:?}", h.loadout);
}

#[test]
fn a_hero_takes_the_warlord_or_the_juggernaut_onto_each_key() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.slot_ability(p, "utility", "tackle").unwrap();
    assert_eq!(s.players[&p].loadout.get("utility").map(String::as_str), Some("tackle"));
    s.slot_ability(p, "utility", "charge").unwrap();
    assert_eq!(s.players[&p].loadout.get("utility").map(String::as_str), Some("charge"), "one of the two on the key");
}

// --- Tackle --------------------------------------------------------------------------------------------

#[test]
fn tackle_drives_an_enemy_into_a_wall_and_it_crashes_stunned() {
    let mut s = world_on(false, walled([6.5, 0.0], 1.0));
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "brave", [2.0, 0.0, 0.0]);
    let before = hp(&s, k);
    let stamina = s.players[&p].stamina;
    let mut ev = cast(&mut s, p, "tackle", false, 0.0, 0.0);
    assert!(s.players[&p].stamina < stamina - 10.0, "it costs 12 stamina");
    assert!(s.unstaggerable(p), "nothing staggers the hero during the rush");
    ev.extend(run(&mut s, 0.8));
    assert!(flashed(&ev, p, "Crash!"), "told as a Crash");
    assert!(struck(&ev, k).contains(&20), "20 damage on the wall: {:?}", struck(&ev, k));
    assert!(hp(&s, k) < before);
    assert!(s.has_cc(k, "stun"), "stunned against the wall");
    assert!(ev.iter().any(|e| matches!(e, Event::Visual { id, kit: 83, .. } if *id == k)), "the Crash's mark on it");
    let at = s.mobs[&k].pos;
    assert!(at[0] > 3.5 && at[0] < 5.6, "carried up to the wall's face: {at:?}");
    assert!(s.players[&p].pos[0] > 2.0, "the hero rushed in behind it: {:?}", s.players[&p].pos);
}

#[test]
fn tackle_kicks_up_dust_from_its_first_step() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let _k = mob(&mut s, "brave", [6.0, 0.0, 0.0]);
    let mut ev = cast(&mut s, p, "tackle", false, 0.0, 0.0);
    ev.extend(run(&mut s, 0.45));
    let puffs = ev.iter().filter(|e| matches!(e, Event::Visual { id, kit, .. } if *id == p && *kit == 44)).count(); // 44: a landing's small dust cloud
    assert!(puffs >= 2, "dust from the feet through the rush, not Charge's purple: {puffs}");
}

#[test]
fn tackle_into_another_enemy_knocks_both_down() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let a = mob(&mut s, "brave", [2.0, 0.0, 0.0]);
    let b = mob(&mut s, "brave", [5.5, 0.0, 0.0]);
    let mut ev = cast(&mut s, p, "tackle", false, 0.0, 0.0);
    ev.extend(run(&mut s, 0.8));
    assert!(struck(&ev, a).contains(&14), "the carried one takes 14: {:?}", struck(&ev, a));
    assert!(struck(&ev, b).contains(&14), "the one it's driven into too: {:?}", struck(&ev, b));
    assert!(s.has_cc(a, "stun") && s.has_cc(b, "stun"), "both knocked down");
    assert!(ev.iter().any(|e| matches!(e, Event::React { id, anim, .. } if *id == b && *anim == KNOCKDOWN_ANIM)), "thrown to the ground");
    assert!(ev.iter().any(|e| matches!(e, Event::Visual { id, kit: 83, .. } if *id == b)), "the Crash's mark on the one driven into");
    // The knockdown is long enough to read: still down well after a second (the Crash landed
    // about 0.25 s in).
    run(&mut s, 0.9);
    assert!(s.has_cc(a, "stun") && s.has_cc(b, "stun"), "a 2 sec knockdown: {:?} / {:?} at {}", s.cc_of(a), s.cc_of(b), s.clock);
    run(&mut s, 1.0);
    assert!(!s.has_cc(a, "stun") && !s.has_cc(b, "stun"), "and up again");
    // The one driven into was shoved on by the Crash.
    assert!(s.mobs[&b].pos[0] > 6.5, "shoved on from 5.5: {:?}", s.mobs[&b].pos);
}

#[test]
fn tackle_with_nothing_behind_flings_the_enemy_on_ahead() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "brave", [2.0, 0.0, 0.0]);
    let mut ev = cast(&mut s, p, "tackle", false, 0.0, 0.0);
    ev.extend(run(&mut s, 0.35));
    assert!(s.mobs[&k].pos[0] > 8.0, "carried the rest of the 8 yd: {:?}", s.mobs[&k].pos);
    ev.extend(run(&mut s, 0.3));
    assert_eq!(struck(&ev, k), vec![10], "dropped for 10");
    assert!(!s.has_cc(k, "stun"), "no Crash, no stun");
    assert!(s.players[&p].pos[0] > 7.0, "the hero went the whole rush: {:?}", s.players[&p].pos);
    // Not set down at the hero's feet: flung on ahead, so the hit is seen to move it.
    run(&mut s, 0.8);
    assert!(s.mobs[&k].pos[0] > 10.5, "flung 4 yd on from the rush's end: {:?}", s.mobs[&k].pos);
}

#[test]
fn tackle_stops_against_a_boss_and_shakes_its_stance() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let w = mob(&mut s, "warlord", [2.5, 0.0, 0.0]);
    let mut ev = cast(&mut s, p, "tackle", false, 0.0, 0.0);
    ev.extend(run(&mut s, 0.1));
    assert!(s.posture_of(w) >= 30, "35 posture: {}", s.posture_of(w));
    assert!(flashed(&ev, p, "Stopped hard"));
    assert!(!ev.iter().any(|e| matches!(e, Event::MobMove { id, speed, .. } if *id == w && *speed > 10.0)), "the boss isn't carried");
    assert!(s.players[&p].pos[0] < 2.0, "the hero stops against it");
}

// --- Intervene ----------------------------------------------------------------------------------------

#[test]
fn intervene_rushes_to_a_friend_and_takes_the_next_blow_meant_for_them() {
    let mut s = world();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [10.0, 0.0, 0.0], 0.0);
    let k = mob(&mut s, "brave", [11.5, 0.5, 0.0]);
    s.set_valor(a, 25.0);
    let mut ev = cast(&mut s, a, "tackle", true, 0.0, 0.0);
    ev.extend(run(&mut s, 0.5));
    assert!(dist2d(s.players[&a].pos, s.players[&b].pos) < 2.0, "Ayla is at Bren's side: {:?}", s.players[&a].pos);
    assert!(s.has_cc(b, "guarded"), "Bren is covered");
    let (ha, hb) = (s.players[&a].health, s.players[&b].health);
    let from = s.mobs[&k].pos;
    s.strike_fair(k, b, 20, from, true, false, true, Blow::Light, None, Follow::None);
    let ev = s.step(DT);
    assert_eq!(s.players[&b].health, hb, "the blow meant for Bren");
    assert!(s.players[&a].health < ha, "strikes Ayla instead");
    assert!(flashed(&ev, a, "Intervened"));
    assert!(!s.has_cc(b, "guarded"), "only the first blow");
    s.strike_fair(k, b, 20, from, true, false, true, Blow::Light, None, Follow::None);
    s.step(DT);
    assert!(s.players[&b].health < hb, "the next one lands on Bren");
}

#[test]
fn intervene_takes_a_friends_share_of_an_arc_as_one_blow_not_two() {
    let mut s = world();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [10.0, 0.0, 0.0], 0.0);
    let k = mob(&mut s, "brave", [11.5, 0.5, 0.0]);
    s.set_valor(a, 25.0);
    cast(&mut s, a, "tackle", true, 0.0, 0.0);
    run(&mut s, 0.5);
    assert!(s.has_cc(b, "guarded"), "Bren is covered");
    let from = s.mobs[&k].pos;
    // The brave's arc takes in both: Bren's share comes to Ayla, and Ayla is in it herself.
    let ha = s.players[&a].health;
    s.strike_fair(k, b, 20, from, true, false, true, Blow::Light, None, Follow::None);
    s.strike_fair(k, a, 20, from, true, false, true, Blow::Light, None, Follow::None);
    s.step(DT);
    let once = ha - s.players[&a].health;
    assert!(once > 0, "the blow lands on Ayla");
    // The same blow on Ayla alone, later: the same amount, so the arc landed on her once.
    run(&mut s, 1.5);
    let ha = s.players[&a].health;
    s.strike_fair(k, a, 20, from, true, false, true, Blow::Light, None, Follow::None);
    s.step(DT);
    assert_eq!(ha - s.players[&a].health, once, "one blow, one hit");
}

#[test]
fn intervene_with_the_guard_up_blocks_the_blow() {
    let mut s = world();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [10.0, 0.0, 0.0], 0.0);
    let k = mob(&mut s, "brave", [10.0, 2.5, 0.0]);
    s.set_valor(a, 25.0);
    cast(&mut s, a, "tackle", true, 0.0, 0.0);
    run(&mut s, 0.5);
    s.input(a, Input::Guard);
    run(&mut s, 0.6);
    let (ha, hb) = (s.players[&a].health, s.players[&b].health);
    let from = s.mobs[&k].pos;
    s.strike_fair(k, b, 20, from, true, false, true, Blow::Light, None, Follow::None);
    let ev = s.step(DT);
    assert_eq!((s.players[&a].health, s.players[&b].health), (ha, hb), "Ayla's guard takes it");
    assert!(ev.iter().any(|e| matches!(e, Event::Swing { victim: Some(v), outcome: Outcome::Blocked, .. } if *v == a)), "blocked on Ayla");
}

#[test]
fn intervene_with_no_friend_in_reach_spends_nothing() {
    let mut s = world();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.set_valor(a, 25.0);
    s.set_cooldowns(a, false);
    let slot = "utility";
    s.slot_ability(a, slot, "tackle").unwrap();
    s.input(a, Input::Ability(key_of(slot), true));
    s.step(DT);
    s.set_aim_line(a, 0.0, 0.0);
    s.input(a, Input::Attack);
    let ev = s.step(DT);
    assert!(flashed(&ev, a, "No ally in reach"));
    assert_eq!(s.players[&a].valor, 25.0, "the Valor back");
    let t = s.content.ability("tackle").unwrap();
    assert!(s.cooldown(a, t).charges > 0, "and the cooldown");
}

// --- War Banner -----------------------------------------------------------------------------------------

/// Ayla plants her banner 6 yd ahead.
fn plant(s: &mut Survival, p: Id) -> Vec<Event> {
    let mut ev = cast(s, p, "war_banner", false, 0.0, at_ground(6.0));
    ev.extend(run(s, 0.2));
    ev
}

fn banner_at(s: &Survival, p: Id) -> Option<Vec3> {
    s.warlord.banners.get(&p).map(|b| b.at)
}

#[test]
fn war_banner_stands_where_aimed_for_10_sec() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let ev = plant(&mut s, p);
    let at = banner_at(&s, p).expect("planted");
    assert!((at[0] - 6.0).abs() < 1.0 && at[1].abs() < 0.5, "where she aimed: {at:?}");
    let obj = s.warlord.banners[&p].obj;
    assert!(ev.iter().any(|e| matches!(e, Event::ObjSpawned { id } if *id == obj)), "drawn for everyone");
    assert!(matches!(s.objs[&obj].kind, ObjKind::Prop { display: 4313, .. }), "the banner's look");
    let ev = run(&mut s, 10.0);
    assert!(banner_at(&s, p).is_none(), "down after 10 sec");
    assert!(ev.iter().any(|e| matches!(e, Event::ObjGone { id } if *id == obj)));
}

#[test]
fn ctrl_q_roars_while_the_banner_stands_and_q_still_moves_it() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "brave", [5.0, 0.0, 0.0]);
    plant(&mut s, p);
    let at = banner_at(&s, p).expect("planted");
    // Ctrl+Q with the banner standing: the Roar, not the banner's move.
    cast(&mut s, p, "war_banner", true, 0.0, 0.0);
    run(&mut s, 0.5);
    assert!(s.has_cc(k, "cowed"), "the brave in the cone is Cowed: the Roar went");
    assert_eq!(banner_at(&s, p), Some(at), "and the banner stayed where it was");
    // Q again moves it, as before.
    cast(&mut s, p, "war_banner", false, 0.0, at_ground(3.0));
    run(&mut s, 0.3);
    let moved = banner_at(&s, p).expect("still standing");
    assert!((moved[0] - 3.0).abs() < 1.0, "moved to the new spot: {moved:?}");
}

#[test]
fn war_banner_quickens_the_stamina_of_heroes_near_it() {
    let mut s = world();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [8.0, 3.0, 0.0], 0.0);
    let c = hero(&mut s, "Cara", [30.0, 0.0, 0.0], 0.0);
    plant(&mut s, a);
    for h in [a, b, c] {
        s.players.get_mut(&h).unwrap().stamina = 0.0;
    }
    run(&mut s, 1.0);
    let (sa, sb, sc) = (s.players[&a].stamina, s.players[&b].stamina, s.players[&c].stamina);
    assert!(sc > 5.0, "Cara regains it at her own pace: {sc}");
    assert!(sa > sc * 1.8 && sb > sc * 1.8, "twice as fast within 8 yd: {sa} {sb} against {sc}");
}

#[test]
fn war_banner_marks_the_heroes_near_it() {
    let mut s = world();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [8.0, 3.0, 0.0], 0.0);
    let c = hero(&mut s, "Cara", [30.0, 0.0, 0.0], 0.0);
    plant(&mut s, a);
    assert!(s.has_cc(a, "war_banner") && s.has_cc(b, "war_banner"), "Ayla and Bren near it");
    assert!(!s.has_cc(c, "war_banner"), "Cara far off");
    assert!(s.auras_of(b).contains(&18499), "the banner's look over Bren: {:?}", s.auras_of(b));
    s.move_player(b, [30.0, 5.0, 0.0], 0.0, Moving::default());
    run(&mut s, 0.5);
    assert!(!s.has_cc(b, "war_banner"), "gone once he walks away");
}

#[test]
fn war_banner_makes_heavy_attacks_near_it_stagger_30_percent_longer() {
    let mut s = world();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let far = hero(&mut s, "Bren", [40.0, 0.0, 0.0], 0.0);
    let k = mob(&mut s, "brave", [3.0, 3.0, 0.0]);
    plant(&mut s, a);
    assert!((s.stagger_share(a, k, true) - 1.3).abs() < 1e-4, "a heavy attack by the banner: {}", s.stagger_share(a, k, true));
    assert!((s.stagger_share(a, k, false) - 1.0).abs() < 1e-4, "a light one as ever");
    assert!((s.stagger_share(far, k, true) - 1.0).abs() < 1e-4, "away from it, as ever");
}

#[test]
fn war_banner_falls_to_an_enemys_blows() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    plant(&mut s, p);
    let at = banner_at(&s, p).unwrap();
    // A brave beside the banner, swinging at Ayla as she stands by it.
    s.move_player(p, [at[0] - 1.6, 0.0, 0.0], 0.0, Moving::default());
    let k = mob(&mut s, "brave", [at[0] + 1.2, 0.0, 0.0]);
    {
        let m = s.mobs.get_mut(&k).unwrap();
        m.state = MobState::Chase { foe: Foe::Hero(p) };
        m.ready_at = 0.0;
    }
    let before = s.warlord.banners[&p].health;
    let mut fell = false;
    for _ in 0..200 {
        s.step(DT);
        if !s.warlord.banners.contains_key(&p) {
            fell = true;
            break;
        }
        s.players.get_mut(&p).unwrap().health = 200;
    }
    assert!(before == 60.0, "60 health");
    assert!(fell, "its blows bring it down: {:?}", s.warlord.banners.get(&p).map(|b| b.health));
}

#[test]
fn war_banner_is_damaged_by_a_heros_own_swing() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    plant(&mut s, p);
    let at = banner_at(&s, p).unwrap();
    s.move_player(p, [at[0] - 1.5, at[1], 0.0], 0.0, Moving::default());
    s.step(DT);
    light(&mut s, p);
    let left = s.warlord.banners.get(&p).map_or(0.0, |b| b.health);
    assert!(left < 60.0, "her own blow takes from it: {left}");
}

#[test]
fn war_banner_recast_moves_it_to_the_new_spot() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    plant(&mut s, p);
    let first = banner_at(&s, p).unwrap();
    let old = s.warlord.banners[&p].obj;
    let until = s.warlord.banners[&p].until;
    // The key again (no cooldown spent: the recast's free), aimed to the side.
    s.slot_ability(p, "ranged", "war_banner").unwrap();
    s.input(p, Input::Ability(key_of("ranged"), false));
    s.step(DT);
    s.set_aim_line(p, std::f32::consts::FRAC_PI_2, at_ground(6.0));
    s.input(p, Input::Attack);
    let mut ev = s.step(DT);
    s.input(p, Input::AttackUp);
    ev.extend(run(&mut s, 0.2));
    let now = banner_at(&s, p).expect("still standing");
    assert!(dist2d(first, now) > 5.0 && now[1] > 4.0, "planted at the new spot: {first:?} to {now:?}");
    assert!(ev.iter().any(|e| matches!(e, Event::ObjGone { id } if *id == old)), "uprooted from the old");
    assert!((s.warlord.banners[&p].until - until).abs() < 1e-3, "its time left goes with it");
    assert_eq!(s.warlord.banners.len(), 1, "one banner");
}

// --- Intimidating Roar --------------------------------------------------------------------------------

#[test]
fn intimidating_roar_cows_the_pack_their_blows_softer_and_slower() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "brave", [6.0, 0.0, 0.0]);
    let behind = mob(&mut s, "brave", [-6.0, 0.0, 0.0]);
    let (d0, w0) = (s.attack_now(k).damage, s.attack_now(k).windup);
    s.set_valor(p, 25.0);
    let ev0 = cast(&mut s, p, "war_banner", true, 0.0, 0.0);
    assert!(s.has_cc(k, "cowed") && !s.has_cc(behind, "cowed"), "the cone in front");
    assert!((left(&s, k, "cowed") - 6.0).abs() < 0.2, "6 sec: {}", left(&s, k, "cowed"));
    let a = s.attack_now(k);
    assert!((a.damage as f32 - d0 as f32 * 0.75).abs() <= 1.0, "25% less: {} from {d0}", a.damage);
    assert!((a.windup - w0 * 1.15).abs() < 1e-3, "15% slower: {} from {w0}", a.windup);
    assert!(!s.auras_of(k).contains(&5246), "no fear swirl on it: {:?}", s.auras_of(k));
    // Its mark: Demoralizing Shout's over the head, now and again every 1.5 sec while it holds.
    let marks = |ev: &[Event]| ev.iter().filter(|e| matches!(e, Event::Visual { id, kit: 693, .. } if *id == k)).count();
    assert_eq!(marks(&ev0), 1, "the mark as it starts");
    let ev = run(&mut s, 3.2);
    assert_eq!(marks(&ev), 2, "and again every 1.5 sec");
}

#[test]
fn intimidating_roar_only_takes_10_percent_off_a_bosses_blows() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let w = mob(&mut s, "warlord", [6.0, 0.0, 0.0]);
    let (d0, w0) = (s.attack_now(w).damage, s.attack_now(w).windup);
    s.set_valor(p, 25.0);
    cast(&mut s, p, "war_banner", true, 0.0, 0.0);
    let a = s.attack_now(w);
    assert!((a.damage as f32 - d0 as f32 * 0.9).abs() <= 1.0, "10% less: {} from {d0}", a.damage);
    assert!((a.windup - w0).abs() < 1e-3, "no slower");
    s.mobs.get_mut(&w).unwrap().health = 50;
    run(&mut s, 0.3);
    assert!(!s.has_cc(w, "panic"), "a boss never flees");
}

#[test]
fn a_cowed_enemy_near_death_flees() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let weak = mob(&mut s, "brave", [6.0, 1.0, 0.0]);
    let strong = mob(&mut s, "brave", [6.0, -1.0, 0.0]);
    s.mobs.get_mut(&weak).unwrap().health = 30;
    s.set_valor(p, 25.0);
    cast(&mut s, p, "war_banner", true, 0.0, 0.0);
    run(&mut s, 0.2);
    assert!(s.has_cc(weak, "panic"), "below 20%: it runs");
    assert!(!s.has_cc(strong, "panic"), "a healthy one keeps fighting");
}

#[test]
fn a_cowed_hero_is_cowed_half_as_long() {
    let mut s = world_on(true, FlatLand::default());
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [6.0, 0.0, 0.0], std::f32::consts::PI);
    s.set_valor(a, 25.0);
    cast(&mut s, a, "war_banner", true, 0.0, 0.0);
    assert!(s.has_cc(b, "cowed"));
    assert!((left(&s, b, "cowed") - 3.0).abs() < 0.2, "3 sec: {}", left(&s, b, "cowed"));
    assert!((s.blow_share(b) - 0.75).abs() < 1e-3, "his blows 25% softer: {}", s.blow_share(b));
}

// --- Cleave, Rising Cut -----------------------------------------------------------------------------------

#[test]
fn cleave_hits_an_opened_enemy_for_250_percent() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let open = mob(&mut s, "brave", [2.5, 1.5, 0.0]);
    let shut = mob(&mut s, "brave", [2.5, -1.5, 0.0]);
    let behind = mob(&mut s, "brave", [-2.5, 0.0, 0.0]);
    let mut ev = cast(&mut s, p, "cleave", false, 0.0, 0.0);
    s.stagger_mob(open, 3.0, false);
    assert!(s.opened(open) && !s.opened(shut));
    ev.extend(run(&mut s, 0.8));
    let (o, c) = (struck(&ev, open), struck(&ev, shut));
    assert_eq!((o.len(), c.len()), (1, 1), "each struck once: {o:?} {c:?}");
    assert!(o[0] as f32 >= c[0] as f32 * 2.3, "250% against 100%: {} and {}", o[0], c[0]);
    assert!(struck(&ev, behind).is_empty(), "nothing behind the 180° arc");
    assert!(ev.iter().any(|e| matches!(e, Event::Visual { id, kit, .. } if *id == open && *kit == KIT_RIPOSTE)), "a red spark on the one cashed in");
}

#[test]
fn opened_means_staggered_stunned_knocked_down_exposed_or_a_boss_stance_broken() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "brave", [3.0, 0.0, 0.0]);
    assert!(!s.opened(k));
    s.apply_cc(k, p, "stun", 2.0, 0.0, 0.0, "test");
    assert!(s.opened(k), "stunned");
    let x = mob(&mut s, "brave", [3.0, -5.0, 0.0]);
    s.apply_cc(x, p, "exposed", 2.0, 0.2, 0.0, "test");
    assert!(s.opened(x), "Exposed");
    let w = mob(&mut s, "warlord", [3.0, 5.0, 0.0]);
    s.stagger_mob(w, 1.0, false);
    assert!(!s.opened(w), "a boss is opened only by a broken stance");
    let limit = s.posture_limit(w);
    assert!(s.build_posture(w, p, limit));
    assert!(s.opened(w), "stance-broken");
}

#[test]
fn rising_cut_cannot_be_staggered_in_its_windup() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "brave", [2.5, 0.0, 0.0]);
    s.set_valor(p, 25.0);
    cast(&mut s, p, "cleave", true, 0.0, 0.0);
    assert!(matches!(s.players[&p].action, Action::Cast { .. }), "winding up");
    assert!(!s.apply_cc(p, k, "stagger", 1.0, 0.0, 0.0, "test"), "a stagger doesn't take");
    assert!(matches!(s.players[&p].action, Action::Cast { .. }), "still winding up");
    let ev = run(&mut s, 0.6);
    assert_eq!(struck(&ev, k).len(), 1, "and it cuts");
}

#[test]
fn rising_cut_exposes_what_it_hits_for_5_sec_a_boss_for_2_5() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "brave", [2.5, 1.0, 0.0]);
    let w = mob(&mut s, "warlord", [2.5, -1.5, 0.0]);
    let behind = mob(&mut s, "brave", [-2.5, 0.0, 0.0]);
    s.set_valor(p, 25.0);
    cast(&mut s, p, "cleave", true, 0.0, 0.0);
    run(&mut s, 0.5);
    assert!((left(&s, k, "exposed") - 5.0).abs() < 0.2, "5 sec: {}", left(&s, k, "exposed"));
    assert!((left(&s, w, "exposed") - 2.5).abs() < 0.2, "a boss's 2.5: {}", left(&s, w, "exposed"));
    assert!(!s.has_cc(behind, "exposed"), "only in front");
    assert!(s.auras_of(k).contains(&8647), "the Exposed look on it");
}

#[test]
fn an_exposed_enemy_takes_20_percent_more_and_reels_30_percent_longer() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "brave", [3.0, 0.0, 0.0]);
    assert!((s.taken_share(k) - 1.0).abs() < 1e-4);
    s.apply_cc(k, p, "exposed", 5.0, 0.2, 0.0, "test");
    assert!((s.taken_share(k) - 1.2).abs() < 1e-4, "20% more: {}", s.taken_share(k));
    assert!((s.stagger_share(p, k, false) - 1.3).abs() < 1e-4, "30% longer staggers");
    let now = s.clock;
    s.apply_cc(k, p, "stagger", 1.0, 0.0, 0.0, "test");
    assert!((s.mobs[&k].stagger_until - now - 1.3).abs() < 0.01, "a 1 sec stagger is 1.3: {}", s.mobs[&k].stagger_until - now);
}

#[test]
fn an_exposed_hero_cannot_block_a_heros_blow() {
    let mut s = world_on(true, FlatLand::default());
    let pi = std::f32::consts::PI;
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [1.8, 0.0, 0.0], pi);
    s.input(b, Input::Guard);
    run(&mut s, 0.6);
    let h = s.players[&b].health;
    light(&mut s, a);
    assert_eq!(s.players[&b].health, h, "guarded, the blow is blocked");
    s.apply_cc(b, a, "exposed", 5.0, 0.2, 0.0, "test");
    let ev = light(&mut s, a);
    assert!(s.players[&b].health < h, "Exposed, it lands through the guard: {:?}", struck(&ev, b));
}

// --- Rallying Cry ------------------------------------------------------------------------------------------

#[test]
fn rallying_cry_gets_everyone_near_up_with_full_stamina() {
    let mut s = world();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [15.0, 0.0, 0.0], 0.0);
    let c = hero(&mut s, "Cara", [30.0, 0.0, 0.0], 0.0);
    let k = mob(&mut s, "brave", [16.0, 1.0, 0.0]);
    for h in [a, b, c] {
        s.players.get_mut(&h).unwrap().stamina = 5.0;
    }
    // Bren knocked down, Cara staggered.
    s.knock_down(k, b, 3.0, "test");
    assert!(s.has_cc(b, "stun"));
    s.set_valor(a, 100.0);
    let ev = press(&mut s, a, "rallying_cry", false);
    assert_eq!(s.players[&a].valor, 0.0, "the whole Valor bar");
    assert!(s.players[&a].stamina >= 99.0 && s.players[&b].stamina >= 99.0, "all of it: {} {}", s.players[&a].stamina, s.players[&b].stamina);
    assert!(!s.has_cc(b, "stun"), "Bren up off the ground");
    assert!(s.players[&c].stamina < 10.0, "Cara, 30 yd off, out of reach");
    let shouted: Vec<Id> = ev.iter().filter_map(|e| match e { Event::Missile { spell: 6673, target: Target::Unit(u), .. } => Some(*u), _ => None }).collect();
    assert!(shouted.contains(&a) && shouted.contains(&b) && !shouted.contains(&c), "Battle Shout's look on each one reached: {shouted:?}");
}

#[test]
fn rallying_cry_leaves_everyone_near_unstaggerable_for_8_sec() {
    let mut s = world();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [10.0, 0.0, 0.0], 0.0);
    let k = mob(&mut s, "brave", [11.0, 1.0, 0.0]);
    assert!(s.apply_cc(b, k, "stagger", 2.0, 0.0, 0.0, "test"));
    assert!(matches!(s.players[&b].action, Action::Stagger { .. }));
    s.set_valor(a, 100.0);
    press(&mut s, a, "rallying_cry", false);
    assert!(!matches!(s.players[&b].action, Action::Stagger { .. }), "his stagger ended");
    assert!(!s.apply_cc(b, k, "stagger", 1.0, 0.0, 0.0, "test") && !s.apply_cc(a, k, "stagger", 1.0, 0.0, 0.0, "test"), "neither can be staggered");
    assert!(s.auras_of(b).contains(&5515), "the mark on Bren: {:?}", s.auras_of(b));
    run(&mut s, 8.1);
    assert!(s.apply_cc(b, k, "stagger", 1.0, 0.0, 0.0, "test"), "8 sec");
}

#[test]
fn rallying_cry_grows_the_warrior_and_frees_and_quickens_their_attacks() {
    let mut s = world();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [10.0, 0.0, 0.0], 0.0);
    let (wa, wb) = (s.strike_at(a, StrikeKind::Light, 0).windup, s.strike_at(b, StrikeKind::Light, 0).windup);
    s.set_valor(a, 100.0);
    let mut ev = press(&mut s, a, "rallying_cry", false);
    ev.extend(run(&mut s, 0.5));
    assert!(ev.iter().any(|e| matches!(e, Event::Grow { id, scale } if *id == a && (*scale - 1.15).abs() < 0.01)), "15% larger");
    assert!(!ev.iter().any(|e| matches!(e, Event::Grow { id, .. } if *id == b)), "Bren stays his size");
    let (qa, qb) = (s.strike_at(a, StrikeKind::Light, 0).windup, s.strike_at(b, StrikeKind::Light, 0).windup);
    assert!((qa - wa / 1.2).abs() < 1e-3, "Ayla's 20% faster: {wa} to {qa}");
    assert!((qb - wb / 1.1).abs() < 1e-3, "Bren's 10%: {wb} to {qb}");
    let stamina = s.players[&a].stamina;
    light(&mut s, a);
    assert!(s.players[&a].stamina >= stamina, "her attacks cost nothing");
    let ev = run(&mut s, 8.0);
    assert!(ev.iter().any(|e| matches!(e, Event::Grow { id, scale } if *id == a && (*scale - 1.0).abs() < 0.01)), "back to size");
}

// --- Bladestorm (the Juggernaut's) --------------------------------------------------------------------------

fn spin(s: &mut Survival, p: Id) -> Vec<Event> {
    s.set_valor(p, 100.0);
    cast(s, p, "warrior_bladestorm", false, 0.0, 0.0)
}

#[test]
fn bladestorm_ramps_a_point_a_strike_up_to_16() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "warlord", [3.5, 0.0, 0.0]);
    let mut ev = spin(&mut s, p);
    assert!(matches!(s.players[&p].action, Action::Channel { .. }), "spinning");
    ev.extend(run(&mut s, 6.5));
    let hits = struck(&ev, k);
    assert_eq!(hits.len(), 12, "twice a second for 6 sec: {hits:?}");
    assert_eq!(hits, (5..=16).collect::<Vec<u32>>(), "5, then 1 more a strike, up to 16");
    assert_eq!(hits.iter().sum::<u32>(), 126);
    assert!(!matches!(s.players[&p].action, Action::Channel { .. }), "done after 6 sec");
}

#[test]
fn bladestorm_tells_the_clients_its_flurry_and_flashes_each_cut_that_lands() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "warlord", [3.5, 0.0, 0.0]);
    let mut ev = spin(&mut s, p);
    let flurry = ev.iter().find_map(|e| match e {
        Event::Flurry { id, secs, def } if *id == p => Some((*secs, def.clone())),
        _ => None,
    });
    let (secs, def) = flurry.expect("the flurry told as the spin starts");
    assert!((secs - 6.0).abs() < 1e-3, "for the spin's whole time: {secs}");
    assert_eq!(def.from.as_deref(), Some("OrcMale"), "the same model's cuts on every race");
    assert!(def.cuts.len() >= 4 && def.cut > 0.0 && def.finish_secs > 0.0, "{def:?}");
    assert_eq!(def.cuts_text().split(',').count(), def.cuts.len());
    assert!(!ev.iter().any(|e| matches!(e, Event::ClipStop { id } if *id == p)), "not stopped as it starts");
    ev.extend(run(&mut s, 1.0));
    let kit = s.content.abilities.iter().find(|a| a.key == "warrior_bladestorm").unwrap().impact_kit;
    assert!(kit != 0, "a flash on what a cut lands on");
    assert!(ev.iter().any(|e| matches!(e, Event::Visual { id, kit: k2, .. } if *id == k && *k2 == kit)), "the flash on the struck brave");
    ev.extend(run(&mut s, 6.0));
    assert!(ev.iter().any(|e| matches!(e, Event::ClipStop { id } if *id == p)), "the spin's end finishes the flurry");
}

#[test]
fn shockwave_flashes_its_cone_on_the_ground_as_it_lands() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let mut ev = cast(&mut s, p, "shockwave", false, 0.0, 0.0);
    ev.extend(run(&mut s, 0.6));
    let wedge = ev.iter().find_map(|e| match e {
        Event::Mark { tele: Some(t), .. } if t.hero && t.mob == p && t.at.is_some() => Some(*t),
        _ => None,
    });
    let t = wedge.expect("the cone on the ground as it lands");
    let d = s.content.abilities.iter().find(|a| a.key == "shockwave").unwrap();
    assert!((t.reach - d.range[0]).abs() < 0.1 && (t.arc - d.arc).abs() < 0.1 && (t.secs - d.ground_flash).abs() < 1e-3, "{t:?}");
}

#[test]
fn bladestorm_walks_at_70_percent_of_a_run() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    spin(&mut s, p);
    let run_speed = s.content.player.run_speed;
    assert!((s.speed_for(p) - run_speed * 0.7).abs() < 0.01, "{} of {run_speed}", s.speed_for(p));
}

#[test]
fn bladestorm_ends_into_a_guard_on_that_press() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    spin(&mut s, p);
    run(&mut s, 1.0);
    s.input(p, Input::Guard);
    assert!(matches!(s.players[&p].action, Action::Guard), "the guard up at once: {:?}", s.players[&p].action);
    assert!((s.players[&p].guard_at - s.clock).abs() < 1e-3, "a fresh guard: its parry window open");
}

#[test]
fn bladestorm_ends_into_a_dodge_on_that_press() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    spin(&mut s, p);
    run(&mut s, 1.0);
    s.input(p, Input::Dodge);
    assert!(matches!(s.players[&p].action, Action::Dodge { .. }), "the dodge at once: {:?}", s.players[&p].action);
}

#[test]
fn bladestorm_takes_no_other_ability_and_a_stagger_ends_it() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    spin(&mut s, p);
    s.slot_ability(p, "utility", "tackle").unwrap();
    s.input(p, Input::Ability(key_of("utility"), false));
    s.step(DT);
    assert!(matches!(s.players[&p].action, Action::Channel { .. }), "no other ability while it spins");
    let k = mob(&mut s, "brave", [1.5, 0.0, 0.0]);
    assert!(s.apply_cc(p, k, "stagger", 1.0, 0.0, 0.0, "test"), "it can be staggered");
    assert!(!matches!(s.players[&p].action, Action::Channel { .. }), "and the stagger ends it");
}

// --- Berserker Rage, Blood Fury -------------------------------------------------------------------------------

#[test]
fn berserker_rage_gets_you_up_from_a_knockdown() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "brave", [2.0, 0.0, 0.0]);
    s.knock_down(k, p, 4.0, "test");
    assert!(s.has_cc(p, "stun"), "on the ground");
    let ev = press(&mut s, p, "berserker_rage", false);
    assert!(!s.has_cc(p, "stun"), "up at once");
    assert!(ev.iter().any(|e| matches!(e, Event::ClipStop { id } if *id == p)), "the knockdown's clip let go");
    assert!(!s.apply_cc(p, k, "stagger", 1.0, 0.0, 0.0, "test"), "and can't be staggered");
}

#[test]
fn berserker_rage_breaks_a_stagger_a_root_and_a_slow() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "brave", [2.0, 0.0, 0.0]);
    s.apply_cc(p, k, "root", 4.0, 0.0, 0.0, "test");
    s.apply_cc(p, k, "snare", 4.0, 0.5, 0.0, "test");
    s.apply_cc(p, k, "stagger", 3.0, 0.0, 0.0, "test");
    assert!(matches!(s.players[&p].action, Action::Stagger { .. }));
    press(&mut s, p, "berserker_rage", false);
    assert!(!matches!(s.players[&p].action, Action::Stagger { .. }), "out of the stagger");
    assert!(!s.has_cc(p, "root") && !s.has_cc(p, "snare"), "free to move");
}

#[test]
fn berserker_rage_holds_off_staggers_for_3_sec() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "brave", [2.0, 0.0, 0.0]);
    press(&mut s, p, "berserker_rage", false);
    run(&mut s, 2.8);
    assert!(!s.apply_cc(p, k, "stagger", 1.0, 0.0, 0.0, "test"), "still steadfast");
    run(&mut s, 0.3);
    assert!(s.apply_cc(p, k, "stagger", 1.0, 0.0, 0.0, "test"), "3 sec, then a stagger takes");
}

#[test]
fn blood_fury_costs_15_percent_of_your_health_and_never_kills() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.players.get_mut(&p).unwrap().health = 100;
    s.set_valor(p, 25.0);
    press(&mut s, p, "berserker_rage", true);
    assert_eq!(s.players[&p].health, 85, "15% of 100");
    run(&mut s, 0.5);
    s.players.get_mut(&p).unwrap().health = 1;
    s.set_valor(p, 25.0);
    press(&mut s, p, "berserker_rage", true);
    assert_eq!(s.players[&p].health, 1, "never the last point");
}

#[test]
fn blood_fury_makes_attacks_free_and_25_percent_harder_for_8_sec() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.slot_ability(p, "ranged", "war_banner").unwrap();
    s.slot_ability(p, "close", "cleave").unwrap();
    s.slot_ability(p, "utility", "tackle").unwrap();
    s.slot_ability(p, "ultimate", "rallying_cry").unwrap();
    s.set_valor(p, 25.0);
    press(&mut s, p, "berserker_rage", true);
    s.slot_ability(p, "defense", "retaliation").unwrap();
    assert!(s.has_cc(p, "fury"));
    assert!((s.blow_share(p) - 1.25).abs() < 1e-3, "25% harder: {}", s.blow_share(p));
    let stamina = s.players[&p].stamina;
    light(&mut s, p);
    assert!(s.players[&p].stamina >= stamina, "free");
    run(&mut s, 7.5);
    assert!(!s.has_cc(p, "fury"), "8 sec");
}

// --- Momentum --------------------------------------------------------------------------------------------------

#[test]
fn momentum_stacks_with_each_hit_up_to_5() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "training_dummy", [1.8, 0.0, 0.0]);
    s.mobs.get_mut(&k).unwrap().health = 10_000;
    assert!(s.momentum_on(p), "a new Warrior's keys are the Juggernaut's");
    for n in 1..=6u32 {
        let ev = light(&mut s, p);
        assert_eq!(struck(&ev, k).len(), 1, "hit {n} lands");
        s.players.get_mut(&p).unwrap().stamina = 100.0;
        assert_eq!(s.momentum(p), n.min(5), "a stack a hit, 5 at most");
    }
    assert!((s.blow_share(p) - 1.25).abs() < 1e-3, "5% a stack: {}", s.blow_share(p));
}

#[test]
fn momentum_is_lost_when_you_raise_your_guard() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "brave", [1.8, 0.0, 0.0]);
    light(&mut s, p);
    light(&mut s, p);
    assert_eq!(s.momentum(p), 2);
    let _ = k;
    s.input(p, Input::Guard);
    s.step(DT);
    assert_eq!(s.momentum(p), 0, "the guard up: gone");
}

#[test]
fn momentum_is_lost_4_sec_after_the_last_hit() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    mob(&mut s, "brave", [1.8, 0.0, 0.0]);
    light(&mut s, p);
    assert_eq!(s.momentum(p), 1);
    run(&mut s, 2.5);
    assert_eq!(s.momentum(p), 1, "still there");
    run(&mut s, 1.5);
    assert_eq!(s.momentum(p), 0, "4 sec without a hit");
}

#[test]
fn a_hero_with_no_juggernaut_card_on_a_key_has_no_momentum() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    for (slot, key) in [("ranged", "war_banner"), ("close", "cleave"), ("utility", "tackle"), ("defense", "retaliation"), ("ultimate", "rallying_cry")] {
        s.slot_ability(p, slot, key).unwrap();
    }
    assert!(!s.momentum_on(p), "all the Warlord's");
    mob(&mut s, "brave", [1.8, 0.0, 0.0]);
    light(&mut s, p);
    assert_eq!(s.momentum(p), 0);
    s.slot_ability(p, "defense", "berserker_rage").unwrap();
    assert!(s.momentum_on(p), "one of the Juggernaut's is enough");
}
