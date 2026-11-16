//! The camp's people talk and the story's conditions: a person with lines is usable and says the
//! last line the camp has earned, `kill:` and `open:` read what the camp has done, a condition
//! that names nothing is refused, and every name the content points at is real.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::*;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn world() -> Survival {
    let mut c = Content::builtin();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 1, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

pub(crate) fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / 0.05).round() as usize {
        out.extend(s.step(0.05));
    }
    out
}

/// The camp's person called `name`: their prop, and the object that stands for it.
pub(crate) fn person(s: &Survival, name: &str) -> (usize, Id) {
    let def = s.content.props.iter().position(|p| p.npc && p.name == name).unwrap_or_else(|| panic!("no {name}"));
    let id = s.objs.iter().find(|(_, o)| matches!(o.kind, ObjKind::Piece { def: d } if d == def)).map(|(id, _)| *id).unwrap_or_else(|| panic!("{name} isn't standing"));
    (def, id)
}

/// A hero beside `id`, with nothing else in reach to use.
pub(crate) fn beside(s: &mut Survival, id: Id) -> Id {
    let at = s.objs[&id].pos;
    s.objs.retain(|k, o| *k == id || dist2d(o.pos, at) > 6.0);
    let hero = s.join("ayla", "Ayla", None);
    s.move_player(hero, [at[0] + 1.5, at[1], at[2]], 0.0, Moving::default());
    let _ = s.step(0.0);
    hero
}

/// What the camp's people said: who (their prop's object), their words and the answers offered.
pub(crate) fn talks(ev: &[Event]) -> Vec<(Id, String, Vec<(String, String)>)> {
    ev.iter().filter_map(|e| match e {
        Event::Talk { speaker, text, links, .. } => Some((*speaker, text.clone(), links.clone())),
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
    let ev = s.step(0.0);
    let said = talks(&ev);
    assert_eq!(said.len(), 1, "one line said");
    assert_eq!(said[0].0, id, "by Hale, where he stands");
    assert!(said[0].1.contains("Keep the storage stocked"));
    assert!(!ev.iter().any(|e| matches!(e, Event::Letter { .. })), "no page");
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

pub(crate) fn source(name: &str) -> String {
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
            "raids" => arg.parse::<u32>().is_ok(),
            "built" => station(arg),
            "found" => c.sites.iter().any(|s| s.key == arg),
            "kill" => c.mob(arg).is_some(),
            "open" => c.chests.iter().any(|x| x.key == arg),
            "chose" => c.choices().iter().any(|(g, o)| arg == g || arg == format!("{g}:{o}")),
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
            // A page of the book a person points to is a dungeon there is.
            for l in &t.links {
                if let Some(key) = l.split('~').next().and_then(|c| c.strip_prefix(".es lore ")) {
                    assert!(c.dungeons().any(|d| d.key == key.trim()), "talk of {}: no dungeon {key}", p.name);
                }
            }
        }
    }
    for x in &c.chests {
        if !x.after.is_empty() {
            if x.after.contains(':') {
                check(&x.after, &format!("chest {}", x.key));
            } else {
                assert!(c.mob(&x.after).is_some(), "chest {} after {}", x.key, x.after);
            }
        }
        for k in &x.ambush {
            assert!(c.packs.iter().any(|p| &p.key == k), "chest {} ambush {k}", x.key);
        }
    }
    for p in &c.packs {
        assert!(c.mob(&p.mob).is_some(), "pack {} mob {}", p.key, p.mob);
        if let Some(a) = &p.after {
            check(a, &format!("pack {} of {}", p.key, p.mob));
        }
        if !p.wake.is_empty() {
            assert!(c.chests.iter().any(|x| x.key == p.wake), "pack {} wakes on {}", p.key, p.wake);
        }
        if !p.camp.is_empty() {
            assert!(c.camps.iter().any(|x| x.key == p.camp), "pack {} camp {}", p.key, p.camp);
        }
    }
}

// --- Stage 2: the Brotherhood's hand -----------------------------------------------------------

use crate::sim::mobs::{Foe, MobState};

/// The built-in world on flat ground with only the Brotherhood expansion's creatures standing.
pub(crate) fn bh_world() -> Survival {
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

pub(crate) fn bh_hero(s: &mut Survival, at: Vec3) -> Id {
    let h = s.join("ayla", "Ayla", None);
    s.move_player(h, at, 0.0, Moving::default());
    let _ = s.step(0.0);
    if let Some(p) = s.players.get_mut(&h) {
        p.max_health = 1_000_000;
        p.health = 1_000_000;
    }
    h
}

pub(crate) fn chest(s: &Survival, key: &str) -> Option<Id> {
    let def = s.content.chests.iter().position(|c| c.key == key)?;
    s.objs.iter().find(|(_, o)| matches!(o.kind, ObjKind::Chest { def: d, .. } if d == def)).map(|(id, _)| *id)
}

/// Stand at a chest and open it, the work and all.
pub(crate) fn open_at(s: &mut Survival, h: Id, key: &str) -> Vec<Event> {
    let c = s.content.chests.iter().find(|c| c.key == key).unwrap().clone();
    s.move_player(h, [c.at[0] - 1.5, c.at[1], 0.0], 0.0, Moving::default());
    let o = chest(s, key).expect("the chest is there");
    s.interact(h, Some(o));
    run(s, c.secs + 0.3)
}

pub(crate) fn mob_of(s: &Survival, key: &str) -> Vec<Id> {
    s.mobs.iter().filter(|(_, m)| s.content.mobs[m.def].key == key && m.health > 0).map(|(id, _)| *id).collect()
}

pub(crate) fn fell(s: &mut Survival, by: Id, id: Id) {
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

// --- Stage 3: Nightbane Crossing ---------------------------------------------------------------

/// The built-in world on flat ground, the hero far from everything.
pub(crate) fn real_world() -> (Survival, Id) {
    let mut c = Content::builtin();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 5, 1_000_000.0);
    s.respawns.clear();
    let h = bh_hero(&mut s, [3000.0, 3000.0, 0.0]);
    (s, h)
}

pub(crate) fn pack_of(s: &Survival, mob: &str) -> usize {
    s.content.packs.iter().position(|p| p.mob == mob).unwrap_or_else(|| panic!("no pack of {mob}"))
}

#[test]
fn the_nightbane_crossing_is_taken_when_every_worgen_there_is_dead() {
    let (mut s, h) = real_world();
    let packs: Vec<usize> = s.content.packs.iter().enumerate().filter(|(_, p)| p.camp == "nightbane").map(|(k, _)| k).collect();
    assert_eq!(packs.len(), 3, "prowlers, weavers and worgs");
    let all: Vec<Id> = s.mobs.iter().filter(|(_, m)| m.pack.is_some_and(|k| packs.contains(&k))).map(|(id, _)| *id).collect();
    assert_eq!(all.len(), 6);
    let mut banners = Vec::new();
    for (i, id) in all.iter().enumerate() {
        let max = s.mobs[id].max_health;
        s.hurt_mob(*id, h, max);
        for e in s.step(0.0) {
            if let Event::Banner { title, .. } = e {
                banners.push((i, title));
            }
        }
        assert_eq!(s.camp.counts.get("clear:nightbane").copied().unwrap_or(0), u32::from(i + 1 == all.len()), "taken with the last of them only");
    }
    assert!(banners.iter().any(|(i, t)| *i == all.len() - 1 && t.contains("Nightbane Crossing Taken")), "{banners:?}");
}

#[test]
fn fenros_rises_after_4_seconds_with_his_yell_and_falls_for_good_with_the_news_once() {
    let (mut s, h) = real_world();
    let first = mob_of(&s, "fenros");
    assert_eq!(first.len(), 1);
    let fenros = s.content.mob("fenros").unwrap();
    assert_eq!(s.content.mobs[fenros].then, "fenros_risen");
    assert_eq!(s.content.mobs[fenros].then_after, 4.0);
    // He falls: not told as news yet, and the second form lies unseen.
    let max = s.mobs[&first[0]].max_health;
    s.hurt_mob(first[0], h, max);
    let mut said = Vec::new();
    let mut banners = Vec::new();
    let take = |ev: Vec<Event>, said: &mut Vec<String>, banners: &mut Vec<String>| {
        for e in ev {
            match e {
                Event::Yell { text, .. } => said.push(text),
                Event::Banner { title, .. } => banners.push(title),
                _ => {}
            }
        }
    };
    let ev = s.step(0.0);
    take(ev, &mut said, &mut banners);
    assert!(mob_of(&s, "fenros_risen").iter().all(|id| s.mobs[id].hidden), "not there to be seen yet");
    let ev = run(&mut s, 3.5);
    take(ev, &mut said, &mut banners);
    assert!(!said.iter().any(|t| t.contains("Not... yet")), "not before 4 sec: {said:?}");
    assert!(mob_of(&s, "fenros_risen").iter().all(|id| s.mobs[id].hidden));
    let ev = run(&mut s, 1.0);
    take(ev, &mut said, &mut banners);
    assert!(said.iter().any(|t| t.contains("Not... yet")), "his yell as he rises: {said:?}");
    let risen = mob_of(&s, "fenros_risen");
    assert_eq!(risen.len(), 1);
    assert!(!s.mobs[&risen[0]].hidden, "risen");
    assert!(!banners.iter().any(|t| t.contains("Falls")), "no fall announced for the first form: {banners:?}");
    // No fall for the first form comes back on its own; the risen form's fall is the news.
    assert!(s.respawns.is_empty(), "nothing is put back while the second form is up");
    let max = s.mobs[&risen[0]].max_health;
    s.hurt_mob(risen[0], h, max);
    let ev = s.step(0.0);
    take(ev, &mut said, &mut banners);
    assert!(banners.iter().any(|t| t == "Fenros, Risen Falls"), "the news: {banners:?}");
    assert!(!banners.iter().any(|t| t == "Fenros Falls"), "never the first form's: {banners:?}");
    assert!(s.camp_has("kill:fenros_risen"));
    // What comes back is the first form, at one of his spots.
    assert_eq!(s.respawns.len(), 1);
    assert_eq!(s.respawns[0].0, fenros, "the first form, not the risen one");
}

/// Fenros's rule: parry the first form, and the second can't be parried (a guard only blocks).
#[test]
fn a_parry_works_on_fenros_and_fails_on_fenros_risen() {
    for (key, want, reels) in [("fenros", Outcome::Parried, true), ("fenros_risen", Outcome::Blocked, false)] {
        let (mut s, _) = real_world();
        s.mobs.clear();
        let p = bh_hero(&mut s, [0.0, 0.0, 0.0]);
        let def = s.content.mob(key).unwrap();
        let w = s.spawn_mob(def, [2.0, 0.0, 0.0], None, false);
        s.input(p, Input::Guard);
        run(&mut s, 0.1);
        assert_eq!(s.strike_player(w, p, 16, [2.0, 0.0, 0.0], true, false, true, Blow::Light), want, "{key}");
        assert_eq!(s.mobs[&w].stagger_until > s.clock + 1.0, reels, "{key} reels only when parried");
    }
}

#[test]
fn fenros_leaps_from_range_and_strikes_in_quick_strings_and_the_risen_form_is_slower_and_heavier() {
    let c = Content::builtin();
    let (a, b) = (&c.mobs[c.mob("fenros").unwrap()], &c.mobs[c.mob("fenros_risen").unwrap()]);
    let strings = a.patterns.iter().find(|p| p.name == "claw string").unwrap();
    assert!(strings.steps.len() >= 4 && strings.steps[1..].iter().all(|st| st.windup.is_some_and(|w| w < 0.3)), "quick claws after the first");
    assert!(a.patterns.iter().any(|p| p.name == "leap" && p.min_range >= 5.0), "a leap from range");
    assert!(!a.unparryable && b.unparryable);
    assert!(b.attack.cooldown > a.attack.cooldown && b.run_speed < a.run_speed, "slower");
    let heaviest = |m: &crate::content::MobDef| m.patterns.iter().flat_map(|p| p.steps.iter()).filter_map(|st| st.damage).max().unwrap();
    assert!(heaviest(b) > heaviest(a), "heavier");
}

#[test]
fn watcher_dodds_comes_with_the_crossing_found_and_names_the_keep_once_fenros_has_fallen() {
    let mut s = Survival::new({ let mut c = Content::builtin(); c.pvp = Default::default(); c }, Box::new(FlatLand::default()), 5, 1_000_000.0);
    s.mobs.clear();
    let def = s.content.props.iter().position(|p| p.npc && p.name == "Watcher Dodds").expect("Watcher Dodds");
    let standing = |s: &Survival| s.objs.values().any(|o| matches!(o.kind, ObjKind::Piece { def: d } if d == def));
    assert!(!standing(&s), "not before the crossing is found");
    s.camp.found.insert("nightbane_crossing".into());
    run(&mut s, 5.5);
    assert!(standing(&s), "there once it's found");
    assert!(s.talk_line(def).unwrap().text.contains("Fenros leads them"));
    s.camp.counts.insert("kill:fenros_risen".into(), 1);
    let line = s.talk_line(def).unwrap();
    assert!(line.text.contains("from a keep your book should name"));
    assert_eq!(line.links, vec![".es lore shadowfang~Shadowfang Keep".to_string()]);
    let key = line.links[0].split('~').next().unwrap().strip_prefix(".es lore ").unwrap().to_string();
    assert!(s.content.dungeons().any(|d| d.key == key && d.name == "Shadowfang Keep"), "the link names the Shadowfang Keep's own key");
}

#[test]
fn fenros_wanders_between_the_crossing_and_the_woods_by_eastvale() {
    let c = Content::builtin();
    let k = c.packs.iter().position(|p| p.mob == "fenros").unwrap();
    let pack = &c.packs[k];
    assert!(pack.rare);
    let site = c.sites.iter().find(|x| x.key == "nightbane_crossing").unwrap();
    assert!(pack.spots.iter().any(|p| dist2d([p[0], p[1], 0.0], [site.at[0], site.at[1], 0.0]) < 60.0), "one of his spots is at the crossing");
    assert!(pack.spots.iter().any(|p| dist2d([p[0], p[1], 0.0], [-9660.0, -1200.0, 0.0]) < 30.0), "and one in the woods by Eastvale");
    let _ = pack_of;
}

// --- Stage 4: Stitches on the Goldshire road ---------------------------------------------------

fn stitches_bare() -> (Survival, Id, Id) {
    let (mut s, _) = real_world();
    s.mobs.clear();
    let p = bh_hero(&mut s, [0.0, 0.0, 0.0]);
    let def = s.content.mob("stitches").unwrap();
    let id = s.spawn_mob(def, [3.0, 0.0, 0.0], None, false);
    s.mobs.get_mut(&id).unwrap().state = MobState::Chase { foe: Foe::Hero(p) };
    (s, p, id)
}

pub(crate) fn flames_near(s: &Survival, at: Vec3, within: f32) -> usize {
    s.objs.values().filter(|o| matches!(o.kind, ObjKind::Fire { .. }) && dist2d(o.pos, at) <= within).count()
}

#[test]
fn stitches_walks_the_goldshire_road_with_a_hero_near_and_keeps_clear_of_the_camp() {
    let c = Content::builtin();
    let k = c.packs.iter().position(|p| p.mob == "stitches").unwrap();
    let pack = &c.packs[k];
    assert!(pack.rare && !pack.route.is_empty());
    let camp = [c.world.build_center[0], c.world.build_center[1], 0.0];
    for p in std::iter::once(&pack.at).chain(pack.route.iter()) {
        let far = dist2d([p[0], p[1], 0.0], camp);
        assert!(far > 200.0, "{p:?} is {far:.0} yd from the camp (the plan asks for 110 at least, this road keeps 200)");
    }
    let (mut s, _) = real_world();
    let id = mob_of(&s, "stitches")[0];
    let start = s.mobs[&id].pos;
    // A hero within sight keeps the road awake, well off his aggro.
    let h = bh_hero(&mut s, [start[0] + 50.0, start[1], 0.0]);
    let mut went = 0.0f32;
    for _ in 0..(240.0 / 0.05) as usize {
        let _ = s.step(0.05);
        let _ = s.players.get_mut(&h).map(|p| p.health = 1_000_000);
        went = went.max(dist2d(s.mobs[&id].pos, start));
    }
    assert!(went > 40.0, "he walks the road: {went:.0} yd from where he stood");
}

#[test]
fn the_fall_of_stitches_is_announced_as_a_rares() {
    let (mut s, h) = real_world();
    let id = mob_of(&s, "stitches")[0];
    let max = s.mobs[&id].max_health;
    s.hurt_mob(id, h, max);
    let ev = s.step(0.0);
    assert!(ev.iter().any(|e| matches!(e, Event::Banner { title, .. } if title == "Stitches Falls")), "{ev:?}");
    assert!(ev.iter().any(|e| matches!(e, Event::Announce { text } if text.starts_with("Stitches fell"))));
    assert!(s.camp_has("kill:stitches"));
}

/// Stitches's rule: too big to parry, and from half his health every heavy blow leaves bile.
#[test]
fn a_parry_fails_on_stitches_and_his_blows_leave_bile_only_from_half_his_health() {
    // A guard only blocks him, and he doesn't reel.
    let (mut s, p, w) = stitches_bare();
    s.input(p, Input::Guard);
    run(&mut s, 0.1);
    assert_eq!(s.strike_player(w, p, 16, [2.0, 0.0, 0.0], true, false, true, Blow::Light), Outcome::Blocked);
    assert!(s.mobs[&w].stagger_until <= s.clock, "no reel");
    // Above half: his blows leave nothing.
    let (mut s, p, w) = stitches_bare();
    let mut blows = 0;
    for _ in 0..(14.0 / 0.05) as usize {
        for e in s.step(0.05) {
            blows += usize::from(matches!(e, Event::Swing { attacker, .. } if attacker == w));
        }
        s.players.get_mut(&p).unwrap().health = 1_000_000;
    }
    assert!(blows > 0, "he struck");
    assert_eq!(flames_near(&s, [0.0, 0.0, 0.0], 30.0), 0, "no bile before half his health");
    // Below half: each heavy blow that lands leaves bile near the hero, and it bites.
    let max = s.mobs[&w].max_health;
    s.mobs.get_mut(&w).unwrap().health = (max as f32 * 0.45) as u32;
    let mut said = Vec::new();
    for _ in 0..(14.0 / 0.05) as usize {
        for e in s.step(0.05) {
            if let Event::Yell { text, .. } = e {
                said.push(text);
            }
        }
        s.players.get_mut(&p).unwrap().health = 1_000_000;
    }
    assert!(said.iter().any(|t| t.contains("tears at his own seams")), "{said:?}");
    let at = s.players[&p].pos;
    assert!(flames_near(&s, at, 8.0) > 0, "bile where his blows land, round the hero");
    // Standing in it costs health; stepping out of it doesn't.
    s.mobs.get_mut(&w).unwrap().state = MobState::Idle { wander_at: 1.0e9 };
    s.mobs.get_mut(&w).unwrap().ready_at = 1.0e9;
    s.mobs.get_mut(&w).unwrap().pos = [60.0, 0.0, 0.0];
    s.boss_blow_lands(w, at, 0.0, 3.0, 360.0, true);
    s.players.get_mut(&p).unwrap().health = 100_000;
    run(&mut s, 3.0);
    let standing = 100_000 - s.players[&p].health;
    assert!(standing > 0, "standing in bile costs health");
    s.move_player(p, [at[0] - 20.0, at[1], at[2]], 0.0, Moving::default());
    s.players.get_mut(&p).unwrap().health = 100_000;
    run(&mut s, 3.0);
    assert_eq!(100_000 - s.players[&p].health, 0, "walking away from it doesn't");
}

#[test]
fn stitches_is_a_blockable_chop_a_swing_that_nothing_blocks_a_lurch_and_a_throw() {
    let c = Content::builtin();
    let m = &c.mobs[c.mob("stitches").unwrap()];
    assert!(m.boss && m.unparryable && m.posture >= 300 && m.respawn >= 2400.0);
    let pat = |n: &str| m.patterns.iter().find(|p| p.name == n).unwrap_or_else(|| panic!("no {n}"));
    assert_ne!(pat("cleaver chop").steps[0].blockable, Some(false), "the chop is blocked");
    let swing = &pat("gut swing").steps[0];
    assert_eq!((swing.blockable, swing.arc), (Some(false), Some(360.0)), "all round, nothing blocks it");
    assert!(pat("gut swing").max_range > 0.0 && pat("gut swing").max_range < 5.0, "when crowded");
    assert!(pat("lurch").steps[0].lunge >= 5.0 && pat("lurch").min_range >= 5.0, "a charge from range");
    let throw = &pat("thrown cleaver").steps[0];
    assert!(throw.ranged == Some(true) && throw.spell.is_some() && pat("thrown cleaver").min_range >= 5.0, "a throw");
    assert!(m.attack.heavy, "every blow is heavy, so every blow leaves bile once he burns");
    let phase = &m.phases[0];
    assert_eq!(phase.at, 0.5);
    assert!(phase.burn.is_some());
}

// --- Stage 5: the fifth raid, "The Brotherhood's Price" ----------------------------------------

fn counts(p: &crate::content::RaidPlan) -> BTreeMap<String, usize> {
    p.raiders.iter().map(|(k, n)| (k.clone(), *n)).collect()
}

#[test]
fn the_fifth_raid_is_the_brotherhoods_price_and_the_sixth_comes_back_harder() {
    let c = Content::builtin();
    assert_eq!(c.raid.count(), 5, "five raids in all");
    assert_eq!(c.raid.plan(3).title, "Blackrock Assault", "the fourth is as it was");
    let p = c.raid.plan(4);
    assert_eq!(p.title, "The Brotherhood's Price");
    assert_eq!(p.needs, vec!["barracks".to_string(), "infirmary".to_string()]);
    assert_eq!(p.from, [-9690.0, 420.0]);
    let want: BTreeMap<String, usize> = [("defias_thug", 5), ("defias_footpad", 3), ("defias_archer", 2), ("defias_conjurer", 2), ("brannick", 1)].into_iter().map(|(k, n)| (k.to_string(), n)).collect();
    assert_eq!(counts(&p), want);
    for k in p.needs.iter() {
        assert!(c.station(k).is_some(), "{k}");
    }
    // Beaten once, it comes back with a raider more of each kind and its title says so.
    let again = c.raid.plan(5);
    assert_eq!(again.title, "The Brotherhood's Price (2)");
    let more: BTreeMap<String, usize> = want.iter().map(|(k, n)| (k.clone(), n + 1)).collect();
    assert_eq!(counts(&again), more);
}

fn brannick_bare() -> (Survival, Id, Id) {
    let (mut s, _) = real_world();
    s.mobs.clear();
    let p = bh_hero(&mut s, [0.0, 0.0, 0.0]);
    let def = s.content.mob("brannick").unwrap();
    let id = s.spawn_mob(def, [4.5, 0.0, 0.0], None, false);
    s.mobs.get_mut(&id).unwrap().state = MobState::Chase { foe: Foe::Hero(p) };
    (s, p, id)
}

/// Brannick's rule: he turns slowly, so a hero who works round him in one wind-up ends up at his
/// back, and a blow there lands harder.
#[test]
fn a_hero_circling_brannick_ends_up_at_his_back_and_the_backstab_applies() {
    let (mut s, p, id) = brannick_bare();
    let turn = s.content.mobs[s.mobs[&id].def].attack.turn_rate;
    assert!(turn <= 90.0, "slow to turn: {turn}");
    // Wait for a wind-up, then run a circle round him just outside his stomp, at a sprint.
    let mut behind = false;
    let mut windup = false;
    let sprint = s.content.player.sprint_speed;
    let r = 4.0f32;
    let mut a = 0.0f32;
    for _ in 0..(40.0 / 0.05) as usize {
        let _ = s.step(0.05);
        s.players.get_mut(&p).unwrap().health = 1_000_000;
        let m = &s.mobs[&id];
        if matches!(m.state, MobState::Windup { .. }) {
            windup = true;
        }
        if windup {
            // Round him at a sprint, from where the hero stood.
            a += sprint / r * 0.05;
            let at = [m.pos[0] + a.cos() * r, m.pos[1] + a.sin() * r, 0.0];
            s.move_player(p, at, a + std::f32::consts::PI, Moving::default());
            if s.from_behind(id, at) {
                behind = true;
                break;
            }
        }
    }
    assert!(windup, "he wound up");
    assert!(behind, "the hero came round to his back while he turned");
    // From behind a blow does the backstab's multiple of what the same blow does from the front.
    let hit = |behind: bool| {
        let (mut s, p, id) = brannick_bare();
        // (He notices no one: the rule under test is the blow's, not his turning.)
        let d = s.mobs[&id].def;
        s.content.mobs[d].aggro = 0.0;
        let m = s.mobs.get_mut(&id).unwrap();
        m.state = MobState::Idle { wander_at: 1.0e9 };
        m.ready_at = 1.0e9;
        m.pos = [0.0, 0.0, 0.0];
        m.facing = 0.0;
        let at = if behind { [-1.8, 0.0, 0.0] } else { [1.8, 0.0, 0.0] };
        s.move_player(p, at, if behind { 0.0 } else { std::f32::consts::PI }, Moving::default());
        let _ = s.step(0.0);
        s.input(p, Input::Attack);
        let ev = run(&mut s, 0.8);
        ev.iter().find_map(|e| match e {
            Event::Swing { victim: Some(v), damage, anim: false, .. } if *v == id && *damage > 0 => Some(*damage),
            _ => None,
        })
    };
    let (front, back) = (hit(false).expect("a blow from the front"), hit(true).expect("a blow from behind"));
    assert!(back as f32 >= front as f32 * Content::builtin().combat.backstab * 0.9, "from behind {back}, from the front {front}");
}

#[test]
fn brannick_stomps_a_hero_hugging_him_and_swings_the_maul_at_arms_length() {
    let c = Content::builtin();
    let def = c.mob("brannick").unwrap();
    let name = |k: usize| c.mobs[def].patterns[k].name.clone();
    let (mut s, _, id) = brannick_bare();
    let size = s.content.mobs[def].body();
    for _ in 0..40 {
        let k = s.pick_pattern(id, 2.0, size).expect("a blow for a hero hugging him");
        assert_eq!(name(k), "stomp", "up close it is the stomp");
    }
    let mut seen = BTreeSet::new();
    for _ in 0..80 {
        let k = s.pick_pattern(id, 4.2, size).expect("a blow at arm's length");
        seen.insert(name(k));
    }
    assert!(!seen.contains("stomp") && seen.contains("maul"), "at arm's length the maul: {seen:?}");
    let stomp = &c.mobs[def].patterns.iter().find(|p| p.name == "stomp").unwrap().steps[0];
    assert_eq!((stomp.blockable, stomp.arc), (Some(false), Some(360.0)));
}

#[test]
fn brannick_calls_for_pitch_at_half_health_and_his_blows_burn_from_then() {
    let (mut s, p, id) = brannick_bare();
    let mut said = Vec::new();
    let take = |ev: Vec<Event>, said: &mut Vec<String>| {
        for e in ev {
            if let Event::Yell { text, .. } = e {
                said.push(text);
            }
        }
    };
    // He turns on the hero: his line.
    s.engage(id, Foe::Hero(p));
    let ev = s.step(0.0);
    take(ev, &mut said);
    assert!(said.iter().any(|t| t.contains("I built this wall")), "{said:?}");
    let max = s.mobs[&id].max_health;
    s.mobs.get_mut(&id).unwrap().health = (max as f32 * 0.45) as u32;
    for _ in 0..(14.0 / 0.05) as usize {
        let ev = s.step(0.05);
        take(ev, &mut said);
        s.players.get_mut(&p).unwrap().health = 1_000_000;
    }
    assert!(said.iter().any(|t| t.contains("Pitch on the stones")), "{said:?}");
    assert!(flames_near(&s, s.players[&p].pos, 10.0) > 0, "pitch burns where his heavy blows land");
}

#[test]
fn the_stonemason_comes_with_brannicks_fall_and_not_with_the_fifth_raid() {
    let mut s = Survival::new({ let mut c = Content::builtin(); c.pvp = Default::default(); c }, Box::new(FlatLand::default()), 5, 1_000_000.0);
    s.mobs.clear();
    let def = s.content.props.iter().position(|p| p.npc && p.name == "Stormwind Stonemason").expect("the stonemason");
    assert_eq!(s.content.props[def].after.as_deref(), Some("kill:brannick"));
    let standing = |s: &Survival| s.objs.values().any(|o| matches!(o.kind, ObjKind::Piece { def: d } if d == def));
    assert!(!standing(&s));
    s.camp.raids_won = 5;
    run(&mut s, 5.5);
    assert!(!standing(&s), "the raid's end isn't what brings him");
    s.camp.counts.insert("kill:brannick".into(), 1);
    let ev = run(&mut s, 5.5);
    assert!(standing(&s), "Brannick fallen: he comes");
    assert!(ev.iter().any(|e| matches!(e, Event::Announce { text } if text.contains("stonemason from Stormwind"))));
    assert!(s.talk_line(def).unwrap().text.contains("Five raids"), "and talks once five are won");
}
