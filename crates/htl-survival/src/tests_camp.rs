//! The camp at Westbrook: its things in their places, what comes as it grows, stations carried
//! over from where the camp stood before.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::*;

fn world() -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 1, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

fn pieces(s: &Survival) -> Vec<usize> {
    s.objs.values().filter_map(|o| match o.kind { ObjKind::Piece { def } => Some(def), _ => None }).collect()
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / 0.05).round() as usize {
        out.extend(s.step(0.05));
    }
    out
}

#[test]
fn the_camp_starts_with_what_it_has_and_grows_as_it_goes() {
    let mut s = world();
    let c = s.content.clone();
    let standing = pieces(&s);
    for (k, p) in c.props.iter().enumerate() {
        assert_eq!(standing.contains(&k), p.after.is_none(), "{k} ({:?}) at the start", p.after);
    }
    // A cooking fire built: the quartermaster and the wagon come, and the camp hears of it.
    s.camp.counts.insert("build:cooking".into(), 1);
    let ev = run(&mut s, 5.5);
    let now = pieces(&s);
    let cooking: Vec<usize> = (0..c.props.len()).filter(|k| c.props[*k].after.as_deref() == Some("built:cooking")).collect();
    assert!(!cooking.is_empty(), "the content has a cooking tier");
    assert!(cooking.iter().all(|k| now.contains(k)), "the cooking tier stands");
    assert!(ev.iter().any(|e| matches!(e, Event::Announce { text } if text.contains("quartermaster"))), "the camp hears of it");
    assert!(ev.iter().any(|e| matches!(e, Event::ObjSpawned { .. })));
    // Once only.
    let ev = run(&mut s, 5.5);
    assert_eq!(pieces(&s).len(), now.len());
    assert!(!ev.iter().any(|e| matches!(e, Event::Announce { text } if text.contains("quartermaster"))));
    // The raids beaten: each tier comes in its turn.
    s.camp.raids_won = 2;
    run(&mut s, 5.5);
    let now = pieces(&s);
    for (k, p) in c.props.iter().enumerate() {
        match p.after.as_deref() {
            Some("raids:1") | Some("raids:2") => assert!(now.contains(&k), "{k} stands after two raids"),
            Some("raids:3") | Some("raids:4") => assert!(!now.contains(&k), "{k} waits"),
            _ => {}
        }
    }
}

#[test]
fn what_the_camp_has_reads_its_progress() {
    let mut s = world();
    assert!(s.camp_has("start"));
    assert!(!s.camp_has("raids:1"));
    assert!(!s.camp_has("built:smithy"));
    assert!(!s.camp_has("nonsense"));
    s.camp.raids_won = 1;
    s.camp.goal = 6;
    s.camp.found.insert("goldshire".into());
    s.camp.counts.insert("build:smithy".into(), 1);
    assert!(s.camp_has("raids:1") && !s.camp_has("raids:2"));
    assert!(s.camp_has("goal:6") && !s.camp_has("goal:7"));
    assert!(s.camp_has("found:goldshire") && !s.camp_has("found:azora"));
    assert!(s.camp_has("built:smithy"));
}

#[test]
fn the_camps_own_things_keep_their_ground() {
    let s = world();
    let fire = s.content.station("cooking").unwrap();
    for o in s.objs.values() {
        if let ObjKind::Piece { def } = o.kind {
            if s.content.props[def].clear > 0.0 {
                assert!(s.site_problem(fire, [o.pos[0], o.pos[1], 0.0], None).is_some(), "nothing built on {}", def);
            }
        }
    }
}

#[test]
fn a_station_from_where_the_camp_stood_before_is_carried_over() {
    let s = world();
    let mut save = s.save(&Default::default());
    // A smithy where the camp stood by Goldshire, damaged; and one already in the yard.
    let old = [-9548.0, 118.0, 0.0];
    save.stations.push(crate::save::StationSave { uid: 7, key: "smithy".into(), pos: old, facing: 1.0, health: 120, damaged: true });
    let c = Content::dev();
    let s = Survival::load(c, Box::new(FlatLand::default()), 1, 1_000_000.0, &save);
    let (pos, health, damaged) = s
        .objs
        .values()
        .find_map(|o| match o.kind {
            ObjKind::Station { uid: 7, health, damaged, .. } => Some((o.pos, health, damaged)),
            _ => None,
        })
        .unwrap();
    let w = &s.content.world;
    assert!(dist2d(pos, [w.build_center[0], w.build_center[1], 0.0]) <= w.build_radius, "moved into the yard: {pos:?}");
    assert_eq!((health, damaged), (120, true), "kept as it was");
    // Its fire went with it.
    assert!(s.objs.values().any(|o| matches!(o.kind, ObjKind::Fire { .. }) && dist2d(o.pos, pos) < 0.1));
}
