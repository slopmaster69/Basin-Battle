//! The Hunter's bow (#110): every bow ability brings the bow into the hands, draws it with the
//! base game's draw, looses it with the bow's own release and gives the melee weapon back; and
//! Skyfall's hang over the fight, aiming its volley down at the ground.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.05;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];
/// The bow's draw: its shot (Auto Shot) begun and held, as the base game's Aimed Shot.
const DRAW: u32 = 75;

fn world() -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 7, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

fn hero(s: &mut Survival, pos: Vec3) -> Id {
    let id = s.join("ayla", "Ayla", None);
    s.move_player(id, pos, 0.0, Moving::default());
    let _ = s.step(0.0);
    id
}

/// A creature standing still, facing the hero, too tough to fall.
fn mob(s: &mut Survival, pos: Vec3) -> Id {
    let def = s.content.mob("grunt").unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.pos = pos;
    m.facing = std::f32::consts::PI;
    m.health = 9999;
    id
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        out.extend(s.step(DT));
    }
    out
}

/// Run on with these creatures held where they stand, idle.
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

/// Put an ability on its row's key (cooldowns off, Valor full) and press the key (`ex`: with Ctrl).
fn press(s: &mut Survival, p: Id, key: &str, ex: bool) -> Vec<Event> {
    let slot = s.content.abilities[idx(s, key)].slot.clone();
    s.slot_ability(p, &slot, key).unwrap();
    s.set_cooldowns(p, true);
    s.set_valor(p, 100.0);
    s.input(p, Input::Ability(key_of(&slot), ex));
    s.step(DT)
}

fn aiming(s: &Survival, p: Id) -> Option<usize> {
    match s.players[&p].action {
        Action::Aim { ability } => Some(ability),
        _ => None,
    }
}

fn hits(ev: &[Event], p: Id, m: Id) -> usize {
    ev.iter()
        .filter(|e| matches!(e, Event::Swing { attacker, victim: Some(v), damage, outcome: Outcome::Hit, anim: false, .. } if *attacker == p && *v == m && *damage > 0))
        .count()
}

fn drawn(ev: &[Event], p: Id) -> bool {
    ev.iter().any(|e| matches!(e, Event::Cast { caster, spell: DRAW, .. } if *caster == p))
}

fn loosed(ev: &[Event], p: Id) -> bool {
    ev.iter().any(|e| matches!(e, Event::Loose { caster, spell: DRAW } if *caster == p))
}

fn geared(ev: &[Event], p: Id) -> bool {
    ev.iter().any(|e| matches!(e, Event::Gear { player } if *player == p))
}

#[test]
fn every_hunter_bow_ability_shows_the_bow() {
    let s = world();
    for key in ["aimed_arrow", "multi_shot", "bank_shot", "split_arrow", "grapple_arrow", "reverse_grapple", "tripwire", "tripwire_second", "kill_shot", "skyfall", "skyfall_volley"] {
        assert!(s.content.abilities[idx(&s, key)].bow, "{key} shows the bow");
    }
    // What isn't drawn from the bow stays in the melee weapon's hands.
    for key in ["call_of_the_wild", "raptor_strike", "arrow_recall", "explosive_trap"] {
        assert!(!s.content.abilities[idx(&s, key)].bow, "{key} keeps the weapon");
    }
}

#[test]
fn aimed_shot_raises_the_bow_draws_it_as_it_charges_looses_it_and_gives_the_weapon_back() {
    let mut s = world();
    let p = hero(&mut s, ORIGIN);
    let ev = press(&mut s, p, "aimed_arrow", false);
    assert_eq!(s.players[&p].kit, Kit::Bow, "the bow in hand as it's aimed");
    assert!(geared(&ev, p), "and shown");
    assert!(!drawn(&ev, p), "not drawn before the click: it has a charge");
    s.set_aim_line(p, 0.0, 0.0);
    s.input(p, Input::Attack);
    let ev = run(&mut s, 0.6);
    assert!(drawn(&ev, p), "the click draws it");
    s.set_aim_line(p, 0.0, 0.0);
    s.input(p, Input::AttackUp);
    let ev = s.step(DT);
    assert!(loosed(&ev, p), "the release looses the bow");
    assert!(!ev.iter().any(|e| matches!(e, Event::Clip { id, .. } if *id == p)), "no spell clip over the bow's release");
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { caster, .. } if *caster == p)), "the arrow flies");
    assert_eq!(s.players[&p].kit, Kit::Bow, "the release plays out with the bow");
    let ev = run(&mut s, 1.0);
    assert_eq!(s.players[&p].kit, Kit::Melee, "then the weapon is back");
    assert!(geared(&ev, p));
}

/// Draw Aimed Shot at `k` along the x axis, hold it a moment and let go; `k` is held where it
/// stands (facing as set) until the arrow lands.
fn aimed_arrow_into(s: &mut Survival, p: Id, k: Id) -> Vec<Event> {
    press(s, p, "aimed_arrow", false);
    s.set_aim_line(p, 0.0, 0.0);
    s.input(p, Input::Attack);
    let mut ev = run_pinned(s, 0.6, &[k]);
    s.set_aim_line(p, 0.0, 0.0);
    s.input(p, Input::AttackUp);
    ev.extend(run_pinned(s, 1.5, &[k]));
    ev
}

/// The backstab is any hit in an enemy's back (Corey), an arrow's too: a creature facing away
/// takes the stab's flash, a dagger's crit and the sharp beat with the arrow; one facing the
/// archer takes the arrow alone. No word either way, and the arrow's damage is the same.
#[test]
fn an_arrow_in_a_creatures_back_is_a_backstab_and_one_in_its_face_is_not() {
    use crate::sim::cues::seen::{beat, looked, no_words, sounded};
    use crate::sim::cues::{BEAT_BACKSTAB, SOUND_BACKSTAB};
    let mut s = world();
    let p = hero(&mut s, ORIGIN);
    let k = mob(&mut s, [12.0, 0.0, 0.0]);
    // Facing away from the archer (along +x, as she looks).
    s.mobs.get_mut(&k).unwrap().facing = 0.0;
    let ev = aimed_arrow_into(&mut s, p, k);
    assert_eq!(hits(&ev, p, k), 1, "the arrow lands in its back");
    assert!(looked(&ev, k, KIT_RIPOSTE), "the stab's flash on it");
    let look = s.content.backstab.kit;
    assert!(look != 0 && looked(&ev, k, look), "and the backstab's own look, which reads from behind");
    assert!(sounded(&ev, k, SOUND_BACKSTAB), "a dagger's crit");
    assert!(beat(&ev, p, BEAT_BACKSTAB), "the sharp beat on her screen");
    assert!(no_words(&ev, p), "no word");
    // The same arrow into its face: the arrow alone.
    s.mobs.get_mut(&k).unwrap().facing = std::f32::consts::PI;
    let ev = aimed_arrow_into(&mut s, p, k);
    assert_eq!(hits(&ev, p, k), 1, "the arrow lands in its face");
    assert!(!looked(&ev, k, KIT_RIPOSTE), "no stab flash from the front");
    assert!(!looked(&ev, k, look), "nor the backstab's look");
    assert!(!sounded(&ev, k, SOUND_BACKSTAB));
    assert!(!beat(&ev, p, BEAT_BACKSTAB));
    assert!(no_words(&ev, p), "no word");
}

#[test]
fn a_bow_ability_with_nothing_to_charge_is_drawn_as_it_is_aimed() {
    let mut s = world();
    let p = hero(&mut s, ORIGIN);
    let ev = press(&mut s, p, "bank_shot", false);
    assert_eq!(s.players[&p].kit, Kit::Bow);
    assert!(drawn(&ev, p), "drawn as it's aimed");
    // Called off: the draw let down and the weapon back.
    s.input(p, Input::Guard);
    let mut ev = s.step(DT);
    s.input(p, Input::GuardUp);
    ev.extend(s.step(DT));
    assert!(ev.iter().any(|e| matches!(e, Event::CastEnd { caster } if *caster == p)), "the draw let down");
    assert_eq!(s.players[&p].kit, Kit::Melee);
    // Aimed again and clicked: loosed at once.
    press(&mut s, p, "bank_shot", false);
    s.set_aim_line(p, 0.0, 0.0);
    s.input(p, Input::Attack);
    let ev = s.step(DT);
    assert!(loosed(&ev, p), "the click looses it");
    s.input(p, Input::AttackUp);
    run(&mut s, 1.2);
    assert_eq!(s.players[&p].kit, Kit::Melee);
}

#[test]
fn a_swing_puts_the_bow_away_at_once_even_as_its_release_plays_out() {
    let mut s = world();
    let p = hero(&mut s, ORIGIN);
    // (Multi-Shot: Aimed Shot's EX.)
    press(&mut s, p, "aimed_arrow", true);
    assert_eq!(aiming(&s, p), Some(idx(&s, "multi_shot")));
    s.set_aim_line(p, 0.0, 0.0);
    s.input(p, Input::Attack);
    s.step(DT);
    s.input(p, Input::AttackUp);
    run(&mut s, 0.4);
    assert_eq!(s.players[&p].action, Action::Ready);
    s.input(p, Input::Attack);
    s.step(DT);
    assert_eq!(s.players[&p].kit, Kit::Melee, "swinging with the weapon, not the bow");
}

#[test]
fn with_no_bow_of_their_own_a_bow_ability_draws_the_camps_bow() {
    let mut s = world();
    let p = hero(&mut s, ORIGIN);
    let bow = s.content.item("bow").unwrap();
    let crossbow = s.content.item("crossbow").unwrap();
    s.players.get_mut(&p).unwrap().bow = crossbow;
    assert_eq!(s.ranged_shown(&s.players[&p]), crossbow, "on the back, what they carry");
    press(&mut s, p, "aimed_arrow", false);
    assert_eq!(s.bow_look(&s.players[&p]), bow);
    assert_eq!(s.ranged_shown(&s.players[&p]), bow, "in hand, a bow to draw");
    s.input(p, Input::Guard);
    s.step(DT);
    assert_eq!(s.ranged_shown(&s.players[&p]), crossbow, "and theirs again after");
}

#[test]
fn skyfall_hangs_at_its_top_with_the_bow_drawn_and_a_click_looses_the_volley_from_there() {
    let mut s = world();
    let p = hero(&mut s, ORIGIN);
    let k = mob(&mut s, [15.0, 0.0, 0.0]);
    let mut ev = press(&mut s, p, "skyfall", false);
    assert_eq!(s.players[&p].kit, Kit::Bow, "the bow in hand from the leap");
    ev.extend(run_pinned(&mut s, 0.5, &[k]));
    assert!(ev.iter().any(|e| matches!(e, Event::Hop { player, lift, .. } if *player == p && *lift > 10.0)), "straight up");
    assert!(aiming(&s, p).is_none(), "still going up");
    // The key again on the way up does nothing.
    s.input(p, Input::Ability(key_of("ultimate"), false));
    ev.extend(run_pinned(&mut s, 0.5, &[k]));
    // At the top: held there, the bow drawn, aiming the volley's circle.
    assert_eq!(aiming(&s, p), Some(idx(&s, "skyfall_volley")), "hanging, aiming the volley");
    assert!(ev.iter().any(|e| matches!(e, Event::Rooted { player, on: true } if *player == p)), "held in the air");
    assert!(ev.iter().any(|e| matches!(e, Event::FeatherFall { player, on: true } if *player == p)));
    assert!(drawn(&ev, p), "the bow drawn");
    // The hover's pose (the Jump clip) on the legs alone: the arms stay on the bow.
    assert!(
        ev.iter().any(|e| matches!(e, Event::Clip { id, clip: 38, body: ClipBody::Lower, secs, .. } if *id == p && *secs > 5.5)),
        "the hang's pose on the lower body for the whole hang: {:?}",
        ev.iter().filter(|e| matches!(e, Event::Clip { .. } | Event::React { .. })).collect::<Vec<_>>()
    );
    assert!(!ev.iter().any(|e| matches!(e, Event::React { id, .. } if *id == p)), "no whole-body reaction over the draw");
    let h = s.players[&p].pos[2];
    assert!(h > 3.0, "up there: {h}");
    assert_eq!(s.hang_time(p).map(|(_, total)| total), Some(5.5), "for 5.5 sec");
    // Untouchable, and nothing takes hold up there.
    let hp = s.players[&p].health;
    let out = s.strike_player(k, p, 20, [2.0, 0.0, 0.0], true, false, true, Blow::Light);
    assert!(matches!(out, Outcome::Dodged), "{out:?}");
    assert!(!s.apply_cc(p, k, "stun", 2.0, 0.0, 0.0, "test"), "no stun");
    assert_eq!(s.players[&p].health, hp);
    // A roll or a right click doesn't bring them down; it hangs on, aiming.
    s.input(p, Input::Dodge);
    s.input(p, Input::Guard);
    s.input(p, Input::GuardUp);
    run_pinned(&mut s, 1.0, &[k]);
    assert_eq!(aiming(&s, p), Some(idx(&s, "skyfall_volley")), "still hanging");
    // The click looses the volley at the circle under the reticle, from the hero; then down.
    s.set_aim_line(p, 0.0, -(h + 1.9).atan2(15.0));
    s.input(p, Input::Attack);
    let mut ev = s.step(DT);
    s.input(p, Input::AttackUp);
    assert!(loosed(&ev, p), "the bow's release");
    assert!(ev.iter().any(|e| matches!(e, Event::Rooted { player, on: false } if *player == p)), "let go");
    assert!(ev.iter().filter(|e| matches!(e, Event::Missile { caster, .. } if *caster == p)).count() >= 12, "the arrows burst out from the hero");
    ev.extend(run_pinned(&mut s, 1.6, &[k]));
    assert!(hits(&ev, p, k) >= 2, "arrows on it: {}", hits(&ev, p, k));
    assert!(s.planted_of(p).len() >= 10, "and in the ground");
    assert_eq!(s.players[&p].kit, Kit::Melee, "the weapon back");
    // Untouchable till they land; then the slow fall ends.
    let out = s.strike_player(k, p, 20, [2.0, 0.0, 0.0], true, false, true, Blow::Light);
    assert!(matches!(out, Outcome::Dodged), "on the way down: {out:?}");
    s.landed(p);
    let ev = s.step(DT);
    assert!(ev.iter().any(|e| matches!(e, Event::FeatherFall { player, on: false } if *player == p)), "landed");
    let out = s.strike_player(k, p, 20, [2.0, 0.0, 0.0], true, false, false, Blow::Light);
    assert!(!matches!(out, Outcome::Dodged), "down again, it can be struck: {out:?}");
}

#[test]
fn skyfall_left_hanging_looses_the_volley_at_the_reticle_as_the_hang_ends() {
    let mut s = world();
    let p = hero(&mut s, ORIGIN);
    let k = mob(&mut s, [15.0, 0.0, 0.0]);
    press(&mut s, p, "skyfall", false);
    run_pinned(&mut s, 1.0, &[k]);
    assert_eq!(aiming(&s, p), Some(idx(&s, "skyfall_volley")));
    let h = s.players[&p].pos[2];
    // The reticle as the client last told it: on the creature.
    s.set_aim_line(p, 0.0, -(h + 1.9).atan2(15.0));
    let ev = run_pinned(&mut s, 4.5, &[k]);
    assert_eq!(hits(&ev, p, k), 0, "nothing yet");
    assert_eq!(aiming(&s, p), Some(idx(&s, "skyfall_volley")), "still hanging");
    let ev = run_pinned(&mut s, 2.5, &[k]);
    assert!(loosed(&ev, p), "loosed by itself");
    assert!(hits(&ev, p, k) >= 2, "at the reticle: {}", hits(&ev, p, k));
    assert!(ev.iter().any(|e| matches!(e, Event::Rooted { player, on: false } if *player == p)), "and down");
    assert!(aiming(&s, p).is_none());
}
