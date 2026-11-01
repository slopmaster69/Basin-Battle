//! Combos between casts, on abilities made for the test: a recast (the key again within a
//! window fires a second ability for free: a blink there and back, an aimed throw after a slam),
//! a dash that runs through every foe in its way, and an empower (one card's cast makes another's
//! next press its EX version, for no Valor).

use crate::content::{AbilityDef, Content};
use crate::land::FlatLand;
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.05;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];

/// The built-in rules with these abilities (TOML, `[[ability]]` tables) added.
fn world(extra: &str) -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    #[derive(serde::Deserialize)]
    struct Extra {
        ability: Vec<AbilityDef>,
    }
    let e: Extra = toml::from_str(extra).expect("test abilities");
    for mut a in e.ability {
        a.bench = true;
        c.abilities.push(a);
    }
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
    id
}

fn mob(s: &mut Survival, key: &str, pos: Vec3) -> Id {
    let def = s.content.mob(key).unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.pos = pos;
    m.facing = std::f32::consts::PI;
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

fn idx(s: &Survival, key: &str) -> usize {
    s.content.ability(key).unwrap()
}

/// Own an ability and put it on its key (cooldowns left on).
fn learn(s: &mut Survival, p: Id, key: &str) {
    let slot = s.content.abilities[idx(s, key)].slot.clone();
    s.players.get_mut(&p).unwrap().owned.insert(key.to_string());
    s.slot_ability(p, &slot, key).unwrap();
}

/// A key pressed (`ex`: with Ctrl).
fn press(s: &mut Survival, p: Id, slot: &str, ex: bool) -> Vec<Event> {
    s.input(p, Input::Ability(key_of(slot), ex));
    s.step(DT)
}

/// A tap of the click (an aimed ability goes).
fn click(s: &mut Survival, p: Id) -> Vec<Event> {
    s.input(p, Input::Attack);
    let mut ev = s.step(DT);
    s.input(p, Input::AttackUp);
    ev.extend(s.step(DT));
    ev
}

fn aiming(s: &Survival, p: Id) -> Option<usize> {
    match s.players[&p].action {
        Action::Aim { ability } => Some(ability),
        _ => None,
    }
}

fn missile(ev: &[Event], spell: u32) -> bool {
    ev.iter().any(|e| matches!(e, Event::Missile { spell: s, .. } if *s == spell))
}

const STEP_BACK: &str = r#"
[[ability]]
key = "t_step"
name = "Step"
school = "rogue"
slot = "utility"
kind = "blink"
range = [12.0, 12.0]
cooldown = 12.0
spell = 1856
recast = "t_step_back"
recast_secs = 3.0

[[ability]]
key = "t_step_back"
name = "Step Back"
inner = true
slot = "utility"
kind = "blink"
recast_to_start = true
spell = 1856
"#;

/// Blink out along the facing (the reticle's line, level).
fn step_out(s: &mut Survival, p: Id) {
    press(s, p, "utility", false);
    assert_eq!(aiming(s, p), Some(idx(s, "t_step")), "the blink is aimed");
    click(s, p);
    assert!(s.players[&p].pos[0] > 8.0, "gone out: {:?}", s.players[&p].pos);
}

#[test]
fn a_blink_back_goes_and_comes_back_and_its_window_closes_after_its_time() {
    let mut s = world(STEP_BACK);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    learn(&mut s, p, "t_step");
    step_out(&mut s, p);
    let back = idx(&s, "t_step_back");
    let keys = s.recast_keys(p);
    assert_eq!(keys.len(), 1, "the key is armed: {keys:?}");
    assert_eq!((keys[0].0, keys[0].1), (key_of("utility") as usize, back), "on its own key, for its recast");
    // Pressed again within the window: straight back to where it began (no aim).
    run(&mut s, 0.5);
    let ev = press(&mut s, p, "utility", false);
    assert!(dist2d(s.players[&p].pos, ORIGIN) < 0.5, "back where it began: {:?}", s.players[&p].pos);
    assert!(missile(&ev, 1856), "its look");
    assert!(s.recast_keys(p).is_empty(), "the window closed with the recast");
    assert!(ev.iter().any(|e| matches!(e, Event::Abilities { player } if *player == p)), "the bar is told");

    // Another go, left unused: the window closes after its seconds, and the key is just on
    // cooldown then.
    let mut s = world(STEP_BACK);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    learn(&mut s, p, "t_step");
    step_out(&mut s, p);
    let out = s.players[&p].pos;
    run(&mut s, 2.6);
    assert_eq!(s.recast_keys(p).len(), 1, "still open short of its 3 s");
    let ev = run(&mut s, 0.6);
    assert!(s.recast_keys(p).is_empty(), "closed after 3 s");
    assert!(s.players[&p].combos.recasts.is_empty(), "and gone");
    assert!(ev.iter().any(|e| matches!(e, Event::Abilities { player } if *player == p)), "the bar is told it closed");
    let ev = press(&mut s, p, "utility", false);
    assert!(dist2d(s.players[&p].pos, out) < 0.1, "no step back once it's closed");
    assert!(aiming(&s, p).is_none(), "nor a second blink: it's cooling down");
    assert!(ev.iter().any(|e| matches!(e, Event::Flash { text, .. } if text.contains("ready in"))), "said so");
}

#[test]
fn a_recast_costs_no_cooldown_and_a_third_press_is_refused_by_the_cooldown() {
    let mut s = world(STEP_BACK);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    learn(&mut s, p, "t_step");
    s.set_valor(p, 40.0);
    let step = idx(&s, "t_step");
    let cast_at = s.clock;
    step_out(&mut s, p);
    let cd = s.cooldown(p, step);
    assert_eq!(cd.charges, 0, "the first cast spent the cooldown");
    assert!((cd.next_at - (cast_at + 12.0)).abs() < 0.3, "12 s from the cast: {cd:?}");
    run(&mut s, 0.5);
    press(&mut s, p, "utility", false);
    assert!(dist2d(s.players[&p].pos, ORIGIN) < 0.5, "back");
    assert_eq!(s.cooldown(p, step), cd, "the recast spent nothing more");
    assert!(s.players[&p].cooldowns.keys().all(|k| *k == step), "nor a cooldown of its own: {:?}", s.players[&p].cooldowns);
    assert!((s.players[&p].valor - 40.0).abs() < 0.01, "nor Valor");
    // The third press: the cooldown says no.
    run(&mut s, 0.5);
    let ev = press(&mut s, p, "utility", false);
    assert!(aiming(&s, p).is_none() && dist2d(s.players[&p].pos, ORIGIN) < 0.5, "nothing goes");
    assert!(ev.iter().any(|e| matches!(e, Event::Flash { text, .. } if text.contains("ready in"))), "the cooldown refused it");
}

const SLAM_HURL: &str = r#"
[[ability]]
key = "t_slam"
name = "Slam"
school = "warrior"
slot = "close"
kind = "nova"
radius = [4.0, 4.0]
cooldown = 10.0
spell = 6343
recast = "t_hurl"
recast_secs = 3.0

[[ability]]
key = "t_hurl"
name = "Hurl"
inner = true
slot = "close"
kind = "projectile"
speed = 40.0
range = [20.0, 20.0]
radius = [0.4, 0.4]
damage = [10.0, 10.0]
spell = 16075
"#;

#[test]
fn an_aimed_recast_comes_up_on_the_key_and_goes_on_the_click() {
    let mut s = world(SLAM_HURL);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    learn(&mut s, p, "t_slam");
    s.set_valor(p, 40.0);
    let hurl = idx(&s, "t_hurl");
    let ev = press(&mut s, p, "close", false);
    assert!(missile(&ev, 6343), "the slam goes at once");
    run(&mut s, 0.5);
    press(&mut s, p, "close", false);
    assert_eq!(aiming(&s, p), Some(hurl), "the recast is aimed");
    // The key again calls the aim off; the window stays open.
    press(&mut s, p, "close", false);
    assert!(aiming(&s, p).is_none(), "called off");
    assert_eq!(s.recast_keys(p).len(), 1, "still armed");
    press(&mut s, p, "close", false);
    assert_eq!(aiming(&s, p), Some(hurl), "up again");
    let ev = click(&mut s, p);
    assert!(missile(&ev, 16075), "thrown on the click");
    assert!(s.projectiles.iter().any(|x| x.ability == hurl), "in flight");
    assert!(s.recast_keys(p).is_empty(), "the window closed");
    assert!((s.players[&p].valor - 40.0).abs() < 0.01, "for no Valor");
    assert_eq!(s.cooldown(p, idx(&s, "t_slam")).charges, 0, "the slam's cooldown still running");
}

const DASHES: &str = r#"
[[ability]]
key = "t_rush"
name = "Rush"
school = "warrior"
slot = "utility"
kind = "dash"
range = [16.0, 16.0]
damage = [10.0, 10.0]
through = true
spell = 100

[[ability]]
key = "t_charge"
name = "Charge"
school = "warrior"
slot = "utility"
kind = "dash"
range = [16.0, 16.0]
damage = [10.0, 10.0]
spell = 100
"#;

/// Dash along +x at two grunts in a line (4 and 12 yd off): the world, the hero, the dash's
/// length, the grunts with their health before it, and what happened as it began.
fn dash_into_two(key: &str) -> (Survival, Id, f32, (Id, u32), (Id, u32), Vec<Event>) {
    let mut s = world(DASHES);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let a = mob(&mut s, "grunt", [4.0, 0.0, 0.0]);
    let b = mob(&mut s, "grunt", [12.0, 0.0, 0.0]);
    let (ha, hb) = (s.mobs[&a].health, s.mobs[&b].health);
    learn(&mut s, p, key);
    s.set_cooldowns(p, true);
    press(&mut s, p, "utility", false);
    let ev = click(&mut s, p);
    let len = ev.iter().find_map(|e| match e { Event::Dash { player, speed, secs, .. } if *player == p => Some(speed * secs), _ => None }).expect("it dashes");
    (s, p, len, (a, ha), (b, hb), ev)
}

#[test]
fn a_dash_through_strikes_each_foe_it_passes_while_a_dash_stops_at_the_first() {
    let (mut s, p, len, (a, ha), (b, hb), mut ev) = dash_into_two("t_rush");
    assert!(len > 15.0, "through: on to its range, past both: {len}");
    // As it passes them: the near one first (a tenth of a second in), the far one later.
    ev.extend(run(&mut s, 0.1));
    assert!(s.mobs[&a].health < ha, "the first struck as it's passed");
    assert_eq!(s.mobs[&b].health, hb, "the second not reached yet");
    ev.extend(run(&mut s, 0.6));
    assert!(s.mobs[&b].health < hb, "the second struck too");
    let hits = |v: Id| ev.iter().filter(|e| matches!(e, Event::Swing { attacker, victim: Some(x), outcome: Outcome::Hit, .. } if *attacker == p && *x == v)).count();
    assert_eq!((hits(a), hits(b)), (1, 1), "each struck once");
    assert!(s.players[&p].combos.through.is_empty(), "nobody left to strike");

    let (mut s, _, len, (a, ha), (b, hb), _) = dash_into_two("t_charge");
    assert!(len < 4.0, "a plain dash stops at the first body: {len}");
    run(&mut s, 0.7);
    assert!(s.mobs[&a].health < ha, "the first struck at its end");
    assert_eq!(s.mobs[&b].health, hb, "the second untouched");
}

const COMBO: &str = r#"
[[ability]]
key = "t_frost"
name = "Frost"
school = "mage"
slot = "close"
kind = "nova"
radius = [6.0, 6.0]
cooldown = 8.0
spell = 122
empower = { ability = "t_bolt", secs = 4.0 }

[[ability]]
key = "t_bolt"
name = "Bolt"
school = "mage"
slot = "ranged"
kind = "projectile"
speed = 40.0
range = [30.0, 30.0]
radius = [0.3, 0.3]
damage = [10.0, 10.0]
cooldown = 0.5
spell = 133

[[ability]]
key = "t_bolt_ex"
name = "Big Bolt"
ex_of = "t_bolt"
slot = "ranged"
kind = "projectile"
speed = 40.0
range = [30.0, 30.0]
radius = [0.3, 0.3]
damage = [20.0, 20.0]
valor = 25
spell = 2948
"#;

#[test]
fn an_empower_makes_the_next_press_its_ex_version_for_free_then_it_is_normal_again() {
    let mut s = world(COMBO);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    learn(&mut s, p, "t_frost");
    learn(&mut s, p, "t_bolt");
    s.set_valor(p, 50.0);
    let (bolt, ex) = (idx(&s, "t_bolt"), idx(&s, "t_bolt_ex"));
    let ev = press(&mut s, p, "close", false);
    assert!(missile(&ev, 122), "the nova goes");
    let em = s.empowered_keys(p);
    assert_eq!(em.len(), 1, "the bolt's key empowered: {em:?}");
    assert_eq!(em[0].0, key_of("ranged") as usize, "on the other key");
    assert!((em[0].1 - (s.clock + 4.0)).abs() < 0.2, "for 4 s");
    run(&mut s, 0.5);
    // The plain key: the EX version, free.
    press(&mut s, p, "ranged", false);
    assert_eq!(aiming(&s, p), Some(ex), "its EX version is aimed");
    let ev = click(&mut s, p);
    assert!(missile(&ev, 2948), "the EX version flies");
    assert!((s.players[&p].valor - 50.0).abs() < 0.01, "Valor unchanged: {}", s.players[&p].valor);
    assert!(s.empowered_keys(p).is_empty() && s.players[&p].combos.empowered.is_empty(), "the proc is spent");
    assert_eq!(s.cooldown(p, bolt).charges, 0, "the bolt's cooldown spent as usual");
    // Later: the plain key is the plain bolt again, and the EX version costs its Valor.
    run(&mut s, 0.8);
    press(&mut s, p, "ranged", false);
    assert_eq!(aiming(&s, p), Some(bolt), "the bolt again");
    let ev = click(&mut s, p);
    assert!(missile(&ev, 133) && !missile(&ev, 2948), "the plain bolt");
    run(&mut s, 0.8);
    press(&mut s, p, "ranged", true);
    click(&mut s, p);
    assert!((s.players[&p].valor - 25.0).abs() < 0.01, "a paid EX: {}", s.players[&p].valor);
}

#[test]
fn an_empower_left_unused_runs_out() {
    let mut s = world(COMBO);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    learn(&mut s, p, "t_frost");
    learn(&mut s, p, "t_bolt");
    press(&mut s, p, "close", false);
    assert_eq!(s.empowered_keys(p).len(), 1);
    let ev = run(&mut s, 4.2);
    assert!(s.empowered_keys(p).is_empty(), "gone after 4 s");
    assert!(ev.iter().any(|e| matches!(e, Event::Abilities { player } if *player == p)), "the bar is told");
    press(&mut s, p, "ranged", false);
    assert_eq!(aiming(&s, p), Some(idx(&s, "t_bolt")), "the plain bolt");
}
