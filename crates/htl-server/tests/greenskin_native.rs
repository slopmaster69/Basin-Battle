//! Captain Greenskin's harpoon and volleys on the real ship's deck; no game assets stored here.
use htl_server::survival::land::DeadminesLand;
use htl_survival::{dist2d, Content, Event, Id, Moving, Survival};

const DT: f32 = 0.05;

fn greenskin(s: &Survival) -> Option<Id> {
    s.mobs.iter().find(|(_, m)| m.health > 0 && s.content.mobs[m.def].key == "dm_greenskin").map(|(id, _)| *id)
}

/// Heroes stood at many places on the deck (the near end, the far ends of the room, against the
/// rail) take every harpoon haul and volley of a minute without being set down off the deck, in
/// a wall, in the water or out of his reach; and the last hero leaving puts him back at once.
#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn a_hauled_hero_always_lands_on_the_deck_in_reach_and_leaving_puts_the_fight_back() {
    let data = std::env::var("WOW_DATA").expect("set WOW_DATA to installed Data directory");
    let (mut hauls, mut starts) = (0, 0);
    // Where each hero begins, from his home: along and across the deck, near the rail and the far posts.
    let spots: [[f32; 2]; 9] = [[-8.0, 0.0], [-10.0, 6.0], [-10.0, -6.0], [-4.0, 12.0], [-4.0, -12.0], [-14.0, 0.0], [8.0, 0.0], [0.0, 14.0], [0.0, -14.0]];
    for spot in spots {
        let land = DeadminesLand::new(htl_map::deadmines::load(std::path::Path::new(&data)).unwrap());
        let mut c = Content::builtin();
        c.dungeon.as_mut().unwrap().packs.retain(|p| p.key == "greenskin");
        c.dungeon.as_mut().unwrap().doors.clear();
        let mut s = Survival::new_dungeon(c, Box::new(land), 17, 0.0);
        let hero = s.join("hauled", "Hauled", None);
        s.players.get_mut(&hero).unwrap().loadout.clear();
        let far = s.land().footing([-64.0, -835.5, 41.5]);
        let Some(far) = far else { eprintln!("no far"); continue };
        s.players.get_mut(&hero).unwrap().last_safe = far;
        s.move_player(hero, far, 0.0, Moving::default());
        for _ in 0..20 { s.step(DT); }
        let Some(boss) = greenskin(&s) else { eprintln!("no boss"); continue };
        let home = s.mobs[&boss].home;
        let Some(at) = s.land().footing([home[0] + spot[0], home[1] + spot[1], home[2]]) else { eprintln!("no floor at {spot:?}"); continue };
        starts += 1;
        s.players.get_mut(&hero).unwrap().pos = at;
        // (Within his sight, he comes for them as soon as they are set down.)
        let start = s.clock;
        let mut last = at;
        while s.clock - start < 60.0 {
            let p = s.players.get_mut(&hero).unwrap();
            p.health = p.max_health;
            let mob = s.mobs.get_mut(&boss).unwrap();
            mob.health = mob.max_health;
            let ev = s.step(DT);
            for e in &ev {
                if let Event::Hop { player, .. } = e { if *player == hero { hauls += 1; } }
            }
            let now = s.players[&hero].pos;
            let on_deck = s.land().footing(now).is_some() || s.land().footing([now[0], now[1], last[2]]).is_some();
            assert!(on_deck, "spot {spot:?} at {:.1}: set down at {now:?}, off the deck", s.clock - start);
            assert!((now[2] - home[2]).abs() < 3.0, "spot {spot:?}: off the ship's floor at {now:?}");
            assert!(!s.through_rail(last, now), "spot {spot:?}: carried over the rail {last:?} -> {now:?}");
            assert!(dist2d(now, home) < 30.0, "spot {spot:?}: out of reach at {now:?}");
            last = now;
            // The client's report: where they are, standing.
            s.move_player(hero, now, 0.0, Moving::default());
        }
        // The last hero leaves mid-fight: him whole at his post, no crew, nothing in the air.
        let _ = s.leave(hero);
        let m = &s.mobs[&boss];
        assert_eq!(m.health, m.max_health);
        assert!(dist2d(m.pos, m.home) < 0.5, "home at once");
    }
    assert!(starts >= 4, "{starts} spots checked");
    assert!(hauls >= 1, "the sweep saw {hauls} hauls");
}
