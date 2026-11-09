use super::*;
use benilla_protocol::messages::{parse_server_with_tail, Object, ObjectType, ServerPacket};

/// Authored floors, without native game data. The ignored test below supplies the real map.
struct AuthoredFloors;
impl Land for AuthoredFloors {
    fn height(&self, _: f32, _: f32) -> f32 { 0.0 }
    fn height_near(&self, _: f32, _: f32, z: f32) -> f32 { z }
}

fn game() -> SurvivalGame {
    let content = htl_survival::Content::builtin().for_dungeon_of("deadmines");
    let mut game = SurvivalGame::new_in(Realm::BELOW, content, Box::new(AuthoredFloors), None, 73);
    game.sim.debug = true;
    game.aftermath_land = Some(Box::new(AuthoredFloors));
    game
}

fn observer(game: &mut SurvivalGame, conn: ConnId) -> (Id, crossbeam_channel::Receiver<Packet>) {
    let account = format!("observer{conn}");
    let id = game.sim.join(&account, &account, None);
    let pos = game.sim.players[&id].pos;
    let guid = SurvivalGame::player_guid(&account);
    game.bind(id, guid);
    let (tx, rx) = crossbeam_channel::unbounded();
    game.sessions.insert(conn, Session { account: account.clone(), hero: account.clone(), out: tx,
        world: Some(InWorld::new(guid, id, account, pos[2])) });
    (id, rx)
}

fn creates(packets: &[Packet]) -> Vec<(u64, ObjectType, BTreeMap<u16, u32>)> {
    let mut out = Vec::new();
    for p in packets {
        assert!(p.body.len() < 16_384, "bounded batches stay far below the 16-bit wire limit");
        let (parsed, tail) = parse_server_with_tail(p.opcode, &p.body).expect("native client parses packet");
        assert_eq!(tail, 0, "packet consumed completely");
        if let ServerPacket::UpdateObject { objects, .. } = parsed {
            for object in objects {
                if let Object::Create { guid, object_type, mask, .. } = object {
                    out.push((guid, object_type, mask.raw_fields().map(|(i, value)| (i as u16, value)).collect()));
                }
            }
        }
    }
    out
}

#[test]
fn aftermath_is_a_bounded_deterministic_solid_scene_with_side_rooms() {
    let (props, missing) = compose(&AuthoredFloors);
    assert_eq!((props.clone(), missing.clone()), compose(&AuthoredFloors));
    assert!(missing.is_empty());
    assert!((400..700).contains(&props.len()), "{} props", props.len());
    for (region, _) in layout::REGIONS.iter().enumerate() {
        assert!(props.iter().filter(|p| p.region == region).count() >= 20, "region {region}");
        assert!(props.iter().filter(|p| p.region == region && p.site == "Rubble apron").count() >= 4,
            "each view has close debris around its small clear footing, region {region}");
    }
    for branch in ["Johnson's side gallery", "Sealed eastern niche", "Workshop rear cell", "Marisa's side room", "Pantry branch", "Smugglers' cache"] {
        assert!(props.iter().filter(|p| p.site == branch).count() >= 5, "{branch}");
    }
    assert!(props.iter().any(|p| p.site == "Juggernaut collapse basin" && p.scale >= 6.0), "large seated cove masses");
    assert_eq!(props.iter().filter(|p| p.model == CAVE_IN).count(), 1, "native cave-in candidate stays in one site until inspected");
    assert!(!props.iter().any(covers_view), "inspection footings stay clear");
    assert!(!MODELS.iter().any(|m| matches!(m.display, 134 | 4315 | 4316)), "the native check rejected the luminous stone palette");
    for p in props {
        assert!((p.rotation.iter().map(|v| v*v).sum::<f32>()-1.0).abs() < 0.0001);
        assert!(p.pos.iter().all(|v| v.is_finite()) && p.scale > 0.0);
        if p.model <= RUBBLE {
            assert!(1.0-2.0*(p.rotation[0]*p.rotation[0]+p.rotation[1]*p.rotation[1]) > 0.99,
                "terrain rocks keep their open ground-facing skirts underneath");
        }
    }
}

#[test]
fn tilted_wrecks_are_seated_by_their_rotated_bottom() {
    let site = &layout::SITES[20];
    for model in [SUPPORT, MACHINE, CANNON, BOAT] {
        let p = make_prop(site, model, [0.0, 0.0, 18.0], 1.8, 1.3, 0.2, 2.7);
        let world_bottom = p.pos[2] + bottom(model, p.rotation)*p.scale;
        assert!((world_bottom - (18.0-0.08*1.8)).abs() < 0.001);
    }
    assert!(floor(&htl_survival::FlatLand { z: 300.0, ..Default::default() }, [-190.0,-480.0,54.0], false).is_none(),
        "a missing tunnel floor cannot silently move a collapse to the outdoor roof");
    let (props, _) = compose(&AuthoredFloors);
    for p in props.iter().filter(|p| p.model == SUPPORT) {
        let [x,y,z,w] = p.rotation;
        let row = [2.0*(x*z-y*w), 2.0*(y*z+x*w), 1.0-2.0*(x*x+y*y)];
        let height: f32 = (0..3).map(|a| row[a].abs()*(MODELS[SUPPORT].max[a]-MODELS[SUPPORT].min[a])).sum();
        assert!(height < 2.0, "the 11.39 yd Y axis must remain fallen, not stand on end: {height}");
    }
}

#[test]
fn on_off_and_inspection_movement_preserve_the_real_world_and_hero() {
    let mut game = game();
    let (hero, rx) = observer(&mut game, 1);
    let before = game.sim.save(&game.offline);
    let original = game.sim.players[&hero].pos;
    let clock = game.sim.clock;
    let mob_count = game.sim.mobs.len();
    game.aftermath_start(1).unwrap();
    let manifest = game.aftermath.as_ref().unwrap().props.clone();
    game.aftermath_command(1, "visit", "foundry").unwrap();
    let mut movement = Body::new();
    movement.u32(1).u32(0).vec3([-99.0, -820.0, -12.0]).f32(0.6).u32(0);
    game.movement(1, op::MSG_MOVE_FALL_LAND, &movement.into_vec()).unwrap();
    game.command(1, "es reset me");
    game.command(1, "es give wood 100");
    game.packet(1, op::CMSG_GAMEOBJ_USE.into(), &0_u64.to_le_bytes()).unwrap();
    for _ in 0..40 { game.tick(); }
    assert_eq!(game.sim.clock, clock);
    assert_eq!(game.sim.players[&hero].pos, original);
    assert_eq!(game.aftermath_where(1), Some(([-99.0,-820.0,-12.0], 0.6)));
    assert_eq!(game.sim.save(&game.offline), before);
    assert_eq!(game.sim.mobs.len(), mob_count);
    game.sim.debug = false; // Restoring does not depend on debug staying enabled.
    game.aftermath_command(1, "off", "").unwrap();
    assert!(game.aftermath.is_none());
    assert_eq!(game.sim.save(&game.offline), before);
    let sent: Vec<_> = rx.try_iter().collect();
    assert!(sent.iter().any(|p| p.opcode == op::SMSG_DESTROY_OBJECT && p.body.starts_with(&MARKER_GUID.to_le_bytes())));
    assert!(creates(&sent).iter().any(|(_, kind, _)| *kind == ObjectType::Unit), "intact mobs return");
    game.sim.debug = true;
    game.aftermath_start(1).unwrap();
    assert_eq!(game.world_of(1).unwrap().aftermath_return, None, "a fresh preview supersedes an unacknowledged return");
    assert_eq!(game.aftermath.as_ref().unwrap().props, manifest, "on-off-on repeats exact composition");
}

#[test]
fn aftermath_return_ignores_queued_movement_until_its_own_teleport_is_acknowledged() {
    let mut game = game();
    let (hero, _) = observer(&mut game, 1);
    let (_, peer) = observer(&mut game, 2);
    let before = game.sim.save(&game.offline);
    let original = game.sim.players[&hero].pos;
    game.aftermath_start(1).unwrap();
    game.aftermath_command(1, "visit", "mouth").unwrap();
    let old_counter = game.world_of(1).unwrap().counter;
    game.aftermath_stop();
    peer.try_iter().for_each(drop);
    let w = game.world_of(1).unwrap();
    let (guid, counter) = (w.guid, w.counter);
    let ack = |guid, counter| benilla_protocol::messages::teleport_ack(guid, counter, 0);
    game.packet(1, op::MSG_MOVE_TELEPORT_ACK.into(), &ack(guid, old_counter)).unwrap();
    game.packet(1, op::MSG_MOVE_TELEPORT_ACK.into(), &ack(guid+1, counter)).unwrap();
    assert_eq!(game.world_of(1).unwrap().aftermath_return, Some(counter), "stale or foreign ACK cannot end the handoff");
    let mut queued = Body::new();
    queued.u32(1).u32(0).vec3([-40.0,-385.0,55.8]).f32(0.6).u32(0);
    game.movement(1, op::MSG_MOVE_FALL_LAND, &queued.into_vec()).unwrap();
    assert_eq!(game.sim.save(&game.offline), before, "queued preview movement never reaches the restored hero");
    assert!(peer.try_recv().is_err(), "queued movement is not relayed to peers");
    game.packet(1, op::MSG_MOVE_TELEPORT_ACK.into(), &ack(guid, counter)).unwrap();
    assert_eq!(game.world_of(1).unwrap().aftermath_return, None);
    assert!(game.world_of(2).unwrap().aftermath_return.is_some(), "each inspector acknowledges their own return");
    let moved = [original[0]+0.5, original[1], original[2]];
    let mut movement = Body::new();
    movement.u32(1).u32(0).vec3(moved).f32(0.4).u32(0);
    game.movement(1, op::MSG_MOVE_HEARTBEAT, &movement.into_vec()).unwrap();
    assert_eq!(game.sim.players[&hero].pos, moved, "ordinary movement resumes after the acknowledged return");
    assert!(peer.try_recv().is_ok());
    game.leave_world(2);
    observer(&mut game, 2);
    assert_eq!(game.world_of(2).unwrap().aftermath_return, None, "disconnect discards the old handoff");
}

#[test]
fn late_observers_receive_the_same_marker_and_solid_manifest() {
    let mut game = game();
    let (_, rx) = observer(&mut game, 1);
    game.aftermath_start(1).unwrap();
    let first = creates(&rx.try_iter().collect::<Vec<_>>());
    let (_, late_rx) = observer(&mut game, 2);
    game.send_world_to(2, game.world_of(2).unwrap().id);
    let late = creates(&late_rx.try_iter().collect::<Vec<_>>());
    let gos = |objects: Vec<(u64, ObjectType, BTreeMap<u16,u32>)>| objects.into_iter().filter(|(_, kind, _)| *kind == ObjectType::GameObject).collect::<Vec<_>>();
    let first = gos(first);
    let late = gos(late);
    assert_eq!(first, late);
    assert_eq!(first[0].0, MARKER_GUID, "geometry marker comes before ruin objects");
    assert_eq!(first[0].2[&idx::ENTRY], MARKER_ENTRY);
    assert_eq!(first[0].2[&idx::GAMEOBJECT_DISPLAYID], 0);
    for (_, _, fields) in &first[1..] {
        assert_eq!(fields[&idx::GAMEOBJECT_FLAGS], INERT, "no visual-only or no-hull flag on debris");
        assert_ne!(fields[&idx::GAMEOBJECT_DISPLAYID], 0);
        if fields[&idx::GAMEOBJECT_DISPLAYID] == 2230 {
            assert_eq!(fields[&idx::GAMEOBJECT_TYPE_ID], 10, "cave-in uses the native state animation driver");
            assert_eq!(fields[&idx::GAMEOBJECT_STATE], 1, "cave-in holds its settled Closed pose");
        }
    }
    assert_eq!(first.len(), game.aftermath.as_ref().unwrap().props.len()+1);
    let unique: std::collections::BTreeSet<_> = first.iter().map(|(g, _, _)| *g).collect();
    assert_eq!(unique.len(), first.len());
}

#[test]
fn preview_does_not_write_the_save_even_when_an_inspector_disconnects() {
    let mut game = game();
    let (hero, _) = observer(&mut game, 1);
    let original = game.sim.players[&hero].pos;
    let path = std::env::temp_dir().join(format!("htl-aftermath-save-{}.toml", std::process::id()));
    std::fs::write(&path, "unchanged save sentinel").unwrap();
    game.save_path = Some(path.clone());
    game.aftermath_start(1).unwrap();
    game.aftermath_command(1, "visit", "mouth").unwrap();
    game.save();
    game.leave_world(1);
    assert!(game.aftermath.is_none(), "the last inspector restores the normal realm");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "unchanged save sentinel");
    assert_eq!(game.offline["observer1"].pos, original);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn preview_is_scoped_to_debug_and_the_deadmines() {
    let mut game = game();
    game.sim.debug = false;
    assert!(game.aftermath_start(1).is_err());
    game.sim.debug = true;
    game.realm = Realm::CAMP;
    assert!(game.aftermath_start(1).is_err());
    assert!(game.aftermath.is_none());
}

#[test]
fn invalid_inspection_movement_is_never_relayed_to_an_observer() {
    let mut game = game();
    observer(&mut game, 1);
    let (_, rx) = observer(&mut game, 2);
    game.aftermath_start(1).unwrap();
    rx.try_iter().for_each(drop);
    let mut movement = Body::new();
    movement.u32(0).u32(0).vec3([f32::NAN, 0.0, 0.0]).f32(0.0).u32(0);
    game.movement(1, op::MSG_MOVE_HEARTBEAT, &movement.into_vec()).unwrap();
    assert!(rx.try_recv().is_err());
}

#[test]
#[ignore = "requires the user's WoW 1.12.1 Data folder in WOW_DATA"]
fn native_aftermath_floors_support_every_region_and_no_ship_deck() {
    let data = std::env::var_os("WOW_DATA").expect("set WOW_DATA");
    let loaded = htl_map::deadmines::load_aftermath(std::path::Path::new(&data)).unwrap();
    let land = super::super::land::DeadminesLand::new(loaded);
    let (props, missing) = compose(&land);
    println!("{} native ruins; unavailable sites: {missing:?}", props.len());
    for site in layout::SITES.iter().filter(|s| missing.contains(&s.name)) {
        for dx in [-10.0, 0.0, 10.0] {
            for dy in [-10.0, 0.0, 10.0] {
                let (x, y) = (site.at[0]+dx, site.at[1]+dy);
                if let Some((i,j)) = land.ground().cell_at(x,y) {
                    println!("{} near {x},{y}: {:?}", site.name, land.ground().floors_at(i,j));
                }
            }
        }
    }
    for (i, region) in layout::REGIONS.iter().enumerate() {
        let count = props.iter().filter(|p| p.region == i).count();
        println!("{}: {count} solids", region.name);
        assert!(count >= 15, "{}", region.name);
        assert!(floor(&land, region.view, false).is_some(), "{} view", region.name);
    }
    assert!(missing.is_empty(), "fix authored sites instead of silently leaving rooms empty");
    assert!(props.len() >= 350);
    assert!(floor(&land, [-105.0,-821.0,37.9], false).is_none(), "captain's quarters really gone");
    assert!(floor(&land, [-62.0,-815.0,41.5], false).is_none(), "upper ship deck really gone");
    assert!(floor(&land, [-80.0,-820.0,-16.0], true).is_some(), "seabed remains under wreckage");
    assert!(props.iter().any(|p| p.site == "Smite's ruined landing" && p.model == CANNON),
        "the dock retains a grounded cannon wreck beside the inspection approach");
}
