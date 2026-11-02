//! Crowd control on foes that hold their ground: a boss (a champion), a totem, or a creature
//! winding up a blow no guard stops shrugs off stuns, fears, freezes, roots, silences, staggers,
//! knockbacks, pulls and swaps, and the hero is told "Immune". A snare, a burn and a mark still
//! take. An ultimate's effects take on them all the same.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::MobState;
use crate::sim::*;

fn world() -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 7, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

fn hero(s: &mut Survival, pos: Vec3) -> Id {
    let id = s.join("ann", "Ann", None);
    s.move_player(id, pos, 0.0, Moving::default());
    let _ = s.step(0.0);
    id
}

fn mob(s: &mut Survival, def: usize, pos: Vec3) -> Id {
    let id = s.spawn_mob(def, pos, None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.pos = pos;
    id
}

fn boss_def(s: &Survival) -> usize {
    s.content.mobs.iter().position(|m| m.boss && !m.inert()).expect("a boss in the rules")
}

fn plain_def(s: &Survival) -> usize {
    s.content.mob("wolf").expect("a wolf")
}

/// An ability's effects given by `owner` to `victim` (as its landing does).
fn give(s: &mut Survival, owner: Id, victim: Id, ability: &str) -> Vec<Event> {
    let d = s.content.abilities[s.content.ability(ability).unwrap()].clone();
    s.fx_from = s.unit_pos(owner);
    s.apply_effects(owner, victim, &d.effects, 1.0, &d.name);
    s.fx_from = None;
    std::mem::take(&mut s.events)
}

fn told_immune(events: &[Event], hero: Id) -> bool {
    events.iter().any(|e| matches!(e, Event::Flash { player, text } if *player == hero && text.ends_with(": Immune")))
}

#[test]
fn a_boss_shrugs_off_a_stun_that_takes_on_an_ordinary_creature() {
    let mut s = world();
    let a = hero(&mut s, [0.0, 0.0, 0.0]);
    let (bd, wd) = (boss_def(&s), plain_def(&s));
    let boss = mob(&mut s, bd, [3.0, 0.0, 0.0]);
    let wolf = mob(&mut s, wd, [3.0, 3.0, 0.0]);
    let ev = give(&mut s, a, boss, "shadowfury");
    assert!(!s.has_cc(boss, "stun"), "the boss isn't stunned");
    assert!(told_immune(&ev, a), "and the hero is told it's immune");
    give(&mut s, a, wolf, "shadowfury");
    assert!(s.has_cc(wolf, "stun"), "the wolf is");
}

#[test]
fn a_boss_is_not_thrown_back_but_still_slowed() {
    let mut s = world();
    let a = hero(&mut s, [0.0, 0.0, 0.0]);
    let (bd, wd) = (boss_def(&s), plain_def(&s));
    let boss = mob(&mut s, bd, [3.0, 0.0, 0.0]);
    let wolf = mob(&mut s, wd, [0.0, 3.0, 0.0]);
    let d = &s.content.abilities[s.content.ability("thunderstorm").unwrap()];
    assert!(d.effects.iter().any(|e| e.kind == "knockback") && d.effects.iter().any(|e| e.kind == "snare"), "Thunderstorm throws and slows");
    let ev = give(&mut s, a, boss, "thunderstorm");
    assert!(!ev.iter().any(|e| matches!(e, Event::MobMove { id, .. } if *id == boss)), "the boss isn't sent anywhere");
    for _ in 0..20 {
        let _ = s.step(0.05);
    }
    assert!(dist2d(s.mobs[&boss].pos, [3.0, 0.0, 0.0]) < 0.3, "the boss stands where it was");
    assert!(s.has_cc(boss, "snare"), "but is slowed (a snare still takes)");
    assert!(told_immune(&ev, a));
    let ev = give(&mut s, a, wolf, "thunderstorm");
    let thrown = ev.iter().any(|e| matches!(e, Event::MobMove { id, path, .. } if *id == wolf && path.last().is_some_and(|to| dist2d(*to, [0.0, 3.0, 0.0]) > 5.0)));
    assert!(thrown, "the wolf is thrown back");
}

#[test]
fn an_ultimate_stuns_a_boss_all_the_same() {
    let mut s = world();
    let a = hero(&mut s, [0.0, 0.0, 0.0]);
    let bd = boss_def(&s);
    let boss = mob(&mut s, bd, [3.0, 0.0, 0.0]);
    let d = &s.content.abilities[s.content.ability("earthsplitter").unwrap()];
    assert_eq!(d.slot, "ultimate");
    assert!(d.effects.iter().all(|e| e.vs_bosses), "an ultimate's effects are marked as they load");
    give(&mut s, a, boss, "earthsplitter");
    assert!(s.has_cc(boss, "stun"), "Earthsplitter stuns the boss");
}

#[test]
fn the_immune_word_comes_once_a_moment_not_for_every_effect() {
    let mut s = world();
    let a = hero(&mut s, [0.0, 0.0, 0.0]);
    let bd = boss_def(&s);
    let boss = mob(&mut s, bd, [3.0, 0.0, 0.0]);
    let mut ev = give(&mut s, a, boss, "shadowfury");
    ev.extend(give(&mut s, a, boss, "thunderstorm"));
    let told = ev.iter().filter(|e| matches!(e, Event::Flash { text, .. } if text.ends_with(": Immune"))).count();
    assert_eq!(told, 1, "one word for two shrugged-off effects at once");
}

#[test]
fn a_hero_still_takes_control_as_before() {
    // The rule is for foes that hold their ground: a hero (here, a PvP rival) is stunned as ever.
    let mut c = Content::dev();
    c.pvp.friendly_fire = true;
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 7, 1_000_000.0);
    s.mobs.clear();
    let a = s.join("ann", "Ann", None);
    let b = s.join("bob", "Bob", None);
    s.move_player(a, [0.0, 0.0, 0.0], 0.0, Moving::default());
    s.move_player(b, [3.0, 0.0, 0.0], 0.0, Moving::default());
    let _ = s.step(0.0);
    give(&mut s, a, b, "shadowfury");
    assert!(s.has_cc(b, "stun"));
}
