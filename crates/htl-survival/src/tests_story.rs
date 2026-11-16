//! The camp's people talk and the story's conditions: a person with lines is usable and says the
//! last line the camp has earned, `kill:` and `open:` read what the camp has done, a condition
//! that names nothing is refused, and every name the content points at is real.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::*;
use std::collections::{BTreeMap, BTreeSet};

fn world() -> Survival {
    let mut c = Content::builtin();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 1, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / 0.05).round() as usize {
        out.extend(s.step(0.05));
    }
    out
}

/// The camp's person called `name`: their prop, and the object that stands for it.
fn person(s: &Survival, name: &str) -> (usize, Id) {
    let def = s.content.props.iter().position(|p| p.npc && p.name == name).unwrap_or_else(|| panic!("no {name}"));
    let id = s.objs.iter().find(|(_, o)| matches!(o.kind, ObjKind::Piece { def: d } if d == def)).map(|(id, _)| *id).unwrap_or_else(|| panic!("{name} isn't standing"));
    (def, id)
}

/// A hero beside `id`, with nothing else in reach to use.
fn beside(s: &mut Survival, id: Id) -> Id {
    let at = s.objs[&id].pos;
    s.objs.retain(|k, o| *k == id || dist2d(o.pos, at) > 6.0);
    let hero = s.join("ayla", "Ayla", None);
    s.move_player(hero, [at[0] + 1.5, at[1], at[2]], 0.0, Moving::default());
    let _ = s.step(0.0);
    hero
}

fn letters(ev: &[Event]) -> Vec<(String, String, String, Vec<(String, String)>)> {
    ev.iter().filter_map(|e| match e {
        Event::Letter { kind, title, text, links, .. } => Some((kind.clone(), title.clone(), text.clone(), links.clone())),
        _ => None,
    }).collect()
}

#[test]
fn a_person_with_lines_is_usable_and_prompts_talk_to_and_one_without_isnt() {
    let mut s = world();
    s.camp.counts.insert("build:cooking".into(), 1);
    run(&mut s, 5.5);
    let (def, id) = person(&s, "Quartermaster Hale");
    assert!(!s.content.props[def].talk.is_empty());
    assert!(s.object(id).unwrap().usable);
    let hero = beside(&mut s, id);
    assert_eq!(s.use_prompt(hero).map(|(o, n, v)| (o, n, v)), Some((id, "Quartermaster Hale".to_string(), "Talk to")));
    s.interact(hero, None);
    let said = letters(&s.step(0.0));
    assert_eq!(said.len(), 1, "one page");
    assert_eq!(said[0].0, "talk");
    assert_eq!(said[0].1, "Quartermaster Hale");
    assert!(said[0].2.contains("Keep the storage stocked"));
    // The same person with nothing to say isn't usable, and the key finds nothing.
    s.content.props[def].talk.clear();
    assert!(!s.object(id).unwrap().usable);
    assert_eq!(s.use_prompt(hero), None);
    // A prop that's no person never is.
    let lamp = s.content.props.iter().position(|p| !p.npc && p.after.is_none()).unwrap();
    let lamp_id = s.objs.iter().find(|(_, o)| matches!(o.kind, ObjKind::Piece { def: d } if d == lamp)).map(|(id, _)| *id).unwrap();
    assert!(!s.object(lamp_id).unwrap().usable);
}

#[test]
fn a_person_says_the_last_line_the_camp_has_earned() {
    let mut s = world();
    s.camp.counts.insert("build:cooking".into(), 1);
    s.camp.counts.insert("build:smithy".into(), 1);
    run(&mut s, 5.5);
    let (hale, _) = person(&s, "Quartermaster Hale");
    let first = s.talk_line(hale).unwrap().text.clone();
    s.camp.raids_won = 1;
    let after_raid = s.talk_line(hale).unwrap().text.clone();
    assert_ne!(first, after_raid, "the line changes once a raid is won");
    assert!(after_raid.contains("That raid"));
    // A line that waits on a kill.
    let (garrick, _) = person(&s, "Garrick Stone");
    assert!(s.talk_line(garrick).unwrap().text.starts_with("Bring me"));
    s.camp.counts.insert("kill:grunt".into(), 1);
    assert!(s.talk_line(garrick).unwrap().text.starts_with("You've been at the Warsong"));
}

#[test]
fn what_the_camp_has_killed_and_opened_reads_as_conditions() {
    let mut s = world();
    assert!(!s.camp_has("kill:hogger"));
    assert!(!s.camp_has("open:anything"));
    s.camp.counts.insert("kill:hogger".into(), 1);
    assert!(s.camp_has("kill:hogger"));
    assert!(!s.camp_has("kill:overseer"));
    s.camp.counts.insert("open:some_chest".into(), 2);
    assert!(s.camp_has("open:some_chest"));
    // A kill really counts when the creature falls.
    let mut s = world();
    let hero = s.join("ayla", "Ayla", None);
    s.move_player(hero, [300.0, 300.0, 0.0], 0.0, Moving::default());
    let def = s.content.mob("hogger").unwrap();
    let id = s.spawn_mob(def, [301.0, 300.0, 0.0], None, false);
    let max = s.mobs[&id].max_health;
    s.hurt_mob(id, hero, max);
    let _ = s.step(0.0);
    assert!(s.camp_has("kill:hogger"));
}

fn source(name: &str) -> String {
    std::fs::read_to_string(format!("{}/../../content/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

#[test]
fn a_condition_that_names_nothing_is_refused() {
    let text = source("survival.toml");
    let dungeon = source("deadmines.toml");
    assert!(Content::parse_with(&text, &dungeon).is_ok());
    let first = text.find("after = \"raids:1\"").expect("a prop that waits on a raid");
    for bad in ["bogus:x", "kill:nobody_at_all", "open:no_such_chest", "found:nowhere", "built:nothing", "raids:many"] {
        let mut t = text.clone();
        t.replace_range(first..first + "after = \"raids:1\"".len(), &format!("after = \"{bad}\""));
        let err = Content::parse_with(&t, &dungeon).expect_err(bad);
        assert!(format!("{err:#}").contains(bad), "{bad}: {err:#}");
    }
    // The same for a talk line.
    let at = text.find("[[prop.talk]]").unwrap() + "[[prop.talk]]\n".len();
    let mut t = text.clone();
    t.insert_str(at, "after = \"bogus:x\"\n");
    assert!(Content::parse_with(&t, &dungeon).is_err());
}

#[test]
fn every_name_the_story_points_at_is_real() {
    let c = Content::builtin();
    let station = |k: &str| c.station(k).is_some();
    let check = |cond: &str, what: &str| {
        let (kind, arg) = cond.split_once(':').unwrap_or((cond, ""));
        let ok = match kind {
            "start" => true,
            "raids" | "goal" => arg.parse::<u32>().is_ok(),
            "built" => station(arg),
            "found" => c.sites.iter().any(|s| s.key == arg),
            "kill" => c.mob(arg).is_some(),
            "open" => c.chests.iter().any(|x| x.key == arg),
            _ => false,
        };
        assert!(ok, "{what}: {cond}");
    };
    for p in &c.props {
        if let Some(a) = &p.after {
            check(a, &format!("prop {}", p.name));
        }
        for t in &p.talk {
            if let Some(a) = &t.after {
                check(a, &format!("talk of {}", p.name));
            }
        }
    }
    for x in &c.chests {
        if !x.after.is_empty() {
            assert!(c.mob(&x.after).is_some(), "chest {} after {}", x.key, x.after);
        }
        for k in &x.ambush {
            assert!(c.packs.iter().any(|p| &p.key == k), "chest {} ambush {k}", x.key);
        }
    }
    for p in &c.packs {
        assert!(c.mob(&p.mob).is_some(), "pack {} mob {}", p.key, p.mob);
        if !p.wake.is_empty() {
            assert!(c.chests.iter().any(|x| x.key == p.wake), "pack {} wakes on {}", p.key, p.wake);
        }
        if !p.camp.is_empty() {
            assert!(c.camps.iter().any(|x| x.key == p.camp), "pack {} camp {}", p.key, p.camp);
        }
    }
    for g in &c.goals {
        let ok = match g.kind.as_str() {
            "kill" => c.mob(&g.what).is_some(),
            "open" => g.what == "any" || c.chests.iter().any(|x| x.key == g.what),
            "explore" => g.what == "any" || c.sites.iter().any(|s| s.key == g.what),
            "build" => station(&g.what),
            "clear" => c.camps.iter().any(|x| x.key == g.what),
            "raid" => g.what.parse::<u32>().is_ok(),
            _ => true,
        };
        assert!(ok, "goal {}: {} {}", g.title, g.kind, g.what);
    }
}

// --- Stage 2: the Brotherhood's hand -----------------------------------------------------------

use crate::sim::mobs::{Foe, MobState};

/// The built-in world on flat ground with only the Brotherhood expansion's creatures standing.
fn bh_world() -> Survival {
    let mut c = Content::builtin();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 5, 1_000_000.0);
    let keep: Vec<Id> = s
        .mobs
        .iter()
        .filter(|(_, m)| s.content.mobs[m.def].key.starts_with("bh_") || m.pack.is_some_and(|k| s.content.packs[k].key.starts_with("bh_")))
        .map(|(id, _)| *id)
        .collect();
    s.mobs.retain(|id, _| keep.contains(id));
    s.respawns.clear();
    s
}

fn bh_hero(s: &mut Survival, at: Vec3) -> Id {
    let h = s.join("ayla", "Ayla", None);
    s.move_player(h, at, 0.0, Moving::default());
    let _ = s.step(0.0);
    if let Some(p) = s.players.get_mut(&h) {
        p.max_health = 1_000_000;
        p.health = 1_000_000;
    }
    h
}

fn chest(s: &Survival, key: &str) -> Option<Id> {
    let def = s.content.chests.iter().position(|c| c.key == key)?;
    s.objs.iter().find(|(_, o)| matches!(o.kind, ObjKind::Chest { def: d, .. } if d == def)).map(|(id, _)| *id)
}

/// Stand at a chest and open it, the work and all.
fn open_at(s: &mut Survival, h: Id, key: &str) -> Vec<Event> {
    let c = s.content.chests.iter().find(|c| c.key == key).unwrap().clone();
    s.move_player(h, [c.at[0] - 1.5, c.at[1], 0.0], 0.0, Moving::default());
    let o = chest(s, key).expect("the chest is there");
    s.interact(h, Some(o));
    run(s, c.secs + 0.3)
}

fn mob_of(s: &Survival, key: &str) -> Vec<Id> {
    s.mobs.iter().filter(|(_, m)| s.content.mobs[m.def].key == key && m.health > 0).map(|(id, _)| *id).collect()
}

fn fell(s: &mut Survival, by: Id, id: Id) {
    let max = s.mobs[&id].max_health;
    s.hurt_mob(id, by, max);
    let _ = s.step(0.0);
}

#[test]
fn the_messengers_drop_stays_shut_until_he_dies() {
    let mut s = bh_world();
    let h = bh_hero(&mut s, [0.0, 0.0, 0.0]);
    let silver = s.content.item("silver").unwrap();
    let ev = open_at(&mut s, h, "bh_drop");
    assert!(ev.iter().any(|e| matches!(e, Event::Tell { text, .. } if text.contains("chained shut"))), "it says why");
    assert!(chest(&s, "bh_drop").is_some(), "still shut");
    assert!(!s.camp_has("open:bh_drop"));
    let messenger = mob_of(&s, "bh_messenger");
    assert_eq!(messenger.len(), 1, "one messenger walks the road");
    fell(&mut s, h, messenger[0]);
    assert!(s.camp_has("kill:bh_messenger"));
    let ev = open_at(&mut s, h, "bh_drop");
    assert!(ev.iter().any(|e| matches!(e, Event::Tell { text, .. } if text.contains("map of Elwynn's roads"))), "the orders are read");
    assert_eq!(s.players[&h].bag.get(&silver), Some(&1));
    assert!(s.camp_has("open:bh_drop"));
}

#[test]
fn the_messenger_walks_the_road_from_goldshire_towards_the_landing() {
    let mut s = bh_world();
    let id = mob_of(&s, "bh_messenger")[0];
    let start = s.mobs[&id].pos;
    // A hero near keeps the road awake.
    let h = bh_hero(&mut s, [start[0] + 30.0, start[1], 0.0]);
    let _ = h;
    let far = s.content.packs.iter().find(|p| p.mob == "bh_messenger").unwrap().route.last().copied().unwrap();
    let mut best = dist2d(start, [far[0], far[1], 0.0]);
    let before = best;
    for _ in 0..(300.0 / 0.05) as usize {
        let _ = s.step(0.05);
        best = best.min(dist2d(s.mobs[&id].pos, [far[0], far[1], 0.0]));
    }
    assert!(best < before - 20.0, "he set out along the road: {before:.0} yd to the landing, {best:.0} the nearest");
}

#[test]
fn the_strongbox_wakes_the_paymaster_and_his_hirelings_on_the_opener_and_they_wait_again_after_it_fills() {
    let mut s = bh_world();
    let h = bh_hero(&mut s, [0.0, 0.0, 0.0]);
    let voss = mob_of(&s, "bh_paymaster")[0];
    assert!(s.mobs[&voss].hidden, "not seen at the ford until the strongbox is opened");
    let hirelings = mob_of(&s, "defias_thug");
    assert_eq!(hirelings.len(), 2);
    assert!(hirelings.iter().all(|id| s.mobs[id].hidden));
    open_at(&mut s, h, "bh_strongbox");
    for id in std::iter::once(voss).chain(hirelings.iter().copied()) {
        assert!(!s.mobs[&id].hidden, "stepped out");
        assert_eq!(s.mobs[&id].state.foe(), Some(Foe::Hero(h)), "on the opener");
    }
    // They fall; the chest and the ambush fill again in their time, with no hero near.
    for id in std::iter::once(voss).chain(hirelings.iter().copied()) {
        fell(&mut s, h, id);
    }
    assert!(chest(&s, "bh_strongbox").is_none(), "opened");
    s.move_player(h, [3000.0, 3000.0, 0.0], 0.0, Moving::default());
    run(&mut s, 1900.0);
    assert!(chest(&s, "bh_strongbox").is_some(), "full again");
    let voss = mob_of(&s, "bh_paymaster");
    assert_eq!(voss.len(), 1, "he's back");
    assert!(s.mobs[&voss[0]].hidden, "and waiting again");
    assert!(mob_of(&s, "defias_thug").iter().all(|id| s.mobs[id].hidden), "so are his hirelings");
}

/// The Paymaster's rule: while his hirelings fight he circles off, and once they're down he comes.
#[test]
fn the_paymaster_holds_back_while_his_hirelings_fight_and_comes_in_when_they_fall() {
    let mut s = bh_world();
    s.mobs.clear();
    let p = bh_hero(&mut s, [0.0, 0.0, 0.0]);
    let on = |s: &mut Survival, key: &str, at: Vec3| {
        let def = s.content.mob(key).unwrap();
        let id = s.spawn_mob(def, at, None, false);
        s.mobs.get_mut(&id).unwrap().state = MobState::Chase { foe: Foe::Hero(p) };
        id
    };
    let voss = on(&mut s, "bh_paymaster", [3.0, 0.0, 0.0]);
    let thug = on(&mut s, "defias_thug", [2.0, 1.0, 0.0]);
    let mut struck = false;
    for _ in 0..250 {
        for e in s.step(0.02) {
            struck |= matches!(e, Event::MobWindup { id, .. } if id == voss);
        }
    }
    assert!(!struck, "he held back while a hireling fought");
    assert!(dist2d(s.mobs[&voss].pos, [0.0, 0.0, 0.0]) > 5.0, "and stood off");
    // The hireling falls: now it's him.
    s.mobs.get_mut(&thug).unwrap().health = 0;
    let mut struck = false;
    for _ in 0..400 {
        for e in s.step(0.02) {
            struck |= matches!(e, Event::MobWindup { id, .. } if id == voss);
        }
    }
    assert!(struck, "alone, he comes in");
}

#[test]
fn at_35_percent_the_paymaster_runs_to_his_boat_and_fights_again_once_a_hero_is_near() {
    let mut s = bh_world();
    // (The test land is flat; the run's points keep their x and y, at its height.)
    let def = s.content.mob("bh_paymaster").unwrap();
    for p in &mut s.content.mobs[def].phases {
        for r in &mut p.run_to {
            r[2] = 0.0;
        }
    }
    let phase = s.content.mobs[def].phases[0].clone();
    assert_eq!(phase.at, 0.35);
    assert_eq!(phase.wait_for, 10.0);
    let last = *phase.run_to.last().unwrap();
    let h = bh_hero(&mut s, [last[0] - 45.0, last[1] - 45.0, 0.0]);
    open_at(&mut s, h, "bh_strongbox");
    let voss = mob_of(&s, "bh_paymaster")[0];
    // The hero stands well off from the boat meanwhile.
    s.move_player(h, [last[0] - 45.0, last[1] - 45.0, 0.0], 0.0, Moving::default());
    let max = s.mobs[&voss].max_health;
    s.mobs.get_mut(&voss).unwrap().health = (max as f32 * 0.34) as u32;
    let mut said = Vec::new();
    for _ in 0..(14.0 / 0.05) as usize {
        for e in s.step(0.05) {
            if let Event::Yell { text, .. } = &e {
                said.push(text.clone());
            }
        }
        let _ = s.players.get_mut(&h).map(|p| p.health = 1_000_000);
    }
    assert!(said.iter().any(|t| t.contains("paid till sundown")), "his line as he turns: {said:?}");
    let at_boat = dist2d(s.mobs[&voss].pos, last);
    assert!(at_boat < 3.0, "he ran to his boat and waits: {at_boat:.1} yd off");
    assert!(!said.iter().any(|t| t.contains("boat leaves without me")), "and hasn't turned on anyone yet");
    assert_ne!(s.mobs[&voss].home, last, "not his post yet");
    // A hero within 10 yd: he fights again where he stands.
    s.move_player(h, [last[0] - 6.0, last[1], 0.0], 0.0, Moving::default());
    let mut said = Vec::new();
    for _ in 0..(3.0 / 0.05) as usize {
        for e in s.step(0.05) {
            if let Event::Yell { text, .. } = &e {
                said.push(text.clone());
            }
        }
    }
    assert!(said.iter().any(|t| t.contains("boat leaves without me")), "{said:?}");
    assert_eq!(s.mobs[&voss].home, last, "the boat is his post now");
    assert_eq!(s.mobs[&voss].state.foe(), Some(Foe::Hero(h)));
}

#[test]
fn the_brotherhoods_hand_follows_in_the_journal_in_order_ahead_of_every_corner() {
    let mut s = bh_world();
    let at = |s: &Survival, t: &str| s.content.goals.iter().position(|g| g.title == t).unwrap_or_else(|| panic!("no goal {t}"));
    let first = at(&s, "The messenger");
    let titles = ["The messenger", "The drop at the dock", "The Smugglers' Ford", "The Paymaster", "Every corner"];
    for (k, t) in titles.iter().enumerate() {
        assert_eq!(at(&s, t), first + k, "{t} in its place");
    }
    // Every corner asks for every place there is.
    let corner = &s.content.goals[at(&s, "Every corner")];
    assert_eq!(corner.count as usize, s.content.sites.len(), "Every corner counts the sites");
    // Each is done in turn, by what the camp does.
    s.camp.goal = first;
    s.step_goals();
    assert_eq!(s.camp.goal, first, "nothing yet");
    s.camp.counts.insert("open:bh_drop".into(), 1);
    s.camp.counts.insert("kill:bh_paymaster".into(), 1);
    s.step_goals();
    assert_eq!(s.camp.goal, first, "the later ones don't count before the first");
    s.camp.counts.insert("kill:bh_messenger".into(), 1);
    s.step_goals();
    assert_eq!(s.camp.goal, first + 2, "the messenger and the drop are done, the ford not");
    s.camp.counts.insert("explore:smugglers_ford".into(), 1);
    s.step_goals();
    assert_eq!(s.content.goals[s.camp.goal].title, "Every corner", "then the paymaster, and on");
}

/// An old save's goal still points at the goal it did before, or is carried over the new ones.
#[test]
fn an_older_save_keeps_its_goal_though_the_expansions_goals_went_in_ahead_of_every_corner() {
    let s = Survival::new(Content::builtin(), Box::new(FlatLand::default()), 1, 1_000_000.0);
    let new: BTreeSet<&str> = ["The messenger", "The drop at the dock", "The Smugglers' Ford", "The Paymaster"].into_iter().collect();
    let goals = &s.content.goals;
    // The journal as it was before: the same goals without the new ones.
    let before: Vec<&str> = goals.iter().map(|g| g.title.as_str()).filter(|t| !new.contains(t)).collect();
    assert_eq!(before[crate::save::ELWYNN_GOALS_AT], "Every corner", "it was the goal at the place the new ones went in");
    for (old_index, title) in before.iter().enumerate() {
        let mut save = s.save(&BTreeMap::new());
        save.goal = old_index;
        save.goal_title = Some((*title).to_string());
        let back = Survival::load(Content::builtin(), Box::new(FlatLand::default()), 1, 1_000_000.0, &save);
        assert_eq!(back.content.goals[back.camp.goal].title, *title, "goal {old_index} still the same goal");
    }
    // A save from before titles were kept, done with the journal as it stood then: still done.
    let mut save = s.save(&BTreeMap::new());
    save.goal_title = None;
    save.goal = before.len();
    let back = Survival::load(Content::builtin(), Box::new(FlatLand::default()), 1, 1_000_000.0, &save);
    assert_eq!(back.camp.goal, goals.len(), "done stays done, no goal done again");
    // One done up to where the dungeons' steps were added: on at the first of them, as before.
    let ladder_before = before.iter().position(|t| goals.iter().find(|g| g.title == *t).is_some_and(|g| g.kind == "cleared")).unwrap();
    save.goal = ladder_before;
    let back = Survival::load(Content::builtin(), Box::new(FlatLand::default()), 1, 1_000_000.0, &save);
    assert_eq!(back.content.goals[back.camp.goal].kind, "cleared");
    assert_eq!(back.content.goals[back.camp.goal].title, before[ladder_before]);
}
