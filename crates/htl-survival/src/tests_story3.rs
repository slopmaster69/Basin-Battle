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

// --- Stage 12: The Lion's Bargain --------------------------------------------------------------

/// Whether the camp's person called `name` stands.
fn stands(s: &Survival, name: &str) -> bool {
    s.content.props.iter().position(|p| p.npc && p.name == name).is_some_and(|def| s.objs.values().any(|o| matches!(o.kind, ObjKind::Piece { def: d } if d == def)))
}

/// Pell through and the steward at the camp's gate, a hero beside him.
fn venn_world() -> (Survival, Id) {
    let mut c = Content::builtin();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 1, 1_000_000.0);
    // Only the archers' packs stand (unseen until the camp keeps the letter).
    let keep: Vec<Id> = s.mobs.iter().filter(|(_, m)| m.pack.is_some_and(|k| s.content.packs[k].key.starts_with("letter_"))).map(|(id, _)| *id).collect();
    s.mobs.retain(|id, _| keep.contains(id));
    s.respawns.clear();
    s.camp.counts.insert("open:pell_satchel".into(), 1);
    s.camp.raids_won = 3;
    run(&mut s, 5.5);
    let (_, id) = person(&s, "Steward Venn");
    let h = beside(&mut s, id);
    // (The purse beside him, which `beside` cleared away, back where it stands.)
    s.replace_chests();
    (s, h)
}

fn archers(s: &Survival) -> Vec<Id> {
    s.mobs.iter().filter(|(_, m)| s.content.mobs[m.def].key == "stormwind_archer" && !m.hidden && m.health > 0).map(|(id, _)| *id).collect()
}

#[test]
fn steward_venn_waits_for_pell_and_offers_to_buy_the_letter_back() {
    let mut s = world();
    run(&mut s, 5.5);
    assert!(!stands(&s, "Steward Venn"), "not before Pell is through");
    let (mut s, h) = venn_world();
    s.interact(h, None);
    let said = letters(&s.step(0.0));
    assert_eq!(said.len(), 1);
    assert!(said[0].2.starts_with("His lordship hears the camp has something of his."));
    let links: Vec<(&str, &str)> = said[0].3.iter().map(|(c, n)| (c.as_str(), n.as_str())).collect();
    assert_eq!(links, [(".es choose letter:sell", "Sell the Letter"), (".es choose letter:keep", "Keep the Letter")]);
    // A building can't go up on him (he keeps his ground), and he stands outside the camp's ground.
    let venn = s.content.props.iter().find(|p| p.name == "Steward Venn").unwrap();
    let c = s.content.world.build_center;
    assert!(dist2d([venn.at[0], venn.at[1], 0.0], [c[0], c[1], 0.0]) > s.content.world.build_radius, "outside the gate");
}

#[test]
fn sold_the_purse_opens_pell_leaves_and_no_archers_come() {
    let (mut s, h) = venn_world();
    assert!(stands(&s, "Aldric Pell"));
    let ev = open_at(&mut s, h, "lion_purse");
    assert!(ev.iter().any(|e| matches!(e, Event::Tell { text, .. } if text.contains("Nobody has sold"))), "shut before the choice");
    let (_, id) = person(&s, "Steward Venn");
    let at = s.objs[&id].pos;
    s.move_player(h, [at[0] + 1.5, at[1], at[2]], 0.0, Moving::default());
    let _ = s.step(0.0);
    assert!(s.choose(h, "letter:sell").is_ok());
    assert!(s.choose(h, "letter:keep").unwrap_err().contains("already decided"));
    let _ = s.step(0.05);
    assert!(!stands(&s, "Steward Venn"), "he goes with the answer");
    assert!(!stands(&s, "Aldric Pell"), "Pell goes too");
    open_at(&mut s, h, "lion_purse");
    let bag = &s.players[&h].bag;
    for (k, n) in [("silver", 8), ("bronze", 4), ("pearl", 2)] {
        assert_eq!(bag.get(&s.content.item(k).unwrap()), Some(&n), "{k}");
    }
    run(&mut s, 6.0);
    assert!(archers(&s).is_empty(), "no archers for a camp that sold");
    let (wdef, _) = person(&s, "Marshal Wyle");
    assert_eq!(s.talk_line(wdef).unwrap().text, "We sold it. Like everyone else in Elwynn.");
}

#[test]
fn kept_pell_stays_and_two_stormwind_archers_stand_at_the_gate_for_good() {
    let (mut s, h) = venn_world();
    assert!(archers(&s).is_empty(), "unseen before the camp answers");
    let (_, id) = person(&s, "Steward Venn");
    let at = s.objs[&id].pos;
    s.move_player(h, [at[0] + 1.5, at[1], at[2]], 0.0, Moving::default());
    let _ = s.step(0.0);
    assert!(s.choose(h, "letter:keep").is_ok());
    run(&mut s, 6.0);
    assert!(!stands(&s, "Steward Venn"));
    assert_eq!(archers(&s).len(), 2, "the archers come");
    assert!(archers(&s).iter().all(|a| s.mobs[a].friendly));
    let (pdef, _) = person(&s, "Aldric Pell");
    assert!(s.talk_line(pdef).unwrap().text.starts_with("The Guard keeps me at the gate"));
    let ev = open_at(&mut s, h, "lion_purse");
    assert!(ev.iter().any(|e| matches!(e, Event::Tell { text, .. } if text.contains("Nobody has sold"))), "no purse for a camp that kept it");
    // A save keeps them: read back, the archers stand at once.
    let save = s.save(&std::collections::BTreeMap::new());
    let back = Survival::load(s.content.clone(), Box::new(FlatLand::default()), 1, 1_000_000.0, &save);
    assert_eq!(archers(&back).len(), 2);
}

#[test]
fn the_archers_shoot_a_raider_at_the_camps_buildings() {
    let (mut s, h) = venn_world();
    s.camp.counts.insert("chose:letter".into(), 1);
    s.camp.counts.insert("chose:letter:keep".into(), 1);
    run(&mut s, 6.0);
    let a = archers(&s)[0];
    let at = s.mobs[&a].pos;
    // A raider near them, on its way to a station (no hero in its fight).
    let def = s.content.mob("defias_thug").unwrap();
    let r = s.spawn_mob(def, [at[0] + 10.0, at[1], at[2]], None, false);
    s.mobs.get_mut(&r).unwrap().raider = true;
    s.move_player(h, [at[0] - 30.0, at[1] - 30.0, at[2]], 0.0, Moving::default());
    let before = s.mobs[&r].health;
    run(&mut s, 3.0);
    assert!(s.mobs.get(&r).is_none_or(|m| m.health < before), "shot");
    // An idle creature that isn't raiding is left alone.
    let idle = s.spawn_mob(def, [at[0] + 10.0, at[1] + 6.0, at[2]], None, false);
    let before = s.mobs[&idle].health;
    for id in s.mobs.keys().copied().collect::<Vec<_>>() {
        if id != idle && !s.mobs[&id].friendly {
            s.mobs.remove(&id);
        }
    }
    run(&mut s, 3.0);
    assert_eq!(s.mobs[&idle].health, before);
}

#[test]
fn a_pack_with_a_condition_of_its_own_stays_unseen_until_it_holds() {
    let mut s = world();
    let k = s.content.packs.iter().position(|p| p.key == "letter_archer_east").unwrap();
    let def = s.content.mob("stormwind_archer").unwrap();
    let at = s.content.packs[k].at;
    let m = s.spawn_mob(def, [at[0], at[1], 0.0], Some(k), false);
    assert!(s.mobs[&m].hidden, "unseen before the choice");
    run(&mut s, 6.0);
    assert!(s.mobs[&m].hidden);
    s.camp.counts.insert("chose:letter".into(), 1);
    s.camp.counts.insert("chose:letter:sell".into(), 1);
    run(&mut s, 6.0);
    assert!(s.mobs[&m].hidden, "the other option doesn't bring them");
    let mut s = world();
    let m = s.spawn_mob(def, [at[0], at[1], 0.0], Some(k), false);
    s.camp.counts.insert("chose:letter".into(), 1);
    s.camp.counts.insert("chose:letter:keep".into(), 1);
    let ev = run(&mut s, 6.0);
    assert!(!s.mobs[&m].hidden);
    assert!(ev.iter().any(|e| matches!(e, Event::MobSpawned { id } if *id == m)), "drawn as it comes");
}

#[test]
fn the_bargain_follows_the_lions_knife_in_the_journal() {
    let c = Content::builtin();
    let at = |t: &str| c.goals.iter().position(|g| g.title == t).unwrap();
    assert_eq!(at("The Lion's Bargain"), at("The Lion's Knife") + 1);
    let g = &c.goals[at("The Lion's Bargain")];
    assert_eq!((g.kind.as_str(), g.what.as_str()), ("chose", "letter"));
    assert!(crate::save::ELWYNN_GOALS >= 18);
    let mut s = world();
    s.camp.goal = at("The Lion's Bargain");
    s.step_goals();
    assert_eq!(s.camp.goal, at("The Lion's Bargain"));
    s.camp.counts.insert("chose:letter".into(), 1);
    s.camp.counts.insert("chose:letter:keep".into(), 1);
    s.step_goals();
    assert!(s.camp.goal > at("The Lion's Bargain"));
}

// --- Stage 13: Sable, the Lion's Hound ---------------------------------------------------------

/// The built-in world on flat ground with only Sable's pack, and a hero far off.
fn sable_world() -> (Survival, Id) {
    let mut c = Content::builtin();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    let keep: Vec<Id> = s.mobs.iter().filter(|(_, m)| s.content.mobs[m.def].key == "sable").map(|(id, _)| *id).collect();
    s.mobs.retain(|id, _| keep.contains(id));
    s.respawns.clear();
    let h = bh_hero(&mut s, [-9000.0, 0.0, 0.0]);
    (s, h)
}

fn sable(s: &Survival) -> Id {
    *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "sable").map(|(id, _)| id).expect("Sable")
}

#[test]
fn sable_lies_unseen_on_the_roads_until_the_camp_has_answered_venn_then_steps_out_on_a_hero() {
    let (mut s, h) = sable_world();
    let k = sable(&s);
    assert!(s.mobs[&k].hidden);
    let at = s.mobs[&k].pos;
    // Before the answer: a hero walks right past her and nothing comes.
    s.move_player(h, [at[0] + 5.0, at[1], at[2]], 0.0, Moving::default());
    run(&mut s, 6.0);
    assert!(s.mobs[&k].hidden && s.mobs[&k].state.foe().is_none(), "nothing before the camp answers");
    // Either answer sends her.
    s.camp.counts.insert("chose:letter".into(), 1);
    s.camp.counts.insert("chose:letter:sell".into(), 1);
    s.move_player(h, [at[0] + 30.0, at[1], at[2]], 0.0, Moving::default());
    run(&mut s, 6.0);
    assert!(s.mobs[&k].hidden, "still unseen until a hero comes near");
    s.move_player(h, [at[0] + 10.0, at[1], at[2]], 0.0, Moving::default());
    let ev = run(&mut s, 0.5);
    assert!(!s.mobs[&k].hidden, "she steps out within 12 yd");
    assert_eq!(s.mobs[&k].state.foe(), Some(Foe::Hero(h)));
    assert!(ev.iter().any(|e| matches!(e, Event::Yell { text, .. } if text == "His lordship sends his regards.")), "and says why");
}

#[test]
fn sable_walks_three_spots_round_the_camp_clear_of_it_and_of_stitches() {
    let c = Content::builtin();
    let p = c.packs.iter().find(|p| p.mob == "sable").unwrap();
    assert!(p.rare && p.hidden == 12.0 && p.after.as_deref() == Some("chose:letter"));
    let camp = c.world.build_center;
    for sp in &p.spots {
        let d = dist2d([sp[0], sp[1], 0.0], [camp[0], camp[1], 0.0]);
        assert!(d > 100.0, "{sp:?} is {d} yd from the camp");
        for st in c.packs.iter().filter(|q| q.mob == "stitches") {
            for r in std::iter::once(&st.at).chain(st.route.iter()) {
                assert!(dist2d([sp[0], sp[1], 0.0], [r[0], r[1], 0.0]) > 60.0, "{sp:?} clear of Stitches' road");
            }
        }
    }
}

/// Sable on a hero 2.5 yd off, in the chase.
fn sable_bare() -> (Survival, Id, Id) {
    let mut s = world();
    let h = bh_hero(&mut s, [0.0, 0.0, 0.0]);
    s.rng = rand::SeedableRng::seed_from_u64(4);
    let def = s.content.mob("sable").unwrap();
    let k = s.spawn_mob(def, [2.5, 0.0, 0.0], None, false);
    s.engage(k, Foe::Hero(h));
    (s, h, k)
}

/// Seconds until Sable winds up at the hero, the hero turning to face her each step (`face`) or
/// keeping their back to her; none within `secs`.
fn first_windup(s: &mut Survival, h: Id, k: Id, face: bool, secs: f32) -> Option<f32> {
    let start = s.clock;
    for _ in 0..(secs / 0.05) as usize {
        let (me, her) = (s.players[&h].pos, s.mobs[&k].pos);
        let toward = (her[1] - me[1]).atan2(her[0] - me[0]);
        let f = if face { toward } else { toward + std::f32::consts::PI };
        s.move_player(h, me, f, Moving::default());
        let _ = s.step(0.05);
        keep_up(s, h);
        if matches!(s.mobs[&k].state, MobState::Windup { .. }) {
            return Some(s.clock - start);
        }
    }
    None
}

#[test]
fn sable_only_strikes_a_back_turn_with_her_and_she_circles() {
    let (mut s, h, k) = sable_bare();
    assert_eq!(s.content.mobs[s.mobs[&k].def].temper, "stalker");
    // Facing her, idle (no opening): she circles and never winds up.
    assert_eq!(first_windup(&mut s, h, k, true, 8.0), None, "no blow at a hero who turns with her");
    // Her back turned: the cut comes quickly.
    let at = first_windup(&mut s, h, k, false, 6.0);
    assert!(at.is_some_and(|t| t < 4.0), "a back is cut: {at:?}");
}

#[test]
fn at_half_her_health_sable_throws_down_the_heroes_beside_her_once_no_adds_no_enrage() {
    let c = Content::builtin();
    let m = &c.mobs[c.mob("sable").unwrap()];
    assert!(m.boss);
    assert_eq!(m.phases.len(), 1);
    let ph = &m.phases[0];
    assert_eq!((ph.at, ph.shove, ph.speed, ph.might), (0.5, 4.0, 1.0, 1.0));
    assert!(ph.summon.is_empty() && ph.text == "You'll tire before I do.");
}

#[test]
fn sables_fall_is_told_and_the_camp_has_words_for_it() {
    let (mut s, h) = sable_world();
    s.camp.counts.insert("chose:letter".into(), 1);
    s.camp.counts.insert("chose:letter:keep".into(), 1);
    let k = sable(&s);
    let at = s.mobs[&k].pos;
    s.move_player(h, [at[0] + 8.0, at[1], at[2]], 0.0, Moving::default());
    run(&mut s, 0.5);
    let max = s.mobs[&k].max_health;
    s.hurt_mob(k, h, max);
    let ev = run(&mut s, 0.5);
    assert!(ev.iter().any(|e| matches!(e, Event::Announce { text } if text.contains("Sable"))), "her fall is told");
    assert!(s.camp_has("kill:sable"));
    let c = &s.content;
    for name in ["Marshal Wyle", "Scout Marten", "Aldric Pell"] {
        let p = c.props.iter().find(|p| p.npc && p.name == name).unwrap();
        assert_eq!(p.talk.last().unwrap().after.as_deref(), Some("kill:sable"), "{name}'s last word is on her");
    }
}

#[test]
fn the_hound_closes_round_three_in_the_journal() {
    let c = Content::builtin();
    let at = |t: &str| c.goals.iter().position(|g| g.title == t).unwrap();
    assert_eq!(at("The Lion's Hound"), at("The Lion's Bargain") + 1);
    assert_eq!(at("Every corner"), at("The Lion's Hound") + 1);
    let g = &c.goals[at("The Lion's Hound")];
    assert_eq!((g.kind.as_str(), g.what.as_str()), ("kill", "sable"));
    assert_eq!(crate::save::ELWYNN_GOALS, 19);
    assert_eq!(c.sites.len(), 39);
    assert_eq!(c.goals[at("Every corner")].count, 39);
}
