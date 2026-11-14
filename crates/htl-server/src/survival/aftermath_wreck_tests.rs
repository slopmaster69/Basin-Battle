use super::*;
use benilla_protocol::messages::{parse_server_with_tail, Object, ServerPacket};
use htl_survival::content::CollapseSectionDef;

const DT: f32 = 0.05;
/// Where the test tunnel's faces stand, along +x from its door at 0 (as `sectioned` in the rules'
/// own tests).
const LINES: [f32; 4] = [30.0, 44.0, 74.0, 88.0];

/// Rhahk'Zor at 35 yd down a flat tunnel along +x, his collapse in four sections.
fn game() -> SurvivalGame {
    let mut c = htl_survival::Content::builtin().for_dungeon_of("deadmines");
    let d = c.dungeon.as_mut().unwrap();
    d.packs.retain(|p| p.mob == "dm_rhahkzor");
    d.packs[0].at = [35.0, 0.0];
    d.packs[0].z = Some(0.0);
    for l in &mut d.lamps {
        l.at = [400.0, 400.0, 0.0];
    }
    let def = c.mob("dm_rhahkzor").unwrap();
    let col = c.mobs[def].phases.iter_mut().find_map(|p| p.collapse.as_mut()).unwrap();
    assert!(col.wreck, "the mine's collapse wrecks it");
    col.route = vec![[0.0, 0.0, 0.0], [95.0, 0.0, 0.0]];
    col.rest = 2.0;
    col.sections = LINES.iter().map(|x| CollapseSectionDef { face: [[*x, 10.0], [*x, -10.0]], reach: 25.0, height: 8.0 }).collect();
    SurvivalGame::new_in(Realm::BELOW, c, Box::new(htl_survival::FlatLand::default()), None, 41)
}

fn hero(game: &mut SurvivalGame, conn: ConnId, at: [f32; 3], level: u8) -> (Id, crossbeam_channel::Receiver<Packet>) {
    let account = format!("wreck{conn}");
    let id = game.sim.join(&account, &account, None);
    game.sim.move_player(id, at, 0.0, Default::default());
    let guid = SurvivalGame::player_guid(&account);
    game.bind(id, guid);
    let (tx, rx) = crossbeam_channel::unbounded();
    let mut w = InWorld::new(guid, id, account.clone(), at[2]);
    w.level = level;
    game.sessions.insert(conn, Session { account: account.clone(), hero: account, out: tx, world: Some(w), watching: None });
    (id, rx)
}

fn piece(model: usize, x: f32, y: f32) -> Prop {
    Prop { region: 0, site: "test", model, pos: [x, y, 0.0], scale: 1.0, rotation: [0.0, 0.0, 0.0, 1.0], facing: 0.0 }
}

/// A ruin of the test's own: two pieces deep in the first section, one on its face's line (never
/// laid), one in the second, one past the end; one copy of a doodad.
fn ruin() -> Ruin {
    Ruin {
        props: vec![piece(ROCK, 10.0, 0.0), piece(SKIRT, 20.0, 6.0), piece(ROCK, 29.0, 0.0), piece(ROCK, 36.0, 0.0), piece(ROCK, 200.0, 0.0)],
        knocked: vec![piece(SKIRT, 300.0, 0.0)],
    }
}

/// What a client was sent of the wreck: its pieces made and taken, and the marker.
#[derive(Default, Debug)]
struct Seen {
    made: Vec<u64>,
    gone: Vec<u64>,
    marker: bool,
    marker_gone: bool,
}

fn ours(guid: u64) -> bool {
    guid >> 48 == 0xF110 && (0x40_0000..0x80_0000).contains(&(guid & 0xFF_FFFF))
}

fn seen(rx: &crossbeam_channel::Receiver<Packet>, s: &mut Seen) {
    for p in rx.try_iter() {
        let Ok((parsed, _)) = parse_server_with_tail(p.opcode, &p.body) else { continue };
        match parsed {
            ServerPacket::DestroyObject { guid, .. } => {
                if guid == MARKER_GUID {
                    s.marker_gone = true;
                } else if ours(guid) {
                    s.gone.push(guid);
                }
            }
            ServerPacket::UpdateObject { objects, .. } => {
                for o in objects {
                    if let Object::Create { guid, .. } = o {
                        if guid == MARKER_GUID {
                            s.marker = true;
                        } else if ours(guid) {
                            s.made.push(guid);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// One tick: the rules, their events, the wreck; the fighter 3 yd past the section going.
fn tick(game: &mut SurvivalGame, fighter: Id) {
    if let Some((k, ..)) = game.sim.collapse_state() {
        game.sim.move_player(fighter, [LINES[k] + 3.0, 0.0, 0.0], 0.0, Default::default());
        let p = game.sim.players.get_mut(&fighter).unwrap();
        p.health = p.max_health;
    }
    let events = game.sim.step(DT);
    game.play(events);
    game.wreck_tick();
}

fn until(game: &mut SurvivalGame, fighter: Id, want: (usize, &str), then: f32) {
    while game.sim.collapse_state().map(|c| (c.0, c.1)) != Some(want) {
        assert!(game.sim.clock < 300.0, "never at {want:?}: {:?}", game.sim.collapse_state());
        tick(game, fighter);
    }
    for _ in 0..(then / DT) as usize {
        tick(game, fighter);
    }
}

fn begin(game: &mut SurvivalGame, fighter: Id) {
    game.sim.dev_hurt(fighter, 0.68, "rhahkzor").unwrap();
    for _ in 0..200 {
        tick(game, fighter);
        if game.sim.collapse_wreck().is_some() {
            return;
        }
    }
    panic!("the collapse never began");
}

#[test]
fn each_section_lays_its_ruin_behind_its_face_and_the_ship_goes_to_whoever_is_out_of_the_fight() {
    let mut game = game();
    let (fighter, frx) = hero(&mut game, 1, [38.0, 0.0, 0.0], doodads::LEVEL);
    let (_friend, orx) = hero(&mut game, 2, [-150.0, 200.0, 0.0], doodads::LEVEL);
    let (_old, oldrx) = hero(&mut game, 3, [-150.0, 220.0, 0.0], 0);
    game.wreck.ruin = Some(ruin());
    let (mut f, mut o, mut old) = (Seen::default(), Seen::default(), Seen::default());
    begin(&mut game, fighter);
    let laid = game.wreck.laid.clone().expect("the ruin laid out by section");
    assert_eq!(laid, vec![vec![0, 1], vec![3], vec![], vec![]], "deep in their sections only: the piece on a face's line and the one past the end never laid");
    // The first section down: nothing laid until its face is up.
    until(&mut game, fighter, (0, "rest"), 0.5);
    seen(&frx, &mut f);
    assert!(f.made.is_empty(), "nothing laid while the face goes up: {f:?}");
    until(&mut game, fighter, (0, "rest"), SHOW_AFTER);
    seen(&frx, &mut f);
    seen(&orx, &mut o);
    seen(&oldrx, &mut old);
    let first = vec![wreck_guid(0, ROCK), wreck_guid(1, SKIRT)];
    assert_eq!(f.made, first, "the first section's ruin, to the fighter");
    // The ship: gone for the friend far off (with the doodads' copies, of 0091's round), and for
    // an older client too; never under the hero in the fight.
    assert!(o.marker && old.marker, "the ship gone for those out of the fight: {o:?} {old:?}");
    assert!(o.made.contains(&wreck_guid(5, SKIRT)) && !old.made.contains(&wreck_guid(5, SKIRT)), "copies only to 0091's round");
    assert!(!f.marker, "the fighter's mine is not reloaded under him");
    until(&mut game, fighter, (1, "rest"), SHOW_AFTER + 0.5);
    seen(&frx, &mut f);
    assert_eq!(f.made, vec![first[0], first[1], wreck_guid(3, ROCK)], "the second section's");
    assert!(!f.marker);
    // A late arrival is sent what is down.
    let (_late, lrx) = hero(&mut game, 4, [-150.0, 240.0, 0.0], doodads::LEVEL);
    tick(&mut game, fighter);
    let mut late = Seen::default();
    seen(&lrx, &mut late);
    assert_eq!(late.made.iter().filter(|g| **g != wreck_guid(5, SKIRT)).count(), 3, "both sections' ruin to a late arrival: {late:?}");
    assert!(late.marker);
    // At the last stand, the fight on: still not under him.
    while !game.sim.collapse_front().is_some_and(|c| c.1) {
        assert!(game.sim.clock < 300.0);
        tick(&mut game, fighter);
    }
    seen(&frx, &mut f);
    assert!(!f.marker, "not at the last stand while the fight is on");
    // He is down: the ship goes for the fighter then, before any walk out.
    game.sim.dev_hurt(fighter, 0.0, "rhahkzor").unwrap();
    for _ in 0..10 {
        tick(&mut game, fighter);
    }
    seen(&frx, &mut f);
    assert!(f.marker, "the ship gone for the fighter once he is down");
    assert!(f.gone.is_empty() && !f.marker_gone, "the wreck lies on after the kill");
}

#[test]
fn a_lost_fight_takes_the_ruin_and_the_ship_back() {
    let mut game = game();
    let (fighter, frx) = hero(&mut game, 1, [38.0, 0.0, 0.0], doodads::LEVEL);
    let (_friend, orx) = hero(&mut game, 2, [-150.0, 200.0, 0.0], doodads::LEVEL);
    game.wreck.ruin = Some(ruin());
    let (mut f, mut o) = (Seen::default(), Seen::default());
    begin(&mut game, fighter);
    until(&mut game, fighter, (1, "rest"), SHOW_AFTER + 0.5);
    seen(&frx, &mut f);
    seen(&orx, &mut o);
    assert_eq!(f.made.len(), 3);
    assert!(o.marker);
    // Every hero below down: the fight is lost.
    game.sim.fall_damage(fighter, u32::MAX / 2);
    let friend = game.world_of(2).unwrap().id;
    game.sim.fall_damage(friend, u32::MAX / 2);
    for _ in 0..40 {
        let events = game.sim.step(DT);
        game.play(events);
        game.wreck_tick();
    }
    assert!(game.sim.collapse_wreck().is_none(), "the fight lost");
    seen(&frx, &mut f);
    seen(&orx, &mut o);
    for s in [&f, &o] {
        let (mut made, mut gone) = (s.made.clone(), s.gone.clone());
        made.sort();
        gone.sort();
        assert_eq!(made, gone, "every piece sent is taken back: {s:?}");
        assert_eq!(s.marker, s.marker_gone, "and the ship, where it went");
    }
    assert!(game.wreck.sent.is_empty() && game.wreck.laid.is_none());
}

/// The real mine: the ruin fitted to the installed data, laid by Rhahk'Zor's own collapse.
#[test]
#[ignore = "requires the user's WoW 1.12.1 Data folder in WOW_DATA"]
fn the_real_ruin_is_laid_only_deep_behind_the_real_faces() {
    let data = std::path::PathBuf::from(std::env::var_os("WOW_DATA").expect("set WOW_DATA"));
    let ruin = compose_ruin(data.clone()).expect("the ruin fits");
    let land = crate::survival::land::DeadminesLand::new(htl_map::deadmines::load(&data).unwrap());
    let mut c = htl_survival::Content::builtin();
    let def = c.mob("dm_rhahkzor").unwrap();
    let route = c.mobs[def].phases.iter().find_map(|p| p.collapse.as_ref()).unwrap().route.clone();
    c.dungeon.as_mut().unwrap().packs.retain(|p| p.mob == "dm_rhahkzor");
    let mut s = htl_survival::Survival::new_dungeon(c, Box::new(land), 5, 0.0);
    let hero = s.join("ruinproof", "Ruinproof", None);
    let walk = htl_survival::land::walking_route(s.land(), &route).expect("native route");
    let at = s.land().footing(walk[1]).expect("floor on the route");
    s.move_player(hero, at, 0.0, Default::default());
    s.dev_hurt(hero, 0.68, "rhahkzor").unwrap();
    for _ in 0..100 {
        s.step(0.05);
        if s.collapse_wreck().is_some() {
            break;
        }
    }
    assert!(s.collapse_wreck().is_some(), "the collapse wrecks the mine");
    let laid = laid_by_section(&s, &ruin.props);
    for (k, pieces) in laid.iter().enumerate() {
        for i in pieces {
            let p = &ruin.props[*i];
            // Behind its own face, past all it reaches: rubble to the roof between it and anyone.
            assert_eq!(s.collapse_wreck_section(p.pos, BEHIND + reach(p)), Some(k), "{} at {:?}", p.site, p.pos);
            assert!(s.collapse_wreck_off_route(p.pos).is_some(), "by the route");
        }
    }
    println!(
        "the ruin: {} pieces, {} copies; laid by section: {:?}; kinds: {:?}",
        ruin.props.len(),
        ruin.knocked.len(),
        laid.iter().map(Vec::len).collect::<Vec<_>>(),
        laid.iter().flatten().map(|i| MODELS[ruin.props[*i].model].name).collect::<std::collections::BTreeSet<_>>()
    );
}
