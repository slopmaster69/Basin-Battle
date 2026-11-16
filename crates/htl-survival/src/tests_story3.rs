//! The Elwynn expansion's third round, "The Lord's Answer" (docs/elwynn-expansion.md): the
//! seal-cutter's escort. The helpers are `tests_story`'s.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::{Foe, MobState};
use crate::sim::*;
use crate::tests_story::*;

// --- Stage 11: The Witness ---------------------------------------------------------------------

/// The built-in world on flat ground with only the escort's creatures, the letter read (so the
/// satchel opens), and a hero by the satchel.
fn pell_world() -> (Survival, Id) {
    let mut c = Content::builtin();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 5, 1_000_000.0);
    let keep: Vec<Id> = s.mobs.iter().filter(|(_, m)| m.pack.is_some_and(|k| s.content.packs[k].key.starts_with("pell_"))).map(|(id, _)| *id).collect();
    s.mobs.retain(|id, _| keep.contains(id));
    s.respawns.clear();
    s.camp.counts.insert("open:marzon_satchel".into(), 1);
    let h = bh_hero(&mut s, [-9243.0, 178.0, 0.0]);
    (s, h)
}

fn satchel_def(s: &Survival) -> usize {
    s.content.chests.iter().position(|c| c.key == "pell_satchel").unwrap()
}

fn pell(s: &Survival) -> Id {
    *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "aldric_pell").map(|(id, _)| id).expect("Pell")
}

/// The escort's foes that are up (awake, alive, not friendly).
fn knives(s: &Survival) -> Vec<Id> {
    s.mobs.iter().filter(|(_, m)| m.health > 0 && !m.hidden && !m.friendly && m.pack.is_some_and(|k| s.content.packs[k].key.starts_with("pell_"))).map(|(id, _)| *id).collect()
}

fn keep_up(s: &mut Survival, h: Id) {
    if let Some(p) = s.players.get_mut(&h) {
        p.health = 1_000_000;
    }
}

/// A step with the hero kept at Pell's side (3 yd off), alive; with `cut`, every knife that comes is
/// felled at once. The announcements said.
fn walk_with(s: &mut Survival, h: Id, cut: bool) -> Vec<String> {
    if cut {
        for k in knives(s) {
            fell(s, h, k);
        }
    }
    let w = pell(s);
    let at = s.mobs[&w].pos;
    s.move_player(h, [at[0] - 3.0, at[1], at[2]], 0.0, Moving::default());
    let ev = s.step(0.05);
    keep_up(s, h);
    ev.into_iter().filter_map(|e| match e { Event::Announce { text } => Some(text), _ => None }).collect()
}

#[test]
fn the_satchel_waits_for_the_letter_then_sets_pell_walking_and_the_first_knives_go_for_him() {
    let (mut s, h) = pell_world();
    s.camp.counts.remove("open:marzon_satchel");
    let ev = open_at(&mut s, h, "pell_satchel");
    assert!(ev.iter().any(|e| matches!(e, Event::Tell { text, .. } if text.contains("isn't ready to go"))), "shut until the letter is read");
    assert!(s.trials.is_empty());
    s.camp.counts.insert("open:marzon_satchel".into(), 1);
    let ev = open_at(&mut s, h, "pell_satchel");
    let banners: Vec<(String, String)> = ev.iter().filter_map(|e| match e { Event::Banner { title, sub, .. } => Some((title.clone(), sub.clone())), _ => None }).collect();
    assert!(banners.contains(&("The Seal-Cutter's Road".into(), "Stay near Aldric Pell".into())), "{banners:?}");
    assert!(ev.iter().any(|e| matches!(e, Event::Announce { text } if text == "Knives in the trees. They want Pell.")));
    let w = pell(&s);
    assert!(!s.mobs[&w].hidden && s.mobs[&w].friendly, "Pell is up, on the heroes' side");
    let first = knives(&s);
    assert_eq!(first.len(), 3, "wave 1: 3 footpads");
    for k in first {
        assert_eq!(s.mobs[&k].state.foe(), Some(Foe::Mob(w)), "they go for Pell, not the hero");
    }
    // A second use while he walks only says so.
    let o = chest(&s, "pell_satchel").unwrap();
    s.interact(h, Some(o));
    let ev = run(&mut s, 2.3);
    assert!(ev.iter().any(|e| matches!(e, Event::Tell { text, .. } if text.contains("already on the road"))), "{ev:?}");
}

#[test]
fn pell_walks_only_with_a_hero_near_and_waits_while_anything_is_on_him() {
    let (mut s, h) = pell_world();
    let def = satchel_def(&s);
    open_at(&mut s, h, "pell_satchel");
    let w = pell(&s);
    let start = s.mobs[&w].pos;
    // The first knives are coming for him: he waits.
    for _ in 0..40 {
        walk_with(&mut s, h, false);
    }
    assert!(dist2d(s.mobs[&w].pos, start) < 0.5, "he waits while knives are on him");
    // They fall: he sets off.
    for _ in 0..80 {
        walk_with(&mut s, h, true);
    }
    let gone = dist2d(s.mobs[&w].pos, start);
    assert!(gone > 5.0, "he walks with a hero beside him: {gone}");
    // The hero leaves him (past 20 yd): he stops where he is.
    let at = s.mobs[&w].pos;
    s.move_player(h, [at[0] - 30.0, at[1] - 30.0, 0.0], 0.0, Moving::default());
    let _ = s.step(0.05);
    let stopped = s.mobs[&w].pos;
    run(&mut s, 3.0);
    assert!(dist2d(s.mobs[&w].pos, stopped) < 0.6, "no hero near: he waits");
    assert!(s.trials.contains_key(&def), "still on (the hero is within 60 yd)");
}

#[test]
fn the_waves_come_as_pell_sets_off_from_their_points_and_the_escort_passes_at_the_gate() {
    let (mut s, h) = pell_world();
    for m in s.content.mobs.iter_mut() {
        m.drops.clear();
        m.trophy.clear();
    }
    let def = satchel_def(&s);
    let route = s.content.chests[def].route.clone();
    open_at(&mut s, h, "pell_satchel");
    let mut said: Vec<(usize, String)> = Vec::new();
    let mut banners = Vec::new();
    for _ in 0..(400.0 / 0.05) as usize {
        for k in knives(&s) {
            fell(&mut s, h, k);
        }
        let w = pell(&s);
        let at = s.mobs[&w].pos;
        s.move_player(h, [at[0] - 3.0, at[1], at[2]], 0.0, Moving::default());
        let leg = s.trials.get(&def).map_or(usize::MAX, |t| t.leg);
        for e in s.step(0.05) {
            match e {
                Event::Announce { text } if !text.contains("fell") => said.push((leg, text)),
                Event::Banner { title, .. } => banners.push(title),
                _ => {}
            }
        }
        keep_up(&mut s, h);
        if !s.trials.contains_key(&def) {
            break;
        }
    }
    assert!(!s.trials.contains_key(&def), "the escort ended");
    let waves: Vec<(usize, String)> = said.iter().filter(|(_, t)| t.contains("Pell") || t.contains("road ahead") || t.contains("witness")).cloned().collect();
    assert_eq!(waves.len(), 2, "waves 2 and 3 called on the way (wave 1 at the start): {said:?}");
    // Eight points, three waves: the second as he sets off from point 3, the third from point 6.
    assert!(waves[0].1.contains("road ahead") && waves[0].0 >= 2 && waves[0].0 <= 3, "{waves:?}");
    assert!(waves[1].1.contains("witness") && waves[1].0 >= 5 && waves[1].0 <= 6, "{waves:?}");
    assert!(banners.iter().any(|b| b == "Pell Is Through"), "{banners:?}");
    assert!(s.camp_has("open:pell_satchel"), "the escort counts as the satchel opened");
    assert!(chest(&s, "pell_satchel").is_none(), "once a world");
    let end = route.last().unwrap();
    let bag = &s.players[&h].bag;
    for (k, n) in [("silver", 3), ("bronze", 2)] {
        assert_eq!(bag.get(&s.content.item(k).unwrap()), Some(&n), "{k}");
    }
    assert!(dist2d(s.players[&h].pos, [end[0], end[1], 0.0]) < 8.0, "the hero walked him to the gate");
    // The walker stands down, and Pell stands at the gate with words of his own.
    assert!(s.mobs[&pell(&s)].hidden);
    run(&mut s, 5.5);
    let (pdef, _) = person(&s, "Aldric Pell");
    assert!(s.talk_line(pdef).unwrap().text.starts_with("Lescovar's ring, my cut."));
    let (wdef, _) = {
        s.camp.raids_won = 3;
        run(&mut s, 5.5);
        person(&s, "Marshal Wyle")
    };
    assert!(s.talk_line(wdef).unwrap().text.starts_with("Pell's word and the lord's letter"));
}

#[test]
fn a_heros_blow_draws_a_knife_off_pell_for_good() {
    let (mut s, h) = pell_world();
    open_at(&mut s, h, "pell_satchel");
    let w = pell(&s);
    let k = knives(&s)[0];
    assert_eq!(s.mobs[&k].state.foe(), Some(Foe::Mob(w)));
    s.hurt_mob(k, h, 1);
    for _ in 0..(6.0 / 0.05) as usize {
        walk_with(&mut s, h, false);
        assert_eq!(s.mobs[&k].state.foe(), Some(Foe::Hero(h)), "a footpad drawn off stays the hero's");
    }
}

/// Crane and Pell alone, Crane on Pell and a hero beside them.
fn crane_bare() -> (Survival, Id, Id, Id) {
    let mut s = world();
    let h = bh_hero(&mut s, [0.0, 2.0, 0.0]);
    let pd = s.content.mob("aldric_pell").unwrap();
    let w = s.spawn_mob(pd, [0.0, 0.0, 0.0], None, false);
    let cd = s.content.mob("crane").unwrap();
    let c = s.spawn_mob(cd, [6.0, 0.0, 0.0], None, false);
    s.engage(c, Foe::Mob(w));
    (s, h, w, c)
}

#[test]
fn crane_answers_a_heros_blow_for_one_string_then_goes_back_for_pell() {
    let (mut s, h, w, c) = crane_bare();
    assert!(s.content.mobs[s.mobs[&c].def].hunts && s.content.mobs[s.mobs[&c].def].boss);
    assert_eq!(s.mobs[&c].state.foe(), Some(Foe::Mob(w)));
    // He goes for Pell and cuts him.
    let mut hurt = false;
    for _ in 0..(6.0 / 0.05) as usize {
        let _ = s.step(0.05);
        keep_up(&mut s, h);
        hurt |= s.mobs[&w].health < s.mobs[&w].max_health;
    }
    assert!(hurt, "Crane's blows land on Pell");
    // Wait until he is between blows, then strike him: he turns on the hero...
    for _ in 0..200 {
        if matches!(s.mobs[&c].state, MobState::Chase { .. }) {
            break;
        }
        let _ = s.step(0.05);
        keep_up(&mut s, h);
    }
    s.hurt_mob(c, h, 1);
    assert_eq!(s.mobs[&c].state.foe(), Some(Foe::Hero(h)), "he answers the hero");
    // ...for one string, and then he goes back for Pell.
    let mut back_at = None;
    for i in 0..(10.0 / 0.05) as usize {
        let _ = s.step(0.05);
        keep_up(&mut s, h);
        if s.mobs[&c].state.foe() == Some(Foe::Mob(w)) {
            back_at = Some(i);
            break;
        }
    }
    assert!(back_at.is_some(), "after one string on the hero he turns back for Pell");
}

#[test]
fn struck_as_he_winds_up_at_pell_crane_turns_the_blow_on_the_hero() {
    let (mut s, h, w, c) = crane_bare();
    for _ in 0..200 {
        if matches!(s.mobs[&c].state, MobState::Windup { foe: Foe::Mob(_), .. }) {
            break;
        }
        let _ = s.step(0.05);
        keep_up(&mut s, h);
    }
    assert!(matches!(s.mobs[&c].state, MobState::Windup { foe: Foe::Mob(x), .. } if x == w), "winding up at Pell");
    let pell_before = s.mobs[&w].health;
    s.hurt_mob(c, h, 1);
    assert!(matches!(s.mobs[&c].state, MobState::Windup { foe: Foe::Hero(x), .. } if x == h), "the blow turns on the hero");
    for _ in 0..30 {
        let _ = s.step(0.05);
        keep_up(&mut s, h);
    }
    assert_eq!(s.mobs[&w].health, pell_before, "Pell is spared that blow");
}

#[test]
fn cranes_throat_cut_comes_at_half_his_health_glows_red_and_nothing_blocks_it() {
    let c = Content::builtin();
    let crane = &c.mobs[c.mob("crane").unwrap()];
    let cut = crane.patterns.iter().find(|p| p.name == "throat-cut").unwrap();
    assert_eq!(cut.below, 0.5);
    let step = &cut.steps[0];
    assert_eq!((step.blockable, step.kit), (Some(false), 124));
    assert!(step.windup.unwrap() >= 1.2, "wound up long enough to see and to strike him in");
    let phase = &crane.phases[0];
    assert_eq!((phase.at, phase.pattern.as_str(), phase.speed, phase.might), (0.5, "throat-cut", 1.0, 1.0), "no enrage, no adds");
    assert!(phase.summon.is_empty());
}

#[test]
fn pell_struck_down_loses_the_escort_and_it_waits_to_be_tried_again() {
    let (mut s, h) = pell_world();
    let def = satchel_def(&s);
    open_at(&mut s, h, "pell_satchel");
    for _ in 0..60 {
        walk_with(&mut s, h, false);
    }
    let w = pell(&s);
    let start = s.content.packs[s.mobs[&w].pack.unwrap()].at;
    let max = s.mobs[&w].max_health;
    let mut lost = Vec::new();
    s.hurt_ward(w, max, [0.0, 0.0, 0.0]);
    for e in s.step(0.05) {
        if let Event::Announce { text } = e {
            lost.push(text);
        }
    }
    assert!(!s.trials.contains_key(&def), "lost");
    assert!(lost.iter().any(|t| t.starts_with("Aldric Pell is struck down")), "{lost:?}");
    assert!(knives(&s).is_empty(), "the knives slip away");
    assert!(s.mobs.values().filter(|m| !m.friendly).all(|m| m.health == m.max_health), "whole");
    let m = &s.mobs[&w];
    assert!(m.hidden && m.health == m.max_health && dist2d(m.pos, [start[0], start[1], 0.0]) < 1.0, "Pell back where he waits, whole");
    assert!(!s.camp_has("open:pell_satchel") && chest(&s, "pell_satchel").is_some(), "the satchel waits");
    // Used again, it begins from the first wave.
    let ev = open_at(&mut s, h, "pell_satchel");
    assert!(ev.iter().any(|e| matches!(e, Event::Announce { text } if text == "Knives in the trees. They want Pell.")));
    assert_eq!(knives(&s).len(), 3);
}

#[test]
fn walking_away_from_pell_for_12_sec_gives_the_escort_up() {
    let (mut s, h) = pell_world();
    let def = satchel_def(&s);
    open_at(&mut s, h, "pell_satchel");
    s.move_player(h, [-9243.0, 300.0, 0.0], 0.0, Moving::default());
    for _ in 0..(14.0 / 0.05) as usize {
        let _ = s.step(0.05);
        keep_up(&mut s, h);
    }
    assert!(!s.trials.contains_key(&def));
    assert!(s.mobs[&pell(&s)].hidden);
}

#[test]
fn remys_choice_is_paid_on_pells_road_both_ways() {
    // Freed: he sold Pell's hour, and wave 2 brings 2 more footpads.
    let (mut s, h) = pell_world();
    s.camp.counts.insert("chose:remy".into(), 1);
    s.camp.counts.insert("chose:remy:free".into(), 1);
    let def = satchel_def(&s);
    open_at(&mut s, h, "pell_satchel");
    assert!(s.mobs.values().all(|m| s.content.mobs[m.def].key != "pell_watchman" || m.hidden), "no watchman for a freed Remy");
    for _ in 0..(200.0 / 0.05) as usize {
        if s.trials.get(&def).is_some_and(|t| t.called >= 2) {
            break;
        }
        walk_with(&mut s, h, true);
    }
    let keys: Vec<String> = knives(&s).iter().map(|k| s.content.mobs[s.mobs[k].def].key.clone()).collect();
    assert_eq!(keys.iter().filter(|k| *k == "defias_thug").count(), 2, "{keys:?}");
    assert_eq!(keys.iter().filter(|k| *k == "defias_footpad").count(), 2, "Remy's two: {keys:?}");
    // Arrested: no extra knives, and Dughan's watchman waits at Pell's side with a bow.
    let (mut s, h) = pell_world();
    s.camp.counts.insert("chose:remy".into(), 1);
    s.camp.counts.insert("chose:remy:arrest".into(), 1);
    open_at(&mut s, h, "pell_satchel");
    let watch: Vec<Id> = s.mobs.iter().filter(|(_, m)| s.content.mobs[m.def].key == "pell_watchman" && !m.hidden).map(|(id, _)| *id).collect();
    assert_eq!(watch.len(), 1, "the watchman wakes");
    assert!(s.mobs[&watch[0]].friendly && s.content.mobs[s.mobs[&watch[0]].def].shoots.is_some());
    // He shoots what is on Pell.
    let k = knives(&s)[0];
    let before = s.mobs[&k].health;
    let at = s.mobs[&watch[0]].pos;
    s.mobs.get_mut(&k).unwrap().pos = [at[0] + 8.0, at[1], at[2]];
    run(&mut s, 2.0);
    assert!(s.mobs.get(&k).is_none_or(|m| m.health < before), "the watchman shoots a knife on Pell");
    for _ in 0..(200.0 / 0.05) as usize {
        if s.trials.get(&def).is_some_and(|t| t.called >= 2) {
            break;
        }
        walk_with(&mut s, h, true);
    }
    let keys: Vec<String> = knives(&s).iter().map(|k| s.content.mobs[s.mobs[k].def].key.clone()).collect();
    assert_eq!(keys, ["defias_thug", "defias_thug"], "no one sold the hour");
}

#[test]
fn remy_and_dughan_each_have_a_word_once_pell_is_through() {
    let c = Content::builtin();
    let line = |name: &str| c.props.iter().find(|p| p.npc && p.name == name).unwrap().talk.iter().find(|t| t.after.as_deref() == Some("open:pell_satchel")).map(|t| t.text.clone());
    assert!(line("Remy \"Two Times\"").unwrap().contains("I only sold it once this week"));
    assert!(line("Marshal Dughan").unwrap().contains("Remy's word put him there"));
}

#[test]
fn an_escort_must_be_one_friendly_walker_that_wakes_on_it_with_a_route_and_waves() {
    let text = source("survival.toml");
    let dungeon = source("deadmines.toml");
    assert!(Content::parse_with(&text, &dungeon).is_ok());
    let swap = |from: &str, to: &str| {
        let at = text.find(from).unwrap_or_else(|| panic!("no {from}"));
        let mut t = text.clone();
        t.replace_range(at..at + from.len(), to);
        Content::parse_with(&t, &dungeon)
    };
    let err = swap("escort = \"pell_walk\"", "escort = \"pell_nobody\"").unwrap_err();
    assert!(format!("{err:#}").contains("no [[pack]]"), "{err:#}");
    let err = swap("escort = \"pell_walk\"", "escort = \"pell_crane\"").unwrap_err();
    assert!(format!("{err:#}").contains("friendly"), "{err:#}");
    let err = swap("mob = \"aldric_pell\"\nat = [-9238.0, 182.0]\nradius = 0.0\ncount = 1", "mob = \"aldric_pell\"\nat = [-9238.0, 182.0]\nradius = 0.0\ncount = 2").unwrap_err();
    assert!(format!("{err:#}").contains("one creature"), "{err:#}");
    let err = swap("escort = \"pell_walk\"\nroute = [[-9214.0", "hold = 30.0\nescort = \"pell_walk\"\nroute = [[-9214.0").unwrap_err();
    assert!(format!("{err:#}").contains("no `hold`"), "{err:#}");
    let err = swap("escort = \"pell_walk\"\n", "").unwrap_err();
    assert!(format!("{err:#}").contains("a route is for an escort"), "{err:#}");
}

#[test]
fn the_witness_follows_the_seal_in_the_journal_and_the_gate_is_a_place() {
    let c = Content::builtin();
    let at = |t: &str| c.goals.iter().position(|g| g.title == t).unwrap_or_else(|| panic!("no goal {t}"));
    let seal = at("The seal on the letter");
    for (k, (t, kind, what)) in [("The seal-cutter", "open", "pell_satchel"), ("The Lion's Knife", "kill", "crane")].into_iter().enumerate() {
        assert_eq!(at(t), seal + 1 + k, "{t} in its place");
        let g = &c.goals[at(t)];
        assert_eq!((g.kind.as_str(), g.what.as_str()), (kind, what));
    }
    assert!(at("Every corner") > at("The Lion's Knife"));
    assert!(crate::save::ELWYNN_GOALS >= 17);
    let gate = c.sites.iter().find(|s| s.key == "stormwind_gate").unwrap();
    assert_eq!(gate.kind, "landmark");
    let route = &c.chests.iter().find(|ch| ch.key == "pell_satchel").unwrap().route;
    let end = route.last().unwrap();
    assert!(dist2d([end[0], end[1], 0.0], [gate.at[0], gate.at[1], 0.0]) < gate.radius, "the road ends at the gate");
    assert!(c.sites.len() >= 39);
    assert_eq!(c.goals[at("Every corner")].count as usize, c.sites.len());
    // Done in order: the escort, then Crane.
    let mut s = world();
    s.camp.goal = seal + 1;
    s.camp.counts.insert("kill:crane".into(), 1);
    s.step_goals();
    assert_eq!(s.camp.goal, seal + 1, "Crane doesn't count before the escort");
    s.camp.counts.insert("open:pell_satchel".into(), 1);
    s.step_goals();
    assert_eq!(s.camp.goal, seal + 3);
}
