use std::f32::consts::PI;

use super::*;
use benilla_protocol::messages::{parse_server_with_tail, Object, ObjectType, ServerPacket};

/// Authored floors, without native game data: flat ground at each site's (or view's) height, out to
/// halfway to the next. The ignored tests below supply the real map.
struct AuthoredFloors;
impl Land for AuthoredFloors {
    fn height(&self, x: f32, y: f32) -> f32 { self.height_near(x, y, 0.0) }
    fn height_near(&self, x: f32, y: f32, _: f32) -> f32 {
        let spots = layout::SITES.iter().map(|s| s.at).chain(layout::REGIONS.iter().map(|r| r.view));
        spots.min_by(|a, b| (a[0]-x).hypot(a[1]-y).total_cmp(&(b[0]-x).hypot(b[1]-y))).map_or(0.0, |s| s[2])
    }
}

/// The authored floors under a flat roof 7 yd over every site, for the broken roofs to hang from.
fn authored_ground() -> Ground {
    let mut roof = Vec::new();
    for s in layout::SITES {
        let (x, y, z) = (s.at[0], s.at[1], s.at[2] + 7.0);
        roof.push([[x - 15.0, y - 15.0, z], [x + 15.0, y - 15.0, z], [x + 15.0, y + 15.0, z]]);
        roof.push([[x - 15.0, y - 15.0, z], [x + 15.0, y + 15.0, z], [x - 15.0, y + 15.0, z]]);
    }
    let solids = htl_map::solids::Solids::build([-400.0, -1100.0], [100.0, -300.0], 4.0, roof);
    Ground::with_solids(Box::new(AuthoredFloors), solids)
}

fn game() -> SurvivalGame {
    let content = htl_survival::Content::builtin().for_dungeon_of("deadmines");
    let mut game = SurvivalGame::new_in(Realm::BELOW, content, Box::new(AuthoredFloors), None, 73);
    game.sim.debug = true;
    game.aftermath_land = Some(Box::new(authored_ground()));
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
        world: Some(InWorld::new(guid, id, account, pos[2])), watching: None });
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

fn hanging(p: &Prop) -> bool { p.model == ROCK && p.hangs() }

#[test]
fn aftermath_is_a_bounded_deterministic_solid_scene_with_side_rooms() {
    let ground = authored_ground();
    let (props, missing) = compose(&ground);
    assert_eq!((props.clone(), missing.clone()), compose(&ground));
    assert!(missing.is_empty(), "{missing:?}");
    assert!((700..1800).contains(&props.len()), "{} props", props.len());
    for (region, _) in layout::REGIONS.iter().enumerate() {
        assert!(props.iter().filter(|p| p.region == region).count() >= 20, "region {region}");
        assert!(props.iter().filter(|p| p.region == region && p.site == "Rubble apron").count() >= 3,
            "each view has close debris around its small clear footing, region {region}");
    }
    for branch in ["Johnson's side gallery", "Sealed eastern niche", "Workshop rear cell", "Marisa's side room", "Pantry tent stores", "Smugglers' cache"] {
        assert!(props.iter().filter(|p| p.site == branch).count() >= 5, "{branch}");
    }
    assert!(!props.iter().any(covers_view), "inspection footings stay clear");
    // The native checks rejected the luminous stone (134, 4315, 4316), the tall boulder that read as
    // a stalagmite (5737) and the Dark Iron boards that rendered gold (6481, 6193).
    assert!(!MODELS.iter().any(|m| matches!(m.display, 134 | 4315 | 4316 | 5737 | 5740 | 6481 | 6193)));
    for p in props {
        assert!((p.rotation.iter().map(|v| v*v).sum::<f32>()-1.0).abs() < 0.0001);
        assert!(p.pos.iter().all(|v| v.is_finite()) && p.scale > 0.0);
        if rocky(p.model) {
            let up = matrix(p.rotation)[2][2];
            assert!(up > 0.99 || hanging(&p), "rock and soil keep their open skirts in the floor or the roof: {p:?}");
        }
    }
}

/// What each large pile lacks of change A and B: three sizes of rock, a soil skirt, timber at its
/// foot, rock hanging from the broken roof, and a beam snapped out of the roof with a post leaning
/// on the pile (a sealed passage: its lintel instead).
fn pile_shortfalls(ground: &Ground, props: &[Prop]) -> Vec<String> {
    let mut out = Vec::new();
    for site in layout::SITES.iter().filter(|s| matches!(s.fall, Fall::Slump | Fall::Seal | Fall::Lintel)) {
        let mine: Vec<&Prop> = props.iter().filter(|p| p.site == site.name).collect();
        let rocks = |lo: f32, hi: f32| mine.iter().filter(|p| p.model == ROCK && !hanging(p) && p.scale >= lo && p.scale < hi).count();
        let mut miss = |ok: bool, what: &str| if !ok { out.push(format!("{}: {what}", site.name)); };
        miss(rocks(0.75, 9.0) >= 1 && rocks(0.3, 0.75) >= 2 && rocks(0.0, 0.3) >= 3, "three sizes of rock");
        miss(mine.iter().filter(|p| matches!(p.model, SKIRT | SCREE)).count() >= 3, "a gravel skirt");
        miss(mine.iter().filter(|p| matches!(p.model, BOARD | BEAM)).count() >= 3, "timber at its foot");
        // Where the roof is out of sight (the cove's open cavern) there is no break to show. (The
        // rock hung from the roof read as a smooth lump on film and is gone: the snapped beam is the
        // break.)
        let fl = ground.land.height_near(site.at[0], site.at[1], site.at[2]);
        let roofed = ground.roof([site.at[0], site.at[1], fl]).is_some_and(|r| r - fl < 15.0);
        if site.fall == Fall::Slump {
            let lodged = |p: &&Prop| {
                let top = p.world([0.0, 0.0, -5.77]);
                let fl = ground.land.height_near(top[0], top[1], site.at[2]);
                ground.roof([top[0], top[1], fl]).is_some_and(|r| top[2] > r)
            };
            miss(!roofed || mine.iter().copied().filter(|p| p.model == BEAM).any(|p| lodged(&p)), "a beam snapped out of the roof");
            miss(roofed || mine.iter().any(|p| p.model == BEAM), "a post leaning on it");
            miss(!roofed || mine.iter().filter(|p| p.model == BEAM).count() >= 2, "a post leaning on it");
        } else {
            miss(mine.iter().any(|p| p.model == BEAM), "its lintel");
        }
    }
    out
}

#[test]
fn every_large_pile_has_three_sizes_a_skirt_timber_and_the_roof_it_fell_from() {
    let ground = authored_ground();
    let (props, _) = compose(&ground);
    // The authored world is flat plateaus under flat roofs: most piles get all of it there; the
    // native check below holds every pile to it on the real map.
    let short = pile_shortfalls(&ground, &props);
    assert!(short.len() * 5 <= layout::SITES.iter().filter(|s| s.fall != Fall::Spill).count(), "{short:#?}");
    // Boards lie at angles of their own, not in a row.
    let yaws: Vec<f32> = props.iter().filter(|p| p.model == BOARD).map(|p| { let m = matrix(p.rotation); m[1][1].atan2(m[0][1]) }).collect();
    let spread = yaws.iter().map(|y| (y.rem_euclid(PI) * 4.0 / PI) as usize).collect::<std::collections::BTreeSet<_>>();
    assert_eq!(spread.len(), 4, "boards turned every way");
    assert!(props.iter().filter(|p| p.model == BOARD).any(|p| { let m = matrix(p.rotation); m[2][1].abs() > 0.15 }), "some boards lean");
}

#[test]
fn wreckage_is_knocked_over_and_seated_by_its_turned_bottom() {
    let (props, _) = compose(&authored_ground());
    for p in props.iter().filter(|p| matches!(p.model, CART | BARROW | MACHINE | CARGO | CANNON)) {
        let up = matrix(p.rotation)[2][2];
        assert!(up < 0.6, "{} {}: not left standing as it was", p.site, MODELS[p.model].name);
        // Its lowest corner on the floor under it (the authored floors step from site to site).
        let floors: Vec<f32> = p.obb(0.0).samples().iter().map(|c| AuthoredFloors.height_near(c[0], c[1], c[2])).collect();
        let (lo, hi) = floors.iter().fold((f32::MAX, f32::MIN), |(a, b), f| (a.min(*f), b.max(*f)));
        let low = p.pos[2] + bottom(p.model, p.rotation) * p.scale;
        assert!(low <= hi + 0.05 && low >= lo - 0.1, "{}: {low} on {lo}..{hi}", p.site);
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
        assert_eq!(fields[&idx::GAMEOBJECT_TYPE_ID], GENERIC, "ruins use static solid props");
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

/// The installed map and meshes the contact check measures with.
struct Native {
    survey: htl_map::deadmines::aftermath::Survey,
    land: super::super::land::DeadminesLand,
    /// Each model's triangles in its own space.
    meshes: Vec<Vec<[V3; 3]>>,
}

fn native() -> Native {
    let data = std::env::var_os("WOW_DATA").expect("set WOW_DATA");
    let data = std::path::Path::new(&data);
    let survey = htl_map::deadmines::load_aftermath_survey(data).unwrap();
    let land = super::super::land::DeadminesLand::new(htl_map::deadmines::load_aftermath(data).unwrap());
    let displays: Vec<u32> = MODELS.iter().map(|m| m.display).collect();
    let meshes = htl_map::deadmines::aftermath::display_shapes(data, &displays).unwrap();
    Native { survey, land, meshes }
}

/// The ground the server fits the ruins to, from the same data.
fn native_ground(n: &Native) -> Ground {
    let data = std::env::var_os("WOW_DATA").expect("set WOW_DATA");
    Ground::surveyed(htl_map::deadmines::load_aftermath_survey(std::path::Path::new(&data)).unwrap(), n.meshes.clone())
}

/// Inside a rock hung from the roof (its dome below the rim, as the mesh has it).
fn ground_holds(n: &Native, p: &Prop, at: V3) -> bool {
    let [x, y, z] = p.local(at);
    top_of(&n.meshes[p.model], x, y).is_some_and(|t| z >= MODELS[p.model].min[2] && z <= t)
}

/// A prop's triangles in the world.
fn world_tris(n: &Native, p: &Prop) -> Vec<[V3; 3]> {
    n.meshes[p.model].iter().map(|t| t.map(|v| p.world(v))).collect()
}

/// What's wrong with each prop that doesn't rest on the floor or on rock, or passes through a
/// wall, one of the mine's doodads or another piece of wood or iron (none, for a sound ruin).
fn contact_failures(n: &Native, props: &[Prop]) -> Vec<String> {
    let solids = &n.survey.solids;
    let land: &dyn Land = &n.land;
    let natives: Vec<(String, Obb)> = n.survey.standing.iter()
        .filter(|s| !s.model.to_ascii_lowercase().contains("web"))
        .filter_map(|s| s.bounds.map(|(lo, hi)| (s.model.clone(), Obb::native(s.origin, &s.axes, lo, hi, 0.1))))
        .collect();
    let tris: Vec<Vec<[V3; 3]>> = props.iter().map(|p| world_tris(n, p)).collect();
    let roof = |x: f32, y: f32, z: f32| solids.ray([x, y, z + 0.5], [0.0, 0.0, 1.0], 40.0).map(|d| z + 0.5 + d);
    // Inside rock `j` (an upright rock's or mound's body, between its skirt's rim and its top).
    let in_rock = |j: usize, v: V3| -> bool {
        let q = &props[j];
        if !rocky(q.model) || q.hangs() { return false; }
        let rim = q.world([0.0, 0.0, MODELS[q.model].min[2]])[2] - 0.05;
        top_of(&tris[j], v[0], v[1]).is_some_and(|t| v[2] < t - 0.02 && v[2] > rim)
    };
    let rock_top = |i: usize, v: V3| -> Option<f32> {
        props.iter().enumerate().filter(|(j, q)| *j != i && rocky(q.model) && !q.hangs())
            .filter(|(_, q)| q.obb(0.0).contains([v[0], v[1], q.obb(0.0).c[2]]) || (q.pos[0]-v[0]).hypot(q.pos[1]-v[1]) < 12.0 * q.scale)
            .filter_map(|(j, _)| top_of(&tris[j], v[0], v[1]))
            .filter(|t| *t < v[2] + 0.6)
            .fold(None, |a: Option<f32>, t| Some(a.map_or(t, |a| a.max(t))))
    };
    let mut out = Vec::new();
    for (i, p) in props.iter().enumerate() {
        let name = format!("{} {} at {:.1},{:.1},{:.1}", p.site, MODELS[p.model].name, p.pos[0], p.pos[1], p.pos[2]);
        let w: Vec<V3> = tris[i].iter().flatten().copied().collect();
        let afloat = p.region == 6 && land.water_near(p.pos[0], p.pos[1], -16.0) && w.iter().any(|v| v[2] < -0.02);
        let c = p.obb(0.0).c;
        // Resting. A rock's or mound's skirt is below the floor or the rock it lies on all round its
        // rim. Wood and iron touch the floor or rock on two opposite sides of their middle (along
        // their own two flattest axes), sink no further than a buried end, or hang by an end lodged
        // in the roof.
        let low = w.iter().map(|v| v[2]).fold(f32::INFINITY, f32::min);
        let base = |v: &V3| floor_under(land, [v[0], v[1], low]);
        let anchored = w.iter().any(|v| roof(v[0], v[1], base(v)).is_some_and(|r| v[2] > r + 0.1));
        let gap = |v: V3| -> (f32, f32, bool) {
            let fl = floor_under(land, v);
            let surface = rock_top(i, v).map_or(fl, |t| t.max(fl));
            (v[2] - surface, v[2] - fl, (0..props.len()).any(|j| j != i && in_rock(j, v)))
        };
        if hanging(p) {
            for v in w.iter().filter(|v| (p.local(**v)[2] - MODELS[p.model].min[2]).abs() < 0.05) {
                if !roof(v[0], v[1], base(v)).is_some_and(|r| v[2] >= r - 0.05) {
                    out.push(format!("{name}: its open skirt shows under the roof at {:.1},{:.1},{:.1}", v[0], v[1], v[2]));
                    break;
                }
            }
        } else if rocky(p.model) {
            let low = MODELS[p.model].min[2];
            // (A rim point inside the mine's wall is hidden by the wall: a line across at knee
            // height from the rock's middle meets the wall before it.)
            let walled = |v: V3| {
                let from = [p.pos[0], p.pos[1], v[2] + 1.1];
                let d = [v[0] - from[0], v[1] - from[1], 0.0];
                let l = shapes::len(d);
                l > 0.05 && solids.ray(from, shapes::unit(d), l).is_some()
            };
            if let Some(v) = w.iter().filter(|v| (p.local(**v)[2] - low).abs() < 0.05).find(|v| { let (g, _, pinned) = gap(**v); g > 0.05 && !pinned && !walled(**v) }) {
                out.push(format!("{name}: its skirt shows {:.2} over what's under it at {:.1},{:.1}", gap(*v).0, v[0], v[1]));
            }
        } else if !afloat && !(p.model == BEAM && anchored) {
            let bx = p.obb(0.0);
            let flat = piles::rest_axes(&bx);
            let mut ends = [[false; 2]; 2];
            let mut sunk = None;
            for v in &w {
                let (g, below_floor, pinned) = gap(*v);
                if pinned || (-0.35..=0.15).contains(&g) {
                    for (axis, (a, h)) in flat.iter().enumerate() {
                        let d = shapes::dot(shapes::sub(*v, c), *a);
                        if d < -0.25 * h { ends[axis][0] = true; }
                        if d > 0.25 * h { ends[axis][1] = true; }
                    }
                }
                if below_floor < -0.4 && !pinned { sunk = Some(*v); }
            }
            if !ends.iter().any(|e| e[0] && e[1]) {
                let v = w.iter().min_by(|a, b| a[2].total_cmp(&b[2])).copied().unwrap_or(c);
                out.push(format!("{name}: doesn't rest, its lowest point {:.2} over what's under it at {:.1},{:.1}", gap(v).0, v[0], v[1]));
            }
            if let Some(v) = sunk { out.push(format!("{name}: sinks through the floor at {:.1},{:.1},{:.1}", v[0], v[1], v[2])); }
        }
        // Through nothing: no wall between its middle and any of its points, nothing of the mine's
        // own inside it, no other wood or iron through it, not swallowed by rock.
        if !rocky(p.model) && !afloat {
            let fc = floor_under(land, c);
            let from = [c[0], c[1], c[2].max(fc + 0.35)];
            let hit = w.iter().find(|v| {
                if p.model == BEAM && anchored && roof(v[0], v[1], base(v)).is_some_and(|r| v[2] > r - 0.6) { return false; }
                let to = [v[0], v[1], v[2].max(floor_under(land, **v) + 0.35)];
                let d = shapes::sub(to, from);
                let l = shapes::len(d);
                l > 0.05 && solids.ray(from, shapes::unit(d), l - 0.05).is_some()
            });
            if let Some(v) = hit { out.push(format!("{name}: passes into the rock at {:.1},{:.1},{:.1}", v[0], v[1], v[2])); }
            if (0..props.len()).any(|j| j != i && in_rock(j, c)) { out.push(format!("{name}: swallowed by rock")); }
            for (j, q) in props.iter().enumerate() {
                // (The wrecked ship's mast stands through its broken hull.)
                let stepped = p.site == "Juggernaut's mast" && q.site == "Juggernaut's broken hull";
                if j != i && !stepped && !rocky(q.model) && q.obb(0.0).overlaps(&p.obb(0.0)) && w.iter().any(|v| q.obb(0.15).contains(*v)) {
                    out.push(format!("{name}: through {} {}", q.site, MODELS[q.model].name));
                }
            }
        }
        let bx = p.obb(0.0);
        for (model, nb) in natives.iter().filter(|(_, nb)| nb.overlaps(&bx)) {
            let through = if rocky(p.model) {
                rock_through(model, nb, 1.0, |s| if hanging(p) { ground_holds(n, p, s) } else { in_rock(i, s) })
            } else {
                w.iter().any(|v| nb.contains(*v))
            };
            if through { out.push(format!("{name}: through the mine's own {}", model.rsplit(['\\', '/']).next().unwrap_or(model))); }
        }
    }
    out
}

#[test]
#[ignore = "requires the user's WoW 1.12.1 Data folder in WOW_DATA"]
fn native_ruins_rest_on_floor_or_rock_and_pass_through_nothing() {
    let n = native();
    let started = std::time::Instant::now();
    let ground = native_ground(&n);
    let surveyed = started.elapsed();
    let (props, missing) = compose(&ground);
    println!("survey {:?}, composition {:?}", surveyed, started.elapsed() - surveyed);
    let mut by_site: BTreeMap<&str, usize> = BTreeMap::new();
    for p in &props { *by_site.entry(p.site).or_default() += 1; }
    println!("{} native ruins; unavailable sites: {missing:?}; per site: {by_site:?}", props.len());
    let failures = contact_failures(&n, &props);
    for f in &failures { println!("  {f}"); }
    assert!(failures.is_empty(), "{} pieces don't rest or pass through something", failures.len());
}

#[test]
#[ignore = "requires the user's WoW 1.12.1 Data folder in WOW_DATA"]
fn native_aftermath_floors_support_every_region_and_no_ship_deck() {
    let n = native();
    let ground = native_ground(&n);
    let land = &*ground.land;
    let (props, missing) = compose(&ground);
    for (i, region) in layout::REGIONS.iter().enumerate() {
        let count = props.iter().filter(|p| p.region == i).count();
        println!("{}: {count} solids", region.name);
        assert!(count >= 15, "{}", region.name);
        assert!(floor(land, region.view, false).is_some(), "{} view", region.name);
    }
    assert!(missing.is_empty(), "fix authored sites instead of silently leaving rooms empty: {missing:?}");
    let thin: Vec<&str> = layout::SITES.iter().filter(|s| s.fall != Fall::Spill)
        .filter(|s| !props.iter().any(|p| p.site == s.name && p.model == ROCK && !p.hangs() && p.scale >= 0.75))
        .map(|s| s.name).collect();
    assert!(thin.is_empty(), "piles without their main mass: {thin:?}");
    // Every pile slumped against the wall has all of it, but for a few whose roof or floor leaves
    // no room for one piece: those are listed, and kept to a handful.
    let short = pile_shortfalls(&ground, &props);
    for s in &short { println!("  short: {s}"); }
    assert!(short.len() <= 4, "{} piles lack part of their fall", short.len());
    assert!(floor(land, [-105.0,-821.0,37.9], false).is_none(), "captain's quarters really gone");
    assert!(floor(land, [-62.0,-815.0,41.5], false).is_none(), "upper ship deck really gone");
    assert!(floor(land, [-80.0,-820.0,-16.0], true).is_some(), "seabed remains under wreckage");
    // (Smite's dock is kept clear for its cameras now: its cannon went to the shore.)
    assert!(!props.iter().any(covers_view), "the views stay clear");
}

#[test]
#[ignore = "requires the user's WoW 1.12.1 Data folder in WOW_DATA"]
fn native_way_from_the_entrance_to_the_cove_stays_open() {
    let n = native();
    let ground = native_ground(&n);
    let walk = &n.land;
    let (props, _) = compose(&ground);
    let tops: Vec<f32> = props.iter().map(|p| world_tris(&n, p).iter().flatten().map(|v| v[2]).fold(f32::MIN, f32::max)).collect();
    let mut blocked = Vec::new();
    // The landing is reached by boat now the ship is gone: the walk is the entrance to the cove.
    for leg in layout::REGIONS[..7].windows(2) {
        let (a, b) = (&leg[0], &leg[1]);
        let from = [a.view[0], a.view[1], floor(&*ground.land, a.view, false).unwrap()];
        let to = [b.view[0], b.view[1], floor(&*ground.land, b.view, false).unwrap()];
        let Some(path) = walk.walk(from, to) else { panic!("no walk from {} to {} in the collapsed map", a.name, b.name) };
        for w in path.windows(2) {
            let steps = (shapes::len(shapes::sub(w[1], w[0])) / 0.5).ceil().max(1.0) as usize;
            for k in 0..=steps {
                let at = shapes::add(w[0], shapes::mul(shapes::sub(w[1], w[0]), k as f32 / steps as f32));
                // Small stones, boards and the soil skirt are stepped over; what stands knee-high blocks.
                // (Over deep water the way is swum, not walked.)
                if walk.water_near(at[0], at[1], at[2]) { continue; }
                let tall = |(_, top): &(&Prop, &f32)| **top > at[2] + 0.7;
                if let Some((p, _)) = props.iter().zip(&tops).filter(tall).find(|(p, _)| covers(p, at, 0.45)) {
                    blocked.push(format!("{} to {}: {} {} at {:.1},{:.1}", a.name, b.name, p.site, MODELS[p.model].name, at[0], at[1]));
                }
            }
        }
    }
    blocked.dedup();
    for b in &blocked { println!("  {b}"); }
    assert!(blocked.is_empty(), "the way through is blocked in {} places", blocked.len());
}

/// A fight that's over doesn't hold the preview: a crossbowman taken away mid-draw left its cast
/// on the books, and Cookie's thrown pots their stand-ins, and either refused it till a restart.
#[test]
fn an_ended_fight_leaves_nothing_that_refuses_the_preview() {
    for key in ["dm_crossbowman", "dm_cookie", "dm_squallshaper"] {
        let mut game = game();
        let (id, rx) = observer(&mut game, 1);
        game.sim.summon(id, key).unwrap();
        for _ in 0..300 { game.tick(); rx.try_iter().for_each(drop); }
        game.sim.dev_clear_foes("").unwrap();
        for _ in 0..20 { game.tick(); rx.try_iter().for_each(drop); }
        assert!(game.casting.keys().all(|c| game.sim.players.contains_key(c)), "{key}: a gone creature's cast is let go");
        game.aftermath_start(1).unwrap_or_else(|e| panic!("{key}: {e}"));
        assert!(game.aftermath.is_some() && game.stand_ins.is_empty() && game.casting.is_empty());
        game.aftermath_command(1, "off", "").unwrap();
    }
}

/// A guid packed as the wire packs it.
fn packed(guid: u64) -> Vec<u8> {
    let bytes = guid.to_le_bytes();
    let mask = (0..8).filter(|i| bytes[*i] != 0).fold(0u8, |m, i| m | (1 << i));
    std::iter::once(mask).chain(bytes.iter().copied().filter(|b| *b != 0)).collect()
}

/// A packed guid from the front of `b`, and the bytes after it.
fn packed_guid(b: &[u8]) -> (u64, &[u8]) {
    let mask = b[0];
    let (mut guid, mut k) = (0u64, 1);
    for bit in 0..8 {
        if mask & (1 << bit) != 0 { guid |= u64::from(b[k]) << (8 * bit); k += 1; }
    }
    (guid, &b[k..])
}

#[test]
fn the_piles_raise_dust_and_grit_and_their_timber_creaks_while_the_preview_lasts() {
    let g = authored_ground();
    let (props, _) = compose(&g);
    let mut air = air::Air::new(&props, &g, std::time::Instant::now());
    let spots: BTreeMap<u64, [f32; 3]> = air.dust.iter().map(|s| (s.guid, s.at)).collect();
    assert_eq!(spots.len(), air.dust.len(), "a stand-in of its own for every spot");
    for s in &air.dust {
        let p = &props[s.prop];
        assert!(p.site != "Rubble apron");
        assert!(p.model == SKIRT || p.model == SCREE || (p.model == ROCK && p.scale < 0.3), "dust from the gravel");
        assert!((s.at[0] - p.pos[0]).hypot(s.at[1] - p.pos[1]) < 0.01 && (s.at[2] - p.pos[2]).abs() < 2.0, "on the floor at the gravel");
        assert!(!props.iter().enumerate().any(|(i, r)| prop_guid(i, r.model) == s.guid), "the stand-ins are no ruin's guid");
    }
    let made = air.stand_ins();
    assert_eq!(made.len(), air.dust.len());
    assert!(made.iter().all(|p| p.opcode == op::SMSG_UPDATE_OBJECT));
    let mut sent: Vec<Packet> = Vec::new();
    for step in 1..=600 { sent.extend(air.due(&props, step as f32 * 0.1)); }
    let ruins: BTreeMap<u64, &Prop> = props.iter().enumerate().map(|(i, p)| (prop_guid(i, p.model), p)).collect();
    let (mut puffs, mut grit, mut creaks) = (BTreeMap::<u64, usize>::new(), 0, 0);
    for p in &sent {
        if p.opcode == op::SMSG_SPELL_GO {
            // Cast by the spot's stand-in, as a ground look at the spot, naming no one.
            let (caster, rest) = packed_guid(&p.body);
            let (_, rest) = packed_guid(rest);
            let spot = spots.get(&caster).expect("dust comes from a spot's stand-in");
            assert_eq!(u32::from_le_bytes(rest[0..4].try_into().unwrap()), air::SPELL_DUST);
            assert_eq!(&rest[6..8], &[0, 0], "no one struck or missed");
            let at: Vec<f32> = (0..3).map(|k| f32::from_le_bytes(rest[10 + 4 * k..14 + 4 * k].try_into().unwrap())).collect();
            assert_eq!(at, spot.to_vec(), "on the spot's floor");
            *puffs.entry(caster).or_default() += 1;
            continue;
        }
        assert_eq!(p.opcode, op::SMSG_PLAY_OBJECT_SOUND);
        let guid = u64::from_le_bytes(p.body[4..12].try_into().unwrap());
        let ruin = ruins.get(&guid).expect("every sound comes from a ruin in the scene");
        let sound = u32::from_le_bytes(p.body[0..4].try_into().unwrap());
        match sound {
            air::SOUND_GRIT => { grit += 1; assert!(ruin.model != BEAM, "grit from the gravel") }
            s if air::SOUND_CREAK.contains(&s) => { creaks += 1; assert_eq!(ruin.model, BEAM, "timber creaks") }
            s => panic!("an unknown sound {s}"),
        }
    }
    // A minute: every spot puffs now and then, never in a stream; some puffs bring grit.
    assert!(puffs.len() >= 60, "{} dust spots", puffs.len());
    assert!(puffs.values().all(|n| (5..=14).contains(n)), "{puffs:?}");
    let total: usize = puffs.values().sum();
    assert!(grit * 5 > total && grit * 2 < total, "{grit} of {total}");
    assert!(creaks >= 20, "{creaks} creaks");
}

#[test]
fn the_air_goes_out_to_inspectors_and_stops_with_the_preview() {
    let mut game = game();
    let (_, rx) = observer(&mut game, 1);
    game.aftermath_start(1).unwrap();
    rx.try_iter().for_each(drop);
    let stand_ins: Vec<u64> = game.aftermath.as_ref().unwrap().air.dust.iter().map(|s| s.guid).collect();
    assert!(!stand_ins.is_empty());
    game.aftermath.as_mut().unwrap().air.started -= std::time::Duration::from_secs(30);
    game.tick();
    let sent: Vec<Packet> = rx.try_iter().collect();
    assert!(sent.iter().any(|p| p.opcode == op::SMSG_SPELL_GO && stand_ins.contains(&packed_guid(&p.body).0)), "the dust reaches the inspector");
    assert!(sent.iter().any(|p| p.opcode == op::SMSG_PLAY_OBJECT_SOUND));
    // A newcomer is sent the stand-ins with the ruins.
    let (_, late) = observer(&mut game, 2);
    late.try_iter().for_each(drop);
    game.send_aftermath_to(2);
    let units = late.try_iter().filter(|p| p.opcode == op::SMSG_UPDATE_OBJECT).count();
    assert!(units > stand_ins.len(), "{units} updates for {} stand-ins and the ruins", stand_ins.len());
    game.aftermath_command(1, "off", "").unwrap();
    let gone: Vec<u64> = rx.try_iter().filter(|p| p.opcode == op::SMSG_DESTROY_OBJECT)
        .map(|p| u64::from_le_bytes(p.body[0..8].try_into().unwrap())).collect();
    assert!(stand_ins.iter().all(|g| gone.contains(g)), "the stand-ins go with the preview");
    game.tick();
    assert!(!rx.try_iter().any(|p| p.opcode == op::SMSG_SPELL_GO || p.opcode == op::SMSG_PLAY_OBJECT_SOUND), "silent once restored");
}

#[test]
#[ignore = "requires the user's WoW 1.12.1 Data folder in WOW_DATA"]
fn native_ship_is_wrecked_not_deleted() {
    let n = native();
    let ground = native_ground(&n);
    let (props, _) = compose(&ground);
    let land = &n.land;
    let tops = |p: &Prop| world_tris(&n, p).iter().flatten().fold((f32::MAX, f32::MIN), |(lo, hi), v| (lo.min(v[2]), hi.max(v[2])));
    let at = |site: &str| props.iter().filter(|p| p.site == site).collect::<Vec<_>>();
    for p in &props { if p.site.contains("Juggernaut") || p.site.contains("Cannons") || p.site.contains("Wreckage") || p.site.contains("dock") || p.site.contains("anchor") { let (lo, hi) = tops(p); println!("  {} {} at {:.1},{:.1},{:.1}: {lo:.1}..{hi:.1}", p.site, MODELS[p.model].name, p.pos[0], p.pos[1], p.pos[2]); } }
    let solids = &n.survey.solids;
    let seen = |eye: V3, at: V3| { let d = shapes::sub(at, eye); solids.ray(eye, shapes::unit(d), shapes::len(d) - 0.5).is_none() };
    // In the landing's picture: a clear line to it, and within its cameras' view (north).
    let in_view = |v: V3| {
        let d = [v[0] - piles::LANDING_EYE[0], v[1] - piles::LANDING_EYE[1]];
        let off = (d[1].atan2(d[0]) - piles::LANDING_LOOK + 3.0 * PI).rem_euclid(2.0 * PI) - PI;
        off.abs() < 0.6 && d[0].hypot(d[1]) < 140.0 && seen(piles::LANDING_EYE, v)
    };
    let top = |p: &Prop| world_tris(&n, p).iter().flatten().copied().max_by(|a, b| a[2].total_cmp(&b[2])).unwrap();
    // The dock's cameras: the cove's view and the review's two beside it.
    let dock: Vec<V3> = std::iter::once(layout::REGIONS[6].view)
        .chain(layout::CAMERAS.iter().map(|(at, _)| *at).filter(|at| (at[0] - layout::REGIONS[6].view[0]).hypot(at[1] - layout::REGIONS[6].view[1]) < 15.0))
        .collect();
    assert!(dock.len() >= 3);
    let clear_of_dock = |p: &Prop| world_tris(&n, p).iter().flatten().all(|v| dock.iter().all(|c| (v[0] - c[0]).hypot(v[1] - c[1]) > piles::DOCK_CLEAR));
    // The hull, broken in two across the old berth: big, well out of the water, seen from the
    // landing, and clear of the dock's cameras and the walk.
    let hull = at("Juggernaut's broken hull");
    assert_eq!(hull.len(), 1, "the bow half");
    let (lo, hi) = tops(hull[0]);
    assert!(hull[0].scale >= 7.5 && lo < -12.0 && hi > 20.0, "big, on the seabed, reared high out of the water: {} {lo}..{hi}", hull[0].scale);
    let stern = at("Juggernaut's stern");
    assert_eq!(stern.len(), 1, "the stern half");
    assert!(stern[0].scale >= 4.5 && tops(stern[0]).1 > 8.0, "the stern half stands out of the water too");
    for p in hull.iter().chain(&stern) {
        let c = p.obb(0.0).c;
        assert!((-130.0..=-2.0).contains(&c[0]) && (-860.0..=-780.0).contains(&c[1]), "in the berth: {c:?}");
        assert!(in_view(top(p)), "{} seen from the landing", p.site);
        assert!(clear_of_dock(p), "{} clear of the dock's cameras", p.site);
        assert!(!ground.route().iter().any(|r| covers(p, *r, 0.6)), "{} clear of the walk", p.site);
    }
    // The mast out of the water, aslant, a plain wooden pole; its yard hanging into the water.
    let mast = at("Juggernaut's mast");
    assert_eq!(mast.len(), 2, "mast and yard");
    assert!(mast.iter().all(|p| p.model == MAST));
    let up = |p: &Prop| shapes::matrix(p.rotation)[2][2];
    assert!(tops(mast[0]).1 > 25.0 && (0.6..0.97).contains(&up(mast[0])), "the mast stands aslant high out of the water");
    assert!(tops(mast[1]).0 < piles::WATER && up(mast[1]) < 0.7, "the yard hangs into the water");
    assert!(in_view(top(mast[0])) && clear_of_dock(mast[0]));
    // Cannons at the water's edge under the landing (out of the water, in its view) and on the
    // seabed; the anchor standing in the shallows beside them.
    let shore = at("Cannons on the shore");
    assert!(shore.len() >= 2 && shore.iter().all(|p| tops(p).1 > piles::WATER + 1.5), "{} on the shore", shore.len());
    assert!(shore.iter().all(|p| in_view(top(p))), "the beached cannons are in the landing's view");
    assert!(at("Cannons on the seabed").len() >= 2);
    let anchor = at("The anchor");
    assert!(anchor.len() == 1 && tops(anchor[0]).1 > piles::WATER + 3.0 && in_view(top(anchor[0])), "the anchor stands in the landing's view");
    // Every piece of the ship stands with its pivot over the floor or seabed under it, none in it
    // (on film the mast footed on the seabed and the anchor seated under the beach never showed).
    for p in hull.iter().chain(&stern).chain(&mast).chain(&shore).chain(&anchor) {
        let fl = floor_under(land, p.pos);
        assert!(p.pos[2] > fl + 0.1, "{} {}: pivot {:.1} over the floor at {fl:.1}", p.site, MODELS[p.model].name, p.pos[2]);
    }
    // A field of floating wreckage from the hull to the dock, every piece across the waterline.
    let drift = at("Wreckage drifting to the dock");
    assert!(drift.len() >= 12, "{} pieces adrift", drift.len());
    for p in &drift {
        let (lo, hi) = tops(p);
        assert!(lo < piles::WATER && hi > piles::WATER, "{} rides the water: {lo}..{hi}", MODELS[p.model].name);
    }
    // The dock's snapped planks: from the dry dock down under the water.
    let planks = at("Smite's broken dock");
    assert!(planks.len() >= 2, "{} snapped planks", planks.len());
    for p in &planks { let (lo, hi) = tops(p); assert!(lo < piles::WATER && hi > 5.0, "{lo}..{hi}"); }
}

/// The cameraman follows the hero round the preview: its client is put where the hero inspects,
/// so its world is loaded there (it stayed at the landing and filmed the cove's water).
#[test]
fn the_cameraman_is_taken_where_the_hero_inspects() {
    let mut game = game();
    let (_, hero) = observer(&mut game, 1);
    let (_, cam) = observer(&mut game, 2);
    game.watch_command(2, "observer1").unwrap();
    game.aftermath_start(1).unwrap();
    game.aftermath_command(1, "visit", "mouth").unwrap();
    hero.try_iter().for_each(drop);
    cam.try_iter().for_each(drop);
    game.tick();
    let mouth = layout::REGIONS[0].view;
    let me = packed(game.world_of(2).unwrap().guid);
    let moved = cam.try_iter().filter(|p| p.opcode == op::MSG_MOVE_TELEPORT_ACK).any(|p| {
        let at = me.len() + 12;
        let pos: Vec<f32> = (0..3).map(|k| f32::from_le_bytes(p.body[at + 4 * k..at + 4 * k + 4].try_into().unwrap())).collect();
        p.body.starts_with(&me) && (pos[0] - mouth[0]).hypot(pos[1] - mouth[1]) < 3.0
    });
    assert!(moved, "the cameraman goes to the mouth with the hero");
    assert!(!hero.try_iter().any(|p| p.opcode == op::MSG_MOVE_HEARTBEAT), "no one is shown it");
    // Once there, it stays put while the hero looks round.
    game.tick();
    assert!(!cam.try_iter().any(|p| p.opcode == op::MSG_MOVE_TELEPORT_ACK));
}

/// A region's rock, measured as the review measures a pile: each rock's footprint (its model's
/// box across, scaled) times how far its turned box stands over the floor under it.
fn rock_volumes(land: &dyn Land, props: &[Prop]) -> Vec<f32> {
    let mut out = vec![0.0; layout::REGIONS.len()];
    for p in props.iter().filter(|p| rocky(p.model) && p.site != "Juggernaut collapse basin") {
        let m = &MODELS[p.model];
        let b = p.obb(0.0);
        let top = b.c[2] + (0..3).map(|i| b.a[i][2].abs() * b.h[i]).sum::<f32>();
        let fl = floor(land, [p.pos[0], p.pos[1], p.pos[2] + 1.0], true).unwrap_or(p.pos[2]);
        out[p.region] += (m.max[0] - m.min[0]) * (m.max[1] - m.min[1]) * p.scale * p.scale * (top - fl).max(0.0);
    }
    out
}

/// Every region's collapse reads at least as big as the old layout's in the same views (the
/// first rebuild's entrance was one cone smaller than the dome it replaced). The old layout
/// (main at e6eb2081, its boulders and rubble less the drowned ship's basin), measured the same
/// way on the installed map.
#[test]
#[ignore = "requires the user's WoW 1.12.1 Data folder in WOW_DATA"]
fn native_every_region_has_at_least_the_old_layouts_rock() {
    const OLD: [f32; 8] = [4052.0, 9487.0, 5870.0, 4514.0, 8146.0, 6597.0, 13675.0, 4869.0];
    let n = native();
    let ground = native_ground(&n);
    let (props, _) = compose(&ground);
    let new = rock_volumes(&n.land, &props);
    let mut short = Vec::new();
    for (i, r) in layout::REGIONS.iter().enumerate() {
        println!("{}: {:.0} (old {:.0})", r.name, new[i], OLD[i]);
        if new[i] < OLD[i] { short.push(r.name); }
    }
    assert!(short.is_empty(), "less rock than before in {short:?}");
}

/// The dust reaches whoever is looking, where they look: the hero and the cameraman following
/// them are both sent the stand-ins and, at the pile in front of the entrance's view, their
/// clouds (none showed on the second film: the cameraman never left the landing).
#[test]
fn the_dust_goes_out_where_the_hero_and_the_cameraman_look() {
    let mut game = game();
    let (_, hero) = observer(&mut game, 1);
    let (_, cam) = observer(&mut game, 2);
    game.watch_command(2, "observer1").unwrap();
    hero.try_iter().for_each(drop);
    cam.try_iter().for_each(drop);
    game.aftermath_start(1).unwrap();
    game.aftermath_command(1, "visit", "mouth").unwrap();
    let a = game.aftermath.as_ref().unwrap();
    let mouth = layout::REGIONS[0].view;
    let near: BTreeMap<u64, [f32; 3]> = a.air.dust.iter().filter(|s| (s.at[0] - mouth[0]).hypot(s.at[1] - mouth[1]) < 35.0).map(|s| (s.guid, s.at)).collect();
    assert!(near.len() >= 3, "{} dust spots by the entrance", near.len());
    // Every pile has its dust, the cove's and the landing's too.
    for r in 0..layout::REGIONS.len() {
        assert!(a.air.dust.iter().any(|s| a.props[s.prop].region == r), "dust in {}", layout::REGIONS[r].name);
    }
    game.aftermath.as_mut().unwrap().air.started -= std::time::Duration::from_secs(12);
    game.tick();
    for (who, rx) in [("hero", &hero), ("cameraman", &cam)] {
        let sent: Vec<Packet> = rx.try_iter().collect();
        let made: Vec<u64> = sent.iter().filter(|p| p.opcode == op::SMSG_UPDATE_OBJECT)
            .flat_map(|p| near.keys().copied().filter(|g| p.body.windows(8).any(|w| w == g.to_le_bytes()) || p.body.windows(packed(*g).len()).any(|w| w == packed(*g).as_slice())).collect::<Vec<_>>())
            .collect();
        assert!(near.keys().all(|g| made.contains(g)), "the {who} is sent the stand-ins by the entrance");
        let clouds = sent.iter().filter(|p| p.opcode == op::SMSG_SPELL_GO).filter(|p| {
            let (caster, rest) = packed_guid(&p.body);
            near.contains_key(&caster) && u32::from_le_bytes(rest[packed(caster).len()..packed(caster).len() + 4].try_into().unwrap()) == air::SPELL_DUST
        }).count();
        assert!(clouds >= 3, "the {who} sees {clouds} clouds at the entrance in its first seconds");
    }
    assert!(game.aftermath.as_ref().unwrap().air.puffs >= 3, "counted for `.es aftermath status`");
}
