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

// --- Stage 7: the Mason's Marks ----------------------------------------------------------------

/// The built-in world on flat ground with only the cairn's ambushers standing.
fn mason_world() -> Survival {
    let mut c = Content::builtin();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 5, 1_000_000.0);
    let keep: Vec<Id> = s.mobs.iter().filter(|(_, m)| m.pack.is_some_and(|k| s.content.packs[k].key.starts_with("mason_"))).map(|(id, _)| *id).collect();
    s.mobs.retain(|id, _| keep.contains(id));
    s.respawns.clear();
    s
}

fn told(ev: &[Event], what: &str) -> bool {
    ev.iter().any(|e| matches!(e, Event::Tell { text, .. } if text.contains(what)))
}

/// E1: a chest's `after` with a colon is a condition the camp meets, and it stays shut, with its
/// own words, until then.
#[test]
fn a_chest_that_waits_on_another_stays_shut_with_its_locked_text_until_that_one_is_opened() {
    let mut s = mason_world();
    let h = bh_hero(&mut s, [0.0, 0.0, 0.0]);
    let ev = open_at(&mut s, h, "mason_trowel");
    assert!(told(&ev, "The trowel's stone won't move. Something was laid before it."), "it says why");
    assert!(chest(&s, "mason_trowel").is_some(), "still shut");
    assert!(!s.camp_has("open:mason_trowel"));
    // The one it waits on opens, and then so does it.
    let ev = open_at(&mut s, h, "mason_plumb");
    assert!(told(&ev, "The plumb line hangs true."));
    assert!(s.camp_has("open:mason_plumb"));
    let ev = open_at(&mut s, h, "mason_trowel");
    assert!(told(&ev, "Mortar gives way under the trowel's mark."));
    assert!(s.camp_has("open:mason_trowel"));
}

/// A bare key is still a creature: the camp must have killed one.
#[test]
fn a_bare_after_still_waits_on_a_kill() {
    let (mut s, h) = real_world();
    let ev = open_at(&mut s, h, "westbrook_armory");
    assert!(told(&ev, "barred while Hogger roams"), "{ev:?}");
    s.camp.counts.insert("kill:hogger".into(), 1);
    assert!(s.chest_unlocked("hogger"));
    assert!(s.chest_unlocked("kill:hogger"));
    assert!(!s.chest_unlocked("found:mason_cairn"));
}

#[test]
fn the_marks_are_opened_in_the_masons_order_and_only_that_order() {
    let mut s = mason_world();
    let h = bh_hero(&mut s, [0.0, 0.0, 0.0]);
    let silver = s.content.item("silver").unwrap();
    let pearl = s.content.item("pearl").unwrap();
    let stone = s.content.item("stone").unwrap();
    // The key's stone first: set fast. The trowel first: it won't move.
    let ev = open_at(&mut s, h, "mason_cache");
    assert!(told(&ev, "The key's stone is set fast. The trowel comes before it."));
    open_at(&mut s, h, "mason_trowel");
    assert!(!s.camp_has("open:mason_trowel"));
    assert!(!s.camp_has("open:mason_cache"));
    // The right order.
    for key in ["mason_plumb", "mason_trowel", "mason_cache"] {
        open_at(&mut s, h, key);
        assert!(s.camp_has(&format!("open:{key}")), "{key} opened in its turn");
    }
    let bag = &s.players[&h].bag;
    assert_eq!(bag.get(&silver), Some(&2));
    assert_eq!(bag.get(&pearl), Some(&1));
    assert_eq!(bag.get(&stone), Some(&6));
    // They are once a world: none stands again.
    assert!(["mason_plumb", "mason_trowel", "mason_cache"].iter().all(|k| s.content.chests.iter().find(|c| c.key == *k).unwrap().respawn == 0.0));
    s.move_player(h, [3000.0, 3000.0, 0.0], 0.0, Moving::default());
    run(&mut s, 3000.0);
    assert!(["mason_plumb", "mason_trowel", "mason_cache"].iter().all(|k| chest(&s, k).is_none()), "never full again");
    // Opening the cache is the goal.
    let g = s.content.goals.iter().find(|g| g.title == "The mason's marks").unwrap();
    assert_eq!((g.kind.as_str(), g.what.as_str()), ("open", "mason_cache"));
    assert!(g.text.contains("A wrong stone wakes the Brotherhood."));
}

/// A wrong stone wakes its two hidden footpads on the opener, and they wait again once it fills.
#[test]
fn a_decoy_wakes_its_ambush_and_waits_again_once_it_refills() {
    for key in ["mason_crown", "mason_lion"] {
        let mut s = mason_world();
        let h = bh_hero(&mut s, [0.0, 0.0, 0.0]);
        let ambush = format!("{key}_ambush");
        let pack = s.content.packs.iter().position(|p| p.key == ambush).unwrap();
        let of = |s: &Survival| -> Vec<Id> { s.mobs.iter().filter(|(_, m)| m.pack == Some(pack) && m.health > 0).map(|(id, _)| *id).collect() };
        let foot = of(&s);
        assert_eq!(foot.len(), 2, "two footpads");
        assert!(foot.iter().all(|id| s.mobs[id].hidden), "unseen until the stone is touched");
        let c = s.content.chests.iter().find(|c| c.key == key).unwrap();
        assert!(c.loot.is_empty() && c.roll.is_empty(), "no loot");
        assert_eq!(c.respawn, 300.0);
        let ev = open_at(&mut s, h, key);
        assert!(told(&ev, "Not Brannick's mark. The Brotherhood was waiting for a wrong guess."));
        for id in &foot {
            assert!(!s.mobs[id].hidden, "stepped out");
            assert_eq!(s.mobs[id].state.foe(), Some(Foe::Hero(h)), "on the opener");
        }
        assert!(!s.camp_has("open:mason_cache") && chest(&s, "mason_cache").is_some(), "a decoy opens nothing true");
        for id in &foot {
            fell(&mut s, h, *id);
        }
        assert!(chest(&s, key).is_none(), "opened");
        s.move_player(h, [3000.0, 3000.0, 0.0], 0.0, Moving::default());
        run(&mut s, 600.0);
        assert!(chest(&s, key).is_some(), "full again");
        let back = of(&s);
        assert_eq!(back.len(), 2, "the footpads are back");
        assert!(back.iter().all(|id| s.mobs[id].hidden), "and waiting again");
    }
}

#[test]
fn a_chests_condition_must_name_something_real_and_one_a_chest_can_meet() {
    let text = source("survival.toml");
    let dungeon = source("deadmines.toml");
    assert!(Content::parse_with(&text, &dungeon).is_ok());
    let line = "after = \"open:mason_plumb\"";
    let at = text.find(line).expect("the trowel's condition");
    for (bad, why) in [
        ("open:nope", "nope"),
        ("found:nowhere", "nowhere"),
        ("kill:nobody_at_all", "nobody_at_all"),
        ("raids:3", "open:, found: or kill:"),
        ("bogus:x", "open:, found: or kill:"),
        ("open:mason_trowel", "wait on itself"),
    ] {
        let mut t = text.clone();
        t.replace_range(at..at + line.len(), &format!("after = \"{bad}\""));
        let err = Content::parse_with(&t, &dungeon).expect_err(bad);
        assert!(format!("{err:#}").contains(why), "{bad}: {err:#}");
    }
    // The ones that are real still read.
    for good in ["found:mason_cairn", "kill:hogger", "open:mason_crown"] {
        let mut t = text.clone();
        t.replace_range(at..at + line.len(), &format!("after = \"{good}\""));
        assert!(Content::parse_with(&t, &dungeon).is_ok(), "{good}");
    }
}

#[test]
fn the_stonemasons_words_change_in_order_with_the_cairn() {
    let mut s = world();
    s.camp.counts.insert("kill:brannick".into(), 1);
    run(&mut s, 5.5);
    let (mason, _) = person(&s, "Stormwind Stonemason");
    let say = |s: &Survival| s.talk_line(mason).unwrap().text.clone();
    assert!(say(&s).starts_with("Brannick laid half this wall"));
    s.camp.found.insert("mason_cairn".into());
    assert!(say(&s).starts_with("Brannick signed every wall the same. Plumb, then trowel, then key."));
    // After his fifth-raid line too: the last line that holds wins.
    s.camp.raids_won = 5;
    assert!(say(&s).starts_with("Brannick signed"), "the cairn's line follows the raid's");
    s.camp.counts.insert("open:mason_cache".into(), 1);
    assert!(say(&s).starts_with("So he kept books too."));
}

#[test]
fn the_cairn_is_a_place_and_every_corner_counts_it() {
    let c = Content::builtin();
    let site = c.sites.iter().find(|s| s.key == "mason_cairn").unwrap();
    assert_eq!((site.kind.as_str(), site.radius, site.at), ("landmark", 30.0, [-8890.0, -840.0]));
    assert_eq!(c.sites.len(), 37);
    let corner = c.goals.iter().find(|g| g.title == "Every corner").unwrap();
    assert_eq!(corner.count as usize, c.sites.len());
    // The stones stand inside it.
    for k in ["mason_plumb", "mason_trowel", "mason_cache", "mason_crown", "mason_lion"] {
        let ch = c.chests.iter().find(|ch| ch.key == k).unwrap();
        assert!(dist2d([ch.at[0], ch.at[1], 0.0], [site.at[0], site.at[1], 0.0]) < site.radius, "{k} is at the cairn");
    }
}
