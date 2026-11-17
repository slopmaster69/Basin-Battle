//! The Rogue's core (docs/rogue-roles.md, build plan entry 1): two blades and no guard, the
//! perfect dodge and its Shadow Cut, and the footwork in the two-blade run.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.02;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];

fn world_on(land: FlatLand) -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(land), 7, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

fn world() -> Survival {
    world_on(FlatLand::default())
}

/// A hero of this class, made as the creation screen makes one, at the origin facing +x.
fn hero_of(s: &mut Survival, name: &str, class: &str) -> Id {
    let account = name.to_lowercase();
    let save = s.fresh_save(&account, name, class, HeroLook::default());
    let id = s.join(&account, name, Some(&save));
    s.move_player(id, ORIGIN, 0.0, Moving::default());
    let _ = s.step(0.0);
    id
}

fn rogue(s: &mut Survival) -> Id {
    hero_of(s, "Ayla", "rogue")
}

/// A creature 2 yd in front of the hero, facing it, that never strikes on its own.
fn foe(s: &mut Survival, key: &str) -> Id {
    let def = s.content.mob(key).unwrap();
    let id = s.spawn_mob(def, [2.0, 0.0, 0.0], None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.facing = std::f32::consts::PI;
    m.ready_at = 1e9;
    m.health = 100_000;
    m.max_health = 100_000;
    id
}

/// The creature back where `foe` put it, facing the hero's first spot, in its blow's recovery as
/// after a blow that was dodged (it doesn't turn till that's over).
fn pin(s: &mut Survival, m: Id, hero: Id) {
    let until = s.clock + 1.0;
    let m = s.mobs.get_mut(&m).unwrap();
    m.state = MobState::Recover { foe: crate::sim::mobs::Foe::Hero(hero), until };
    m.pos = [2.0, 0.0, 0.0];
    m.facing = std::f32::consts::PI;
    m.ready_at = 1e9;
    // (Nor stepping round the hero: a creature circling turns to its foe as it goes.)
    m.path.clear();
    m.sidle = None;
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        out.extend(s.step(DT));
    }
    out
}

fn key(s: &Survival, i: usize) -> String {
    s.content.items[i].key.clone()
}

fn hits_on(ev: &[Event], v: Id) -> Vec<(u32, Outcome)> {
    ev.iter()
        .filter_map(|e| match e {
            Event::Swing { victim: Some(x), damage, outcome, style, .. } if *x == v && !matches!(style, Blow::Dot) => Some((*damage, *outcome)),
            _ => None,
        })
        .collect()
}

fn beat(ev: &[Event], p: Id, b: &str) -> bool {
    ev.iter().any(|e| matches!(e, Event::Beat { player, beat, .. } if *player == p && beat == b))
}

fn flashed(ev: &[Event], p: Id, text: &str) -> bool {
    ev.iter().any(|e| matches!(e, Event::Flash { player, text: t } if *player == p && t.contains(text)))
}

fn strike(s: &Survival, p: Id) -> Option<Strike> {
    match &s.players[&p].action {
        Action::Swing { strike, .. } => Some(strike.clone()),
        _ => None,
    }
}

/// A dodge on the spot, `secs` into it.
fn dodge_for(s: &mut Survival, p: Id, secs: f32) {
    s.players.get_mut(&p).unwrap().stamina = 100.0;
    s.input(p, Input::Dodge);
    assert!(matches!(s.players[&p].action, Action::Dodge { .. }), "dodging: {:?}", s.players[&p].action);
    run(s, secs);
}

/// A blow from `attacker` (2 yd in front) on hero `p`, at once.
fn blow(s: &mut Survival, attacker: Id, p: Id) -> Outcome {
    s.strike_player(attacker, p, 16, [2.0, 0.0, 0.0], true, false, true, Blow::Light)
}

fn cut_open(s: &Survival, p: Id) -> bool {
    s.players[&p].rogue.cut_until > s.clock
}

/// Run a blow until late in its recovery, where the next of a run is pressed.
fn to_late_recovery(s: &mut Survival, p: Id) {
    for _ in 0..80 {
        let _ = s.step(DT);
        if matches!(&s.players[&p].action, Action::Swing { t, strike, .. } if *t >= strike.windup + strike.active + strike.recovery * 0.75) {
            return;
        }
    }
}

// --- the hands -------------------------------------------------------------------------------------

#[test]
fn a_rogue_starts_with_two_daggers_and_no_shield() {
    let mut s = world();
    let p = rogue(&mut s);
    let h = &s.players[&p];
    assert_eq!(key(&s, h.weapon), "dagger");
    assert_eq!(h.off_weapon.map(|i| key(&s, i)).as_deref(), Some("dagger"));
    assert!(s.off_weapon_of(h).is_some() && s.two_hands(h), "a blade in each hand");
    // Everyone else still starts with the axe and the buckler.
    let w = hero_of(&mut s, "Bren", "warrior");
    let h = &s.players[&w];
    assert_eq!((key(&s, h.weapon).as_str(), key(&s, h.shield).as_str()), ("axe", "buckler"));
    assert!(h.off_weapon.is_none() && !h.off_empty, "the buckler held");
}

#[test]
fn a_rogue_cant_equip_a_shield_a_two_hander_or_an_axe() {
    let mut s = world();
    s.content.world.armory = true;
    let p = rogue(&mut s);
    let rack = s.objs[&s.rack_id()].pos;
    s.players.get_mut(&p).unwrap().pos = rack;
    assert_eq!(s.arm(p, "buckler").unwrap_err(), "Rogues can't use shields.");
    assert_eq!(s.arm(p, "greatsword").unwrap_err(), "Rogues can't use two-handed weapons.");
    assert_eq!(s.arm(p, "ember_staff").unwrap_err(), "Rogues can't use two-handed weapons.");
    assert_eq!(s.arm(p, "axe").unwrap_err(), "Rogues can't use axes.");
    assert_eq!(s.arm_off(p, "axe").unwrap_err(), "Rogues can't use axes.");
    assert_eq!(key(&s, s.players[&p].weapon), "dagger", "nothing changed hands");
    // A Rogue's own weapons: a sword, a mace.
    s.arm(p, "sword").unwrap();
    s.arm_off(p, "mace").unwrap();
    assert_eq!(key(&s, s.players[&p].weapon), "sword");
    // Nor from the bags.
    let buckler = s.content.item("buckler").unwrap();
    s.players.get_mut(&p).unwrap().bag.insert(buckler, 1);
    assert_eq!(s.wear_from_bag(p, buckler).unwrap_err(), "Rogues can't use shields.");
    // A Warrior still takes up any of them.
    let w = hero_of(&mut s, "Bren", "warrior");
    s.players.get_mut(&w).unwrap().pos = rack;
    s.arm(w, "greatsword").unwrap();
    s.arm(w, "buckler").unwrap();
}

#[test]
fn a_shield_a_rogue_forges_goes_to_storage_and_the_blades_stay_in_hand() {
    let mut s = world();
    let p = rogue(&mut s);
    let (at, f) = crate::tests::build_spot(&s);
    s.move_player(p, at, f, Moving::default());
    let _ = s.step(0.0);
    for (k, n) in [("wood", 20), ("ore", 12)] {
        let i = s.content.item(k).unwrap();
        s.players.get_mut(&p).unwrap().bag.insert(i, n);
    }
    s.build_start(p, "smithy").unwrap();
    let _ = s.step(DT);
    s.build_confirm(p).unwrap();
    let anvil = *s.objs.iter().find(|(_, o)| matches!(o.kind, ObjKind::Station { .. })).unwrap().0;
    s.events.clear();
    s.craft(p, anvil, "reinforced_shield").unwrap();
    let ev = run(&mut s, 0.5);
    let shield = s.content.item("reinforced_shield").unwrap();
    assert_eq!(s.storage.get(&shield), Some(&1), "in storage");
    let h = &s.players[&p];
    assert_eq!((key(&s, h.weapon).as_str(), h.off_weapon.map(|i| key(&s, i))), ("dagger", Some("dagger".to_string())));
    assert_ne!(h.shield, shield, "never put on");
    let told = ev.iter().chain(s.events.iter()).any(|e| matches!(e, Event::Tell { text, .. } | Event::Flash { text, .. } if text.contains("Rogues can't use shields.")));
    assert!(told, "the hero is told why");
}

#[test]
fn a_rogues_old_save_with_a_shield_loads_with_it_in_the_bags() {
    let mut s = world();
    let p = rogue(&mut s);
    // As every Rogue was saved before: the axe and the buckler in hand.
    let mut save = s.leave(p).unwrap();
    save.weapon = "axe".into();
    save.shield = "buckler".into();
    save.off_hand = String::new();
    let q = s.join("ayla", "Ayla", Some(&save));
    let h = &s.players[&q];
    assert_eq!(key(&s, h.weapon), "dagger", "the Rogue's blade in the main hand");
    assert!(h.off_weapon.is_none() && h.off_empty, "the off hand empty: it jabs");
    let bag: Vec<String> = h.bag.iter().filter(|(_, n)| **n > 0).map(|(i, _)| key(&s, *i)).collect();
    assert!(bag.contains(&"buckler".to_string()) && bag.contains(&"axe".to_string()), "both in the bags: {bag:?}");
    // Saved and loaded again, nothing more turns up: it's put right once.
    let again = s.leave(q).unwrap();
    let r = s.join("ayla", "Ayla", Some(&again));
    let n = |s: &Survival, k: &str| s.players[&r].bag.get(&s.content.item(k).unwrap()).copied().unwrap_or(0);
    assert_eq!((n(&s, "buckler"), n(&s, "axe")), (1, 1));
}

#[test]
fn a_rogues_right_click_swings_the_off_hand_and_never_blocks_or_parries() {
    let mut s = world();
    let p = rogue(&mut s);
    let m = foe(&mut s, "dm_woodcutter");
    s.input(p, Input::Guard);
    assert!(strike(&s, p).is_some_and(|st| st.kind == StrikeKind::Off), "an off-hand blow: {:?}", s.players[&p].action);
    let ev = run(&mut s, 0.8);
    assert!(hits_on(&ev, m).iter().any(|(d, o)| *o == Outcome::Hit && *d > 0), "it lands");
    s.input(p, Input::GuardUp);
    // Held through, the guard never comes up.
    s.input(p, Input::Guard);
    for _ in 0..60 {
        let _ = s.step(DT);
        assert!(!matches!(s.players[&p].action, Action::Guard), "no guard");
    }
    // A blow on a Rogue who pressed the right click just in time lands: no parry, no block.
    s.input(p, Input::GuardUp);
    run(&mut s, 1.0);
    let now = s.clock;
    let h = s.players.get_mut(&p).unwrap();
    h.guard_at = now;
    h.prev_guard_at = now - 5.0;
    h.action = Action::Ready;
    assert_eq!(blow(&mut s, m, p), Outcome::Hit);
}

#[test]
fn a_rogue_with_an_empty_off_hand_jabs_on_right_click() {
    let mut s = world();
    let p = rogue(&mut s);
    let m = foe(&mut s, "dm_woodcutter");
    let blade = s.strike_at(p, StrikeKind::Off, 0).damage;
    let h = s.players.get_mut(&p).unwrap();
    h.off_weapon = None;
    h.off_empty = true;
    let jab = s.strike_at(p, StrikeKind::Off, 0).damage;
    assert!((jab - blade * 0.5).abs() < 1.0e-4, "half the blow: {jab} of {blade}");
    s.input(p, Input::Guard);
    assert!(strike(&s, p).is_some_and(|st| st.kind == StrikeKind::Off), "a jab, not a guard: {:?}", s.players[&p].action);
    let ev = run(&mut s, 0.8);
    assert!(hits_on(&ev, m).iter().any(|(d, o)| *o == Outcome::Hit && *d > 0), "it lands");
}

// --- the perfect dodge ---------------------------------------------------------------------------

#[test]
fn a_dodge_whose_blow_lands_by_0_18_sec_is_perfect_and_a_later_one_only_evades() {
    let mut s = world();
    let p = rogue(&mut s);
    let m = foe(&mut s, "grunt");
    dodge_for(&mut s, p, 0.1);
    assert_eq!(blow(&mut s, m, p), Outcome::Dodged);
    assert!(cut_open(&s, p), "perfect: the Shadow Cut is open");
    let _ = s.step(DT);
    assert!(s.auras_of(p).contains(&s.content.rogue.flicker_aura), "the body flickers");
    run(&mut s, 0.4);
    assert!(!s.auras_of(p).contains(&s.content.rogue.flicker_aura), "for 0.3 sec");
    // 0.25 sec in: still dodged, not perfect.
    let mut s = world();
    let p = rogue(&mut s);
    let m = foe(&mut s, "grunt");
    dodge_for(&mut s, p, 0.24);
    assert_eq!(blow(&mut s, m, p), Outcome::Dodged);
    assert!(!cut_open(&s, p), "only evaded");
    assert!(!s.auras_of(p).contains(&s.content.rogue.flicker_aura));
}

#[test]
fn the_fight_line_counts_perfect_dodges_once_per_dodge_and_not_plain_ones() {
    let mut s = world();
    let p = rogue(&mut s);
    let m = foe(&mut s, "grunt");
    // A perfect one, and a second blow slipped by the same dodge: one perfect.
    dodge_for(&mut s, p, 0.1);
    assert_eq!(blow(&mut s, m, p), Outcome::Dodged);
    assert_eq!(blow(&mut s, m, p), Outcome::Dodged);
    let t = &s.players[&p].melee;
    assert_eq!((t.dodges, t.perfect_dodges), (1, 1), "{t:?}");
    // A late one only evades: a dodge, not a perfect.
    run(&mut s, 1.5);
    dodge_for(&mut s, p, 0.24);
    assert_eq!(blow(&mut s, m, p), Outcome::Dodged);
    let t = &s.players[&p].melee;
    assert_eq!((t.dodges, t.perfect_dodges), (2, 1), "{t:?}");
    let line = t.line("Ayla").unwrap();
    assert!(line.contains("dodges 2 (perfect 1)"), "{line}");
    let dodges = t.dodge_line("Ayla").unwrap();
    assert_eq!(dodges, "Ayla's dodges: rolls 2 (slipped a blow 2, perfect 1), quickstep 0 (slipped a blow 0, perfect 0)", "{dodges}");
}

#[test]
fn a_perfect_dodge_gives_20_stamina_and_10_valor_and_a_plain_one_10_and_8() {
    for (secs, stamina, valor) in [(0.1, 20.0, 10.0), (0.24, 10.0, 8.0)] {
        let mut s = world();
        let p = rogue(&mut s);
        let m = foe(&mut s, "grunt");
        dodge_for(&mut s, p, secs);
        s.players.get_mut(&p).unwrap().stamina = 50.0;
        s.set_valor(p, 25.0);
        s.events.clear();
        assert_eq!(blow(&mut s, m, p), Outcome::Dodged);
        let h = &s.players[&p];
        assert!((h.stamina - 50.0 - stamina).abs() < 1.0e-3, "{secs}: stamina {}", h.stamina);
        assert!((h.valor - 25.0 - valor).abs() < 1.0e-3, "{secs}: valor {}", h.valor);
        // The perfect one shows: Vanish's sound, its puff where the blow fell, the beat.
        let ev = std::mem::take(&mut s.events);
        let perfect = secs < 0.18;
        assert_eq!(ev.iter().any(|e| matches!(e, Event::Sound { id, sound } if *id == p && *sound == crate::sim::cues::SOUND_PERFECT_DODGE)), perfect);
        assert_eq!(ev.iter().any(|e| matches!(e, Event::Missile { caster, spell, .. } if *caster == p && *spell == 1856)), perfect);
        assert_eq!(beat(&ev, p, "perfect_dodge"), perfect);
    }
}

#[test]
fn an_attack_within_0_6_sec_of_a_perfect_dodge_steps_behind_the_striker_and_backstabs() {
    let mut s = world();
    let p = rogue(&mut s);
    let m = foe(&mut s, "grunt");
    dodge_for(&mut s, p, 0.1);
    assert_eq!(blow(&mut s, m, p), Outcome::Dodged);
    run(&mut s, 0.3);
    pin(&mut s, m, p);
    s.events.clear();
    s.input(p, Input::Attack);
    let mut ev = std::mem::take(&mut s.events);
    // Behind it (it faces the Rogue's old spot, -x): past it on +x, facing its back.
    let h = &s.players[&p];
    assert!(h.pos[0] > 2.5 && h.pos[1].abs() < 1.0, "behind it: {:?}", h.pos);
    assert!(angle_between(h.facing, std::f32::consts::PI) < 0.3, "facing its back: {}", h.facing);
    assert!(ev.iter().any(|e| matches!(e, Event::Teleport { player, .. } if *player == p)), "stepped there");
    assert_eq!(ev.iter().filter(|e| matches!(e, Event::Missile { caster, spell, .. } if *caster == p && *spell == 1856)).count(), 2, "a puff at both ends");
    assert!(strike(&s, p).is_some_and(|st| st.kind == StrikeKind::Light), "the main hand's blow");
    assert!(!cut_open(&s, p), "spent");
    ev.extend(run(&mut s, 0.6));
    assert!(hits_on(&ev, m).iter().any(|(d, o)| *o == Outcome::Hit && *d > 0), "it lands");
    assert!(beat(&ev, p, "backstab"), "in its back");
    // After 0.6 sec, an attack is only an attack.
    let mut s = world();
    let p = rogue(&mut s);
    let m = foe(&mut s, "grunt");
    dodge_for(&mut s, p, 0.1);
    blow(&mut s, m, p);
    run(&mut s, 0.65);
    s.input(p, Input::Attack);
    assert!(s.players[&p].pos[0] < 1.0, "no step: {:?}", s.players[&p].pos);
}

#[test]
fn no_shadow_cut_when_the_spot_behind_is_in_a_wall() {
    // A wall just behind the grunt.
    let land = FlatLand { walls: vec![([3.4, 0.0], 1.0)], ..Default::default() };
    let mut s = world_on(land);
    let p = rogue(&mut s);
    let m = foe(&mut s, "grunt");
    dodge_for(&mut s, p, 0.1);
    assert_eq!(blow(&mut s, m, p), Outcome::Dodged);
    run(&mut s, 0.4);
    pin(&mut s, m, p);
    s.events.clear();
    s.input(p, Input::Attack);
    let ev = std::mem::take(&mut s.events);
    assert!(!ev.iter().any(|e| matches!(e, Event::Teleport { .. })), "no step");
    assert!(s.players[&p].pos[0] < 1.0);
    assert!(!cut_open(&s, p), "the cut is spent");
    // The press is the attack out of the dodge, as ever.
    run(&mut s, 0.2);
    assert!(strike(&s, p).is_some(), "an attack: {:?}", s.players[&p].action);
}

#[test]
fn a_perfect_dodge_fills_12_percent_of_a_bosss_posture_and_never_staggers_it() {
    let mut s = world();
    let p = rogue(&mut s);
    let m = foe(&mut s, "overseer");
    let limit = s.posture_limit(m);
    assert!(limit > 0, "a champion with a posture");
    dodge_for(&mut s, p, 0.1);
    assert_eq!(blow(&mut s, m, p), Outcome::Dodged);
    let want = (limit as f32 * 0.12).round() as u32;
    assert_eq!(s.posture_of(m), want, "12% of {limit}");
    assert!(s.mobs[&m].stagger_until <= s.clock, "no reel");
    // A plain dodge fills nothing.
    let mut s = world();
    let p = rogue(&mut s);
    let m = foe(&mut s, "overseer");
    dodge_for(&mut s, p, 0.24);
    blow(&mut s, m, p);
    assert_eq!(s.posture_of(m), 0);
}

#[test]
fn a_perfect_dodge_of_a_heros_blow_pays_too() {
    let mut s = world();
    let p = rogue(&mut s);
    let w = hero_of(&mut s, "Bren", "warrior");
    s.move_player(w, [2.0, 0.0, 0.0], std::f32::consts::PI, Moving::default());
    dodge_for(&mut s, p, 0.1);
    s.players.get_mut(&p).unwrap().stamina = 50.0;
    assert_eq!(blow(&mut s, w, p), Outcome::Dodged);
    assert!((s.players[&p].stamina - 70.0).abs() < 1.0e-3, "20 back: {}", s.players[&p].stamina);
    assert_eq!(s.players[&p].rogue.cut_on, Some(w), "the cut behind the hero who swung");
}

#[test]
fn a_perfect_dodge_pressed_within_the_round_trip_still_counts() {
    let mut s = world();
    let p = rogue(&mut s);
    let m = foe(&mut s, "grunt");
    s.set_rtt(p, 0.15);
    // The blow lands on the server and waits out their round trip...
    s.strike_fair(m, p, 20, [2.0, 0.0, 0.0], true, false, true, Blow::Light, None, Follow::None);
    let ev = s.step(0.04);
    assert!(hits_on(&ev, p).is_empty(), "held for their round trip");
    // ...and the dodge they pressed as they saw it, late, arrives after.
    s.players.get_mut(&p).unwrap().stamina = 100.0;
    s.input(p, Input::Dodge);
    let ev = run(&mut s, 0.2);
    assert_eq!(hits_on(&ev, p), vec![(0, Outcome::Dodged)]);
    assert!(beat(&ev, p, "perfect_dodge"), "perfect");
}

// --- footwork in the run -----------------------------------------------------------------------------

#[test]
fn a_dodge_in_your_own_wind_up_calls_the_blow_off_and_gives_its_stamina_back() {
    let mut s = world();
    let p = rogue(&mut s);
    let m = foe(&mut s, "dm_woodcutter");
    s.players.get_mut(&p).unwrap().stamina = 80.0;
    s.input(p, Input::Attack);
    let st = strike(&s, p).unwrap();
    let paid = 80.0 - s.players[&p].stamina;
    assert!(paid > 0.0);
    let _ = s.step(st.windup * 0.4);
    s.input(p, Input::Dodge);
    assert!(matches!(s.players[&p].action, Action::Dodge { .. }), "the dodge goes: {:?}", s.players[&p].action);
    let left = s.players[&p].stamina;
    assert!((left - (80.0 - s.content.dodge.stamina)).abs() < 0.5, "the blow's stamina back, the dodge's paid: {left}");
    let ev = run(&mut s, 0.6);
    assert!(hits_on(&ev, m).is_empty(), "the blow never comes");
    // A Warrior can't: the dodge waits for the blow.
    let w = hero_of(&mut s, "Bren", "warrior");
    s.players.get_mut(&w).unwrap().stamina = 80.0;
    s.input(w, Input::Attack);
    let _ = s.step(0.03);
    s.input(w, Input::Dodge);
    assert!(strike(&s, w).is_some(), "still swinging");
}

#[test]
fn a_dodge_cuts_an_off_hand_blow_short_at_once_and_a_main_hand_blow_only_after_45_percent() {
    let after_landing = |s: &mut Survival, p: Id| {
        for _ in 0..60 {
            let _ = s.step(DT);
            if matches!(&s.players[&p].action, Action::Swing { t, strike, .. } if *t >= strike.windup + strike.active + 0.01) {
                return;
            }
        }
    };
    let mut s = world();
    let p = rogue(&mut s);
    s.players.get_mut(&p).unwrap().stamina = 100.0;
    s.input(p, Input::Guard);
    after_landing(&mut s, p);
    s.input(p, Input::Dodge);
    assert!(matches!(s.players[&p].action, Action::Dodge { .. }), "the off hand: at once");
    run(&mut s, 1.0);
    s.players.get_mut(&p).unwrap().stamina = 100.0;
    s.input(p, Input::Attack);
    after_landing(&mut s, p);
    s.input(p, Input::Dodge);
    assert!(strike(&s, p).is_some(), "the main hand: not yet");
    // Late in its recovery, it cuts short.
    let mut s = world();
    let p = rogue(&mut s);
    s.players.get_mut(&p).unwrap().stamina = 100.0;
    s.input(p, Input::Attack);
    for _ in 0..60 {
        let _ = s.step(DT);
        if matches!(&s.players[&p].action, Action::Swing { t, strike, .. } if *t >= strike.windup + strike.active + strike.recovery * 0.46) {
            break;
        }
    }
    s.input(p, Input::Dodge);
    assert!(matches!(s.players[&p].action, Action::Dodge { .. }), "after 45%");
    // A Warrior's off hand still waits for 45%.
    let mut s = world();
    let w = hero_of(&mut s, "Bren", "warrior");
    let dagger = s.content.item("dagger");
    let h = s.players.get_mut(&w).unwrap();
    h.weapon = dagger.unwrap();
    h.off_weapon = dagger;
    h.stamina = 100.0;
    s.input(w, Input::Guard);
    after_landing(&mut s, w);
    s.input(w, Input::Dodge);
    assert!(strike(&s, w).is_some(), "a Warrior's off hand commits");
}

#[test]
fn a_dodge_between_blows_keeps_the_run_and_a_shadow_cut_counts_as_the_next_hand() {
    // Left, right, a pause, a dodge, then left: the dodge kept the run where the pause alone
    // would have ended it.
    let run_after_dodge = |class: &str| -> u8 {
        let mut s = world();
        let p = hero_of(&mut s, "Ayla", class);
        let dagger = s.content.item("dagger");
        let h = s.players.get_mut(&p).unwrap();
        h.weapon = dagger.unwrap();
        h.off_weapon = dagger;
        foe(&mut s, "dm_woodcutter");
        for input in [Input::Attack, Input::Guard] {
            s.players.get_mut(&p).unwrap().stamina = 100.0;
            s.input(p, input);
            to_late_recovery(&mut s, p);
        }
        // The second blow ends; the dodge comes 1.2 sec later, inside the run's window...
        while strike(&s, p).is_some() {
            let _ = s.step(DT);
        }
        run(&mut s, 1.2);
        let secs = s.content.dodge.secs + 0.5;
        dodge_for(&mut s, p, secs);
        // ...and the next blow 0.5 sec after the dodge: 2.15 sec after the last blow.
        s.players.get_mut(&p).unwrap().stamina = 100.0;
        s.input(p, Input::Attack);
        s.players[&p].dual_run
    };
    assert_eq!(run_after_dodge("rogue"), 3, "the Rogue's run goes on");
    assert_eq!(run_after_dodge("warrior"), 1, "anyone else's ended");

    // A perfect dodge mid-run, and the Shadow Cut on the same hand as the last blow: still the
    // run's next.
    let mut s = world();
    let p = rogue(&mut s);
    let m = foe(&mut s, "grunt");
    for input in [Input::Attack, Input::Guard] {
        s.players.get_mut(&p).unwrap().stamina = 100.0;
        s.input(p, input);
        to_late_recovery(&mut s, p);
    }
    assert_eq!(s.players[&p].dual_run, 2);
    s.input(p, Input::Dodge);
    assert!(matches!(s.players[&p].action, Action::Dodge { .. }));
    run(&mut s, 0.1);
    assert_eq!(blow(&mut s, m, p), Outcome::Dodged);
    s.input(p, Input::Guard);
    assert!(strike(&s, p).is_some_and(|st| st.kind == StrikeKind::Off), "the cut on the off hand");
    assert_eq!(s.players[&p].dual_run, 3, "it counts");
    to_late_recovery(&mut s, p);
    s.input(p, Input::Attack);
    assert!(strike(&s, p).is_some_and(|st| st.clip == DUAL_FINISH_CLIP), "the finisher, both blades");
}

#[test]
fn retaliation_and_riposte_refuse_on_a_rogue() {
    let mut s = world();
    let p = rogue(&mut s);
    for (card, why) in [("retaliation", "Rogues can't block."), ("riposte", "Rogues can't parry.")] {
        let slot = s.content.abilities[s.content.ability(card).unwrap()].slot.clone();
        s.slot_ability(p, &slot, card).unwrap();
        s.set_cooldowns(p, true);
        s.set_valor(p, 100.0);
        let k = crate::content::SLOTS.iter().position(|x| *x == slot).unwrap() as u8;
        s.input(p, Input::Ability(k, false));
        let ev = s.step(DT);
        assert!(flashed(&ev, p, why), "{card}: {why}");
        assert!(!matches!(s.players[&p].action, Action::Stance { .. }), "{card} never goes up");
        run(&mut s, 0.5);
    }
    // A Warrior's still goes.
    let w = hero_of(&mut s, "Bren", "warrior");
    let slot = s.content.abilities[s.content.ability("retaliation").unwrap()].slot.clone();
    s.slot_ability(w, &slot, "retaliation").unwrap();
    s.set_cooldowns(w, true);
    let k = crate::content::SLOTS.iter().position(|x| *x == slot).unwrap() as u8;
    s.input(w, Input::Ability(k, false));
    let _ = s.step(DT);
    assert!(matches!(s.players[&w].action, Action::Stance { .. }));
}

#[test]
fn a_warrior_with_two_blades_has_no_perfect_dodge_payoff() {
    let mut s = world();
    let w = hero_of(&mut s, "Bren", "warrior");
    let dagger = s.content.item("dagger");
    s.players.get_mut(&w).unwrap().off_weapon = dagger;
    let m = foe(&mut s, "overseer");
    dodge_for(&mut s, w, 0.1);
    s.players.get_mut(&w).unwrap().stamina = 50.0;
    s.set_valor(w, 25.0);
    s.events.clear();
    assert_eq!(blow(&mut s, m, w), Outcome::Dodged);
    let h = &s.players[&w];
    assert!((h.stamina - 60.0).abs() < 1.0e-3 && (h.valor - 33.0).abs() < 1.0e-3, "a plain dodge's 10 and 8: {} {}", h.stamina, h.valor);
    assert!(h.rogue == Default::default(), "no cut, no flicker");
    assert_eq!(s.posture_of(m), 0);
    assert!(!beat(&s.events, w, "perfect_dodge"));
}

#[test]
fn the_hunters_call_still_primes_on_any_dodge_through_a_blow() {
    for secs in [0.1, 0.24] {
        let mut s = world();
        let h = hero_of(&mut s, "Ayla", "hunter");
        let slot = s.content.abilities[s.content.ability("call_of_the_wild").unwrap()].slot.clone();
        s.slot_ability(h, &slot, "call_of_the_wild").unwrap();
        let m = foe(&mut s, "grunt");
        dodge_for(&mut s, h, secs);
        assert_eq!(blow(&mut s, m, h), Outcome::Dodged);
        assert!(s.wild_now(h).primed, "{secs}: primed");
        assert!(!cut_open(&s, h), "no Rogue's cut for a Hunter");
    }
}

#[test]
fn the_rogue_numbers_say_what_the_doc_says() {
    let c = Content::dev();
    assert_eq!((c.dodge.evade_from, c.dodge.perfect_to, c.dodge.perfect_refund, c.dodge.perfect_posture), (0.04, 0.18, 20.0, 0.12));
    assert_eq!(c.valor.perfect, 10.0);
    let r = &c.rogue;
    assert_eq!((r.cut_secs, r.cut_reach, r.off_cancel_after, r.jab), (0.6, 6.0, 0.0, 0.5));
    assert_eq!((r.flicker_aura, r.flicker_secs, r.puff), (5215, 0.3, 1856));
    assert_eq!((r.weapon.as_str(), r.off_hand.as_str()), ("dagger", "dagger"));
    // An older content file with no [rogue] table loads these.
    let d = crate::content::RogueDef::default();
    assert_eq!((d.cut_secs, d.cut_reach, d.weapons.clone()), (r.cut_secs, r.cut_reach, r.weapons.clone()));
}
