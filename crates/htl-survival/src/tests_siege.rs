//! Siege: the hall's ways to play, a match's camps and waves, sends that raise income and march
//! on a rival with its next wave, gold for kills and veins, gold-bought defences and cards, rivals
//! (not partners) striking each other, a fall that gets you up at your Hearth, a Hearth fallen
//! and a winner, and everyone back in the hall, fresh.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::{Foe, MobState};
use crate::sim::siege::SiegePhase;
use crate::sim::*;

const DT: f32 = 0.05;

fn world() -> Survival {
    let mut c = Content::dev();
    c.dev.full_tree = false;
    let mut s = Survival::new_siege(c, Box::new(FlatLand::default()), 1, 1_000_000.0);
    s.events.clear();
    s
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        out.extend(s.step(DT));
    }
    out
}

fn arrive(s: &mut Survival, name: &str) -> Id {
    let id = s.join(&name.to_lowercase(), name, None);
    s.siege_arrive(id);
    id
}

fn def() -> crate::content::SiegeDef {
    Content::dev().siege.unwrap()
}

/// Heroes in the hall, into a way to play, and the match under way.
fn a_match(mode: &str, names: &[&str]) -> (Survival, Vec<Id>) {
    let mut s = world();
    let ids: Vec<Id> = names.iter().map(|n| arrive(&mut s, n)).collect();
    for id in &ids {
        s.siege_join(*id, mode).unwrap();
    }
    if !matches!(s.siege.as_ref().unwrap().phase, SiegePhase::Countdown { .. }) {
        s.siege_start_now(ids[0]).unwrap();
    }
    run(&mut s, def().countdown + 0.2);
    assert_eq!(s.siege.as_ref().unwrap().phase, SiegePhase::Running);
    (s, ids)
}

fn hearth_of(s: &Survival, camp: usize) -> Id {
    s.siege.as_ref().unwrap().camps[camp].hearth
}

#[test]
fn a_full_way_begins_and_puts_each_hero_at_their_own_camp_fresh() {
    let mut s = world();
    let d = def();
    let a = arrive(&mut s, "Ayla");
    let b = arrive(&mut s, "Bren");
    assert!(dist2d(s.players[&a].pos, [d.lobby[0], d.lobby[1], 0.0]) < 10.0, "in the hall");
    assert!(s.objs.values().any(|o| o.kind == ObjKind::SiegeBanner));
    s.siege_join(a, "1v1").unwrap();
    assert_eq!(s.siege.as_ref().unwrap().phase, SiegePhase::Lobby, "one of two");
    assert!(s.siege_modes(a).iter().any(|m| m.key == "1v1" && m.mine && m.heroes == vec!["Ayla".to_string()]));
    s.siege_join(b, "1v1").unwrap();
    assert!(matches!(s.siege.as_ref().unwrap().phase, SiegePhase::Countdown { .. }), "full: it counts down");
    let c = arrive(&mut s, "Cass");
    assert!(s.siege_join(c, "1v1").is_err(), "no seat left");
    run(&mut s, d.countdown + 0.2);
    let sg = s.siege.as_ref().unwrap();
    assert_eq!(sg.phase, SiegePhase::Running);
    assert_eq!(sg.camps.len(), 2);
    for (k, h) in [a, b].into_iter().enumerate() {
        assert_eq!(s.camp_of(h), Some(k));
        let hearth = s.objs[&hearth_of(&s, k)].pos;
        assert!(dist2d(s.players[&h].pos, hearth) < 10.0, "by their Hearth");
        assert_eq!(s.gold_of(h), d.start_gold);
        // Their class's first two cards, nothing else.
        assert_eq!(s.players[&h].owned.len(), 2, "{:?}", s.players[&h].owned);
    }
    assert!(s.siege_hud(a).is_some_and(|h| h.camps.len() == 2 && h.camps[0].mine && h.camps[1].target));
    // Each camp has its ledger and rack.
    assert_eq!(s.objs.values().filter(|o| matches!(o.kind, ObjKind::SiegeLedger { .. })).count(), 2);
    assert_eq!(s.objs.values().filter(|o| o.kind == ObjKind::Rack).count(), 2);
}

#[test]
fn a_send_raises_income_and_marches_on_the_rival_with_its_next_wave() {
    let (mut s, ids) = a_match("1v1", &["Ayla", "Bren"]);
    let d = def();
    let (a, b) = (ids[0], ids[1]);
    assert!(s.siege_send(a, "overseer").is_err(), "not until its wave");
    let msg = s.siege_send(a, "kobolds").unwrap();
    assert!(msg.contains("the Farm"), "{msg}");
    assert_eq!(s.gold_of(a), d.start_gold - 15);
    assert_eq!(s.siege.as_ref().unwrap().income[&a], d.start_income + 1);
    // Warned before it comes.
    let ev = run(&mut s, d.first_gap - d.warn_secs + 0.2);
    assert!(ev.iter().any(|e| matches!(e, Event::Banner { player: Some(p), title, .. } if *p == b && title == "Coming at you")));
    assert!(s.siege_hud(b).unwrap().coming.iter().any(|(n, k)| n == "Kobold Laborer" && *k == 3));
    run(&mut s, d.warn_secs);
    assert_eq!(s.siege.as_ref().unwrap().wave, 1);
    // It comes in groups: the last of them a few seconds on.
    run(&mut s, 13.0);
    let sg = s.siege.as_ref().unwrap();
    let kobold = s.content.mob("kobold_laborer").unwrap();
    let at_b: Vec<&Id> = sg.foes.iter().filter(|(_, c)| **c == 1).map(|(m, _)| m).collect();
    let kobolds = at_b.iter().filter(|m| s.mobs.get(m).is_some_and(|x| x.def == kobold)).count();
    assert!(kobolds >= 3, "the three sent, with the base wave: {kobolds}");
    // Every creature sent at the Farm came in at its gate and goes for it, not the Stables.
    let (farm, stables) = (s.objs[&sg.camps[1].hearth].pos, s.objs[&sg.camps[0].hearth].pos);
    for m in &at_b {
        assert!(dist2d(s.mobs[m].pos, farm) < dist2d(s.mobs[m].pos, stables), "at the Farm's end");
    }
    // Income paid at the wave's start.
    assert_eq!(s.gold_of(a), d.start_gold - 15 + d.start_income + 1);
    assert_eq!(s.gold_of(b), d.start_gold + d.start_income);
}

#[test]
fn the_waves_march_on_the_hearth_and_felling_them_pays() {
    let (mut s, ids) = a_match("1v1", &["Ayla", "Bren"]);
    let a = ids[0];
    run(&mut s, def().first_gap + 0.2);
    // Bren off at the hall: nothing stands between his camp's foes and its Hearth.
    let lobby = def().lobby;
    s.move_player(ids[1], [lobby[0], lobby[1], 0.0], 0.0, Moving::default());
    let farm: Vec<Id> = s.siege.as_ref().unwrap().foes.iter().filter(|(_, c)| **c == 1).map(|(m, _)| *m).collect();
    let hearth = hearth_of(&s, 1);
    run(&mut s, 1.0);
    assert!(farm.iter().all(|m| matches!(s.mobs[m].state, mobs::MobState::Chase { foe: Foe::Obj(h) } if h == hearth)), "on the Hearth");
    let before = match s.objs[&hearth].kind {
        ObjKind::Station { health, .. } => health,
        _ => 0,
    };
    run(&mut s, 25.0);
    let after = match s.objs[&hearth].kind {
        ObjKind::Station { health, .. } => health,
        _ => 0,
    };
    assert!(after < before, "they hurt it: {before} to {after}");
    // A foe felled by Ayla pays her gold.
    let stables: Vec<Id> = s.siege.as_ref().unwrap().foes.iter().filter(|(_, c)| **c == 0).map(|(m, _)| *m).collect();
    let m = stables[0];
    let gold = s.gold_of(a);
    s.mobs.get_mut(&m).unwrap().health = 1;
    let mp = s.mobs[&m].pos;
    s.move_player(a, [mp[0] - 1.5, mp[1], 0.0], 0.0, Moving::default());
    s.hurt_mob(m, a, 5);
    assert!(s.gold_of(a) > gold, "gold for the kill");
    assert!(!s.siege.as_ref().unwrap().foes.contains_key(&m));
}

#[test]
fn a_hearth_fallen_ends_it_and_everyone_is_back_in_the_hall_fresh() {
    let (mut s, ids) = a_match("1v1", &["Ayla", "Bren"]);
    let (a, b) = (ids[0], ids[1]);
    let farm = hearth_of(&s, 1);
    s.hurt_station(farm, 1_000_000);
    let ev = run(&mut s, 0.2);
    assert!(matches!(s.siege.as_ref().unwrap().phase, SiegePhase::Over { winner: Some(0), .. }));
    assert!(ev.iter().any(|e| matches!(e, Event::Banner { player: Some(p), title, .. } if *p == a && title == "Victory")));
    assert!(ev.iter().any(|e| matches!(e, Event::Banner { player: Some(p), title, .. } if *p == b && title == "Defeat")));
    // Some card bought meanwhile: gone with the match.
    s.players.get_mut(&b).unwrap().owned.insert("x".into());
    run(&mut s, def().over_secs + 0.2);
    assert_eq!(s.siege.as_ref().unwrap().phase, SiegePhase::Lobby);
    assert!(!s.objs.contains_key(&farm), "the field cleared");
    assert!(s.mobs.is_empty());
    let lobby = def().lobby;
    for h in [a, b] {
        assert!(dist2d(s.players[&h].pos, [lobby[0], lobby[1], 0.0]) < 12.0, "back in the hall");
        assert_eq!(s.players[&h].owned.len(), 2, "fresh again");
        assert!(s.siege_hud(h).is_none());
    }
}

#[test]
fn after_the_last_wave_the_healthiest_hearth_wins() {
    let (mut s, _) = a_match("1v1", &["Ayla", "Bren"]);
    s.siege.as_mut().unwrap().wave = def().waves;
    s.siege.as_mut().unwrap().timer = 0.1;
    let stables = hearth_of(&s, 0);
    if let Some(Obj { kind: ObjKind::Station { health, .. }, .. }) = s.objs.get_mut(&stables) {
        *health -= 100;
    }
    run(&mut s, 0.3);
    assert!(matches!(s.siege.as_ref().unwrap().phase, SiegePhase::Over { winner: Some(1), .. }), "{:?}", s.siege.as_ref().unwrap().phase);
}

#[test]
fn partners_dont_strike_each_other_rivals_do_and_nobody_in_the_hall() {
    let mut s = world();
    let ids: Vec<Id> = ["Ayla", "Bren", "Cass", "Dorn"].iter().map(|n| arrive(&mut s, n)).collect();
    assert!(!s.hostile(ids[0], ids[1]), "in the hall: nobody");
    for id in &ids {
        s.siege_join(*id, "2v2").unwrap();
    }
    run(&mut s, def().countdown + 0.2);
    let sg = s.siege.as_ref().unwrap();
    assert_eq!(sg.camps.len(), 2);
    assert_eq!(sg.camps[0].heroes.len(), 2);
    let (x, y) = (sg.camps[0].heroes[0], sg.camps[0].heroes[1]);
    let z = sg.camps[1].heroes[0];
    assert!(!s.hostile(x, y), "partners");
    assert!(s.hostile(x, z), "rivals");
    // A partner's swing passes through; a rival's lands.
    let at = s.players[&x].pos;
    s.move_player(y, [at[0] + 1.5, at[1], at[2]], std::f32::consts::PI, Moving::default());
    s.input(y, Input::Attack);
    let ev = run(&mut s, 0.8);
    assert!(!ev.iter().any(|e| matches!(e, Event::Swing { victim: Some(v), outcome: Outcome::Hit, .. } if *v == x)), "partners don't strike each other");
    s.move_player(z, [at[0] + 1.5, at[1], at[2]], std::f32::consts::PI, Moving::default());
    s.input(z, Input::Attack);
    let ev = run(&mut s, 0.8);
    assert!(ev.iter().any(|e| matches!(e, Event::Swing { attacker, victim: Some(v), .. } if *attacker == z && *v == x)), "a rival's swing lands");
}

#[test]
fn a_hero_felled_by_a_rival_loses_gold_to_them_and_gets_up_at_their_hearth() {
    let (mut s, ids) = a_match("1v1", &["Ayla", "Bren"]);
    let (a, b) = (ids[0], ids[1]);
    let d = def();
    let at = s.players[&a].pos;
    s.move_player(b, [at[0] + 1.5, at[1], at[2]], std::f32::consts::PI, Moving::default());
    s.players.get_mut(&a).unwrap().health = 1;
    s.input(b, Input::Attack);
    run(&mut s, 0.8);
    assert_eq!(s.players[&a].health, 0, "felled");
    let take = (d.start_gold as f32 * d.steal).floor() as u32;
    assert_eq!(s.gold_of(a), d.start_gold - take);
    assert_eq!(s.gold_of(b), d.start_gold + take);
    run(&mut s, d.respawn_secs + 0.3);
    assert!(s.players[&a].health > 0, "up again");
    let hearth = s.objs[&hearth_of(&s, 0)].pos;
    assert!(dist2d(s.players[&a].pos, hearth) < 8.0, "at their Hearth");
}

#[test]
fn defences_and_cards_cost_gold_and_stand_only_on_your_own_ground() {
    let (mut s, ids) = a_match("1v1", &["Ayla", "Bren"]);
    let a = ids[0];
    let d = def();
    let menu = s.build_menu(a);
    let keys: Vec<&str> = menu.iter().map(|m| m.0.as_str()).collect();
    assert_eq!(keys, vec!["palisade", "barricade", "watchtower"], "only defences, for gold");
    assert!(menu[0].2.contains("15 Gold"), "{}", menu[0].2);
    // On her own ground, a stride from the Hearth's side.
    let hearth = s.objs[&hearth_of(&s, 0)].pos;
    let f = d.bases[0].facing;
    s.move_player(a, [hearth[0] - f.cos() * 14.0, hearth[1] - f.sin() * 14.0, 0.0], f + std::f32::consts::PI, Moving::default());
    s.build_start(a, "palisade").unwrap();
    run(&mut s, 0.1);
    s.build_confirm(a).unwrap();
    let secs = s.content.work.build * 2.0 + 0.5;
    run(&mut s, secs);
    assert_eq!(s.gold_of(a), d.start_gold - 15);
    let wall = s.objs.iter().find(|(_, o)| matches!(o.kind, ObjKind::Station { def, .. } if s.content.stations[def].key == "palisade")).map(|(id, _)| *id).unwrap();
    assert_eq!(s.siege.as_ref().unwrap().owner.get(&wall), Some(&0), "her camp's");
    // Not on the rival's.
    let far = s.objs[&hearth_of(&s, 1)].pos;
    s.move_player(a, [far[0] + 10.0, far[1], 0.0], 0.0, Moving::default());
    s.build_start(a, "palisade").unwrap();
    run(&mut s, 0.1);
    assert!(s.build_confirm(a).is_err(), "the rival's ground");
    s.cancel_preview(a);
    // A card off the board: its price by row.
    let p = &s.players[&a];
    let class = p.class.clone();
    let card = s.content.abilities.iter().find(|x| x.on_board() && !p.owned.contains(&x.key) && x.slot == "utility" && x.school == class).map(|x| x.key.clone()).unwrap();
    let price = d.cards["utility"];
    let gold = s.gold_of(a);
    s.take_node(a, &card).unwrap();
    assert!(s.players[&a].owned.contains(&card));
    assert_eq!(s.gold_of(a), gold - price);
    s.siege.as_mut().unwrap().gold.insert(a, 0);
    let other = s.content.abilities.iter().find(|x| x.on_board() && !s.players[&a].owned.contains(&x.key)).map(|x| x.key.clone()).unwrap();
    assert!(s.take_node(a, &other).is_err(), "short of gold");
}

#[test]
fn three_each_for_themselves_send_where_they_choose_and_a_camp_out_is_skipped() {
    let (mut s, ids) = a_match("1v1v1", &["Ayla", "Bren", "Cass"]);
    let a = ids[0];
    assert_eq!(s.siege.as_ref().unwrap().camps.len(), 3);
    assert_eq!(s.siege_target_name(a).as_deref(), Some("the Gold Mine"));
    s.siege_target(a, None).unwrap();
    assert_eq!(s.siege_target_name(a).as_deref(), Some("the Lumber Mill"));
    s.siege_target(a, None).unwrap();
    assert_eq!(s.siege_target_name(a).as_deref(), Some("the Gold Mine"), "never her own");
    // The Gold Mine falls: her sends turn to the camp still standing.
    let mine = hearth_of(&s, 1);
    s.hurt_station(mine, 1_000_000);
    run(&mut s, 0.2);
    assert_eq!(s.siege.as_ref().unwrap().phase, SiegePhase::Running, "two still stand");
    assert_eq!(s.siege_target_name(a).as_deref(), Some("the Lumber Mill"));
    assert!(s.siege_target(a, Some("gold_mine")).is_err());
}

#[test]
fn starting_short_plays_the_camps_with_someone_in_them_and_leaving_drops_you_out() {
    let mut s = world();
    let ids: Vec<Id> = ["Ayla", "Bren", "Cass"].iter().map(|n| arrive(&mut s, n)).collect();
    for id in &ids {
        s.siege_join(*id, "1v1v1v1").unwrap();
    }
    assert_eq!(s.siege.as_ref().unwrap().phase, SiegePhase::Lobby, "three of four");
    assert!(s.siege_start_now(ids[0]).is_ok());
    run(&mut s, def().countdown + 0.2);
    assert_eq!(s.siege.as_ref().unwrap().camps.len(), 3);
    // Cass goes home: out of the match.
    s.travel(ids[2], "camp").unwrap();
    assert!(s.camp_of(ids[2]).is_none());
    let left = s.leave(ids[2]);
    assert!(left.is_some());
    run(&mut s, 0.2);
    assert_eq!(s.siege.as_ref().unwrap().phase, SiegePhase::Running, "her camp holds without her");
}

#[test]
fn the_middles_veins_pay_gold_in_a_match() {
    let (mut s, ids) = a_match("1v1", &["Ayla", "Bren"]);
    let a = ids[0];
    let vein = s.objs.iter().find(|(_, o)| matches!(o.kind, ObjKind::Node { .. })).map(|(id, o)| (*id, o.pos)).unwrap();
    s.move_player(a, [vein.1[0] - 1.5, vein.1[1], 0.0], 0.0, Moving::default());
    let gold = s.gold_of(a);
    s.interact(a, Some(vein.0));
    run(&mut s, 4.0);
    assert_eq!(s.gold_of(a), gold + def().vein_gold);
    assert!(s.players[&a].bag.keys().all(|i| s.content.items[*i].key != "ore"), "gold, not ore");
}

#[test]
fn only_so_many_can_be_sent_for_one_wave_and_the_base_wave_starts_small() {
    let (mut s, ids) = a_match("1v1", &["Ayla", "Bren"]);
    let a = ids[0];
    let d = def();
    s.siege.as_mut().unwrap().gold.insert(a, 1000);
    s.siege_send(a, "kobolds").unwrap();
    let e = s.siege_send(a, "kobolds").unwrap_err();
    assert!(e.contains("limit"), "{e}");
    // The wave in: the next one's cap is one more, the count starts again.
    let first = s.siege.as_ref().unwrap().timer + 0.2;
    run(&mut s, first);
    assert_eq!(s.siege.as_ref().unwrap().wave, 1);
    s.siege_send(a, "kobolds").unwrap();
    assert!(s.siege_send(a, "kobolds").is_err(), "3 of 5, then 6 is too many");
    // A small first wave, weaker than the camp's own foes.
    let mut w = world();
    let first = w.base_wave(1, 1);
    assert!((2..=3).contains(&first.len()), "{}", first.len());
    assert!(w.base_wave(15, 1).len() <= d.most);
    let farm: Vec<Id> = s.siege.as_ref().unwrap().foes.iter().filter(|(_, c)| **c == 1).map(|(m, _)| *m).collect();
    let m = &s.mobs[&farm[0]];
    assert!(m.max_health < s.base_health(m.def), "weaker than at camp");
    assert!(s.foe_blow_share(farm[0]) < s.content.combat.foe_damage);
}

#[test]
fn the_battle_trainer_sells_the_classs_other_cards_and_a_free_key_takes_one_at_once() {
    let (mut s, ids) = a_match("1v1", &["Ayla", "Bren"]);
    let a = ids[0];
    let camp = s.siege.as_ref().unwrap().camps[0].clone();
    assert!(matches!(s.objs[&camp.trainer].kind, ObjKind::SiegeTrainer { .. }), "a trainer at each camp");
    let cards = s.siege_trainer(a);
    let class = s.players[&a].class.clone();
    assert!(!cards.is_empty());
    assert!(cards.iter().all(|(k, ..)| s.content.abilities[s.content.ability(k).unwrap()].school == class), "the class's own");
    assert!(cards.windows(2).all(|w| w[0].2 <= w[1].2), "cheapest first");
    // A utility card: its key (1) is free, so it goes straight on.
    let (key, _, price, can, ..) = cards.iter().find(|c| s.content.abilities[s.content.ability(&c.0).unwrap()].slot == "utility").cloned().unwrap();
    assert!(can);
    let gold = s.gold_of(a);
    s.take_node(a, &key).unwrap();
    assert_eq!(s.gold_of(a), gold - price);
    assert_eq!(s.players[&a].loadout.get("utility"), Some(&key), "on its key at once");
    assert!(!s.siege_trainer(a).iter().any(|c| c.0 == key), "off the trainer's list");
}

#[test]
fn everyone_ready_at_their_hearth_brings_the_next_wave_early() {
    let (mut s, ids) = a_match("1v1", &["Ayla", "Bren"]);
    let (a, b) = (ids[0], ids[1]);
    let d = def();
    assert!(s.siege.as_ref().unwrap().timer > d.ready_secs + 10.0);
    // F at her own Hearth: ready, one of two; the wave doesn't move yet.
    let hearth = hearth_of(&s, 0);
    let at = s.objs[&hearth].pos;
    s.move_player(a, [at[0] + 2.0, at[1], at[2]], std::f32::consts::PI, Moving::default());
    s.interact(a, Some(hearth));
    assert_eq!(s.siege_hud(a).unwrap().ready, (1, 2, true));
    assert!(s.siege.as_ref().unwrap().timer > d.ready_secs + 10.0);
    // Taken back and given again; then the other: it comes in `ready_secs`.
    s.siege_ready(a).unwrap();
    assert_eq!(s.siege_hud(a).unwrap().ready, (0, 2, false));
    s.siege_ready(a).unwrap();
    let msg = s.siege_ready(b).unwrap();
    assert!(msg.contains("Everyone"), "{msg}");
    assert!(s.siege.as_ref().unwrap().timer <= d.ready_secs);
    run(&mut s, d.ready_secs + 0.2);
    assert_eq!(s.siege.as_ref().unwrap().wave, 1);
    assert_eq!(s.siege_hud(a).unwrap().ready, (0, 2, false), "a fresh vote each wave");
}

#[test]
fn every_wave_brings_supplies_and_comes_in_groups() {
    let (mut s, ids) = a_match("1v1", &["Ayla", "Bren"]);
    let a = ids[0];
    let d = def();
    let roast = s.content.item("roast").unwrap();
    let before = s.players[&a].bag.get(&roast).copied().unwrap_or(0);
    // A big wave for the Farm: more than one group.
    s.siege.as_mut().unwrap().gold.insert(a, 1000);
    s.siege_send(a, "kobolds").unwrap();
    let first = s.siege.as_ref().unwrap().timer + 0.2;
    run(&mut s, first);
    let have = s.players[&a].bag.get(&roast).copied().unwrap_or(0);
    assert_eq!(have, (before + d.supply["roast"]).min(d.supply_most.max(before)), "food with the wave");
    let at_farm = |s: &Survival| s.siege.as_ref().unwrap().foes.values().filter(|c| **c == 1).count();
    let now = at_farm(&s);
    assert!(now <= 3, "the first group only: {now}");
    run(&mut s, 13.0);
    assert!(at_farm(&s) > now, "the rest after");
    // Never more than the most.
    for _ in 0..5 {
        let t = s.siege.as_ref().unwrap().timer + 0.2;
        run(&mut s, t);
    }
    assert!(s.players[&a].bag.get(&roast).copied().unwrap_or(0) <= d.supply_most.max(before));
}

#[test]
fn a_hero_who_strikes_a_creature_at_the_hearth_draws_it_off() {
    let (mut s, ids) = a_match("1v1", &["Ayla", "Bren"]);
    let a = ids[0];
    let hearth = hearth_of(&s, 0);
    let at = s.objs[&hearth].pos;
    // A creature at work on Ayla's Hearth, mid-blow; Ayla far off, then she strikes it.
    let def = s.content.mob("kobold_laborer").unwrap();
    let m = s.spawn_mob(def, [at[0] + 3.0, at[1], at[2]], None, true);
    s.siege.as_mut().unwrap().foes.insert(m, 0);
    s.move_player(a, [at[0] + 30.0, at[1] + 30.0, at[2]], 0.0, Moving::default());
    s.siege_next(m);
    let mut busy = false;
    for _ in 0..200 {
        s.step(DT);
        if matches!(s.mobs[&m].state, MobState::Windup { foe: Foe::Obj(o), .. } if o == hearth) {
            busy = true;
            break;
        }
    }
    assert!(busy, "at the Hearth");
    s.move_player(a, [at[0] + 6.0, at[1], at[2]], std::f32::consts::PI, Moving::default());
    s.hurt_mob(m, a, 1);
    let mut drawn = false;
    for _ in 0..200 {
        s.step(DT);
        if s.mobs.get(&m).and_then(|x| x.state.target()) == Some(a) {
            drawn = true;
            break;
        }
    }
    assert!(drawn, "it turns on her once its blow is done");
}
