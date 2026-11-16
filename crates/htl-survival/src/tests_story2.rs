//! The Elwynn expansion's second round, "The Lion's Seal" (docs/elwynn-expansion.md): Gruff
//! Swiftbite, the mason's marks, Remy "Two Times", the Stonefield wagon and the Parley Stones.
//! The helpers are `tests_story`'s.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::{Foe, MobState};
use crate::sim::*;
use crate::tests_story::*;

// --- Stage 6: Gruff Swiftbite ------------------------------------------------------------------

/// Gruff alone on flat ground with a hero 2.5 yd off, in the chase, and a world of his own seed.
fn gruff_bare(seed: u64) -> (Survival, Id, Id) {
    let mut c = Content::builtin();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), seed, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    let p = bh_hero(&mut s, [0.0, 0.0, 0.0]);
    let def = s.content.mob("gruff_swiftbite").unwrap();
    let g = s.spawn_mob(def, [2.5, 0.0, 0.0], None, false);
    let m = s.mobs.get_mut(&g).unwrap();
    m.state = MobState::Chase { foe: Foe::Hero(p) };
    // He doesn't strike while the heavy is tried.
    m.ready_at = 1e9;
    (s, p, g)
}

/// Gruff's rule: he steps out of every heavy attack wound up at him. The tell is his backstep.
#[test]
fn gruff_steps_back_from_at_least_9_of_10_front_heavies() {
    let mut stepped = 0;
    for seed in 1..=10u64 {
        let (mut s, p, g) = gruff_bare(seed);
        s.input(p, Input::Heavy);
        let ev = s.step(0.05);
        if ev.iter().any(|e| matches!(e, Event::React { id, anim: 30, .. } if *id == g)) {
            stepped += 1;
            let mut far = 0.0f32;
            for _ in 0..12 {
                let _ = s.step(0.05);
                far = far.max(s.mobs[&g].pos[0]);
            }
            assert!(far > 4.5, "seed {seed}: his backstep is a real step: {far}");
        }
    }
    assert!(stepped >= 9, "stepped out of {stepped} of 10 heavies");
}

/// A heavy from behind or the side is not stepped out of (it is wound up at the ones in front).
#[test]
fn gruff_does_not_step_out_of_a_heavy_wound_up_at_someone_else() {
    let (mut s, p, g) = gruff_bare(3);
    // He is behind the hero.
    s.move_player(p, [0.0, 0.0, 0.0], std::f32::consts::PI, Moving::default());
    let _ = s.step(0.0);
    s.input(p, Input::Heavy);
    let ev = s.step(0.05);
    assert!(!ev.iter().any(|e| matches!(e, Event::React { id, anim: 30, .. } if *id == g)), "no step from behind");
}

/// The rule's other half: parry him, and the heavy lands while he reels.
#[test]
fn after_a_parry_gruff_reels_and_the_heavy_lands() {
    let (mut s, p, g) = gruff_bare(3);
    let content = &s.content.mobs[s.mobs[&g].def];
    assert!(!content.boss, "a boss only fills its posture on a parry: Gruff has to reel");
    s.input(p, Input::Guard);
    run(&mut s, 0.1);
    assert_eq!(s.strike_player(g, p, 16, [2.0, 0.0, 0.0], true, false, true, Blow::Light), Outcome::Parried);
    assert!(s.mobs[&g].stagger_until > s.clock + 0.5, "he reels");
    s.input(p, Input::GuardUp);
    let _ = s.step(0.05);
    s.input(p, Input::Heavy);
    let before = s.mobs[&g].health;
    let mut stepped = false;
    for _ in 0..40 {
        stepped |= s.step(0.05).iter().any(|e| matches!(e, Event::React { id, anim: 30, .. } if *id == g));
    }
    assert!(!stepped, "a reeling Gruff can't step back");
    assert!(s.mobs[&g].health < before, "the heavy landed: {} of {before}", s.mobs[&g].health);
}

#[test]
fn gruff_bites_in_quick_strings_and_lunges_from_5_to_12_yd() {
    let c = Content::builtin();
    let g = &c.mobs[c.mob("gruff_swiftbite").unwrap()];
    assert!(g.evade >= 0.95 && !g.boss && !g.unparryable);
    assert_eq!(g.health, 380);
    let bites = g.patterns.iter().find(|p| p.name == "bite string").unwrap();
    assert!(bites.steps.len() >= 3 && bites.steps[1..].iter().all(|st| st.windup.is_some_and(|w| w < 0.3)), "quick bites after the first");
    let lunge = g.patterns.iter().find(|p| p.name == "lunge").unwrap();
    assert_eq!((lunge.min_range, lunge.max_range), (5.0, 12.0));
    assert!(lunge.steps[0].lunge > 0.0);
}

/// His three spots are more than 250 yd from the camp and clear of Hogger's.
#[test]
fn gruffs_spots_keep_clear_of_the_camp_and_of_hoggers() {
    let c = Content::builtin();
    let k = c.packs.iter().position(|p| p.mob == "gruff_swiftbite").unwrap();
    let pack = &c.packs[k];
    assert!(pack.rare);
    assert_eq!(pack.spots.len(), 3);
    let camp = [c.world.build_center[0], c.world.build_center[1], 0.0];
    let hogger = c.packs.iter().find(|p| p.mob == "hogger").unwrap();
    for p in &pack.spots {
        let at = [p[0], p[1], 0.0];
        assert!(dist2d(at, camp) > 250.0, "{p:?} is {:.0} yd from the camp", dist2d(at, camp));
        for h in &hogger.spots {
            assert!(dist2d(at, [h[0], h[1], 0.0]) >= 20.0, "{p:?} sits on Hogger's {h:?}");
        }
    }
}

#[test]
fn the_fall_of_gruff_is_announced_and_the_tribute_stays_shut_until_he_dies() {
    let (mut s, h) = real_world();
    // (The Riverpaw brutes round the tribute would break off the hero's work: Gruff alone here.)
    let gruff = mob_of(&s, "gruff_swiftbite")[0];
    s.mobs.retain(|id, _| *id == gruff);
    s.respawns.clear();
    let silver = s.content.item("silver").unwrap();
    let ev = open_at(&mut s, h, "riverpaw_tribute");
    assert!(ev.iter().any(|e| matches!(e, Event::Tell { text, .. } if text.contains("sits on it"))), "it says why");
    assert!(chest(&s, "riverpaw_tribute").is_some(), "still shut");
    assert!(!s.camp_has("open:riverpaw_tribute"));
    let id = mob_of(&s, "gruff_swiftbite")[0];
    let max = s.mobs[&id].max_health;
    s.hurt_mob(id, h, max);
    let ev = s.step(0.0);
    assert!(ev.iter().any(|e| matches!(e, Event::Banner { title, .. } if title == "Gruff Swiftbite Falls")), "{ev:?}");
    assert!(ev.iter().any(|e| matches!(e, Event::Announce { text } if text.starts_with("Gruff Swiftbite fell"))));
    assert!(s.camp_has("kill:gruff_swiftbite"));
    let before = s.players[&h].bag.get(&silver).copied().unwrap_or(0);
    let ev = open_at(&mut s, h, "riverpaw_tribute");
    assert!(ev.iter().any(|e| matches!(e, Event::Tell { text, .. } if text.contains("Brotherhood silver"))), "the find is read");
    assert!(s.players[&h].bag.get(&silver).copied().unwrap_or(0) >= before + 2);
    assert!(s.camp_has("open:riverpaw_tribute"));
}

#[test]
fn wren_has_a_word_once_gruff_is_dead_and_the_journal_asks_for_him() {
    let mut s = world();
    s.camp.counts.insert("build:tanning".into(), 1);
    run(&mut s, 5.5);
    let (wren, _) = person(&s, "Wren Ashby");
    assert!(!s.talk_line(wren).unwrap().text.contains("Swiftbite"));
    s.camp.counts.insert("kill:gruff_swiftbite".into(), 1);
    assert!(s.talk_line(wren).unwrap().text.starts_with("Swiftbite's gone."));
    let g = s.content.goals.iter().find(|g| g.title == "Gruff Swiftbite").unwrap();
    assert_eq!((g.kind.as_str(), g.what.as_str()), ("kill", "gruff_swiftbite"));
    assert!(g.text.contains("steps out of every heavy attack"), "the journal names his rule");
}
