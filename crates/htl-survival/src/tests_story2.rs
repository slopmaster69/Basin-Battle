//! The Elwynn expansion's second round, "The Lion's Seal" (docs/elwynn-expansion.md): Gruff
//! Swiftbite, the mason's marks, Remy "Two Times", the Stonefield wagon and the Parley Stones.
//! The helpers are `tests_story`'s.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::{Foe, MobState};
use crate::sim::*;
use crate::tests_story::*;
use std::collections::BTreeMap;

// --- Stage 6: Gruff Swiftbite ------------------------------------------------------------------

/// Gruff alone on flat ground with a hero 2.5 yd off, in the chase, and a world of his own seed.
fn gruff_bare(seed: u64) -> (Survival, Id, Id) {
    let mut c = Content::builtin();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), seed, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    let p = bh_hero(&mut s, [0.0, 0.0, 0.0]);
    // (The dice are this seed's from here, whatever the world's content drew getting this far.)
    s.rng = rand::SeedableRng::seed_from_u64(seed);
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
        ("raids:3", "open:, found:, kill: or chose:"),
        ("bogus:x", "open:, found:, kill: or chose:"),
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
    assert!(c.sites.len() >= 37, "the cairn is one of the places");
    let corner = c.goals.iter().find(|g| g.title == "Every corner").unwrap();
    assert_eq!(corner.count as usize, c.sites.len());
    // The stones stand inside it.
    for k in ["mason_plumb", "mason_trowel", "mason_cache", "mason_crown", "mason_lion"] {
        let ch = c.chests.iter().find(|ch| ch.key == k).unwrap();
        assert!(dist2d([ch.at[0], ch.at[1], 0.0], [site.at[0], site.at[1], 0.0]) < site.radius, "{k} is at the cairn");
    }
}

// --- Stage 8: Remy "Two Times" -----------------------------------------------------------------

const REMY: &str = "Remy \"Two Times\"";

/// The world with the mason's book open, Remy standing outside the inn, and a hero beside him.
fn remy_world() -> (Survival, Id, Id) {
    let mut s = world();
    s.camp.counts.insert("open:mason_cache".into(), 1);
    run(&mut s, 5.5);
    let (_, id) = person(&s, REMY);
    let h = beside(&mut s, id);
    (s, h, id)
}

/// Whether the camp's person called `name` stands.
fn stands(s: &Survival, name: &str) -> bool {
    s.content.props.iter().position(|p| p.npc && p.name == name).is_some_and(|def| s.objs.values().any(|o| matches!(o.kind, ObjKind::Piece { def: d } if d == def)))
}

#[test]
fn remy_waits_for_the_mason_book_and_offers_two_choices_on_his_page() {
    let mut s = world();
    run(&mut s, 5.5);
    assert!(!stands(&s, REMY), "not before the road book is open");
    s.camp.counts.insert("open:mason_cache".into(), 1);
    run(&mut s, 5.5);
    let (_, id) = person(&s, REMY);
    let h = beside(&mut s, id);
    s.interact(h, None);
    let said = letters(&s.step(0.0));
    assert_eq!(said.len(), 1);
    assert!(said[0].2.starts_with("Rosters. Who walks which road, and when."));
    let links: Vec<&str> = said[0].3.iter().map(|(c, _)| c.as_str()).collect();
    assert_eq!(links, [".es choose remy:arrest", ".es choose remy:free"]);
    assert_eq!(said[0].3[0].1, "Arrest Him");
    assert_eq!(said[0].3[1].1, "Free Him");
}

#[test]
fn a_choice_is_refused_far_from_remy_twice_over_and_for_the_other_option() {
    let (mut s, h, _) = remy_world();
    // Far from him: nobody is waiting on it.
    s.move_player(h, [4000.0, 4000.0, 0.0], 0.0, Moving::default());
    let _ = s.step(0.0);
    assert!(s.choose(h, "remy:arrest").unwrap_err().contains("Nobody here"));
    assert!(!s.camp_has("chose:remy"));
    // Close, but nobody offers that.
    let (_, id) = person(&s, REMY);
    let at = s.objs[&id].pos;
    s.move_player(h, [at[0] + 1.5, at[1], at[2]], 0.0, Moving::default());
    let _ = s.step(0.0);
    assert!(s.choose(h, "remy:hang").unwrap_err().contains("Nobody here"));
    assert!(s.choose(h, "remy").unwrap_err().starts_with("Try .es choose"));
    assert!(s.choose(h, "remy:arrest:now").unwrap_err().starts_with("Try .es choose"));
    // The choice is made, the camp's, once.
    assert!(s.choose(h, "remy:free").is_ok());
    assert!(s.camp_has("chose:remy") && s.camp_has("chose:remy:free") && !s.camp_has("chose:remy:arrest"));
    assert_eq!(s.camp.counts.get("chose:remy"), Some(&1));
    assert!(s.choose(h, "remy:free").unwrap_err().contains("already decided"), "a second time");
    assert!(s.choose(h, "remy:arrest").unwrap_err().contains("already decided"), "the other option");
    assert!(!s.camp_has("chose:remy:arrest"));
}

#[test]
fn a_hero_must_be_within_6_yd_and_any_hero_may_choose() {
    let (mut s, _, id) = remy_world();
    let at = s.objs[&id].pos;
    let far = s.join("bren", "Bren", None);
    s.move_player(far, [at[0] + 7.0, at[1], at[2]], 0.0, Moving::default());
    let _ = s.step(0.0);
    assert!(s.choose(far, "remy:arrest").unwrap_err().contains("Nobody here"), "7 yd is too far");
    s.move_player(far, [at[0] + 5.5, at[1], at[2]], 0.0, Moving::default());
    let _ = s.step(0.0);
    assert!(s.choose(far, "remy:arrest").is_ok(), "5.5 yd is near enough, for any hero");
}

#[test]
fn arrested_remy_goes_and_dughan_and_the_bounty_come() {
    let (mut s, h, _) = remy_world();
    assert!(!stands(&s, "Marshal Dughan"));
    // The bounty and the stash are shut before the choice.
    let ev = open_at(&mut s, h, "remy_bounty");
    assert!(told(&ev, "Nobody has been taken to the Marshal yet."), "{ev:?}");
    let ev = open_at(&mut s, h, "remy_stash");
    assert!(told(&ev, "Nobody has told you where Remy hides his silver."));
    let (_, id) = person(&s, REMY);
    let at = s.objs[&id].pos;
    s.move_player(h, [at[0] + 1.5, at[1], at[2]], 0.0, Moving::default());
    let _ = s.step(0.0);
    s.choose(h, "remy:arrest").unwrap();
    let ev = s.step(0.0);
    assert!(!stands(&s, REMY), "Remy is taken away");
    assert!(stands(&s, "Marshal Dughan"), "and the Marshal stands");
    assert!(ev.iter().any(|e| matches!(e, Event::Announce { text } if text == "Marshal Dughan takes Remy \"Two Times\" to the Stockade.")), "{ev:?}");
    // He never stands again, however long the camp looks.
    run(&mut s, 30.0);
    assert!(!stands(&s, REMY));
    // The bounty opens, the stash does not.
    let silver = s.content.item("silver").unwrap();
    let bronze = s.content.item("bronze").unwrap();
    let before = (s.players[&h].bag.get(&silver).copied().unwrap_or(0), s.players[&h].bag.get(&bronze).copied().unwrap_or(0));
    let ev = open_at(&mut s, h, "remy_bounty");
    assert!(told(&ev, "Stormwind pays for informers, once they stop informing."));
    assert_eq!(s.players[&h].bag.get(&silver).copied().unwrap_or(0), before.0 + 3);
    assert_eq!(s.players[&h].bag.get(&bronze).copied().unwrap_or(0), before.1 + 2);
    let ev = open_at(&mut s, h, "remy_stash");
    assert!(told(&ev, "Nobody has told you where Remy hides his silver."), "the stash stays shut after an arrest");
    // Dughan names the Stockade, with its real page.
    let (dughan, did) = person(&s, "Marshal Dughan");
    let t = s.talk_line(dughan).unwrap();
    assert!(t.text.starts_with("Remy sold the rosters"));
    assert_eq!(t.links, [".es lore stockade~The Stockade"]);
    assert!(s.content.dungeons().any(|d| d.key == "stockade"));
    let _ = did;
}

#[test]
fn freed_remy_stays_with_his_word_and_his_stash_opens() {
    let (mut s, h, id) = remy_world();
    s.choose(h, "remy:free").unwrap();
    let _ = s.step(0.0);
    run(&mut s, 30.0);
    assert!(stands(&s, REMY), "Remy stays");
    assert!(!stands(&s, "Marshal Dughan"), "no marshal");
    let (def, _) = person(&s, REMY);
    let t = s.talk_line(def).unwrap();
    assert!(t.text.starts_with("You never saw me. The man in black meets his buyers past Jasperlode."));
    assert!(t.links.is_empty(), "nothing left to choose");
    let _ = id;
    let silver = s.content.item("silver").unwrap();
    let pearl = s.content.item("pearl").unwrap();
    let ev = open_at(&mut s, h, "remy_bounty");
    assert!(told(&ev, "Nobody has been taken to the Marshal yet."), "no bounty for a free man");
    let ev = open_at(&mut s, h, "remy_stash");
    assert!(told(&ev, "Two silver a road, saved."));
    assert_eq!(s.players[&h].bag.get(&silver).copied().unwrap_or(0), 2);
    assert_eq!(s.players[&h].bag.get(&pearl).copied().unwrap_or(0), 2);
}

#[test]
fn the_choice_counts_for_the_journal_and_a_save_keeps_it() {
    let (mut s, h, _) = remy_world();
    let g = s.content.goals.iter().position(|g| g.title == "Remy Two Times").unwrap();
    let corner = s.content.goals.iter().position(|g| g.title == "Every corner").unwrap();
    assert!(g < corner, "ahead of Every corner");
    assert_eq!((s.content.goals[g].kind.as_str(), s.content.goals[g].what.as_str()), ("chose", "remy"));
    s.camp.goal = g;
    s.step_goals();
    assert_eq!(s.camp.goal, g, "not before the choice");
    s.choose(h, "remy:arrest").unwrap();
    s.step_goals();
    assert_eq!(s.camp.goal, g + 1, "the choice is the goal");
    // A save keeps the choice: Remy is still gone, the Marshal still stands, the bounty still opens.
    let save = s.save(&BTreeMap::new());
    let mut back = Survival::load(Content::builtin(), Box::new(FlatLand::default()), 1, 1_000_000.0, &save);
    back.mobs.clear();
    assert!(back.camp_has("chose:remy:arrest") && !back.camp_has("chose:remy:free"));
    run(&mut back, 5.5);
    assert!(!stands(&back, REMY));
    assert!(stands(&back, "Marshal Dughan"));
    assert!(back.choose(h, "remy:free").is_err(), "and it can't be made again");
}

#[test]
fn a_freed_remy_is_still_there_after_a_save() {
    let (mut s, h, _) = remy_world();
    s.choose(h, "remy:free").unwrap();
    let save = s.save(&BTreeMap::new());
    let mut back = Survival::load(Content::builtin(), Box::new(FlatLand::default()), 1, 1_000_000.0, &save);
    back.mobs.clear();
    run(&mut back, 5.5);
    assert!(stands(&back, REMY));
    assert!(!stands(&back, "Marshal Dughan"));
}

#[test]
fn a_choice_nothing_offers_or_nothing_waits_on_is_refused_by_validate() {
    let text = source("survival.toml");
    let dungeon = source("deadmines.toml");
    assert!(Content::parse_with(&text, &dungeon).is_ok());
    let swap = |from: &str, to: &str| {
        let at = text.find(from).unwrap_or_else(|| panic!("no {from}"));
        let mut t = text.clone();
        t.replace_range(at..at + from.len(), to);
        Content::parse_with(&t, &dungeon)
    };
    // A condition that names an option nobody offers.
    let err = swap("after = \"chose:remy:free\"\ntext = \"You never", "after = \"chose:remy:nope\"\ntext = \"You never").unwrap_err();
    assert!(format!("{err:#}").contains("nope"), "{err:#}");
    let err = swap("after = \"chose:remy:arrest\"\nclear", "after = \"chose:nobody:arrest\"\nclear").unwrap_err();
    assert!(format!("{err:#}").contains("nobody:arrest"), "{err:#}");
    // A link that is no group and option, or an option nothing waits on.
    let two = "\".es choose remy:free~Free Him\"]";
    let err = swap(two, "\".es choose remy:free~Free Him\", \".es choose remy~Bad\"]").unwrap_err();
    assert!(format!("{err:#}").contains("not `.es choose <group>:<option>`"), "{err:#}");
    let err = swap(two, "\".es choose remy:free~Free Him\", \".es choose remy:pardon~Pardon\"]").unwrap_err();
    assert!(format!("{err:#}").contains("nothing waits on it"), "{err:#}");
    // A goal for a group nobody offers.
    let err = swap("kind = \"chose\"\nwhat = \"remy\"", "kind = \"chose\"\nwhat = \"nobody\"").unwrap_err();
    assert!(format!("{err:#}").contains("nobody"), "{err:#}");
    // And a prop that goes for a condition that names nothing.
    let err = swap("until = \"chose:remy:arrest\"", "until = \"chose:remy:bogus\"").unwrap_err();
    assert!(format!("{err:#}").contains("bogus"), "{err:#}");
}

#[test]
fn a_prop_with_until_never_stands_once_it_holds_even_at_the_start() {
    let mut c = Content::builtin();
    c.pvp = Default::default();
    // A person from the start whose `until` already holds is not set out.
    let k = c.props.iter().position(|p| p.npc && p.name == REMY).unwrap();
    c.props[k].after = None;
    let mut s = Survival::new(c.clone(), Box::new(FlatLand::default()), 1, 1_000_000.0);
    assert!(stands(&s, REMY), "from the start he stands");
    s.camp.counts.insert("chose:remy:arrest".into(), 1);
    run(&mut s, 5.5);
    assert!(!stands(&s, REMY), "and goes once his until holds");
    let mut s2 = Survival::new(c, Box::new(FlatLand::default()), 1, 1_000_000.0);
    s2.camp.counts.insert("chose:remy:arrest".into(), 1);
    s2.objs.retain(|_, o| !matches!(o.kind, ObjKind::Piece { .. }));
    s2.place_props();
    assert!(!stands(&s2, REMY), "never set out at all");
}

// --- Stage 9: Hold the Stonefield Wagon --------------------------------------------------------

/// The built-in world on flat ground with only the wagon's creatures standing, and a hero beside it.
fn wagon_world() -> (Survival, Id) {
    let mut c = Content::builtin();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 5, 1_000_000.0);
    let keep: Vec<Id> = s.mobs.iter().filter(|(_, m)| m.pack.is_some_and(|k| s.content.packs[k].key.starts_with("sf_"))).map(|(id, _)| *id).collect();
    s.mobs.retain(|id, _| keep.contains(id));
    s.respawns.clear();
    let h = bh_hero(&mut s, [-9893.5, 370.0, 0.0]);
    (s, h)
}

/// Every creature of the wagon's waves that is up (awake and alive), not the farmer's son.
fn wave_mobs(s: &Survival) -> Vec<Id> {
    s.mobs.iter().filter(|(_, m)| m.health > 0 && !m.hidden && !m.friendly && m.pack.is_some_and(|k| s.content.packs[k].key.starts_with("sf_wave"))).map(|(id, _)| *id).collect()
}

fn step_keeping_alive(s: &mut Survival, h: Id) -> Vec<Event> {
    let ev = s.step(0.05);
    if let Some(p) = s.players.get_mut(&h) {
        p.health = 1_000_000;
    }
    ev
}

/// The wagon's use: the work of opening it begins the hold (the hold's clock starts as it ends).
fn load_wagon(s: &mut Survival, h: Id) -> Vec<Event> {
    open_at(s, h, "sf_wagon")
}

fn wagon_def(s: &Survival) -> usize {
    s.content.chests.iter().position(|c| c.key == "sf_wagon").unwrap()
}

#[test]
fn the_waves_come_at_0_30_and_60_sec_with_their_texts() {
    let (mut s, h) = wagon_world();
    let def = wagon_def(&s);
    let c = &s.content.chests[def];
    assert_eq!((c.hold, c.trial.len(), c.wave_text.len()), (90.0, 3, 3));
    assert!(wave_mobs(&s).is_empty(), "nothing in sight before the wagon is used");
    let ev = load_wagon(&mut s, h);
    let t = &s.trials[&def];
    let started = t.started;
    let mut said: Vec<(f32, String)> = ev.iter().filter_map(|e| match e { Event::Announce { text } => Some((s.clock - started, text.clone())), _ => None }).collect();
    let titles: Vec<String> = ev.iter().filter_map(|e| match e { Event::Banner { title, sub, .. } if title == "Hold the Wagon" => Some(sub.clone()), _ => None }).collect();
    assert_eq!(titles, ["Stay near it for 90 sec"], "{ev:?}");
    let first = wave_mobs(&s);
    assert_eq!(first.len(), 3, "wave 1: 3 kobold laborers");
    assert!(first.iter().all(|id| s.content.mobs[s.mobs[id].def].key == "kobold_laborer"));
    // Run on, keeping the hero up, and note when each text comes.
    for _ in 0..(75.0 / 0.05) as usize {
        for e in step_keeping_alive(&mut s, h) {
            if let Event::Announce { text } = e {
                said.push((s.clock - started, text));
            }
        }
    }
    let said: Vec<(f32, String)> = said.into_iter().filter(|(_, t)| !t.contains("fell") && !t.contains("completed")).collect();
    let want = [(0.0, "Kobolds pour out of the Fargodeep road."), (30.0, "Tunnelers come up under the fence."), (60.0, "Fedfennel comes for the grain himself.")];
    assert_eq!(said.len(), 3, "{said:?}");
    for ((at, text), (w_at, w_text)) in said.iter().zip(want) {
        assert_eq!(text, w_text);
        assert!((at - w_at).abs() < 0.6, "{text} came at {at:.2}, not {w_at}");
    }
    // Wave 2 is 2 tunnelers and a geomancer, wave 3 Fedfennel and 2 laborers.
    let keys: Vec<String> = wave_mobs(&s).iter().map(|id| s.content.mobs[s.mobs[id].def].key.clone()).collect();
    assert_eq!(keys.iter().filter(|k| *k == "fedfennel").count(), 1);
    assert_eq!(keys.iter().filter(|k| *k == "kobold_tunneler").count(), 2);
    assert_eq!(keys.iter().filter(|k| *k == "kobold_geomancer").count(), 1);
}

#[test]
fn the_hold_passes_at_90_sec_once_the_last_wave_is_dead_and_not_before() {
    // Fell everything that comes: the hold still runs its 90 sec, then passes.
    let (mut s, h) = wagon_world();
    // (What the felled drop is theirs too: only the chest's loot is counted.)
    for m in s.content.mobs.iter_mut() {
        m.drops.clear();
    }
    let def = wagon_def(&s);
    load_wagon(&mut s, h);
    let started = s.trials[&def].started;
    let mut banners = Vec::new();
    let mut tells = Vec::new();
    let mut passed_at = None;
    for _ in 0..(100.0 / 0.05) as usize {
        for id in wave_mobs(&s) {
            fell(&mut s, h, id);
        }
        for e in step_keeping_alive(&mut s, h) {
            match e {
                Event::Banner { title, .. } => banners.push(title),
                Event::Tell { text, .. } => tells.push(text),
                _ => {}
            }
        }
        if passed_at.is_none() && !s.trials.contains_key(&def) {
            passed_at = Some(s.clock - started);
        }
    }
    let at = passed_at.expect("the hold passed");
    assert!(at >= 89.9 && at < 91.0, "passed at {at:.2} sec");
    assert!(banners.iter().any(|b| b == "Wagon Held"), "{banners:?}");
    assert!(tells.iter().any(|t| t.contains("Grain for Westbrook")), "the find is read");
    assert!(s.camp_has("open:sf_wagon") && chest(&s, "sf_wagon").is_none(), "opened, and spent");
    let bag = &s.players[&h].bag;
    for (k, n) in [("silver", 2), ("meat", 6), ("linen", 4)] {
        assert_eq!(bag.get(&s.content.item(k).unwrap()), Some(&n), "{k}");
    }
    // The wagon fills again in its time, for another hold.
    s.move_player(h, [4000.0, 4000.0, 0.0], 0.0, Moving::default());
    run(&mut s, 1900.0);
    assert!(chest(&s, "sf_wagon").is_some(), "full again");
}

#[test]
fn the_hold_waits_for_the_last_wave_to_fall_even_after_90_sec() {
    let (mut s, h) = wagon_world();
    let def = wagon_def(&s);
    load_wagon(&mut s, h);
    let started = s.trials[&def].started;
    // Waves 1 and 2 are felled as they come; wave 3 is left standing.
    for _ in 0..(95.0 / 0.05) as usize {
        for id in wave_mobs(&s) {
            let key = s.content.mobs[s.mobs[&id].def].key.clone();
            if s.clock - started < 60.0 || key != "fedfennel" && key != "kobold_laborer" {
                fell(&mut s, h, id);
            }
        }
        let _ = step_keeping_alive(&mut s, h);
    }
    assert!(s.clock - started >= 95.0);
    assert!(s.trials.contains_key(&def), "the last wave still stands, so the hold goes on");
    assert!(!s.camp_has("open:sf_wagon"));
    // The last of them falls, and it is held at once.
    for id in wave_mobs(&s) {
        fell(&mut s, h, id);
    }
    let _ = step_keeping_alive(&mut s, h);
    assert!(!s.trials.contains_key(&def));
    assert!(s.camp_has("open:sf_wagon"));
}

#[test]
fn leaving_for_12_sec_resets_the_waves_and_the_wagon_stays_shut() {
    let (mut s, h) = wagon_world();
    let def = wagon_def(&s);
    load_wagon(&mut s, h);
    for _ in 0..(40.0 / 0.05) as usize {
        let _ = step_keeping_alive(&mut s, h);
    }
    assert_eq!(s.trials[&def].called, 2, "two waves have come");
    assert!(!wave_mobs(&s).is_empty());
    // The hero goes off: more than 60 yd from the wagon, for more than 12 sec.
    s.move_player(h, [-9893.5, 470.0, 0.0], 0.0, Moving::default());
    let mut lost = Vec::new();
    for _ in 0..(14.0 / 0.05) as usize {
        for e in step_keeping_alive(&mut s, h) {
            if let Event::Announce { text } = e {
                lost.push(text);
            }
        }
    }
    assert!(!s.trials.contains_key(&def), "given up");
    assert!(lost.iter().any(|t| t == "The wagon is lost. Use it again to load it."), "{lost:?}");
    assert!(wave_mobs(&s).is_empty(), "the waves are unseen again");
    assert!(s.mobs.values().filter(|m| !m.friendly).all(|m| m.health == m.max_health), "and whole");
    assert!(chest(&s, "sf_wagon").is_some() && !s.camp_has("open:sf_wagon"), "the wagon stays shut");
    // Used again, it begins from the first wave.
    let ev = load_wagon(&mut s, h);
    assert_eq!(s.trials[&def].called, 1);
    assert!(ev.iter().any(|e| matches!(e, Event::Announce { text } if text == "Kobolds pour out of the Fargodeep road.")));
    assert_eq!(wave_mobs(&s).len(), 3);
}

#[test]
fn tommy_joe_wakes_with_the_hold_and_stands_down_after_it() {
    let (mut s, h) = wagon_world();
    let tommy: Vec<Id> = s.mobs.iter().filter(|(_, m)| m.friendly).map(|(id, _)| *id).collect();
    assert_eq!(tommy.len(), 1);
    let t = tommy[0];
    assert!(s.mobs[&t].hidden, "unseen until the wagon is used");
    let ev = load_wagon(&mut s, h);
    assert!(!s.mobs[&t].hidden, "at his post for the hold");
    assert!(ev.iter().any(|e| matches!(e, Event::MobSpawned { id } if *id == t)));
    let at = s.mobs[&t].pos;
    for _ in 0..200 {
        let _ = step_keeping_alive(&mut s, h);
    }
    assert!(dist2d(s.mobs[&t].pos, at) < 0.1, "he stands where he was put");
    assert_ne!(s.mobs[&t].state.foe().map(|_| ()), Some(()), "he fights no hero");
    for id in wave_mobs(&s) {
        fell(&mut s, h, id);
    }
    // The hold passes, then he stands down.
    for _ in 0..(95.0 / 0.05) as usize {
        for id in wave_mobs(&s) {
            fell(&mut s, h, id);
        }
        let _ = step_keeping_alive(&mut s, h);
    }
    assert!(s.camp_has("open:sf_wagon"));
    assert!(s.mobs[&t].hidden, "and stands down after it");
}

/// The ally shoots only foes that are fighting a hero, never an idle pack and never a hero.
#[test]
fn tommy_joe_shoots_kobolds_fighting_a_hero_and_never_an_idle_pack() {
    let (mut s, h) = wagon_world();
    let t = *s.mobs.iter().find(|(_, m)| m.friendly).map(|(id, _)| id).unwrap();
    let laborer = s.content.mob("kobold_laborer").unwrap();
    let tommy_at = s.mobs[&t].pos;
    // Before the hold he is unseen, and shoots nothing even at a kobold on a hero.
    let near = s.spawn_mob(laborer, [tommy_at[0] + 8.0, tommy_at[1], tommy_at[2]], None, false);
    s.mobs.get_mut(&near).unwrap().state = MobState::Chase { foe: Foe::Hero(h) };
    let hp = s.mobs[&near].health;
    run(&mut s, 5.0);
    assert!(s.mobs.get(&near).is_none_or(|m| m.health >= hp.min(m.max_health) || m.health == hp), "no shot before the hold");
    // The hold begins.
    s.mobs.remove(&near);
    load_wagon(&mut s, h);
    // An idle kobold within his reach, unseen by the hero: never shot.
    let idle = s.spawn_mob(laborer, [tommy_at[0] - 17.0, tommy_at[1] + 10.0, tommy_at[2]], None, false);
    s.mobs.get_mut(&idle).unwrap().state = MobState::Idle { wander_at: 1e9 };
    let idle_hp = s.mobs[&idle].health;
    // One fighting the hero, within his reach.
    let busy = s.spawn_mob(laborer, [tommy_at[0] + 10.0, tommy_at[1] - 10.0, tommy_at[2]], None, false);
    s.mobs.get_mut(&busy).unwrap().state = MobState::Chase { foe: Foe::Hero(h) };
    s.mobs.get_mut(&busy).unwrap().ready_at = 1e9;
    s.mobs.get_mut(&busy).unwrap().pos = [tommy_at[0] + 10.0, tommy_at[1] - 10.0, tommy_at[2]];
    // The wave's own kobolds are felled so only these two are in play.
    for id in wave_mobs(&s) {
        s.mobs.remove(&id);
    }
    let busy_hp = s.mobs[&busy].health;
    let mut shots = Vec::new();
    for _ in 0..(6.0 / 0.05) as usize {
        for e in step_keeping_alive(&mut s, h) {
            if let Event::Missile { caster, target, .. } = e {
                if caster == t {
                    shots.push(target);
                }
            }
        }
        s.mobs.get_mut(&busy).unwrap().state = MobState::Chase { foe: Foe::Hero(h) };
        s.mobs.get_mut(&busy).unwrap().pos = [tommy_at[0] + 10.0, tommy_at[1] - 10.0, tommy_at[2]];
        s.mobs.get_mut(&busy).unwrap().health = s.mobs[&busy].health.max(1);
    }
    assert!(shots.len() >= 2 && shots.len() <= 4, "a shot every 2 sec: {}", shots.len());
    assert!(shots.iter().all(|t| *t == Target::Unit(busy)), "only the one on a hero: {shots:?}");
    assert!(!shots.iter().any(|t| *t == Target::Unit(h)), "never a hero");
    assert!(s.mobs[&busy].health < busy_hp, "and it hurts");
    assert_eq!(s.mobs[&idle].health, idle_hp, "the idle one is never shot");
}

#[test]
fn a_creature_tommy_joe_finishes_counts_for_the_camp() {
    let (mut s, h) = wagon_world();
    load_wagon(&mut s, h);
    for id in wave_mobs(&s) {
        s.mobs.remove(&id);
    }
    let t = *s.mobs.iter().find(|(_, m)| m.friendly).map(|(id, _)| id).unwrap();
    let at = s.mobs[&t].pos;
    let laborer = s.content.mob("kobold_laborer").unwrap();
    let k = s.spawn_mob(laborer, [at[0] + 8.0, at[1], at[2]], None, false);
    s.mobs.get_mut(&k).unwrap().state = MobState::Chase { foe: Foe::Hero(h) };
    s.mobs.get_mut(&k).unwrap().ready_at = 1e9;
    s.mobs.get_mut(&k).unwrap().health = 5;
    assert!(!s.camp_has("kill:kobold_laborer"));
    for _ in 0..(4.0 / 0.05) as usize {
        let _ = step_keeping_alive(&mut s, h);
    }
    assert!(s.mobs.get(&k).is_none_or(|m| m.health == 0), "he finished it");
    assert!(s.camp_has("kill:kobold_laborer"), "and it counts for the camp");
}

// Fedfennel: his shield takes light blows from the front between his blows.

/// Fedfennel in the chase facing the hero, a hero 2 yd off facing him.
fn fed_bare(seed: u64) -> (Survival, Id, Id) {
    let mut c = Content::builtin();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), seed, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    let p = bh_hero(&mut s, [0.0, 0.0, 0.0]);
    let def = s.content.mob("fedfennel").unwrap();
    let k = s.spawn_mob(def, [2.0, 0.0, 0.0], None, false);
    let m = s.mobs.get_mut(&k).unwrap();
    m.facing = std::f32::consts::PI;
    m.state = MobState::Recover { foe: Foe::Hero(p), until: 1e9 };
    m.ready_at = 1e9;
    (s, p, k)
}

/// The outcomes of the hero's blows on `k` over `swings` tries of `input`, the mob set as `set` each time.
fn swings(s: &mut Survival, p: Id, k: Id, input: Input, swings: usize, secs: f32, set: &dyn Fn(&mut Survival, Id)) -> Vec<Outcome> {
    let mut out = Vec::new();
    for _ in 0..swings {
        let max = s.mobs[&k].max_health;
        s.mobs.get_mut(&k).unwrap().health = max;
        s.mobs.get_mut(&k).unwrap().posture = 0;
        if let Some(pl) = s.players.get_mut(&p) {
            pl.stamina = 1000.0;
        }
        set(s, k);
        s.input(p, input);
        for _ in 0..(secs / 0.05) as usize {
            for e in s.step(0.05) {
                if let Event::Swing { victim: Some(v), outcome, anim: false, style, .. } = e {
                    if v == k && style != Blow::Dot {
                        out.push(outcome);
                    }
                }
            }
            set(s, k);
        }
    }
    out
}

fn between_blows(s: &mut Survival, k: Id) {
    let foe = s.mobs[&k].state.foe().unwrap_or(Foe::Hero(1));
    let m = s.mobs.get_mut(&k).unwrap();
    m.facing = std::f32::consts::PI;
    m.state = MobState::Recover { foe, until: 1e9 };
}

#[test]
fn fedfennels_shield_blocks_at_least_80_percent_of_light_blows_from_the_front() {
    let (mut s, p, k) = fed_bare(5);
    let out = swings(&mut s, p, k, Input::Attack, 40, 0.8, &between_blows);
    assert!(out.len() >= 40, "{} blows landed on him", out.len());
    let blocked = out.iter().filter(|o| **o == Outcome::Blocked).count();
    assert!(blocked * 100 >= out.len() * 80, "{blocked} of {} blocked", out.len());
}

#[test]
fn fedfennels_shield_does_not_help_from_behind_on_a_heavy_or_while_he_winds_up() {
    // From behind (he faces away): nothing is blocked.
    let (mut s, p, k) = fed_bare(5);
    let behind = |s: &mut Survival, k: Id| {
        let foe = s.mobs[&k].state.foe().unwrap_or(Foe::Hero(1));
        let m = s.mobs.get_mut(&k).unwrap();
        m.facing = 0.0;
        m.state = MobState::Recover { foe, until: 1e9 };
    };
    let out = swings(&mut s, p, k, Input::Attack, 25, 0.8, &behind);
    assert!(out.len() >= 25 && !out.contains(&Outcome::Blocked), "from behind: {out:?}");
    // A heavy blow, from the front: none blocked.
    let (mut s, p, k) = fed_bare(5);
    let out = swings(&mut s, p, k, Input::Heavy, 20, 1.6, &between_blows);
    assert!(!out.is_empty() && !out.contains(&Outcome::Blocked), "heavy: {out:?}");
    // While he winds up a blow (poised or not): none blocked.
    let (mut s, p, k) = fed_bare(5);
    let winding = |s: &mut Survival, k: Id| {
        let foe = s.mobs[&k].state.foe().unwrap_or(Foe::Hero(1));
        let m = s.mobs.get_mut(&k).unwrap();
        m.facing = std::f32::consts::PI;
        m.state = MobState::Windup { foe, t: 0.0, mark: None, shown: true };
    };
    let out = swings(&mut s, p, k, Input::Attack, 25, 0.4, &winding);
    assert!(!out.is_empty() && !out.contains(&Outcome::Blocked), "windup: {out:?}");
}

#[test]
fn fedfennel_is_a_boss_with_a_shield_and_two_blows_one_glowing_amber() {
    let c = Content::builtin();
    let f = &c.mobs[c.mob("fedfennel").unwrap()];
    assert!(f.boss && f.shield == 18399 && f.block == 0.9);
    // (Every boss is drawn `combat.boss_scale` times its scale.)
    assert_eq!((f.display, f.health), (2152, 520));
    assert!((f.scale - 1.3 * c.combat.boss_scale).abs() < 1e-3, "{}", f.scale);
    let chop = f.patterns.iter().find(|p| p.name == "pick chop").unwrap();
    assert_eq!(chop.steps[0].kit, 0, "the chop is plain");
    let high = f.patterns.iter().find(|p| p.name == "overhead").unwrap();
    assert_eq!(high.steps[0].kit, 379, "the overhead glows amber");
    // Tommy Joe is a friendly shooter and no one else is.
    let t = &c.mobs[c.mob("tommy_joe_stonefield").unwrap()];
    assert!(t.friendly);
    let sh = t.shoots.as_ref().unwrap();
    assert_eq!((sh.damage, sh.range, sh.every), (9, 25.0, 2.0));
    assert!(c.mobs.iter().filter(|m| m.shoots.is_some()).all(|m| m.friendly), "only a friendly creature shoots");
}

#[test]
fn a_hold_and_a_shooter_must_be_made_right_or_validate_refuses_them() {
    let text = source("survival.toml");
    let dungeon = source("deadmines.toml");
    assert!(Content::parse_with(&text, &dungeon).is_ok());
    let swap = |from: &str, to: &str| {
        let at = text.find(from).unwrap_or_else(|| panic!("no {from}"));
        let mut t = text.clone();
        t.replace_range(at..at + from.len(), to);
        Content::parse_with(&t, &dungeon)
    };
    // A wave that names a pack that doesn't wake on the wagon.
    let err = swap("\"sf_wave2_tunnelers+sf_wave2_geomancer\"", "\"sf_wave2_tunnelers+nope\"").unwrap_err();
    assert!(format!("{err:#}").contains("nope"), "{err:#}");
    // More wave texts than waves.
    let err = swap("\"Fedfennel comes for the grain himself.\"]", "\"Fedfennel comes for the grain himself.\", \"One too many.\"]").unwrap_err();
    assert!(format!("{err:#}").contains("wave texts"), "{err:#}");
    // A shooter that isn't friendly.
    let err = swap("friendly = true\nshoots", "shoots").unwrap_err();
    assert!(format!("{err:#}").contains("only a friendly creature shoots"), "{err:#}");
}

#[test]
fn the_wagon_is_a_goal_and_the_journal_keeps_its_place() {
    let c = Content::builtin();
    let at = |t: &str| c.goals.iter().position(|g| g.title == t).unwrap();
    assert!(at("The Stonefield wagon") < at("Every corner"), "ahead of Every corner");
    let g = &c.goals[at("The Stonefield wagon")];
    assert_eq!((g.kind.as_str(), g.what.as_str()), ("open", "sf_wagon"));
    assert!(crate::save::ELWYNN_GOALS >= 12, "the wagon is one of the goals the expansion put in");
    assert_eq!(c.goals[at("Every corner")].count as usize, c.sites.len());
}

// --- Stage 10: The Parley Stones ---------------------------------------------------------------

/// The world with only the parley table's packs standing (Marzon and the two helpers).
fn parley_world() -> (Survival, Id) {
    let mut c = Content::builtin();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 5, 1_000_000.0);
    let keep: Vec<Id> = s.mobs.iter().filter(|(_, m)| m.pack.is_some_and(|k| ["marzon", "parley_dughan", "parley_remy"].contains(&s.content.packs[k].key.as_str()))).map(|(id, _)| *id).collect();
    s.mobs.retain(|id, _| keep.contains(id));
    s.respawns.clear();
    let h = bh_hero(&mut s, [0.0, 0.0, 0.0]);
    (s, h)
}

/// The creature of `key` that stands (alive), if any, whether it is seen or not.
fn parley_mob(s: &Survival, key: &str) -> Id {
    mob_of(s, key).first().copied().unwrap_or_else(|| panic!("no {key}"))
}

#[test]
fn the_packs_after_decides_which_ally_wakes() {
    for (choice, dughan, remy) in [(Some("chose:remy:arrest"), true, false), (Some("chose:remy:free"), false, true), (None, false, false)] {
        let (mut s, h) = parley_world();
        if let Some(c) = choice {
            s.camp.counts.insert(c.into(), 1);
        }
        let (d, r, m) = (parley_mob(&s, "parley_dughan"), parley_mob(&s, "parley_remy"), parley_mob(&s, "marzon"));
        assert!(s.mobs[&d].hidden && s.mobs[&r].hidden && s.mobs[&m].hidden, "all asleep before the table is opened");
        open_at(&mut s, h, "parley_price");
        assert!(!s.mobs[&m].hidden, "Marzon always wakes ({choice:?})");
        assert_eq!(!s.mobs[&d].hidden, dughan, "Dughan, with {choice:?}");
        assert_eq!(!s.mobs[&r].hidden, remy, "Remy, with {choice:?}");
        assert_eq!(s.mobs[&m].state.foe(), Some(Foe::Hero(h)), "on the opener");
    }
}

#[test]
fn the_allies_of_the_parley_shoot_what_fights_a_hero_in_their_own_way() {
    let c = Content::builtin();
    let d = c.mobs[c.mob("parley_dughan").unwrap()].shoots.clone().unwrap();
    let r = c.mobs[c.mob("parley_remy").unwrap()].shoots.clone().unwrap();
    assert!(d.damage > r.damage * 4 && d.every > r.every * 3.0, "Dughan a heavy bolt, Remy quick knives");
    for (key, want) in [("parley_dughan", "chose:remy:arrest"), ("parley_remy", "chose:remy:free")] {
        let p = c.packs.iter().find(|p| p.key == key).unwrap();
        assert_eq!((p.wake.as_str(), p.after.as_deref()), ("parley_price", Some(want)));
    }
    // And a woken Dughan shoots Marzon's guard when it fights the hero.
    let (mut s, h) = parley_world();
    s.camp.counts.insert("chose:remy:arrest".into(), 1);
    open_at(&mut s, h, "parley_price");
    let dughan = parley_mob(&s, "parley_dughan");
    let at = s.mobs[&dughan].pos;
    let thug = s.content.mob("defias_thug").unwrap();
    let k = s.spawn_mob(thug, [at[0] + 8.0, at[1], at[2]], None, false);
    s.mobs.get_mut(&k).unwrap().state = MobState::Chase { foe: Foe::Hero(h) };
    s.mobs.get_mut(&k).unwrap().ready_at = 1e9;
    let m = parley_mob(&s, "marzon");
    fell(&mut s, h, m);
    let mut shot = false;
    for _ in 0..(8.0 / 0.05) as usize {
        for e in step_keeping_alive(&mut s, h) {
            shot |= matches!(e, Event::Missile { caster, target: Target::Unit(t), .. } if caster == dughan && t == k);
        }
        s.mobs.get_mut(&k).unwrap().state = MobState::Chase { foe: Foe::Hero(h) };
        s.mobs.get_mut(&k).unwrap().pos = [at[0] + 8.0, at[1], at[2]];
    }
    assert!(shot, "Dughan shot the one on the hero");
}

#[test]
fn the_satchel_stays_shut_until_marzon_dies_and_the_letter_is_read_once_he_has() {
    let (mut s, h) = parley_world();
    let ev = open_at(&mut s, h, "marzon_satchel");
    assert!(told(&ev, "Marzon keeps the satchel on his belt."), "{ev:?}");
    assert!(chest(&s, "marzon_satchel").is_some());
    open_at(&mut s, h, "parley_price");
    let m = parley_mob(&s, "marzon");
    fell(&mut s, h, m);
    assert!(s.camp_has("kill:marzon"));
    let silver = s.content.item("silver").unwrap();
    let before = s.players[&h].bag.get(&silver).copied().unwrap_or(0);
    let ev = open_at(&mut s, h, "marzon_satchel");
    assert!(told(&ev, "A letter under a lion's seal, signed only 'G. L.'"));
    assert!(s.camp_has("open:marzon_satchel"));
    assert!(s.players[&h].bag.get(&silver).copied().unwrap_or(0) > before);
}

#[test]
fn wyle_names_the_lord_and_the_deadmines_once_the_satchel_is_open() {
    let mut s = world();
    s.camp.raids_won = 3;
    s.camp.counts.insert("kill:bh_paymaster".into(), 1);
    run(&mut s, 5.5);
    let (wyle, _) = person(&s, "Marshal Wyle");
    assert!(s.talk_line(wyle).unwrap().text.starts_with("The Brotherhood paid the Horde's way"));
    s.camp.counts.insert("open:marzon_satchel".into(), 1);
    let t = s.talk_line(wyle).unwrap();
    assert!(t.text.starts_with("Lord Gregor Lescovar. A seat in the House of Nobles."), "{}", t.text);
    assert_eq!(t.links, [".es lore deadmines~The Deadmines"]);
    assert!(s.content.dungeons().any(|d| d.key == "deadmines"));
}

/// Marzon's rule: he answers every swing. A hero who swings and misses in reach is stabbed within
/// 0.3 sec of the moment they are open; one who stands still waits about his (slow) pace.
#[test]
fn after_a_swing_that_misses_marzon_starts_his_windup_within_a_third_of_a_second() {
    let (mut s, h) = parley_world();
    s.mobs.clear();
    let def = s.content.mob("marzon").unwrap();
    let m = s.spawn_mob(def, [2.4, 0.0, 0.0], None, false);
    s.mobs.get_mut(&m).unwrap().state = MobState::Chase { foe: Foe::Hero(h) };
    s.mobs.get_mut(&m).unwrap().ready_at = s.clock + 6.0;
    s.mobs.get_mut(&m).unwrap().facing = std::f32::consts::PI;
    // The hero swings the other way: it misses.
    s.move_player(h, [0.0, 0.0, 0.0], std::f32::consts::PI, Moving::default());
    let _ = s.step(0.0);
    s.input(h, Input::Attack);
    let mut open_at_clock = None;
    let mut windup_at = None;
    for _ in 0..(1.5 / 0.02) as usize {
        let _ = s.step(0.02);
        if let Some(p) = s.players.get_mut(&h) {
            p.health = 1_000_000;
        }
        let open = matches!(&s.players[&h].action, Action::Swing { t, strike, .. } if *t > strike.windup + strike.active);
        if open && open_at_clock.is_none() {
            open_at_clock = Some(s.clock);
        }
        if windup_at.is_none() && matches!(s.mobs[&m].state, MobState::Windup { .. }) {
            windup_at = Some(s.clock);
        }
    }
    let (open, wind) = (open_at_clock.expect("the swing ended"), windup_at.expect("he answered"));
    assert!(wind - open <= 0.3, "he began {:.2} sec after they were open", wind - open);
    assert!(wind >= open - 0.001, "and not before: the cooldown still had 6 sec to run");
}

#[test]
fn with_no_swing_marzon_waits_about_his_slow_pace_between_strings() {
    // The cooldown he sets as a string of his ends, with a hero who stands and takes it: his 1.3 sec
    // times his 1.8 pace, where an ordinary one's is the 1.3 sec.
    let cooldowns = |temper: &str| -> Vec<f32> {
        let (mut s, h) = parley_world();
        s.mobs.clear();
        let def = s.content.mob("marzon").unwrap();
        s.content.mobs[def].temper = temper.to_string();
        // (No dice: only the pace is measured.)
        s.content.ai.jitter = 0.0;
        s.content.ai.quirk = 0.0;
        let m = s.spawn_mob(def, [2.4, 0.0, 0.0], None, false);
        s.mobs.get_mut(&m).unwrap().state = MobState::Chase { foe: Foe::Hero(h) };
        let mut out = Vec::new();
        let mut in_string = false;
        for _ in 0..(60.0 / 0.02) as usize {
            let _ = s.step(0.02);
            if let Some(p) = s.players.get_mut(&h) {
                p.health = 1_000_000;
            }
            let on = s.mobs[&m].pattern.is_some();
            if in_string && !on {
                out.push(s.mobs[&m].ready_at - s.clock);
            }
            in_string = on;
        }
        out
    };
    let slow = cooldowns("silent");
    let plain = cooldowns("");
    assert!(slow.len() >= 3 && plain.len() >= 3, "strings came and went: {slow:?} / {plain:?}");
    let c = Content::builtin();
    let cooldown = c.mobs[c.mob("marzon").unwrap()].attack.cooldown;
    let near = |v: &[f32], want: f32| v.iter().all(|x| (x - want).abs() < 0.15);
    assert!(near(&slow, cooldown * 1.8), "his cooldown is {} sec: {slow:?}", cooldown * 1.8);
    assert!(near(&plain, cooldown), "an ordinary one's is {cooldown} sec: {plain:?}");
}

#[test]
fn marzons_fan_of_knives_comes_once_at_half_health_and_nothing_blocks_it() {
    let c = Content::builtin();
    let d = &c.mobs[c.mob("marzon").unwrap()];
    assert!(d.boss && d.health == 700 && d.unparryable == false);
    assert_eq!((d.aggro_text.as_str(), d.death_text.as_str()), ("You read the book. Pity.", "He'll only send another."));
    let fan = d.patterns.iter().find(|p| p.name == "fan of knives").unwrap();
    assert_eq!(fan.weight, 0.0, "kept for the turn alone");
    let st = &fan.steps[0];
    assert_eq!((st.arc, st.range, st.blockable, st.kit), (Some(360.0), Some(4.0), Some(false), 124));
    let phase = d.phases.iter().find(|p| p.at == 0.5).unwrap();
    assert_eq!((phase.pattern.as_str(), phase.text.as_str()), ("fan of knives", "Enough talk."));
    assert!(phase.speed == 1.0 && phase.might == 1.0 && phase.summon.is_empty(), "it is not an enrage");
    let stabs = d.patterns.iter().find(|p| p.name == "stabs").unwrap();
    assert!(stabs.parry_ends && stabs.steps.len() == 3, "a parryable three-stab string that a parry ends");
    assert!(d.patterns.iter().any(|p| p.name == "lunge" && p.min_range >= 4.0));
    // The silent temper.
    let t = c.tempers.iter().find(|t| t.key == "silent").unwrap();
    assert_eq!((t.punish, t.pace, t.retreat, t.retreat_yards), (Some(true), 1.8, Some(0.5), Some(4.0)));
    // In the sim: at half his health the fan is begun at once, with his line, and not again.
    let (mut s, h) = parley_world();
    s.mobs.clear();
    let def = s.content.mob("marzon").unwrap();
    let m = s.spawn_mob(def, [2.4, 0.0, 0.0], None, false);
    s.mobs.get_mut(&m).unwrap().state = MobState::Chase { foe: Foe::Hero(h) };
    let max = s.mobs[&m].max_health;
    s.hurt_mob(m, h, max / 2 + 5);
    assert!(s.mobs[&m].pattern.is_none(), "not before half");
    let mut said = Vec::new();
    let mut fans = 0;
    s.hurt_mob(m, h, 20);
    for _ in 0..(8.0 / 0.05) as usize {
        for e in step_keeping_alive(&mut s, h) {
            if let Event::Yell { text, .. } = e {
                said.push(text);
            }
        }
        if s.mobs[&m].pattern.is_some_and(|(k, _)| d.patterns[k].name == "fan of knives") && matches!(s.mobs[&m].state, MobState::Windup { .. }) {
            fans += 1;
        }
    }
    assert!(said.iter().any(|t| t == "Enough talk."), "{said:?}");
    assert!(fans > 0, "the fan was begun");
}

#[test]
fn the_parley_stones_are_a_place_with_guards_an_envoy_and_a_table() {
    let c = Content::builtin();
    let site = c.sites.iter().find(|s| s.key == "parley_stones").unwrap();
    assert_eq!((site.kind.as_str(), site.radius, site.at), ("foes", 35.0, [-9260.0, -800.0]));
    let packs = |mob: &str| c.packs.iter().filter(|p| p.mob == mob && dist2d([p.at[0], p.at[1], 0.0], [site.at[0], site.at[1], 0.0]) < site.radius).collect::<Vec<_>>();
    assert_eq!(packs("defias_thug").iter().map(|p| p.count).sum::<usize>(), 2, "2 thugs");
    assert_eq!(packs("defias_archer").iter().map(|p| p.count).sum::<usize>(), 2, "2 archers");
    let envoy = &c.mobs[c.mob("blackrock_envoy").unwrap()];
    assert_eq!((envoy.display, envoy.scale, envoy.temper.as_str()), (1359, 1.1, "coward"));
    assert_eq!(packs("blackrock_envoy").len(), 1);
    // The table and the satchel stand at it.
    for k in ["parley_price", "marzon_satchel"] {
        let ch = c.chests.iter().find(|ch| ch.key == k).unwrap();
        assert!(dist2d([ch.at[0], ch.at[1], 0.0], [site.at[0], site.at[1], 0.0]) < site.radius, "{k}");
    }
    assert_eq!(c.chests.iter().find(|ch| ch.key == "parley_price").unwrap().ambush, ["marzon", "parley_dughan", "parley_remy"]);
    assert_eq!(c.chests.iter().find(|ch| ch.key == "marzon_satchel").unwrap().after, "marzon");
}

#[test]
fn the_journal_ends_the_story_with_three_goals_and_every_corner_counts_38_places() {
    let c = Content::builtin();
    let at = |t: &str| c.goals.iter().position(|g| g.title == t).unwrap();
    let wagon = at("The Stonefield wagon");
    for (k, (t, kind, what)) in [("The Parley Stones", "explore", "parley_stones"), ("Marzon the Silent Blade", "kill", "marzon"), ("The seal on the letter", "open", "marzon_satchel")].into_iter().enumerate() {
        assert_eq!(at(t), wagon + 1 + k, "{t} in its place");
        let g = &c.goals[at(t)];
        assert_eq!((g.kind.as_str(), g.what.as_str()), (kind, what));
    }
    // (Round three's goals follow the seal, before Every corner: `tests_story3`.)
    assert_eq!(at("The seal-cutter"), wagon + 4);
    assert!(crate::save::ELWYNN_GOALS >= 15);
    assert!(c.sites.len() >= 38);
    assert_eq!(c.goals[at("Every corner")].count as usize, c.sites.len());
    // Each is done by what the camp does, in order.
    let mut s = world();
    s.camp.goal = wagon + 1;
    s.step_goals();
    assert_eq!(s.camp.goal, wagon + 1);
    s.camp.counts.insert("explore:parley_stones".into(), 1);
    s.camp.counts.insert("open:marzon_satchel".into(), 1);
    s.step_goals();
    assert_eq!(s.content.goals[s.camp.goal].title, "Marzon the Silent Blade", "the seal doesn't count before Marzon");
    s.camp.counts.insert("kill:marzon".into(), 1);
    s.step_goals();
    assert_eq!(s.content.goals[s.camp.goal].title, "The seal-cutter");
}

#[test]
fn a_pack_after_must_be_real_and_belong_to_a_chest_or_validate_refuses_it() {
    let text = source("survival.toml");
    let dungeon = source("deadmines.toml");
    assert!(Content::parse_with(&text, &dungeon).is_ok());
    let swap = |from: &str, to: &str| {
        let at = text.find(from).unwrap_or_else(|| panic!("no {from}"));
        let mut t = text.clone();
        t.replace_range(at..at + from.len(), to);
        Content::parse_with(&t, &dungeon)
    };
    let err = swap("after = \"chose:remy:arrest\"\n\n[[pack]]\nmob = \"parley_remy\"", "after = \"chose:remy:nope\"\n\n[[pack]]\nmob = \"parley_remy\"").unwrap_err();
    assert!(format!("{err:#}").contains("nope"), "{err:#}");
    let err = swap("key = \"parley_remy\"\nwake = \"parley_price\"\nafter = \"chose:remy:free\"", "key = \"parley_remy\"\nafter = \"chose:remy:free\"").unwrap_err();
    // (A pack's `after` may stand on its own now, round three's E7, but the parley table still
    // names it in its ambush, and an ambush wakes on its chest.)
    assert!(format!("{err:#}").contains("ambush parley_remy needs a [[pack]]"), "{err:#}");
}
