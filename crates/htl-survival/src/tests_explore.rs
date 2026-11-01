//! Elwynn out past the camp, one rule each on places and chests made for the test: a place found
//! by walking there (the camp's for good, its worth into the chest), a shrine teaching each hero who
//! uses it (and standing for the next), a new character's own class cards, a champion teaching all
//! who fought it, a hero and the camp started over, a secret unseen until a hero comes near, an
//! ambush that wakes as its chest is opened, a chest shut until its guardian falls, one that fills
//! again after a while, a champion who teaches a card, a rare wanderer's spots and its news, a
//! harmless creature that runs, a creature far from everyone asleep, the new draughts, and a
//! save that keeps it all.

use crate::content::{ChestDef, Content, PackDef, SiteDef};
use crate::land::FlatLand;
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.05;

#[derive(serde::Deserialize, Default)]
struct Extra {
    #[serde(default)]
    site: Vec<SiteDef>,
    #[serde(default)]
    chest: Vec<ChestDef>,
    #[serde(default)]
    pack: Vec<PackDef>,
}

/// The built-in rules with only these places, chests and packs (TOML); cards owned only as earned
/// unless `full`.
fn world(extra: &str, full: bool) -> Survival {
    let mut c = Content::builtin();
    let e: Extra = toml::from_str(extra).expect("test places");
    c.sites = e.site;
    c.chests = e.chest;
    c.packs = e.pack;
    c.camps.clear();
    c.dev.full_tree = full;
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 7, 1_000_000.0);
    s.respawns.clear();
    s
}

fn hero(s: &mut Survival, name: &str, pos: Vec3) -> Id {
    let id = s.join(&name.to_lowercase(), name, None);
    s.move_player(id, pos, 0.0, Moving::default());
    let _ = s.step(0.0);
    id
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        out.extend(s.step(DT));
    }
    out
}

fn chest_obj(s: &Survival, key: &str) -> Option<Id> {
    let def = s.content.chests.iter().position(|c| c.key == key)?;
    s.objs.iter().find(|(_, o)| matches!(o.kind, ObjKind::Chest { def: d, .. } if d == def)).map(|(id, _)| *id)
}

/// Stand at a chest and open it, the work and all.
fn open(s: &mut Survival, p: Id, key: &str) -> Vec<Event> {
    let c = s.content.chests.iter().find(|c| c.key == key).unwrap().clone();
    s.move_player(p, [c.at[0] - 1.5, c.at[1], 0.0], 0.0, Moving::default());
    let o = chest_obj(s, key).expect("the chest is there");
    s.interact(p, Some(o));
    run(s, c.secs + 0.3)
}

const SHRINE: &str = r#"
[[chest]]
key = "shrine"
name = "Test Shrine"
at = [30.0, 0.0]
keep = true
secs = 1.0
teaches = ["fireball", "cone_of_cold"]
"#;

#[test]
fn a_place_is_found_by_walking_there_and_is_the_camps_for_good() {
    let mut s = world(
        r#"
[[site]]
key = "tower"
name = "A Tower"
at = [100.0, 0.0]
radius = 20.0
kind = "landmark"
reward = { wood = 3 }
"#,
        true,
    );
    let wood = s.content.item("wood").unwrap();
    let p = hero(&mut s, "Ayla", [0.0, 0.0, 0.0]);
    run(&mut s, 0.5);
    assert!(s.camp.found.is_empty(), "not yet: too far");
    s.move_player(p, [85.0, 0.0, 0.0], 0.0, Moving::default());
    let ev = run(&mut s, 0.2);
    assert!(s.camp.found.contains("tower"));
    assert_eq!(s.camp.counts.get("explore:any"), Some(&1));
    assert_eq!(s.storage.get(&wood), Some(&3), "its worth into the chest");
    assert!(ev.iter().any(|e| matches!(e, Event::Banner { title, .. } if title.contains("A Tower"))));
    // Once only.
    s.move_player(p, [0.0, 0.0, 0.0], 0.0, Moving::default());
    run(&mut s, 0.2);
    s.move_player(p, [100.0, 0.0, 0.0], 0.0, Moving::default());
    run(&mut s, 0.2);
    assert_eq!(s.camp.counts.get("explore:any"), Some(&1));
    assert_eq!(s.storage.get(&wood), Some(&3));
    assert_eq!(s.sites_view().iter().filter(|(_, f)| *f).count(), 1);
}

#[test]
fn a_shrine_teaches_each_hero_who_uses_it_and_stands_for_the_next() {
    let mut s = world(SHRINE, false);
    let a = hero(&mut s, "Ayla", [0.0, 0.0, 0.0]);
    let b = hero(&mut s, "Bren", [0.0, 5.0, 0.0]);
    assert!(!s.players[&a].owned.contains("fireball"), "not a Warrior's card");
    let ev = open(&mut s, a, "shrine");
    assert!(s.players[&a].owned.contains("fireball") && s.players[&a].owned.contains("cone_of_cold"), "the one who used it");
    assert!(!s.players[&b].owned.contains("fireball"), "each hero learns for themselves");
    assert!(ev.iter().any(|e| matches!(e, Event::Banner { player: Some(p), title, .. } if *p == a && title.starts_with("Learned"))));
    assert_eq!(s.camp.counts.get("learn:any"), Some(&2));
    // Still there for the next hero.
    let o = chest_obj(&s, "shrine").expect("a shrine stays");
    assert_eq!(s.objects().iter().find(|v| v.id == o).map(|v| v.usable), Some(true));
    open(&mut s, b, "shrine");
    assert!(s.players[&b].owned.contains("fireball"));
    assert!(s.slot_ability(b, "ranged", "fireball").is_ok());
    // Once is enough: nothing more to learn there.
    let ev = open(&mut s, a, "shrine");
    assert!(ev.iter().any(|e| matches!(e, Event::Tell { text, .. } if text.starts_with("You know all"))));
    assert_eq!(s.camp.counts.get("learn:any"), Some(&4));
    // A hero who comes later hasn't learned it.
    let c = hero(&mut s, "Cass", [0.0, -5.0, 0.0]);
    assert!(!s.players[&c].owned.contains("fireball"));
}

#[test]
fn a_new_character_knows_its_own_classs_q_and_e_on_those_keys() {
    let mut s = world("", false);
    let look = HeroLook { race: 4, gender: 1, skin: 2, face: 3, hair: 4, hair_color: 5, facial: 1 };
    let save = s.fresh_save("ayla", "Thalia", "druid", look);
    let p = s.join("ayla/thalia", "Thalia", Some(&save));
    let h = &s.players[&p];
    // Its column's Q and E, on those keys, and nothing else yet.
    for a in s.content.abilities.iter().filter(|a| a.on_board() && a.school == "druid") {
        let first = a.slot == "ranged" || a.slot == "close";
        assert_eq!(h.owned.contains(&a.key), first, "{} ({})", a.key, a.slot);
        assert_eq!(h.loadout.get(&a.slot) == Some(&a.key), first, "{} on its key", a.key);
    }
    assert_eq!(h.owned.len(), 2, "{:?}", h.owned);
    assert_eq!(h.loadout.len(), 2, "the other keys empty: {:?}", h.loadout);
    assert_eq!((h.class.as_str(), h.look, h.owner.as_str()), ("druid", look, "ayla"));
    // Kept so in a save.
    let back = s.leave(p).unwrap();
    assert_eq!((back.class.as_str(), back.look, back.name.as_str(), back.owner.as_str()), ("druid", Some(look), "Thalia", "ayla"));
    let q = s.join("ayla/thalia", "Thalia", Some(&back));
    assert!(s.players[&q].owned.contains("wrath"));
}

#[test]
fn a_champion_teaches_everyone_who_fought_it() {
    let mut s = world(
        r#"
[[pack]]
mob = "morgan"
at = [10.0, 0.0]
radius = 0.0
count = 1
"#,
        false,
    );
    let a = hero(&mut s, "Ayla", [0.0, 0.0, 0.0]);
    let b = hero(&mut s, "Bren", [20.0, 10.0, 0.0]);
    let c = hero(&mut s, "Cass", [300.0, 0.0, 0.0]);
    let m = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "morgan").unwrap().0;
    let hp = s.mobs[&m].health;
    s.hurt_mob(m, a, hp);
    assert!(s.players[&a].owned.contains("death_mark"));
    assert!(s.players[&b].owned.contains("death_mark"), "near it as it fell");
    assert!(!s.players[&c].owned.contains("death_mark"), "far off");
}

#[test]
fn a_hero_starts_over_and_so_does_the_camp() {
    let mut s = world(
        &format!(
            r#"{SHRINE}
[[site]]
key = "spot"
name = "A Spot"
at = [0.0, 0.0]
radius = 10.0
kind = "landmark"

[[pack]]
mob = "wolf"
at = [60.0, 0.0]
radius = 0.0
count = 1
"#
        ),
        false,
    );
    let wood = s.content.item("wood").unwrap();
    let p = hero(&mut s, "Ayla", [0.0, 0.0, 0.0]);
    run(&mut s, 0.2);
    open(&mut s, p, "shrine");
    s.players.get_mut(&p).unwrap().bag.insert(wood, 9);
    s.set_valor(p, 80.0);
    s.storage.insert(wood, 30);
    s.camp.raids_won = 2;
    // The hero: back to the first day, still Ayla the Warrior.
    s.reset_hero(p).unwrap();
    let h = &s.players[&p];
    assert!(!h.owned.contains("fireball") && h.owned.contains("aimed_throw"));
    assert_eq!(h.bag.get(&wood), None);
    assert_eq!(h.name, "Ayla");
    assert!(h.valor < 80.0);
    assert!(s.camp.found.contains("spot"), "the camp is untouched");
    // The camp: the chest empty, nothing found, no raid beaten; the creatures back.
    let wolf = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "wolf").unwrap().0;
    s.mobs.remove(&wolf);
    let ev = {
        s.reset_camp().unwrap();
        s.step(0.0)
    };
    assert!(s.camp.found.is_empty(), "nothing found (the hero is at the camp's spawn now)");
    assert_eq!(s.camp.raids_won, 0);
    assert_eq!(s.storage.get(&wood), None);
    assert!(s.mobs.values().any(|m| s.content.mobs[m.def].key == "wolf"), "the wolf is back");
    assert!(ev.iter().any(|e| matches!(e, Event::ObjSpawned { .. })), "the new world drawn");
    assert!(s.objs.values().any(|o| o.kind == ObjKind::Storage));
    assert!(s.players.contains_key(&p), "the hero stays");
}

#[test]
fn a_secret_is_unseen_until_a_hero_comes_near() {
    let mut s = world(
        r#"
[[chest]]
key = "stash"
name = "A Stash"
at = [50.0, 0.0]
secret = 6.0
secs = 0.5
loot = { silver = 2 }
"#,
        true,
    );
    let silver = s.content.item("silver").unwrap();
    let p = hero(&mut s, "Ayla", [0.0, 0.0, 0.0]);
    run(&mut s, 0.5);
    assert!(chest_obj(&s, "stash").is_none(), "hidden");
    s.move_player(p, [45.0, 0.0, 0.0], 0.0, Moving::default());
    let ev = run(&mut s, 0.2);
    assert!(chest_obj(&s, "stash").is_some(), "come upon");
    assert!(!ev.iter().any(|e| matches!(e, Event::Banner { .. })), "quietly: no banner and horn");
    assert_eq!(s.camp.counts.get("secret:any"), Some(&1));
    open(&mut s, p, "stash");
    assert_eq!(s.players[&p].bag.get(&silver), Some(&2));
    assert!(chest_obj(&s, "stash").is_none(), "once a world");
    assert_eq!(s.chests_tally(), (0, 0, 1, 1));
}

#[test]
fn an_ambush_wakes_as_its_chest_is_opened_and_one_lies_in_wait_on_the_road() {
    let mut s = world(
        r#"
[[chest]]
key = "box"
name = "A Box"
at = [20.0, 0.0]
secs = 0.5
ambush = ["guards"]
loot = { wool = 1 }

[[pack]]
mob = "defias_thug"
at = [24.0, 0.0]
radius = 2.0
count = 2
link = 8.0
key = "guards"
wake = "box"

[[pack]]
mob = "gnoll_runt"
at = [0.0, 60.0]
radius = 1.0
count = 1
hidden = 8.0
"#,
        true,
    );
    let p = hero(&mut s, "Ayla", [0.0, 0.0, 0.0]);
    run(&mut s, 0.5);
    let hidden = |s: &Survival| s.mobs.values().filter(|m| m.hidden).count();
    assert_eq!(hidden(&s), 3, "all lying in wait");
    let ev = open(&mut s, p, "box");
    assert!(ev.iter().filter(|e| matches!(e, Event::MobSpawned { .. })).count() >= 2, "the guards step out");
    let on_me = s.mobs.values().filter(|m| !m.hidden && m.state.foe() == Some(mobs::Foe::Hero(p))).count();
    assert_eq!(on_me, 2, "on the opener at once");
    // The one on the road waits until a hero comes near.
    s.move_player(p, [0.0, 40.0, 0.0], 0.0, Moving::default());
    run(&mut s, 0.2);
    assert_eq!(hidden(&s), 1);
    s.move_player(p, [0.0, 54.0, 0.0], 0.0, Moving::default());
    run(&mut s, 0.2);
    assert_eq!(hidden(&s), 0, "it steps out");
}

#[test]
fn a_chest_is_shut_until_its_guardian_falls_and_fills_again_in_time() {
    let mut s = world(
        r#"
[[chest]]
key = "armory"
name = "An Armory"
at = [20.0, 0.0]
secs = 0.5
after = "hogger"
locked = "Barred."
respawn = 60.0
loot = { bronze = 2 }
"#,
        true,
    );
    let bronze = s.content.item("bronze").unwrap();
    let p = hero(&mut s, "Ayla", [0.0, 0.0, 0.0]);
    let ev = open(&mut s, p, "armory");
    assert!(ev.iter().any(|e| matches!(e, Event::Tell { text, .. } if text == "Barred.")));
    assert!(chest_obj(&s, "armory").is_some(), "still shut");
    s.camp.counts.insert("kill:hogger".into(), 1);
    open(&mut s, p, "armory");
    assert_eq!(s.players[&p].bag.get(&bronze), Some(&2));
    assert!(chest_obj(&s, "armory").is_none());
    // Not while the hero stands over it; full again once they're away and the time is up.
    run(&mut s, 70.0);
    assert!(chest_obj(&s, "armory").is_none(), "not under a hero's nose");
    s.move_player(p, [200.0, 0.0, 0.0], 0.0, Moving::default());
    run(&mut s, 6.0);
    assert!(chest_obj(&s, "armory").is_some(), "full again");
}

#[test]
fn a_champion_teaches_its_card_and_a_rare_comes_at_its_spots() {
    let mut s = world(
        r#"
[[pack]]
mob = "morgan"
at = [10.0, 0.0]
radius = 0.0
count = 1

[[pack]]
mob = "hogger"
at = [0.0, 0.0]
radius = 0.0
count = 1
spots = [[300.0, 0.0], [0.0, 300.0]]
rare = true
"#,
        false,
    );
    let p = hero(&mut s, "Ayla", [0.0, 0.0, 0.0]);
    let at = |s: &Survival, key: &str| s.mobs.values().find(|m| s.content.mobs[m.def].key == key).map(|m| m.pos).unwrap();
    let h = at(&s, "hogger");
    assert!(dist2d(h, [300.0, 0.0, 0.0]) < 1.0 || dist2d(h, [0.0, 300.0, 0.0]) < 1.0, "at one of its spots, not at its `at`");
    // Morgan falls: the camp learns the Death Mark.
    let morgan = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "morgan").unwrap().0;
    assert!(!s.players[&p].owned.contains("death_mark"));
    let hp = s.mobs[&morgan].health;
    s.hurt_mob(morgan, p, hp);
    assert!(s.players[&p].owned.contains("death_mark"));
    // Hogger's fall is news.
    let hogger = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "hogger").unwrap().0;
    let hp = s.mobs[&hogger].health;
    s.hurt_mob(hogger, p, hp);
    let ev = s.step(DT);
    assert!(ev.iter().any(|e| matches!(e, Event::Announce { text } if text.starts_with("Hogger has fallen to Ayla"))));
    // He comes back at one of his spots.
    let r = s.respawns.iter().find(|r| s.content.mobs[r.0].key == "hogger").cloned().unwrap();
    s.respawns.retain(|x| s.content.mobs[x.0].key != "hogger");
    s.respawns.push((r.0, r.1, r.2, s.clock));
    s.move_player(p, [-500.0, 0.0, 0.0], 0.0, Moving::default());
    let ev = run(&mut s, 0.2);
    let h = s.mobs.values().filter(|m| m.health > 0).find(|m| s.content.mobs[m.def].key == "hogger").unwrap().pos;
    assert!(dist2d(h, [300.0, 0.0, 0.0]) < 1.0 || dist2d(h, [0.0, 300.0, 0.0]) < 1.0);
    assert!(ev.iter().any(|e| matches!(e, Event::Announce { text } if text.starts_with("Hogger has been seen again"))), "his return is news");
}

#[test]
fn a_harmless_creature_runs_and_one_far_from_everyone_sleeps() {
    let mut s = world(
        r#"
[[pack]]
mob = "rabbit"
at = [5.0, 0.0]
radius = 0.0
count = 1

[[pack]]
mob = "wolf"
at = [400.0, 0.0]
radius = 0.0
count = 1
"#,
        true,
    );
    let p = hero(&mut s, "Ayla", [0.0, 0.0, 0.0]);
    let rabbit = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "rabbit").unwrap().0;
    s.hurt_mob(rabbit, p, 1);
    run(&mut s, 2.0);
    let r = &s.mobs[&rabbit];
    assert!(r.state.foe().is_none(), "it never turns on you");
    assert!(dist2d(r.pos, [0.0, 0.0, 0.0]) > 10.0, "it ran");
    // The wolf far off never wanders: it sleeps.
    let wolf = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "wolf").unwrap().0;
    let was = s.mobs[&wolf].pos;
    run(&mut s, 20.0);
    assert!(dist2d(s.mobs[&wolf].pos, was) < 0.01 && matches!(s.mobs[&wolf].state, MobState::Idle { .. }));
}

#[test]
fn clarity_shortens_cooldowns_and_a_poultice_softens_blows() {
    let mut s = world("", true);
    let p = hero(&mut s, "Ayla", [0.0, 0.0, 0.0]);
    let before = s.armor_reduce(&s.players[&p]);
    for k in ["clarity", "poultice"] {
        let i = s.content.item(k).unwrap();
        s.players.get_mut(&p).unwrap().bag.insert(i, 1);
        s.use_item(p, i);
        run(&mut s, 2.5);
    }
    let after = s.armor_reduce(&s.players[&p]);
    assert!((after - before - 0.12).abs() < 1e-3, "an eighth more off every blow: {before} -> {after}");
    // A card's cooldown, a quarter shorter.
    s.slot_ability(p, "ranged", "fireball").unwrap();
    let k = s.content.ability("fireball").unwrap();
    let full = s.content.abilities[k].cooldown;
    s.input(p, Input::Ability(0, false));
    run(&mut s, 0.1);
    if matches!(s.players[&p].action, Action::Aim { .. }) {
        s.input(p, Input::Attack);
        run(&mut s, 0.2);
        s.input(p, Input::AttackUp);
    }
    run(&mut s, 1.5);
    let cd = s.cooldown(p, k);
    let left = cd.next_at - s.clock;
    assert!(cd.charges < s.content.abilities[k].charges.max(1), "spent");
    assert!(left < full * 0.8, "{left} of {full}: shorter");
}

#[test]
fn a_save_keeps_what_was_found_opened_and_learned() {
    let mut s = world(
        &format!(
            r#"{SHRINE}
[[chest]]
key = "box"
name = "A Box"
at = [-20.0, 0.0]
secs = 0.5
loot = {{ wool = 1 }}

[[site]]
key = "spot"
name = "A Spot"
at = [0.0, 0.0]
radius = 10.0
kind = "landmark"
"#
        ),
        false,
    );
    let p = hero(&mut s, "Ayla", [0.0, 0.0, 0.0]);
    run(&mut s, 0.2);
    open(&mut s, p, "shrine");
    open(&mut s, p, "box");
    let save = s.save(&Default::default());
    let text = crate::save::to_toml(&save).unwrap();
    let back = crate::save::from_toml(&text).unwrap();
    let c = s.content.clone();
    let mut t = Survival::load(c, Box::new(FlatLand::default()), 9, 1_000_100.0, &back);
    assert!(t.camp.found.contains("spot"));
    assert!(chest_obj(&t, "box").is_none(), "opened stays opened");
    let sh = chest_obj(&t, "shrine").expect("the shrine stands");
    assert!(matches!(t.objs[&sh].kind, ObjKind::Chest { spent: false, .. }), "for the next hero");
    let q = t.join("ayla", "Ayla", back.players.get("ayla"));
    assert!(t.players[&q].owned.contains("cone_of_cold"), "what she learned is hers");
}

#[test]
fn resources_spread_across_the_whole_land_beside_trees_or_in_the_open_clear_of_the_camp() {
    let mut c = Content::builtin();
    c.camps.clear();
    let mut wood = c.nodes.iter().find(|n| n.id() == "timber_wild").cloned().expect("spread timber");
    let mut bloom = c.nodes.iter().find(|n| n.id() == "peacebloom_wild").cloned().expect("spread peacebloom");
    assert!(wood.by_trees && wood.spread > 0.0 && bloom.spread > 0.0 && !bloom.by_trees);
    (wood.spread, wood.count, wood.spacing) = (100.0, 40, 30.0);
    (bloom.spread, bloom.count, bloom.spacing) = (100.0, 40, 30.0);
    c.nodes = vec![wood, bloom];
    // A land 800 yards a side round the camp, a tree every 50 yards.
    let h = c.world.heart;
    let (lo, hi) = ([h[0] - 400.0, h[1] - 400.0], [h[0] + 400.0, h[1] + 400.0]);
    let mut trees = Vec::new();
    for i in 0..16 {
        for j in 0..16 {
            trees.push([lo[0] + 25.0 + i as f32 * 50.0, lo[1] + 25.0 + j as f32 * 50.0, 0.0]);
        }
    }
    // Each trunk solid, a yard and a half round.
    let walls: Vec<([f32; 2], f32)> = trees.iter().map(|t| ([t[0], t[1]], 1.5)).collect();
    let land = FlatLand { trees: trees.clone(), walls, bounds: Some((lo, hi)), ..Default::default() };
    let s = Survival::new(c.clone(), Box::new(land), 7, 1_000_000.0);
    let of = |def: usize| -> Vec<Vec3> { s.objs.values().filter(|o| matches!(o.kind, ObjKind::Node { def: d, .. } if d == def)).map(|o| o.pos).collect() };
    let (w, b) = (of(0), of(1));
    assert!(w.len() >= 30 && b.len() >= 30, "{} timber, {} peacebloom", w.len(), b.len());
    // In every quarter of the land, not bunched in one.
    for (ws, name) in [(&w, "timber"), (&b, "peacebloom")] {
        for (qx, qy) in [(false, false), (false, true), (true, false), (true, true)] {
            let n = ws.iter().filter(|p| (p[0] > h[0]) == qx && (p[1] > h[1]) == qy).count();
            assert!(n >= 5, "{name}: {n} in quarter {qx},{qy}");
        }
        // Clear of the camp's building ground, and apart.
        let bc = c.world.build_center;
        assert!(ws.iter().all(|p| dist2d(*p, [bc[0], bc[1], 0.0]) > c.world.build_radius), "{name} on the building ground");
        for (k, p) in ws.iter().enumerate() {
            assert!(ws[k + 1..].iter().all(|q| dist2d(*p, *q) >= 30.0), "{name} too close");
        }
    }
    // The timber beside a trunk, not in it: room to stand round every node.
    assert!(w.iter().all(|p| trees.iter().any(|t| dist2d(*p, *t) < 4.0)), "timber by the trees");
    for p in w.iter().chain(&b) {
        assert!(trees.iter().all(|t| dist2d(*p, *t) > 1.5 + 1.2), "in a trunk: {p:?}");
    }
    // On a land with no edge, a spread kind lays none.
    let flat = Survival::new(c, Box::new(FlatLand::default()), 7, 1_000_000.0);
    assert!(!flat.objs.values().any(|o| matches!(o.kind, ObjKind::Node { .. })));
}

#[test]
fn every_foe_is_tougher_by_the_one_share_but_a_critter_or_a_dummy_isnt() {
    let mut s = world("", false);
    assert!((s.content.combat.foe_health - 1.6).abs() < 1e-6, "the rules' share");
    for key in ["boar", "grunt"] {
        let def = s.content.mob(key).unwrap();
        let id = s.spawn_mob(def, [0.0, 0.0, 0.0], None, false);
        let want = (s.content.mobs[def].health as f32 * 1.6).round() as u32;
        assert_eq!((s.mobs[&id].health, s.mobs[&id].max_health), (want, want), "{key}");
    }
    let critter = s.content.mobs.iter().position(|m| m.critter).expect("a critter");
    let id = s.spawn_mob(critter, [0.0, 0.0, 0.0], None, false);
    assert_eq!(s.mobs[&id].max_health, s.content.mobs[critter].health, "a critter as it is");
    let dummy = s.content.mobs.iter().position(|m| m.dummy).expect("a dummy");
    let id = s.spawn_mob(dummy, [0.0, 0.0, 0.0], None, false);
    assert_eq!(s.mobs[&id].max_health, s.content.mobs[dummy].health, "a dummy as it is");
}

#[test]
fn roaming_groups_spread_over_the_land_clear_of_the_camp() {
    let mut c = Content::builtin();
    c.camps.clear();
    c.nodes.clear();
    c.packs.clear();
    c.roams = vec![crate::content::RoamDef { mobs: vec!["wolf".into(), "boar".into()], size: [2, 3], groups: 12, spread: 60.0, spacing: 60.0, radius: 4.0, clear_of_camp: 100.0, by_trees: false }];
    let h = c.world.heart;
    let land = FlatLand { bounds: Some(([h[0] - 400.0, h[1] - 400.0], [h[0] + 400.0, h[1] + 400.0])), ..Default::default() };
    let s = Survival::new(c, Box::new(land), 7, 1_000_000.0);
    assert_eq!(s.content.packs.len(), 12, "every group a pack");
    for (k, p) in s.content.packs.iter().enumerate() {
        assert_eq!(p.mob, if k % 2 == 0 { "wolf" } else { "boar" });
        assert!((2..=3).contains(&p.count));
        assert!((p.at[0] - h[0]).hypot(p.at[1] - h[1]) > 100.0, "clear of the camp");
    }
    let wolves = s.mobs.values().filter(|m| s.content.mobs[m.def].key == "wolf").count();
    assert!(wolves >= 12, "{wolves} wolves out there");
}

const TRIAL: &str = r#"
[[chest]]
key = "shrine"
name = "Test Shrine"
at = [30.0, 0.0]
keep = true
secs = 1.0
teaches = ["fireball", "cone_of_cold"]
loot = { pearl = 1 }
trial = ["guardians"]

[[pack]]
mob = "boar"
at = [36.0, 0.0]
radius = 2.0
count = 2
link = 8.0
key = "guardians"
wake = "shrine"
"#;

fn guardians(s: &Survival) -> Vec<Id> {
    let k = s.content.packs.iter().position(|p| p.key == "guardians").unwrap();
    s.mobs.iter().filter(|(_, m)| m.pack == Some(k) && m.health > 0 && !m.hidden).map(|(id, _)| *id).collect()
}

#[test]
fn a_shrine_teaches_only_once_its_trial_is_passed_and_everyone_near_learns() {
    let mut s = world(TRIAL, false);
    let a = hero(&mut s, "Ayla", [0.0, 0.0, 0.0]);
    let b = hero(&mut s, "Bren", [28.0, 5.0, 0.0]);
    assert!(guardians(&s).is_empty(), "the guardians wait unseen");
    let ev = open(&mut s, a, "shrine");
    assert!(!s.players[&a].owned.contains("fireball"), "nothing learned yet");
    let g = guardians(&s);
    assert_eq!(g.len(), 2, "its guardians come");
    assert!(ev.iter().any(|e| matches!(e, Event::Banner { title, .. } if title.starts_with("The Trial of"))));
    // Using it again meanwhile: no more of them.
    open(&mut s, b, "shrine");
    assert_eq!(guardians(&s).len(), 2, "one trial at a time");
    // Down they go: both heroes there learn, the first finder gets what's in it.
    for m in g {
        s.hurt_mob(m, a, 1_000_000);
    }
    let ev = run(&mut s, 0.3);
    assert!(s.players[&a].owned.contains("fireball") && s.players[&b].owned.contains("fireball"), "everyone near learns");
    assert!(ev.iter().any(|e| matches!(e, Event::Banner { title, .. } if title == "Trial passed")));
    let pearl = s.content.item("pearl").unwrap();
    assert_eq!(s.players[&a].bag.get(&pearl), Some(&1));
    // A hero who comes later calls it up again, the fallen guardians standing at once.
    let c = hero(&mut s, "Cass", [0.0, -5.0, 0.0]);
    open(&mut s, c, "shrine");
    assert_eq!(guardians(&s).len(), 2, "the trial again, for Cass");
    // Walked away from: given up, the guardians back to waiting, nothing learned.
    for h in [a, b, c] {
        s.move_player(h, [-200.0, 0.0, 0.0], 0.0, Moving::default());
    }
    run(&mut s, 14.0);
    assert!(guardians(&s).is_empty(), "back to waiting");
    assert!(!s.players[&c].owned.contains("fireball"));
    // Known already: no trial at all.
    let ev = open(&mut s, a, "shrine");
    assert!(ev.iter().any(|e| matches!(e, Event::Tell { text, .. } if text.starts_with("You know all"))));
    assert!(guardians(&s).is_empty());
}
