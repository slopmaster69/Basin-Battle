//! The redesigned warrior, paladin and hunter cards (#40), each cast from the real content: the thing that makes it
//! play differently.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.05;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];

/// The built-in rules (a dev build: every card owned), friendly fire on or off.
fn world_with(pvp: bool) -> Survival {
    let mut c = Content::dev();
    if !pvp {
        c.pvp = Default::default();
    }
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 7, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

fn world() -> Survival {
    world_with(false)
}

fn hero(s: &mut Survival, name: &str, pos: Vec3, facing: f32) -> Id {
    let id = s.join(&name.to_lowercase(), name, None);
    s.move_player(id, pos, facing, Moving::default());
    let _ = s.step(0.0);
    id
}

/// A creature standing still, facing the hero.
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

/// Run on with these creatures held where they stand, idle (struck, they'd come at the hero).
fn run_pinned(s: &mut Survival, secs: f32, mobs: &[Id]) -> Vec<Event> {
    let at: Vec<(Id, Vec3)> = mobs.iter().filter_map(|m| Some((*m, s.mobs.get(m)?.pos))).collect();
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        for (m, pos) in &at {
            if let Some(x) = s.mobs.get_mut(m) {
                x.state = MobState::Idle { wander_at: 1e9 };
                x.path.clear();
                x.pos = *pos;
            }
        }
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

/// Put a card on its row's key (cooldowns off) and press the key (`ex`: with Ctrl). An aimed one
/// comes up and waits for the click.
fn press(s: &mut Survival, p: Id, key: &str, ex: bool) -> Vec<Event> {
    let slot = s.content.abilities[idx(s, key)].slot.clone();
    s.slot_ability(p, &slot, key).unwrap();
    s.set_cooldowns(p, true);
    s.input(p, Input::Ability(key_of(&slot), ex));
    s.step(DT)
}

/// The key on that row pressed again, as it is (no slotting, no cooldowns touched).
fn press_again(s: &mut Survival, p: Id, slot: &str) -> Vec<Event> {
    s.input(p, Input::Ability(key_of(slot), false));
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

/// The click held `secs` (the reticle kept on `aim`, facing and elevation, as the client keeps
/// saying it) and let go.
fn draw(s: &mut Survival, p: Id, secs: f32, aim: (f32, f32), mobs: &[Id]) -> Vec<Event> {
    s.set_aim_line(p, aim.0, aim.1);
    s.input(p, Input::Attack);
    let mut ev = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        s.set_aim_line(p, aim.0, aim.1);
        ev.extend(run_pinned(s, DT, mobs));
    }
    s.set_aim_line(p, aim.0, aim.1);
    s.input(p, Input::AttackUp);
    ev.extend(run_pinned(s, DT, mobs));
    ev
}

fn aiming(s: &Survival, p: Id) -> Option<usize> {
    match s.players[&p].action {
        Action::Aim { ability } => Some(ability),
        _ => None,
    }
}

/// The damage of each of `p`'s blows that landed on `m`.
fn hits(ev: &[Event], p: Id, m: Id) -> Vec<u32> {
    ev.iter()
        .filter_map(|e| match e {
            Event::Swing { attacker, victim: Some(v), damage, outcome: Outcome::Hit, anim: false, .. } if *attacker == p && *v == m && *damage > 0 => Some(*damage),
            _ => None,
        })
        .collect()
}

fn flashed(ev: &[Event], p: Id, text: &str) -> bool {
    ev.iter().any(|e| matches!(e, Event::Flash { player, text: t } if *player == p && t.contains(text)))
}

fn missiles(ev: &[Event], p: Id, spell: u32) -> usize {
    ev.iter().filter(|e| matches!(e, Event::Missile { caster, spell: s, .. } if *caster == p && *s == spell)).count()
}

/// The elevation that looks at the ground this far ahead (from the eye, 1.9 yd up).
fn down_to(dist: f32) -> f32 {
    -(1.9f32).atan2(dist)
}

/// A hero's save from before the redesign: of this class, owning these old cards, each on its key
/// (slot, key). Loaded in a world without the dev build (so what's owned is what was saved).
fn load_old_save(class: &str, keys: &[(&str, &str)]) -> (Survival, Id) {
    let mut c = Content::dev();
    c.pvp = Default::default();
    c.dev.full_tree = false;
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 7, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let mut save = s.leave(p).unwrap();
    save.class = class.to_string();
    save.owned = keys.iter().map(|(_, k)| k.to_string()).collect();
    save.loadout = keys.iter().map(|(slot, k)| (slot.to_string(), k.to_string())).collect();
    let q = s.join("ayla", "Ayla", Some(&save));
    (s, q)
}

/// The hero owns each new card, on its row's key, and nothing the rules don't know.
fn has_on_keys(s: &Survival, p: Id, cards: &[(&str, &str)]) {
    let h = &s.players[&p];
    for (slot, key) in cards {
        assert!(h.owned.contains(*key), "{key} owned: {:?}", h.owned);
        assert_eq!(h.loadout.get(*slot).map(String::as_str), Some(*key), "{key} on {slot}");
    }
    assert!(h.owned.iter().all(|k| s.content.ability(k).is_some()), "nothing unknown: {:?}", h.owned);
}

// --- Hunter: set up, then snipe -------------------------------------------------------------------

#[test]
fn aimed_shot_drawn_longer_flies_farther_and_harder_and_a_full_draw_goes_through_the_line() {
    // Farther: a boar 33 yd off is out of a short draw's reach, not a full one's.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let far = mob(&mut s, "boar", [33.0, 0.0, 0.0]);
    let aim = (0.0, down_to(40.0));
    press(&mut s, p, "aimed_arrow", false);
    assert_eq!(aiming(&s, p), Some(idx(&s, "aimed_arrow")));
    let mut ev = draw(&mut s, p, 0.2, aim, &[far]);
    ev.extend(run_pinned(&mut s, 1.5, &[far]));
    assert!(hits(&ev, p, far).is_empty(), "a short draw falls short");
    press(&mut s, p, "aimed_arrow", false);
    let mut ev = draw(&mut s, p, 1.5, aim, &[far]);
    ev.extend(run_pinned(&mut s, 1.5, &[far]));
    assert_eq!(hits(&ev, p, far).len(), 1, "a full draw reaches it");

    // Through the line: a short draw stops at the first boar, a full one goes on into the second,
    // and hits harder.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let near = mob(&mut s, "boar", [8.0, 0.0, 0.0]);
    let behind = mob(&mut s, "boar", [13.0, 0.0, 0.0]);
    let pins = [near, behind];
    let aim = (0.0, down_to(20.0));
    press(&mut s, p, "aimed_arrow", false);
    let mut ev = draw(&mut s, p, 0.2, aim, &pins);
    ev.extend(run_pinned(&mut s, 1.0, &pins));
    let short = hits(&ev, p, near);
    assert_eq!(short.len(), 1, "the first struck");
    assert!(hits(&ev, p, behind).is_empty(), "and it stops there");
    press(&mut s, p, "aimed_arrow", false);
    let mut ev = draw(&mut s, p, 1.5, aim, &pins);
    ev.extend(run_pinned(&mut s, 1.0, &pins));
    assert_eq!(missiles(&ev, p, 3044), 1, "fully drawn: the glowing arrow");
    let full = hits(&ev, p, near);
    assert_eq!(full.len(), 1);
    assert!(full[0] > short[0], "harder: {short:?} then {full:?}");
    assert_eq!(hits(&ev, p, behind).len(), 1, "and on through into the one behind");
}

#[test]
fn a_heavy_blow_spoils_the_aimed_shot_draw() {
    let mut s = world_with(true);
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [2.0, 0.0, 0.0], std::f32::consts::PI);
    press(&mut s, a, "aimed_arrow", false);
    s.input(a, Input::Attack);
    run(&mut s, 0.5);
    assert!(matches!(s.players[&a].action, Action::Charge { .. }), "drawing");
    s.strike_player(b, a, 20, [2.0, 0.0, 0.0], true, true, true, Blow::Heavy);
    assert!(!matches!(s.players[&a].action, Action::Charge { .. }), "the draw is spoiled");
    let ev = run(&mut s, 1.0);
    assert_eq!(missiles(&ev, a, 75) + missiles(&ev, a, 3044), 0, "no arrow");
}

#[test]
fn multi_shot_looses_five_arrows_in_a_fan() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    // One on each arrow's line, 7 yd out, 10° apart; one well outside the fan.
    let fan: Vec<Id> = [-20.0f32, -10.0, 0.0, 10.0, 20.0].iter().map(|deg| mob(&mut s, "grunt", [7.0 * deg.to_radians().cos(), 7.0 * deg.to_radians().sin(), 0.0])).collect();
    let wide = mob(&mut s, "grunt", [7.0 * 50f32.to_radians().cos(), 7.0 * 50f32.to_radians().sin(), 0.0]);
    let mut pins = fan.clone();
    pins.push(wide);
    s.set_valor(p, 25.0);
    press(&mut s, p, "aimed_arrow", true);
    assert_eq!(aiming(&s, p), Some(idx(&s, "multi_shot")), "Ctrl: the Multi-Shot");
    s.set_aim_line(p, 0.0, down_to(14.0));
    let mut ev = click(&mut s, p);
    ev.extend(run_pinned(&mut s, 1.0, &pins));
    assert_eq!(missiles(&ev, p, 75), 5, "five arrows, no draw");
    for m in &fan {
        assert_eq!(hits(&ev, p, *m).len(), 1, "each arrow its own foe");
    }
    assert!(hits(&ev, p, wide).is_empty(), "nothing outside the fan");
}

#[test]
fn freezing_trap_readies_a_free_multi_shot_and_the_frozen_shatter() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "grunt", [6.0, 0.0, 0.0]);
    s.slot_ability(p, "ranged", "aimed_arrow").unwrap();
    s.set_valor(p, 50.0);
    // The Freezing Trap at its feet.
    press(&mut s, p, "explosive_trap", true);
    assert_eq!(aiming(&s, p), Some(idx(&s, "freezing_trap")));
    s.set_aim_line(p, 0.0, down_to(6.0));
    click(&mut s, p);
    let em = s.empowered_keys(p);
    assert_eq!(em.len(), 1, "Aimed Shot's key readied: {em:?}");
    assert_eq!(em[0].0, key_of("ranged") as usize);
    assert!((em[0].1 - (s.clock + 8.0)).abs() < 0.3, "for 8 s");
    run_pinned(&mut s, 1.2, &[k]);
    assert!(s.has_cc(k, "petrify"), "sprung: frozen");
    // Aimed Shot's key: the Multi-Shot, free, and the frozen one shatters.
    let valor = s.players[&p].valor;
    press_again(&mut s, p, "ranged");
    assert_eq!(aiming(&s, p), Some(idx(&s, "multi_shot")), "the Multi-Shot comes up");
    s.set_aim_line(p, 0.0, down_to(14.0));
    let mut ev = click(&mut s, p);
    ev.extend(run_pinned(&mut s, 1.0, &[k]));
    assert_eq!(missiles(&ev, p, 75), 5, "five arrows");
    assert!(s.players[&p].valor + 0.01 >= valor, "no Valor spent: {valor} then {}", s.players[&p].valor);
    assert!(flashed(&ev, p, "Shatter!"), "the ice shatters");
    assert!(!s.has_cc(k, "petrify"));
    assert!(s.empowered_keys(p).is_empty(), "spent");
    // The next press is the plain draw again.
    run(&mut s, 0.5);
    press_again(&mut s, p, "ranged");
    assert_eq!(aiming(&s, p), Some(idx(&s, "aimed_arrow")));
}

#[test]
fn flare_lobbed_over_blinds_the_foes_where_it_lands_until_they_are_struck() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let there = mob(&mut s, "grunt", [14.0, 0.0, 0.0]);
    let beside = mob(&mut s, "grunt", [14.0, 2.5, 0.0]);
    let away = mob(&mut s, "grunt", [14.0, 8.0, 0.0]);
    let pins = [there, beside, away];
    press(&mut s, p, "flare", false);
    assert_eq!(aiming(&s, p), Some(idx(&s, "flare")), "aimed, not a stance any more");
    s.set_aim_line(p, 0.0, down_to(14.0));
    let mut ev = click(&mut s, p);
    assert!(missiles(&ev, p, 2812) >= 1, "the flare flies");
    ev.extend(run_pinned(&mut s, 0.3, &pins));
    assert!(!s.has_cc(there, "incapacitate"), "still in the air");
    ev.extend(run_pinned(&mut s, 0.7, &pins));
    assert!(s.has_cc(there, "incapacitate") && s.has_cc(beside, "incapacitate"), "blinded where it landed");
    assert!(!s.has_cc(away, "incapacitate"), "not one well away");
    assert!(hits(&ev, p, there).is_empty(), "it does no harm");
    // A blow ends it.
    s.hurt_mob(there, p, 1);
    assert!(!s.has_cc(there, "incapacitate"), "struck: it sees again");
    assert!(s.has_cc(beside, "incapacitate"));
}

#[test]
fn kill_shot_wants_its_whole_draw_goes_through_the_line_and_finishes_the_weak() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let first = mob(&mut s, "brave", [6.0, 0.0, 0.0]);
    let weak = mob(&mut s, "brave", [11.0, 0.0, 0.0]);
    let last = mob(&mut s, "grunt", [16.0, 0.0, 0.0]);
    let pins = [first, weak, last];
    let max = s.mobs[&weak].max_health;
    s.mobs.get_mut(&weak).unwrap().health = (max as f32 * 0.24) as u32;
    let aim = (0.0, down_to(20.0));
    s.set_valor(p, 100.0);
    // Let go before its draw: lost, and nothing spent.
    press(&mut s, p, "kill_shot", false);
    let ev = draw(&mut s, p, 0.5, aim, &pins);
    assert_eq!(missiles(&ev, p, 3044), 0, "let go too soon");
    assert_eq!(s.players[&p].action, Action::Ready);
    assert_eq!(s.players[&p].valor, 100.0, "nothing spent");
    // The whole draw.
    press(&mut s, p, "kill_shot", false);
    let mut ev = draw(&mut s, p, 1.1, aim, &pins);
    ev.extend(run_pinned(&mut s, 1.0, &pins));
    assert_eq!(missiles(&ev, p, 3044), 1, "loosed");
    assert_eq!(hits(&ev, p, first).len(), 1, "the first struck");
    assert!(s.alive(first), "a hale one lives");
    assert!(!s.alive(weak), "the weak one finished");
    assert!(flashed(&ev, p, "Execute!"));
    assert_eq!(hits(&ev, p, last).len(), 1, "and on through to the last");
}

#[test]
fn an_old_hunters_save_comes_back_with_the_new_cards() {
    let (s, p) = load_old_save("hunter", &[("ranged", "arcane_shot"), ("close", "explosive_trap"), ("utility", "disengage"), ("defense", "deterrence"), ("ultimate", "volley")]);
    has_on_keys(&s, p, &[("ranged", "aimed_arrow"), ("close", "explosive_trap"), ("utility", "disengage"), ("defense", "flare"), ("ultimate", "kill_shot")]);
    assert_eq!(crate::save::renamed("volley"), "kill_shot");
}
