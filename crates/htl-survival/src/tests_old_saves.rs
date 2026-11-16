//! Older saves still load: each file in `testdata/old_saves/` is a save shape written by hand from
//! an earlier version's structs (never a real save), read with today's code. Nothing may panic and
//! nothing the old save held (gear, cards, gold, progress) may be lost, bar what's since been
//! retired, which comes back as its successor.

use std::collections::BTreeMap;

use crate::content::Content;
use crate::save::{from_toml, WorldSave};
use crate::sim::*;
use crate::FlatLand;

const V01: &str = include_str!("../testdata/old_saves/v0_1_footman.toml");
const V04: &str = include_str!("../testdata/old_saves/v0_4_warrior_retired_gear.toml");
const PRIEST: &str = include_str!("../testdata/old_saves/v0_4_priest_old_cards.toml");
const ROGUE: &str = include_str!("../testdata/old_saves/v0_4_rogue_shield_old_cards.toml");
const ONE_FIST: &str = include_str!("../testdata/old_saves/v0_5_rogue_one_fist.toml");

/// Today's rules with the tree not handed out whole, so what a save owned is what a hero owns.
fn rules() -> Content {
    let mut c = Content::dev();
    c.dev.full_tree = false;
    c
}

fn load(text: &str) -> (Survival, WorldSave) {
    let save = from_toml(text).expect("an old save parses");
    let s = Survival::load(rules(), Box::new(FlatLand::default()), 3, 1_000_000.0, &save);
    (s, save)
}

fn key(s: &Survival, i: usize) -> String {
    s.content.items[i].key.clone()
}

fn bag_of(s: &Survival, p: Id) -> BTreeMap<String, u32> {
    s.players[&p].bag.iter().map(|(i, n)| (key(s, *i), *n)).collect()
}

#[test]
fn the_first_release_shape_loads_with_everything_it_held() {
    // It still has the journal's goal, from before the journal was taken out: read past.
    assert!(V01.lines().any(|l| l.trim() == "goal = 5"));
    let (mut s, save) = load(V01);
    assert_eq!(save.raids_won, 2);
    assert_eq!(s.camp.raids_won, 2);
    let wood = s.content.item("wood").unwrap();
    assert_eq!(s.storage[&wood], 9);
    assert_eq!(s.storage[&s.content.item("ore").unwrap()], 4);
    assert_eq!(s.objs.values().filter(|o| matches!(o.kind, ObjKind::Station { damaged: true, .. })).count(), 1);
    assert_eq!(s.objs.values().filter(|o| matches!(o.kind, ObjKind::Node { ready: false, .. })).count(), 1, "the spent node stays spent");
    assert_eq!(s.caches_of("bren").len(), 1);
    assert!(s.next_uid >= 7);
    // The raid waits, with all three raiders.
    assert!(s.raid.active() && s.raid.queue.len() == 3, "{:?}", s.raid.queue);
    // The heroes: where and as hurt as they were; a class they never had is the Warrior.
    let a = s.join("ayla", "Ayla", save.players.get("ayla"));
    let h = &s.players[&a];
    assert_eq!((h.health, h.class.as_str()), (77, "warrior"));
    assert_eq!(key(&s, h.weapon), "sword");
    assert_eq!(bag_of(&s, a)["ore"], 3);
    assert_eq!(bag_of(&s, a)["wood"], 2);
    assert!(!s.players[&a].owned.is_empty(), "a hero with no saved cards starts with their class's first");
    let b = s.join("bren", "Bren", save.players.get("bren"));
    assert!(matches!(s.players[&b].action, Action::Dead { .. }), "one who left down gets up at the camp");
    let _ = s.step(0.05);
}

#[test]
fn a_v04_shape_keeps_its_camp_progress_gear_and_below() {
    // A goal by title too, read past.
    assert!(V04.lines().any(|l| l.trim() == "goal_title = \"The Overseer\""));
    let (mut s, save) = load(V04);
    assert_eq!(s.camp.raids_won, 6);
    assert_eq!(s.camp.counts["grunts"], 12);
    assert!(s.camp.found.contains("fargodeep") && s.camp.opened.contains_key("chest_a"));
    // The chest: the retired weapons are their families' basic ones, wood is kept.
    let n = |s: &Survival, k: &str| s.storage.get(&s.content.item(k).unwrap()).copied().unwrap_or(0);
    assert_eq!((n(&s, "axe"), n(&s, "halberd"), n(&s, "wood")), (1, 1, 5), "{:?}", s.storage);
    // The cache.
    let cache = s.caches_of("ayla");
    assert_eq!(cache.len(), 1);
    // The hero.
    let a = s.join("ayla", "Ayla", save.players.get("ayla"));
    let h = &s.players[&a];
    assert_eq!(h.health, 60);
    assert_eq!(key(&s, h.weapon), "axe", "a honed retired axe is the plain axe");
    assert_eq!(h.off_weapon.map(|i| key(&s, i)).as_deref(), Some("mace"));
    assert_eq!(h.valor, 33.0);
    assert_eq!(h.pack, None, "a pack that is gone leaves none, not a panic");
    assert_eq!(h.armor.get("head").map(|i| key(&s, *i)).as_deref(), Some("leather_cap"));
    let bag = bag_of(&s, a);
    assert_eq!((bag["ore"], bag["axe"], bag["halberd"], bag["ember_staff"]), (3, 1, 1, 1), "{bag:?}");
    assert!(!bag.contains_key("a_vanished_item"));
    // The cards: each old card is the one that took its place, owned and on its key.
    for old in ["aimed_throw", "sundering_blow", "unstoppable_blow", "avatar", "onslaught", "ravager"] {
        assert!(!h.owned.contains(old), "{old} is gone");
    }
    for new in ["hurled_axe", "shockwave", "warrior_bladestorm", "war_banner"] {
        assert!(h.owned.contains(new), "{new} came with the old card: {:?}", h.owned);
    }
    assert!(!h.owned.contains("gone_card"));
    assert_eq!(h.loadout.get("ranged").map(String::as_str), Some("hurled_axe"));
    assert_eq!(h.loadout.get("close").map(String::as_str), Some("shockwave"));
    let _ = s.step(0.05);
}

#[test]
fn a_v04_shape_loads_into_the_deadmines_with_its_run_and_sacks() {
    let save = from_toml(V04).unwrap();
    let mut s = Survival::load_dungeon(rules(), Box::new(FlatLand::default()), 4, 1_000_000.0, &save);
    let b = s.below.as_ref().expect("the run is below");
    assert!(b.lit.contains("landing") && b.open.contains("gate_a") && b.slain.contains("rhahkzor") && b.looted.contains("box_a"));
    assert_eq!(b.run_at, Some(1_000_200.0));
    let h = &b.heroes["ayla"];
    assert_eq!((h.lamp.as_str(), h.draughts, h.flasks), ("landing", 3, 2));
    assert_eq!(s.caches_of("ayla").len(), 1);
    assert_eq!(s.camp.counts["grunts"], 12);
    let a = s.join("ayla", "Ayla", save.players.get("ayla"));
    assert_eq!(s.players[&a].health, 60);
    let _ = s.step(0.05);
}

#[test]
fn a_priest_from_the_early_drafts_comes_back_with_the_cards_that_took_their_place() {
    let (mut s, save) = load(PRIEST);
    let m = s.join("mira", "Mira", save.players.get("mira"));
    let h = &s.players[&m];
    assert_eq!(h.class, "priest");
    for gone in ["mind_flay", "leap_of_faith", "dispersion", "shadowguard", "power_word_barrier"] {
        assert!(!h.owned.contains(gone), "{gone} is gone: {:?}", h.owned);
    }
    // Mind Flay was Mind Blast's key, Leap of Faith Angelic Feather, the Barrier the Shield.
    for new in ["mind_blast", "angelic_feather", "vampiric_embrace", "power_word_shield", "psychic_scream"] {
        assert!(h.owned.contains(new), "{new} came with the old card: {:?}", h.owned);
    }
    // No two keys hold one card, and every key holds a card the hero owns.
    let held: Vec<&String> = h.loadout.values().collect();
    for k in &held {
        assert!(h.owned.contains(*k), "{k} is on a key but not owned");
    }
    let mut dedup = held.clone();
    dedup.sort();
    dedup.dedup();
    assert_eq!(dedup.len(), held.len(), "{:?}", h.loadout);
    assert_eq!(h.loadout.get("defense").map(String::as_str), Some("power_word_shield"));
    assert_eq!(h.valor, 10.0);
    // And they play: a step and a cast do not panic.
    let _ = s.step(0.05);
}

#[test]
fn a_rogue_saved_with_a_shield_a_two_hander_and_old_cards_loads_with_two_hands_and_its_cards() {
    let (mut s, save) = load(ROGUE);
    // A two-hander and the buckler in hand: the Rogue's blade in the main hand, the off hand empty
    // (it jabs), and both in the bags with what was there.
    let a = s.join("ayla", "Ayla", save.players.get("ayla"));
    let h = &s.players[&a];
    assert_eq!(h.class, "rogue");
    assert_eq!(key(&s, h.weapon), "dagger");
    assert!(h.off_weapon.is_none() && h.off_empty, "the off hand empty");
    let bag = bag_of(&s, a);
    assert_eq!((bag["greatsword"], bag["buckler"], bag["ore"]), (1, 1, 2), "{bag:?}");
    // The old cards come back as their successors, on their keys.
    for (gone, new) in [("deadly_throw", "fan_of_knives"), ("evasion", "smoke_bomb")] {
        assert!(!h.owned.contains(gone) && h.owned.contains(new), "{gone} became {new}: {:?}", h.owned);
    }
    assert_eq!(h.loadout.get("ranged").map(String::as_str), Some("fan_of_knives"));
    assert_eq!(h.loadout.get("defense").map(String::as_str), Some("smoke_bomb"));
    assert_eq!(h.valor, 20.0);
    // A sword and an axe in the off hand: the sword stays, the axe goes to the bags.
    let v = s.join("vex", "Vex", save.players.get("vex"));
    let h = &s.players[&v];
    assert_eq!(key(&s, h.weapon), "sword");
    assert!(h.off_weapon.is_none() && h.off_empty);
    let bag = bag_of(&s, v);
    assert_eq!(bag.get("axe"), Some(&1), "{bag:?}");
    assert!(!bag.contains_key("buckler"), "the buckler wasn't in hand: it isn't doubled");
    // And they play: the right click swings the off hand, never a guard.
    s.input(a, Input::Guard);
    assert!(matches!(&s.players[&a].action, Action::Swing { strike, .. } if strike.kind == StrikeKind::Off), "{:?}", s.players[&a].action);
    let _ = s.step(0.05);
    // Saved again and loaded again: nothing more turns up.
    let again = s.leave(a).unwrap();
    let a = s.join("ayla", "Ayla", Some(&again));
    let bag = bag_of(&s, a);
    assert_eq!((bag["greatsword"], bag["buckler"]), (1, 1), "{bag:?}");
}

#[test]
fn a_rogue_saved_with_a_fist_in_one_hand_loads_holding_the_fist_weapon_in_both() {
    use crate::sim::rogue::Pair;
    let (mut s, save) = load(ONE_FIST);
    // (hero, the fist both hands hold, what goes to the bags)
    for (who, fist, bagged) in [
        ("kip", "footpad_knuckles", Some("dagger")),
        ("lark", "cutpurse_claw", Some("sword")),
        // Two different fists: the main hand's is the pair.
        ("moss", "footpad_knuckles", Some("cutpurse_claw")),
        // A fist beside an empty hand: nothing to put away.
        ("nell", "cutpurse_claw", None),
    ] {
        let id = s.join(who, who, save.players.get(who));
        let h = &s.players[&id];
        assert_eq!((key(&s, h.weapon), h.off_weapon.map(|o| key(&s, o)), h.off_empty), (fist.to_string(), Some(fist.to_string()), false), "{who}");
        assert_eq!(s.pair_of(h), Some(Pair::IronKnuckles), "{who}");
        let bag = bag_of(&s, id);
        if let Some(b) = bagged {
            assert_eq!(bag.get(b), Some(&1), "{who}: the {b} in the bags: {bag:?}");
        }
        if who == "kip" {
            assert_eq!(bag.get("ore"), Some(&2), "what the bags held stays");
        }
        // Saved again and loaded again: nothing more turns up.
        let again = s.leave(id).unwrap();
        let id = s.join(who, who, Some(&again));
        assert_eq!(bag_of(&s, id), bag, "{who}");
        assert!(s.fists_of(&s.players[&id]), "{who}");
    }
    // Bags with no room: the hands stay as they were.
    let mut full = save.players["kip"].clone();
    let stack = s.content.items[s.content.item("ore").unwrap()].stack.max(1);
    full.bag = BTreeMap::from([("ore".to_string(), stack * s.content.player.bag_slots as u32)]);
    let id = s.join("kip2", "Kip", Some(&full));
    let h = &s.players[&id];
    assert_eq!((key(&s, h.weapon), h.off_weapon.map(|o| key(&s, o))), ("footpad_knuckles".to_string(), Some("dagger".to_string())), "no room: as it was");
    assert!(!bag_of(&s, id).contains_key("dagger"));
    // Anyone but a Rogue: a fist and a dagger, as saved.
    let id = s.join("bren", "Bren", save.players.get("bren"));
    let h = &s.players[&id];
    assert_eq!((key(&s, h.weapon), h.off_weapon.map(|o| key(&s, o))), ("footpad_knuckles".to_string(), Some("dagger".to_string())));
    assert!(bag_of(&s, id).is_empty());
}

#[test]
fn a_save_that_is_newer_than_this_server_is_refused_not_misread() {
    let newer = V01.replace("version = 1", "version = 99");
    assert!(from_toml(&newer).is_err());
}
