//! The camp's people talk and the story's conditions: a person with lines is usable and says the
//! last line the camp has earned, `kill:` and `open:` read what the camp has done, a condition
//! that names nothing is refused, and every name the content points at is real.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::*;

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
