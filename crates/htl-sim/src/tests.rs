use std::collections::{HashMap, HashSet};

use crate::geom::{dist2d, toward};
use crate::*;

fn arena() -> Arena {
    let hub = [0.0, 0.0, 0.0];
    let lane = |name: &str, x: f32, y: f32| Lane::new(name, vec![[x, y, 0.0], [x * 0.5, y * 0.5, 0.0], [x * 0.06, y * 0.06, 0.0]]);
    let base = |name: &str, x: f32, y: f32, lane: usize| BaseSite { name: name.into(), pos: [x * 1.6, y * 1.6, 0.0], lane };
    Arena {
        map_id: 529,
        area_id: 3358,
        hub,
        hub_facing: 0.0,
        player_spawn: [8.0, 0.0, 0.0],
        player_facing: 3.14,
        posts: Vec::new(),
        map_rect: None,
        lanes: vec![
            lane("the south-west", -90.0, -90.0),
            lane("the north-east", 90.0, 90.0),
            lane("the north-west", 90.0, -90.0),
            lane("the south-east", -90.0, 90.0),
        ],
        bases: vec![
            base("the Farm", -90.0, -90.0, 0),
            base("the Stables", 90.0, 90.0, 1),
            base("the Gold Mine", 90.0, -90.0, 2),
            base("the Lumber Mill", -90.0, 90.0, 3),
        ],
    }
}

/// The game's content, with the pause for picks off: most tests leave picks open while the
/// world runs on.
fn test_content() -> Content {
    let mut c = Content::builtin();
    c.game.pause_for_picks = false;
    c.game.wave_events = false;
    if std::env::var_os("NOSTREAM").is_some() {
        c.scaling.stream_from_wave = 99;
    }
    if let Some(h) = std::env::var("DEFENCE_DAMAGE").ok().and_then(|v| v.parse().ok()) {
        c.scaling.defence_damage = h;
    }
    for (k, f) in [
        ("HD_WAVE", &mut c.scaling.hero_damage_per_wave as &mut f32),
        ("DMG_WAVE", &mut c.scaling.damage_wave),
        ("HP_LATE", &mut c.scaling.health_late),
        ("HP_WAVE", &mut c.scaling.health_per_wave),
    ] {
        if let Some(v) = std::env::var(k).ok().and_then(|v| v.parse().ok()) {
            *f = v;
        }
    }
    if let Some(v) = std::env::var("LATE_FROM").ok().and_then(|v| v.parse().ok()) {
        c.scaling.late_from = v;
    }
    if let Some(h) = std::env::var("HERO_DAMAGE").ok().and_then(|v| v.parse().ok()) {
        c.scaling.hero_damage = h;
    }
    c
}

fn sim() -> Sim {
    Sim::new(test_content(), arena(), Box::new(Flat(0.0)), 7)
}

/// Checks the event stream stays coherent: nothing refers to an entity it never announced, paths
/// start where the mover stands, attacks open before they close.
#[derive(Default)]
struct Ledger {
    known: HashSet<EntityId>,
    zones: HashSet<EntityId>,
    engaged: HashMap<EntityId, EntityId>,
    hits: u32,
    spell_hits: u32,
    deaths: u32,
    text: Vec<String>,
    tells: Vec<String>,
    menus: u32,
    run_over: Option<RunResult>,
}

impl Ledger {
    fn new(sim: &Sim) -> Self {
        let mut l = Self::default();
        for e in sim.entities() {
            if matches!(e.what, What::Zone { .. }) {
                l.zones.insert(e.id);
            } else {
                l.known.insert(e.id);
            }
        }
        l
    }
    fn see(&mut self, sim: &Sim, events: &[Event]) {
        for ev in events {
            match ev {
                Event::SpawnEnemy { id, .. } | Event::SpawnNpc { id, .. } | Event::SpawnAlly { id, .. } => {
                    assert!(self.known.insert(*id), "double spawn {id}")
                }
                Event::Despawn { id } => {
                    assert!(self.known.remove(id), "despawn of unknown {id}");
                    self.engaged.remove(id);
                }
                Event::ZoneStart { id, .. } => assert!(self.zones.insert(*id)),
                Event::ZoneEnd { id } => assert!(self.zones.remove(id), "end of unknown zone {id}"),
                Event::MoveTo { id, path, speed } => {
                    assert!(self.known.contains(id));
                    assert!(path.len() >= 2 && *speed > 0.0);
                }
                Event::AttackStart { attacker, victim } => {
                    assert!(self.known.contains(attacker) && self.known.contains(victim));
                    self.engaged.insert(*attacker, *victim);
                }
                Event::AttackStop { attacker, .. } => {
                    self.engaged.remove(attacker);
                }
                Event::Hit { attacker, victim, .. } => {
                    assert!(self.known.contains(attacker) && self.known.contains(victim));
                    self.hits += 1;
                }
                Event::SpellHit { caster, victim, .. } => {
                    assert!(self.known.contains(caster) && self.known.contains(victim), "spell hit {caster}->{victim}");
                    self.spell_hits += 1;
                }
                Event::SpellGo { caster, hits, .. } => {
                    assert!(self.known.contains(caster));
                    for h in hits {
                        assert!(self.known.contains(h), "spell go hits unknown {h}");
                    }
                }
                Event::Died { id } => {
                    assert!(self.known.contains(id));
                    self.deaths += 1;
                }
                Event::Announce { text, .. } => self.text.push(text.clone()),
                Event::Tell { text, .. } => self.tells.push(text.clone()),
                Event::Menu { menu: Some(_), .. } => self.menus += 1,
                Event::RunOver(r) => self.run_over = Some(r.clone()),
                _ => {}
            }
        }
        // Every live entity the sim reports is one we were told about.
        for e in sim.entities() {
            if matches!(e.what, What::Zone { .. }) {
                assert!(self.zones.contains(&e.id), "sim has zone {} we never saw", e.id);
            } else {
                assert!(self.known.contains(&e.id), "sim has {} we never saw spawn", e.id);
            }
        }
    }
    /// Learn about players added after the ledger was made.
    fn learn(&mut self, id: EntityId) {
        self.known.insert(id);
    }
}

#[test]
fn a_player_arriving_starts_the_countdown_and_start_skips_it() {
    let mut s = sim();
    assert_eq!(s.status().phase, PhaseName::Waiting);
    let p = s.add_player("Ayla", "warrior", Luck::default());
    let mut l = Ledger::new(&s);
    let ev = s.step(0.05);
    l.see(&s, &ev);
    assert_eq!(s.status().phase, PhaseName::Countdown);
    s.start_now();
    let ev = s.step(0.05);
    l.see(&s, &ev);
    assert_eq!(s.status().phase, PhaseName::Wave);
    assert_eq!(s.status().wave, 1);
    assert!(l.text.iter().any(|t| t.contains("Wave 1:")));
    // Spawns arrive over the next seconds and walk.
    for _ in 0..60 {
        let ev = s.step(0.05);
        l.see(&s, &ev);
    }
    let enemies: Vec<_> = s.entities().into_iter().filter(|e| matches!(e.what, What::Enemy { .. })).collect();
    assert!(!enemies.is_empty());
    assert!(enemies.iter().all(|e| e.moving.is_some()));
    s.remove_player(p);
    assert_eq!(s.status().phase, PhaseName::Waiting);
}

#[test]
fn an_undefended_hub_falls_and_the_run_restarts() {
    let mut s = sim();
    let p = s.add_player("Ayla", "warrior", Luck::default());
    s.move_player(p, [500.0, 500.0, 0.0], 0.0); // far off, out of the fight
    let mut l = Ledger::new(&s);
    s.start_now();
    let mut fell = false;
    for _ in 0..(20 * 60 * 12) {
        let ev = s.step(0.05);
        l.see(&s, &ev);
        if s.status().phase == PhaseName::Intermission {
            s.start_now();
        }
        if s.status().phase == PhaseName::Defeat {
            fell = true;
            break;
        }
    }
    assert!(fell, "the hub should fall with nobody defending it");
    assert!(l.text.iter().any(|t| t.contains("has fallen")));
    let result = l.run_over.clone().expect("a run result");
    assert_eq!(result.players.len(), 1);
    // After the pause it starts over at wave 0 with a whole hub.
    for _ in 0..(20 * 25) {
        let ev = s.step(0.05);
        l.see(&s, &ev);
    }
    let st = s.status();
    assert_eq!(st.phase, PhaseName::Countdown);
    assert_eq!(st.wave, 0);
    assert_eq!(st.hub_health, st.hub_max_health);
    assert!(s.entities().iter().all(|e| !matches!(e.what, What::Enemy { .. })));
    assert_eq!(s.player_view(p).unwrap().level, 1);
}

/// How a bot plays.
#[derive(Clone, Copy, PartialEq)]
enum Policy {
    /// Stay at the hub.
    Defend,
    /// Go out to the bases from this wave on.
    Expedition(u32),
}

struct Bot {
    id: EntityId,
    target: Option<EntityId>,
    policy: Policy,
    /// The base it's working on.
    base: Option<usize>,
    /// Caches it has opened, by base and cycle.
    opened: Vec<(usize, u32)>,
    /// Steps out of marked ground, as a player would (DODGE=1 for the sweeps).
    dodge: bool,
}

impl Bot {
    fn new(id: EntityId, policy: Policy) -> Self {
        Self { id, target: None, policy, base: None, opened: Vec::new(), dodge: std::env::var_os("DODGE").is_some() }
    }
}

/// A simple bot: picks the first card offered, wears better gear, walks to the nearest enemy
/// and hits it, like a player holding auto-attack. Expedition bots also clear bases.
fn bot_turn(s: &mut Sim, bot: &mut Bot, dt: f32) {
    let me = bot.id;
    // Choices first.
    if let Some(menu) = s.menu(me).cloned() {
        // Cards: what hits things first, then upgrades; heals and buffs once the damage is in.
        if let Some(offer) = menu.offer.as_ref() {
            let damage_held = s.player_view(me).map_or(0, |v| v.cards.len());
            let best = offer
                .cards
                .iter()
                .enumerate()
                .max_by_key(|(i, c)| {
                    let hits = c.text.text.contains(" damage") || c.text.text.contains("Strikes") || c.text.text.contains("Blasts");
                    let score = match (c.text.kind, hits) {
                        ("Upgrade", _) => 30,
                        (_, true) => 40,
                        _ if damage_held >= 4 => 20,
                        _ => 5,
                    };
                    score * 10 - *i as i32
                })
                .map(|(i, _)| i)
                .unwrap_or(0);
            s.menu_select(me, best);
            return;
        }
        match menu.kind {
            MenuKind::Card | MenuKind::Relic | MenuKind::Legendary | MenuKind::SetPiece | MenuKind::Cache => s.menu_select(me, 0),
            MenuKind::Banner => s.menu_select(me, 0),
            _ => s.menu_select(me, menu.options.len() - 1),
        }
    }
    // Better gear.
    if let Some(view) = s.player_view(me) {
        for (slot, _, entry, _) in view.pack.clone() {
            let Some(t) = s.item_template(entry) else { continue };
            let worn = view.equip.iter().find(|(sl, ..)| *sl == t.slot).and_then(|(_, _, e, _)| s.item_template(*e));
            if worn.map_or(true, |w| t.score() > w.score()) && t.required_level <= view.level {
                let _ = s.auto_equip(me, slot);
                break;
            }
        }
    }
    let views = s.entities();
    let my = views.iter().find(|e| e.id == me).unwrap().clone();
    if !my.alive {
        return;
    }
    let st = s.status();
    // Expeditions: during an intermission (or from the chosen wave), head for an unclaimed base.
    if let Policy::Expedition(from) = bot.policy {
        if st.wave >= from && bot.base.is_none() {
            bot.base = (0..s.arena().bases.len()).find(|&b| matches!(s.base_state(b), Some(BaseState::Guarded | BaseState::Cleared)));
        }
        if let Some(b) = bot.base {
            match s.base_state(b) {
                Some(BaseState::Claimed) | Some(BaseState::Locked) | None => bot.base = None,
                Some(BaseState::Cleared) => {
                    // Open the cache, then claim the banner.
                    let cycle = s.cycle();
                    let cache = views
                        .iter()
                        .find(|e| matches!(e.what, What::Npc { npc: NpcKind::Cache, base } if base == b))
                        .filter(|_| !bot.opened.contains(&(b, cycle)));
                    let banner = views.iter().find(|e| matches!(e.what, What::Npc { npc: NpcKind::Banner { .. }, base } if base == b));
                    for target in [cache, banner].into_iter().flatten() {
                        let d = dist2d(my.pos, target.pos);
                        if d > 4.0 {
                            let to = toward(my.pos, target.pos, (7.0 * dt).min(d - 3.0));
                            if !s.casting(me) {
                s.move_player(me, to, 0.0);
            }
                            return;
                        }
                        if s.menu(me).is_none() {
                            s.interact(me, target.id);
                            if matches!(target.what, What::Npc { npc: NpcKind::Cache, .. }) {
                                bot.opened.push((b, cycle));
                            }
                        }
                    }
                    return;
                }
                Some(BaseState::Guarded) => {
                    let site = s.arena().bases[b].pos;
                    let guardian = views
                        .iter()
                        .filter(|e| e.alive && matches!(e.what, What::Enemy { .. }) && dist2d(e.pos, site) < 45.0)
                        .min_by(|a, c| dist2d(a.pos, my.pos).total_cmp(&dist2d(c.pos, my.pos)));
                    match guardian {
                        Some(g) => {
                            if bot.target != Some(g.id) {
                                bot.target = Some(g.id);
                                s.attack(me, g.id);
                            }
                        }
                        None => {
                            let d = dist2d(my.pos, site);
                            if d > 3.0 {
                                let to = toward(my.pos, site, (7.0 * dt).min(d - 2.0));
                                if !s.casting(me) {
                s.move_player(me, to, 0.0);
            }
                            }
                            return;
                        }
                    }
                }
            }
        }
    }
    if bot.dodge {
        // Out of any enemy's marked ground first, casting or not.
        let enemy = |id: EntityId| views.iter().any(|e| e.id == id && matches!(e.what, What::Enemy { .. }));
        let danger = views
            .iter()
            .filter_map(|e| match &e.what {
                // (A mending aura's 30 yards is no strike.)
                What::Zone { caster, radius, spell, .. } if enemy(*caster) && *radius <= 15.0 && *spell != 27827 => Some((e.pos, *radius)),
                _ => None,
            })
            .find(|(c, r)| dist2d(my.pos, *c) <= r + 1.0);
        if let Some((c, _)) = danger {
            let (dx, dy) = (my.pos[0] - c[0], my.pos[1] - c[1]);
            let len = (dx * dx + dy * dy).sqrt();
            let (ux, uy) = if len < 0.1 { (1.0, 0.0) } else { (dx / len, dy / len) };
            let step = 7.0 * dt;
            s.move_player(me, [my.pos[0] + ux * step, my.pos[1] + uy * step, my.pos[2]], 0.0);
            return;
        }
    }
    let alive_enemy = |id: EntityId| views.iter().any(|e| e.id == id && e.alive && matches!(e.what, What::Enemy { .. }));
    if bot.target.map_or(true, |t| !alive_enemy(t)) {
        // Defenders stay near the hub: the nearest enemy within 60 yards of it.
        let hub = s.arena().hub;
        bot.target = views
            .iter()
            .filter(|e| matches!(e.what, What::Enemy { .. }) && e.alive)
            .filter(|e| bot.base.is_some() || dist2d(e.pos, hub) < 60.0)
            .min_by(|a, b| dist2d(a.pos, my.pos).total_cmp(&dist2d(b.pos, my.pos)))
            .map(|e| e.id);
        if let Some(t) = bot.target {
            s.attack(me, t);
        }
    }
    if let Some(t) = bot.target {
        let tpos = views.iter().find(|e| e.id == t).unwrap().pos;
        let d = dist2d(my.pos, tpos);
        let class = s.class_def(me).unwrap().clone();
        let reach = if class.ranged() { class.range - 4.0 } else { 3.0 };
        if d > reach {
            let step = (7.0 * dt).min(d - reach + 0.5);
            let to = toward(my.pos, tpos, step);
            // A dodging bot doesn't walk back into marked ground.
            let marked = bot.dodge
                && views.iter().any(|e| match &e.what {
                    What::Zone { caster, radius, spell, .. } if *radius <= 15.0 && *spell != 27827 => {
                        views.iter().any(|c| c.id == *caster && matches!(c.what, What::Enemy { .. })) && dist2d(to, e.pos) <= radius + 1.5
                    }
                    _ => false,
                });
            if !s.casting(me) && !marked {
                s.move_player(me, to, 0.0);
            }
        }
    }
}

/// Run bots until defeat or `max_wave` cleared; returns the deepest wave cleared and the ledger.
fn run_bots(s: &mut Sim, bots: &mut [Bot], max_wave: u32, max_secs: f32) -> (u32, Ledger) {
    // Events from before the ledger (the posts' standards going up) aren't its business.
    let _ = s.step(0.0);
    let mut l = Ledger::new(s);
    s.start_now();
    let dt = 0.05;
    let mut t = 0.0;
    loop {
        for b in bots.iter_mut() {
            bot_turn(s, b, dt);
        }
        let ev = s.step(dt);
        l.see(s, &ev);
        t += dt;
        let st = s.status();
        // Between waves they spend their gold at the Ancient, as players do.
        if st.phase == PhaseName::Intermission {
            for b in bots.iter() {
                shop_defences(s, b.id);
            }
        }
        // Expedition bots use the break; others hurry the next wave.
        if st.phase == PhaseName::Intermission && bots.iter().all(|b| b.base.is_none()) && st.seconds_left > 3 {
            s.start_now();
        }
        if st.phase == PhaseName::Defeat || s.depth() >= max_wave || t > max_secs {
            break;
        }
    }
    (s.depth(), l)
}

/// Spend everything on the cheapest defences at the Ancient (the bot goes home to the shop
/// between waves, as players must).
fn shop_defences(s: &mut Sim, p: EntityId) {
    go_home(s, p);
    loop {
        let items = s.shop(p);
        let gold = s.player_view(p).map_or(0, |v| v.gold);
        let best = items.iter().filter(|i| i.group == "Defence" && i.cost.is_some_and(|c| c <= gold)).min_by_key(|i| i.cost);
        let Some(k) = best.map(|i| i.key.clone()) else { break };
        if s.buy(p, &k).is_err() {
            break;
        }
    }
}

/// To the shopkeeper by the Ancient, at once (a bot's walk home between waves).
fn go_home(s: &mut Sim, p: EntityId) {
    if s.at_shop(p) {
        return;
    }
    let hub = s.arena().hub;
    s.move_player(p, [hub[0] + 7.0, hub[1], hub[2]], 0.0);
}

fn four_bots(s: &mut Sim, policy: Policy) -> Vec<Bot> {
    ["warrior", "priest", "mage", "hunter"]
        .iter()
        .enumerate()
        .map(|(i, c)| Bot::new(s.add_player(&format!("Bot{i}"), c, Luck::default()), policy))
        .collect()
}

#[test]
fn four_bots_hold_the_first_waves_and_grow() {
    // The bridged field: the real map's shape, lanes meeting at the bridges.
    let seed = std::env::var("SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(7);
    let mut s = Sim::new(test_content(), bridged_arena(), Box::new(Flat(0.0)), seed);
    let mut bots = four_bots(&mut s, Policy::Defend);
    let (cleared, l) = run_bots(&mut s, &mut bots, 6, 1800.0);
    eprintln!("{:?}", s.status());
    for t in l.text.iter().filter(|t| t.contains("torn") || t.contains("Wave") || t.contains("hires") || t.contains("plants")) {
        eprintln!("  {t}");
    }
    assert!(cleared >= 6, "four fighters should clear wave 6 (cleared {cleared})");
    assert!(l.hits > 50 && l.deaths > 20);
    // They levelled, picked cards and found gear.
    for b in &bots {
        let v = s.player_view(b.id).unwrap();
        // A level a wave, one more for the boss.
        assert!(v.level >= 6, "{} is level {}", v.name, v.level);
        assert!(v.cards.len() >= 3, "{} has {:?}", v.name, v.cards);
    }
    // They earned gold (some may have spent every coin on defences).
    assert!(bots.iter().map(|b| s.player_view(b.id).unwrap().gold).sum::<u32>() > 0);
    assert!(l.spell_hits > 50, "abilities should be firing ({} spell hits)", l.spell_hits);
    assert!(l.menus >= 24);
}

#[test]
fn enemies_switch_to_a_player_who_hits_them() {
    let mut s = sim();
    let p = s.add_player("Ayla", "warrior", Luck::default());
    s.move_player(p, [500.0, 500.0, 0.0], 0.0);
    s.start_now();
    // Let one enemy spawn and walk a bit.
    for _ in 0..60 {
        s.step(0.05);
    }
    let enemy = s.entities().into_iter().find(|e| matches!(e.what, What::Enemy { .. })).unwrap();
    // Stand beside it and hit it.
    s.move_player(p, [enemy.pos[0] + 2.0, enemy.pos[1], 0.0], 0.0);
    s.attack(p, enemy.id);
    let mut turned = false;
    for _ in 0..40 {
        let ev = s.step(0.05);
        if ev.iter().any(|e| matches!(e, Event::AttackStart { attacker, victim } if *attacker == enemy.id && *victim == p)) {
            turned = true;
            break;
        }
        let me = s.entities().into_iter().find(|e| e.id == enemy.id).unwrap();
        s.move_player(p, [me.pos[0] + 2.0, me.pos[1], 0.0], 0.0);
    }
    assert!(turned, "the enemy should turn on the player who hit it");
}

#[test]
fn gear_checks_class_and_level() {
    let mut s = sim();
    let p = s.add_player("Ayla", "mage", Luck::default());
    let v = s.player_view(p).unwrap();
    // The starting kit: shirt, trousers, a staff, and the wand its shots come from.
    assert_eq!(v.equip.len(), 4);
    assert!(v.equip.iter().any(|(sl, _, e, _)| *sl == items::slot::RANGED && *e == items::STARTER_WAND));
    // A Tier 2 chest needs level 18.
    let mage = s.content().class_index("mage").unwrap();
    let chest = items::GearKey { class: mage, tier: 2, quality: 4, piece: 2, archetype: 0 }.entry();
    s.loot_test_item(p, chest);
    let v = s.player_view(p).unwrap();
    let (slot, ..) = *v.pack.iter().find(|(_, _, e, _)| *e == chest).unwrap();
    assert_eq!(s.auto_equip(p, slot), Err(EquipError::Level(18)));
    // A warrior's sword is no use to a mage: it never lands in the bags.
    let sword = items::GearKey { class: 0, tier: 0, quality: 2, piece: items::WEAPON_PIECE, archetype: 0 }.entry();
    s.loot_test_item(p, sword);
    let v = s.player_view(p).unwrap();
    assert!(!v.pack.iter().any(|(_, _, e, _)| *e == sword));
    // Loot goes in the bags, never on by itself: the hero chooses (right-click to wear).
    let green = items::GearKey { class: mage, tier: 0, quality: 2, piece: 2, archetype: 1 }.entry();
    let ev = { s.loot_test_item(p, green); s.step(0.0) };
    let v = s.player_view(p).unwrap();
    assert!(!v.equip.iter().any(|(_, _, e, _)| *e == green), "not put on by itself");
    let (slot, ..) = *v.pack.iter().find(|(_, _, e, _)| *e == green).expect("in the bags");
    assert!(ev.iter().any(|e| matches!(e, Event::Tell { text, .. } if text.contains("Right-click"))), "told how to wear it");
    s.auto_equip(p, slot).unwrap();
    let v = s.player_view(p).unwrap();
    assert!(v.equip.iter().any(|(sl, _, e, _)| *sl == items::slot::CHEST && *e == green), "worn when chosen");
    // A worse one is still offered, and says so.
    let common = items::GearKey { class: mage, tier: 0, quality: 1, piece: 2, archetype: 1 }.entry();
    s.loot_test_item(p, common);
    let ev = s.step(0.0);
    assert!(ev.iter().any(|e| matches!(e, Event::Tell { text, .. } if text.contains("Not as good"))), "{ev:?}");
    // At level 18 the Tier 2 chest can go on: the hero is told, and it stays in the bags until worn.
    s.set_level_for_test(p, 18);
    let v = s.player_view(p).unwrap();
    assert!(v.pack.iter().any(|(_, _, e, _)| *e == chest));
    let (slot, ..) = *v.pack.iter().find(|(_, _, e, _)| *e == chest).unwrap();
    s.auto_equip(p, slot).unwrap();
    assert!(s.player_view(p).unwrap().equip.iter().any(|(sl, _, e, _)| *sl == items::slot::CHEST && *e == chest));
}

#[test]
fn bases_open_get_claimed_and_escalate() {
    // The mechanics, not the balance: these bots hold no line, so the stream is off.
    let mut content = test_content();
    content.scaling.stream_secs = 0.0;
    content.scaling.stream_rest_secs = 0.0;
    content.hub.health *= 4;
    // Nor do they dodge boss abilities.
    content.scaling.elite_abilities.clear();
    for b in content.bosses.iter_mut() {
        b.abilities.clear();
    }
    let mut s = Sim::new(content, arena(), Box::new(Flat(0.0)), 7);
    let mut bots = four_bots(&mut s, Policy::Defend);
    // Two go out, two hold the hub.
    bots[0].policy = Policy::Expedition(3);
    bots[3].policy = Policy::Expedition(3);
    let mut l = Ledger::new(&s);
    s.start_now();
    let dt = 0.05;
    let mut t = 0.0;
    while t < 3600.0 && s.cycle() == 0 && s.status().phase != PhaseName::Defeat {
        for b in bots.iter_mut() {
            bot_turn(&mut s, b, dt);
        }
        let ev = s.step(dt);
        l.see(&s, &ev);
        t += dt;
        // The two at home spend their gold on the Ancient's defences, as players do.
        if s.status().phase == PhaseName::Intermission {
            shop_defences(&mut s, bots[1].id);
            shop_defences(&mut s, bots[2].id);
        }
        if (t % 20.0) < dt {
            let views = s.entities();
            for b in [&bots[0], &bots[3]] {
                let me = views.iter().find(|e| e.id == b.id).unwrap();
                let tgt = b.target.and_then(|t| views.iter().find(|e| e.id == t));
                eprintln!(
                    "t={t:.0} w{} bot {} at {:?} hp {}/{} base {:?} target {:?}",
                    s.wave(), b.id, me.pos.map(|v| v.round()), me.health, me.max_health, b.base,
                    tgt.map(|e| (e.id, e.health, e.max_health, e.pos.map(|v| v.round())))
                );
            }
        }
    }
    for x in l.text.iter().filter(|x| x.contains("claimed") || x.contains("ESCALATION") || x.contains("Gates")) {
        eprintln!("  {x}");
    }
    eprintln!("{:?} after {t:.0}s", s.status());
    for b in 0..s.arena().bases.len() {
        eprintln!("base {b}: {:?} guardians {:?}", s.base_state(b), s.base_guardians_for_test(b));
    }
    assert!(l.text.iter().any(|x| x.contains("Blight Gates have opened")));
    assert!(l.text.iter().filter(|x| x.contains(" claimed ")).count() >= 4, "every base claimed");
    assert_eq!(s.cycle(), 1, "the Escalation happened");
    assert!(l.text.iter().any(|x| x.contains("ESCALATION 1")));
}

/// Balance check: four bots play with different habits; prints how deep they get. Run with
/// `cargo test -p htl-sim balance --release -- --ignored --nocapture`.
#[test]
#[ignore]
fn balance() {
    for (name, policy) in [("defend", Policy::Defend), ("expedition", Policy::Expedition(3))] {
        for seed in 0..3 {
            let mut s = Sim::new(test_content(), arena(), Box::new(Flat(0.0)), seed);
            let mut bots = four_bots(&mut s, Policy::Defend);
            bots[0].policy = policy;
            bots[3].policy = policy;
            let (depth, l) = run_bots(&mut s, &mut bots, 200, 4.0 * 3600.0);
            let levels: Vec<u32> = bots.iter().map(|b| s.player_view(b.id).unwrap().level).collect();
            let cards: Vec<usize> = bots.iter().map(|b| s.player_view(b.id).unwrap().cards.len()).collect();
            println!(
                "{name} seed {seed}: depth {depth}, cycle {}, levels {levels:?}, cards {cards:?}, deaths {}",
                s.cycle(),
                l.deaths
            );
        }
    }
}

#[test]
fn solo_player_clears_wave_one() {
    // A bot that never dodges is luckier on some seeds than others: most runs should get through.
    for class in ["warrior", "priest", "mage", "hunter"] {
        let mut depths = Vec::new();
        for seed in 1..=8u64 {
            let mut s = Sim::new(test_content(), arena(), Box::new(Flat(0.0)), seed);
            let mut bots = vec![Bot::new(s.add_player("Solo", class, Luck::default()), Policy::Defend)];
            let (depth, _) = run_bots(&mut s, &mut bots, 2, 600.0);
            depths.push(depth);
        }
        assert!(depths.iter().all(|d| *d >= 1), "a solo {class} should always clear the first wave: {depths:?}");
        let ok = depths.iter().filter(|d| **d >= 2).count();
        assert!(ok >= 5, "a solo {class} should usually clear the first two waves: {depths:?}");
    }
}

#[test]
fn ledger_learns() {
    let mut s = sim();
    let mut l = Ledger::new(&s);
    let p = s.add_player("Ayla", "priest", Luck { rerolls: 1, ..Luck::default() });
    l.learn(p);
    let ev = s.step(0.05);
    l.see(&s, &ev);
}

#[test]
fn a_run_opens_with_a_choice_of_three_abilities() {
    let mut s = sim();
    let p = s.add_player("Ayla", "warrior", Luck::default());
    for _ in 0..40 {
        s.step(0.05);
    }
    let menu = s.menu(p).expect("a starting pick").clone();
    assert_eq!(menu.kind, MenuKind::Card);
    let offer = menu.offer.as_ref().expect("the card picker's view");
    assert_eq!(offer.kind, crate::OfferKind::Start);
    assert_eq!(offer.cards.len(), 3);
    assert!(offer.cards.iter().all(|c| c.text.kind == "Spell" && c.rank == 1), "three new abilities: {:?}", offer.cards);
    assert!(offer.cards.iter().all(|c| c.text.text.contains(" damage")), "{:?}", offer.cards);
    s.menu_select(p, 0);
    assert_eq!(s.player_view(p).unwrap().cards.len(), 1);
    assert_eq!(s.held_cards(p).len(), 1);
}

/// After a lost run, the next run's starting pick comes up for everyone, dead or alive, once
/// they're back on their feet at the hub.
#[test]
fn a_lost_run_offers_a_new_starting_pick() {
    let mut s = sim();
    let a = s.add_player("Ayla", "warrior", Luck::default());
    let b = s.add_player("Bren", "mage", Luck::default());
    for _ in 0..40 {
        s.step(0.05);
    }
    // One of them leaves their first pick open; the other takes it.
    s.menu_select(b, 0);
    s.start_now();
    // Nobody fights (they wait well away from the lanes, where nothing wanders to them and they
    // hold no enemy up): the hub falls.
    for id in [a, b] {
        s.move_player(id, [0.0, 400.0, 0.0], 0.0);
    }
    let mut over = false;
    let mut restarted = false;
    let mut menus_after: Vec<EntityId> = Vec::new();
    for _ in 0..(20 * 60 * 10) {
        for ev in s.step(0.05) {
            match ev {
                Event::RunOver(_) => over = true,
                Event::Splash { style: crate::Splash::NewRun, .. } => restarted = true,
                Event::Menu { player, menu: Some(m) } if restarted && m.offer.is_some() => menus_after.push(player),
                _ => {}
            }
        }
        if restarted && menus_after.contains(&a) && menus_after.contains(&b) {
            break;
        }
    }
    assert!(over && restarted, "the run ended and a new one began (over {over}, wave {}, status {:?}, hub {:?})", s.status().wave, s.status(), s.entities().iter().find(|e| matches!(e.what, What::Hub)).map(|e| e.health));
    assert!(menus_after.contains(&a) && menus_after.contains(&b), "both got a starting pick: {menus_after:?}");
    for id in [a, b] {
        let offer = s.menu(id).and_then(|m| m.offer.clone()).expect("an open pick");
        assert_eq!(offer.kind, crate::OfferKind::Start);
        assert_eq!(s.player_view(id).unwrap().cards.len(), 0, "a fresh build");
    }
}

#[test]
fn builds_take_shape_by_wave_five() {
    // The bridged field: the real map's shape, lanes meeting at the bridges.
    let mut s = Sim::new(test_content(), bridged_arena(), Box::new(Flat(0.0)), 7);
    let mut bots = four_bots(&mut s, Policy::Defend);
    let (cleared, _) = run_bots(&mut s, &mut bots, 5, 1800.0);
    assert!(cleared >= 5);
    for b in &bots {
        let v = s.player_view(b.id).unwrap();
        // Spells and their ranks, and talents (taken at full rank, a pick each).
        let talents = s.tree_points(b.id).iter().sum::<u32>();
        let picks: u32 = v.cards.iter().map(|(_, r)| *r as u32).sum();
        assert!(v.level >= 6 && picks + talents >= 6, "{} (level {}) made {picks} picks and {talents} talent points by wave 5: {:?}", v.name, v.level, v.cards);
    }
}

/// Every class spell, upgraded, goes off when cast by hand in a fight, and every spell set
/// survives a fight firing on its own.
#[test]
fn every_spell_can_be_cast() {
    let content = test_content();
    for (ci, class) in content.classes.iter().enumerate() {
        let kit = &content.kits[ci];
        for def in &kit.spells {
            let mut s = sim();
            let p = s.add_player("Ayla", &class.key, Luck::default());
            s.close_all_menus_for_test(p);
            s.set_level_for_test(p, 20);
            s.give_test_card(p, &def.key, 2);
            s.set_manual(p, true);
            let mut l = Ledger::new(&s);
            s.start_now();
            for _ in 0..40 {
                let ev = s.step(0.05);
                l.see(&s, &ev);
            }
            let me = s.entities().into_iter().find(|e| e.id == p).unwrap().pos;
            // Something to cast at, close and far; a hurt caster; rage; a fresh blow for Revenge.
            let near = s.spawn_boss_for_test(0, [me[0] + 4.0, me[1], 0.0]);
            let _far = s.spawn_boss_for_test(1, [me[0] + 14.0, me[1] + 2.0, 0.0]);
            s.select(p, Some(near));
            let mut ok = false;
            let mut last = None;
            for _ in 0..200 {
                s.prime_for_test(p);
                match s.cast_slot(p, 0) {
                    Ok(()) => {
                        ok = true;
                        break;
                    }
                    Err(e) => last = Some(e),
                }
                if def.kind == crate::kit::SpellKind::Charge {
                    s.select(p, Some(_far));
                }
                let ev = s.step(0.05);
                l.see(&s, &ev);
            }
            assert!(ok, "{} {}: never cast ({last:?})", class.key, def.key);
            // And the hits land without upsetting anything.
            for _ in 0..80 {
                let ev = s.step(0.05);
                l.see(&s, &ev);
            }
        }
        // A whole kit firing on its own.
        let mut s = sim();
        let p = s.add_player("Bren", &class.key, Luck::default());
        s.close_all_menus_for_test(p);
        s.set_level_for_test(p, 30);
        for def in kit.spells.iter().filter(|d| !d.talent).take(6) {
            s.give_test_card(p, &def.key, 4);
        }
        s.auto_talents(p);
        let mut bots = vec![Bot::new(p, Policy::Defend)];
        let mut l = Ledger::new(&s);
        s.start_now();
        let before = s.casts_for_test(p);
        for i in 0..(20 * 90) {
            bot_turn(&mut s, &mut bots[0], 0.05);
            if i % 60 == 0 {
                s.hurt_for_test(p, 0.1);
            }
            let ev = s.step(0.05);
            l.see(&s, &ev);
        }
        // (A mage at the hub casts about ten times in this fight: only a handful of enemies come.)
        assert!(s.casts_for_test(p) > before + 6, "{}'s spells fire on their own: {} casts, alive {:?}", class.key, s.casts_for_test(p) - before, s.entities().iter().find(|e| e.id == p).map(|e| (e.alive, e.health, e.pos)));
    }
}

/// A boss marks the ground, casts, then hits whoever is still standing there.
#[test]
fn boss_abilities_hit_those_who_stay_and_miss_those_who_move() {
    let mut s = sim();
    let stay = s.add_player("Ayla", "warrior", Luck::default());
    let go = s.add_player("Bren", "warrior", Luck::default());
    s.close_all_menus_for_test(stay);
    s.close_all_menus_for_test(go);
    s.move_player(stay, [40.0, 2.0, 0.0], 0.0);
    s.move_player(go, [40.0, -2.0, 0.0], 0.0);
    let boss = s.spawn_boss_for_test(0, [44.0, 0.0, 0.0]);
    let mut cast = None;
    for _ in 0..(20 * 30) {
        for ev in s.step(0.05) {
            if let Event::EnemyCast { caster, name, endangered, ms, .. } = ev {
                if caster == boss && name == "Bone Storm" {
                    cast = Some((endangered, ms));
                }
            }
        }
        if cast.is_some() {
            break;
        }
    }
    let (endangered, ms) = cast.expect("the boss casts Bone Storm");
    assert!(endangered.contains(&stay) && endangered.contains(&go), "both stand in it: {endangered:?}");
    assert_eq!(ms, 2500);
    // Bren runs; Ayla stays.
    s.move_player(go, [70.0, -2.0, 0.0], 0.0);
    let mut hit = Vec::new();
    for _ in 0..(20 * 3) {
        for ev in s.step(0.05) {
            if let Event::SpellHit { caster, victim, spell, .. } = ev {
                if caster == boss && spell == 16914 {
                    hit.push(victim);
                }
            }
        }
        s.move_player(go, [70.0, -2.0, 0.0], 0.0);
    }
    assert!(hit.contains(&stay), "the one who stayed is hit");
    assert!(!hit.contains(&go), "the one who ran is not");
}

/// Six spells fill the spell slots: after that, offers only rank up what's held.
#[test]
fn full_slots_only_offer_rank_ups() {
    let mut s = sim();
    let p = s.add_player("Ayla", "mage", Luck::default());
    s.close_all_menus_for_test(p);
    let spells = ["fireball", "frostbolt", "arcane_missiles", "scorch", "fire_blast", "blizzard"];
    for k in spells {
        s.give_test_card(p, k, 1);
    }
    for _ in 0..200 {
        for key in s.roll_for_test(p) {
            match key.split_once(':') {
                Some(("talent", _)) => {}
                Some((spell, _)) => assert!(spells.contains(&spell), "{key} upgrades a spell not held"),
                None => panic!("{key} would take a seventh spell slot"),
            }
        }
    }
}

/// Upgrades change how a spell plays, and talents shape the numbers: Frostbite freezes,
/// Shatter makes the frozen crit, Ignite burns after fire crits.
#[test]
fn upgrades_and_talents_change_how_spells_play() {
    let mut s = sim();
    let p = s.add_player("Ayla", "mage", Luck::default());
    s.close_all_menus_for_test(p);
    s.set_manual(p, true);
    s.set_level_for_test(p, 30);
    s.give_test_card(p, "frost_nova", 0);
    s.give_test_card(p, "fire_blast", 0);
    s.give_test_perk(p, "fire_blast", "frostfire");
    let before = s.spell_nums_for_test(p, "fire_blast");
    s.start_now();
    for _ in 0..40 {
        s.step(0.05);
    }
    let me = s.entities().into_iter().find(|e| e.id == p).unwrap().pos;
    let boss = s.spawn_boss_for_test(0, [me[0] + 4.0, me[1], 0.0]);
    s.select(p, Some(boss));
    assert_eq!(s.cast_slot(p, 0), Ok(()), "Frost Nova");
    s.step(0.05);
    assert!(s.is_frozen_for_test(boss), "Frost Nova freezes");
    // Shatter: crits against the frozen. Talent points go down Frost to it.
    let kit = s.kit_for_test(p);
    let frost = kit.tree_index("frost").unwrap();
    let shatter = kit.talents.iter().position(|t| t.key == "shatter").unwrap();
    for _ in 0..12 {
        if s.tree_points(p)[frost] >= 9 {
            break;
        }
        let next = kit
            .talents
            .iter()
            .enumerate()
            .filter(|(i, t)| t.tree == frost && *i != shatter)
            .map(|(i, _)| i)
            .find(|&i| s.talent_for_test(p, i).is_ok());
        if next.is_none() {
            break;
        }
    }
    assert_eq!(s.talent_for_test(p, shatter), Ok(()));
    let cold_snap = kit.talents.iter().position(|t| t.key == "cold_snap").unwrap();
    assert_eq!(s.talent_for_test(p, cold_snap), Ok(()));
    assert!(s.tree_points(p)[frost] >= 9, "{:?}", s.tree_points(p));
    let after = s.spell_nums_for_test(p, "fire_blast");
    assert!(after.crit_vs_frozen > before.crit_vs_frozen, "Shatter adds crit against the frozen");
    assert!(after.vs_frozen >= 1.0, "Fire Blast with Frostfire: double damage to the frozen");
    // A talent that teaches a spell puts it on the bar.
    let bar = s.spell_bar(p).unwrap();
    assert!(bar.slots.iter().any(|b| b.key == "cold_snap" && b.talent), "{:?}", bar.slots.iter().map(|b| &b.key).collect::<Vec<_>>());
    assert!(s.reset_talents(p).is_err() || s.tree_points(p)[frost] == 0);
}

/// Walking into a sprint totem makes you faster for a while.
#[test]
fn sprint_totems_speed_players_up() {
    let mut s = sim();
    let p = s.add_player("Ayla", "warrior", Luck::default());
    s.close_all_menus_for_test(p);
    s.step(0.05);
    let totem = s
        .entities()
        .into_iter()
        .find(|e| matches!(e.what, What::Npc { npc: NpcKind::Sprint, .. }))
        .expect("sprint totems stand on the lanes");
    let before = s.player_speed(p);
    s.move_player(p, totem.pos, 0.0);
    let ev = s.step(0.05);
    assert!(ev.iter().any(|e| matches!(e, Event::Despawn { id } if *id == totem.id)), "the totem is taken");
    assert!(s.player_speed(p) > before * 1.5, "faster: {} -> {}", before, s.player_speed(p));
    for _ in 0..(20 * 10) {
        s.step(0.05);
    }
    assert!((s.player_speed(p) - before).abs() < 0.01, "and back to normal later");
}


/// Stepping in and out of reach doesn't stop an enemy's swings.
#[test]
fn a_shuffling_player_still_gets_hit() {
    let mut content = test_content();
    for b in content.bosses.iter_mut() {
        b.abilities.clear();
    }
    let mut s = Sim::new(content, arena(), Box::new(Flat(0.0)), 7);
    let p = s.add_player("Ayla", "warrior", Luck::default());
    s.close_all_menus_for_test(p);
    s.move_player(p, [40.0, 0.0, 0.0], 0.0);
    let boss = s.spawn_boss_for_test(0, [44.0, 0.0, 0.0]);
    let mut hits = 0;
    for i in 0..(20 * 20) {
        // A little shuffle back and forth every half second.
        if i % 10 == 0 {
            let here = s.entities().into_iter().find(|e| e.id == p).unwrap().pos;
            let b = s.entities().into_iter().find(|e| e.id == boss).unwrap().pos;
            let away = toward(b, here, dist2d(b, here) + if (i / 10) % 2 == 0 { 2.0 } else { -2.0 });
            s.move_player(p, away, 0.0);
        }
        for ev in s.step(0.05) {
            if matches!(ev, Event::Hit { attacker, victim, .. } if attacker == boss && victim == p) {
                hits += 1;
            }
        }
    }
    assert!(hits >= 10, "only {hits} hits in 20 seconds");
}

/// Gold buys the Ancient a Treant Protector, which stands by it and shoots what comes close.
#[test]
fn gold_buys_defences_that_fight() {
    let mut content = test_content();
    for b in content.bosses.iter_mut() {
        b.abilities.clear();
    }
    let mut s = Sim::new(content, arena(), Box::new(Flat(0.0)), 7);
    let p = s.add_player("Ayla", "warrior", Luck::default());
    s.close_all_menus_for_test(p);
    s.give_gold_for_test(p, 1000);
    s.interact(p, s.hub_id());
    let menu = s.menu(p).cloned().expect("the Ancient's menu");
    let plant = menu.options.iter().position(|o| o.text.starts_with("Plant a Treant Protector")).expect("a protector for sale");
    s.menu_select(p, plant);
    let menu = s.menu(p).cloned().expect("still shopping");
    let hire = menu.options.iter().position(|o| o.text.starts_with("Hire a Basin Defender")).expect("defenders for sale");
    s.menu_select(p, hire);
    let views = s.entities();
    let protector = views.iter().find(|e| matches!(e.what, What::Npc { npc: NpcKind::Protector, .. })).expect("it stands in the world");
    assert!(views.iter().any(|e| matches!(e.what, What::Ally { owner, .. } if owner == s.hub_id())), "a defender stands guard");
    // Something to shoot at, well away from the player.
    let defender = views.iter().find(|e| matches!(e.what, What::Ally { owner, .. } if owner == s.hub_id())).unwrap().id;
    let target = s.spawn_boss_for_test(0, [protector.pos[0] + 15.0, protector.pos[1], 0.0]);
    let (mut shot, mut engaged, mut swung) = (false, false, 0);
    for _ in 0..(20 * 10) {
        for ev in s.step(0.05) {
            match ev {
                Event::SpellHit { caster, victim, .. } if caster == protector.id && victim == target => shot = true,
                Event::AttackStart { attacker, victim } if attacker == defender && victim == target => engaged = true,
                Event::Hit { attacker, victim, .. } if attacker == defender && victim == target => swung += 1,
                _ => {}
            }
        }
    }
    assert!(shot, "the protector hits it");
    assert!(engaged && swung >= 2, "the defender closes in and swings (engaged {engaged}, {swung} swings)");
}

/// Enemies that pass a defence stop to tear it down, and a fallen one can be bought again.
#[test]
fn enemies_tear_down_defences_that_can_be_rebuilt() {
    let mut content = test_content();
    for b in content.bosses.iter_mut() {
        b.abilities.clear();
    }
    let mut s = Sim::new(content, arena(), Box::new(Flat(0.0)), 7);
    let p = s.add_player("Ayla", "warrior", Luck::default());
    s.close_all_menus_for_test(p);
    s.give_gold_for_test(p, 1000);
    s.interact(p, s.hub_id());
    let menu = s.menu(p).cloned().expect("the Ancient's menu");
    let plant = menu.options.iter().position(|o| o.text.starts_with("Plant a Treant Protector")).expect("a protector for sale");
    s.menu_select(p, plant);
    s.close_all_menus_for_test(p);
    // The player wanders off; something big walks up to the protector.
    s.move_player(p, [200.0, -200.0, 0.0], 0.0);
    let views = s.entities();
    let protector = views.iter().find(|e| matches!(e.what, What::Npc { npc: NpcKind::Protector, .. })).expect("it stands").clone();
    assert!(protector.max_health > 1, "defences have health");
    s.spawn_boss_for_test(0, [protector.pos[0] + 4.0, protector.pos[1], 0.0]);
    let mut hit = false;
    let mut gone = false;
    for _ in 0..(20 * 400) {
        for ev in s.step(0.05) {
            match ev {
                Event::Hit { victim, .. } if victim == protector.id => hit = true,
                Event::Despawn { id } if id == protector.id => gone = true,
                _ => {}
            }
        }
        if gone {
            break;
        }
    }
    assert!(hit, "the boss attacks the protector");
    assert!(gone, "and tears it down");
    s.move_player(p, [8.0, 0.0, 0.0], 0.0);
    s.interact(p, s.hub_id());
    let menu = s.menu(p).cloned().expect("the Ancient's menu");
    assert!(menu.options.iter().any(|o| o.text.starts_with("Replant a Treant Protector (1/")), "{:?}", menu.options);
}

/// How often a lone, dumb bot gets through the opening waves, over many seeds (run by hand).
#[test]
#[ignore]
fn solo_survival_sweep() {
    let only = std::env::var("CLASSES").unwrap_or_default();
    for class in ["warrior", "priest", "mage", "hunter"] {
        if !only.is_empty() && !only.split(',').any(|c| c == class) {
            continue;
        }
        let mut depths = Vec::new();
        for seed in 1..=16u64 {
            // BRIDGED=1: roads split over two bridges, like Arathi Basin.
            let field = if std::env::var_os("BRIDGED").is_some() { bridged_arena() } else { arena() };
            let mut s = Sim::new(test_content(), field, Box::new(Flat(0.0)), seed);
            let mut bots = vec![Bot::new(s.add_player("Solo", class, Luck::default()), Policy::Defend)];
            let waves: u32 = std::env::var("WAVES").ok().and_then(|w| w.parse().ok()).unwrap_or(4);
            let (depth, _) = run_bots(&mut s, &mut bots, waves, 900.0 * waves as f32 / 4.0);
            depths.push(depth);
        }
        let ok = depths.iter().filter(|d| **d >= 2).count();
        eprintln!("{class}: {ok}/16 clear wave 2; depths {depths:?}");
    }
}

#[test]
#[ignore]
fn solo_death_report() {
    let seed: u64 = std::env::var("SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(2);
    let class = std::env::var("CLASS").unwrap_or("warrior".into());
    let field = if std::env::var_os("BRIDGED").is_some() { bridged_arena() } else { arena() };
    let mut s = Sim::new(test_content(), field, Box::new(Flat(0.0)), seed);
    let p = s.add_player("Solo", &class, Luck::default());
    let mut bots = vec![Bot::new(p, Policy::Defend)];
    s.step(0.0);
    let mut l = Ledger::new(&s);
    s.start_now();
    let mut t = 0.0;
    let mut recent: std::collections::VecDeque<String> = Default::default();
    loop {
        for b in bots.iter_mut() {
            bot_turn(&mut s, b, 0.05);
        }
        let ev = s.step(0.05);
        l.see(&s, &ev);
        for e in &ev {
            match e {
                Event::Hit { attacker, victim, damage, .. } if *victim == p => recent.push_back(format!("{t:.1} melee {damage} from {attacker:?}")),
                Event::SpellHit { caster, victim, spell, damage, .. } if *victim == p => recent.push_back(format!("{t:.1} spell {spell} {damage} from {caster:?}")),
                Event::EnemyCast { name, .. } => recent.push_back(format!("{t:.1} cast {name}")),
                _ => {}
            }
            if recent.len() > 40 { recent.pop_front(); }
        }
        t += 0.05;
        if std::env::var_os("TRACE").is_some() && (t * 20.0) as u32 % 40 == 0 {
            let me = s.entities().into_iter().find(|e| e.id == p).unwrap();
            let hub = s.arena().hub;
            let zones = s.entities().into_iter().filter(|e| matches!(e.what, What::Zone { .. }) && dist2d(e.pos, me.pos) < 12.0).count();
            eprintln!(
                "{t:.0}s at {:.0},{:.0} ({:.0} from hub) target {:?} zones near {zones} casting {}",
                me.pos[0],
                me.pos[1],
                dist2d(me.pos, hub),
                bots[0].target,
                s.casting(p)
            );
        }
        let st = s.status();
        if std::env::var_os("HURRY").is_some() && st.phase == PhaseName::Intermission && st.seconds_left > 3 {
            s.start_now();
        }
        let limit: f32 = std::env::var("SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(400.0);
        if st.phase == PhaseName::Defeat || t > limit {
            eprintln!("{st:?} at {t:.1}");
            break;
        }
    }
    let pv = s.player_view(p).unwrap();
    eprintln!("level {} cards {:?}", pv.level, pv.cards);
    for r in recent { eprintln!("  {r}"); }
    for t in l.text.iter().rev().take(10).rev() { eprintln!("  {t}"); }
}

/// With the add-on's bar, spells wait for the player: a key press casts at the selected enemy,
/// starts the cooldown and a short global cooldown, and says why when it can't.
#[test]
fn spells_cast_by_hand_hit_the_selected_enemy() {
    let mut s = sim();
    let p = s.add_player("Ayla", "mage", Luck::default());
    s.close_all_menus_for_test(p);
    s.set_manual(p, true);
    assert_eq!(s.cast_slot(p, 0), Err(CastError::NoSpell));
    s.give_test_card(p, "fireball", 1);
    s.give_test_card(p, "frostbolt", 1);
    let bar = s.spell_bar(p).unwrap();
    assert!(bar.manual);
    assert_eq!(bar.slots.iter().map(|b| b.key.as_str()).collect::<Vec<_>>(), ["fireball", "frostbolt"]);
    s.start_now();
    let me = s.entities().into_iter().find(|e| e.id == p).unwrap().pos;
    let near = s.spawn_boss_for_test(0, [me[0] + 10.0, me[1], 0.0]);
    let far = s.spawn_boss_for_test(0, [me[0] + 6.0, me[1] + 3.0, 0.0]);
    s.select(p, Some(near));
    // Nothing fires on its own.
    let mut fired = false;
    for _ in 0..60 {
        for ev in s.step(0.05) {
            if matches!(ev, Event::SpellGo { caster, .. } if caster == p) {
                fired = true;
            }
        }
    }
    assert!(!fired, "spells wait for the key");
    s.step(0.05);
    assert_eq!(s.cast_slot(p, 0), Ok(()));
    let mut hit_selected = false;
    for _ in 0..90 {
        s.prime_hp_for_test(p);
        for ev in s.step(0.05) {
            if let Event::SpellHit { caster, victim, .. } = ev {
                if caster == p && victim == near {
                    hit_selected = true;
                }
                assert!(!(caster == p && victim == far), "the bolt goes to the selected enemy, not the closer one");
            }
        }
        if hit_selected {
            break;
        }
    }
    assert!(hit_selected);
    assert_eq!(s.cast_slot(p, 0), Err(CastError::NotReady), "on cooldown");
    let bar = s.spell_bar(p).unwrap();
    assert!(bar.slots[0].remaining > 0.5 && bar.slots[0].remaining <= bar.slots[0].cooldown);
    // The second key is free once the short global cooldown has passed (it has, by now); it
    // has a cast time, so while it's cast nothing else goes.
    assert_eq!(s.cast_slot(p, 1), Ok(()));
    assert!(s.casting(p));
    assert_eq!(s.cast_slot(p, 0), Err(CastError::Busy));
    // Stepping away breaks it.
    let me2 = s.entities().into_iter().find(|e| e.id == p).unwrap().pos;
    s.move_player(p, [me2[0] - 3.0, me2[1], 0.0], 0.0);
    let broke = s.step(0.05).iter().any(|e| matches!(e, Event::CastStop { caster, .. } if *caster == p));
    assert!(broke && !s.casting(p), "moving breaks the cast");
    // Back to firing on their own.
    s.set_manual(p, false);
    let mut auto = false;
    for _ in 0..200 {
        s.prime_hp_for_test(p);
        for ev in s.step(0.05) {
            if matches!(ev, Event::SpellGo { caster, .. } if caster == p) {
                auto = true;
            }
        }
    }
    assert!(auto);
    assert!(!s.guidance(p).is_empty());
}

/// Damage a second each class deals with its first spells (run by hand): against one tough
/// enemy, and against a pack.
#[test]
#[ignore]
fn class_dps_report() {
    let content = test_content();
    for (ci, class) in content.classes.iter().enumerate() {
        let kit = &content.kits[ci];
        for pack in [false, true] {
            let mut s = sim();
            let p = s.add_player("Ayla", &class.key, Luck::default());
            s.close_all_menus_for_test(p);
            s.set_level_for_test(p, 12);
            let picks: Vec<String> = std::env::var(format!("SPELLS_{}", class.key.to_uppercase()))
                .map(|v| v.split(',').map(|x| x.to_string()).collect())
                .unwrap_or_else(|_| kit.spells.iter().filter(|d| d.core).take(4).map(|d| d.key.clone()).collect());
            for k in &picks {
                s.give_test_card(p, k, 1);
            }
            s.start_now();
            for _ in 0..40 {
                s.step(0.05);
            }
            let me = s.entities().into_iter().find(|e| e.id == p).unwrap().pos;
            let mut targets = vec![s.spawn_boss_for_test(0, [me[0] + 5.0, me[1], 0.0])];
            if pack {
                for k in 0..6 {
                    targets.push(s.spawn_boss_for_test(1, [me[0] + 6.0 + k as f32, me[1] + 2.0, 0.0]));
                }
            }
            for t in &targets {
                s.tough_for_test(*t);
            }
            s.attack(p, targets[0]);
            let mut dealt = 0u64;
            let secs = 30.0;
            for _ in 0..(secs as usize * 20) {
                s.prime_hp_for_test(p);
                for ev in s.step(0.05) {
                    match ev {
                        Event::Hit { attacker, victim, damage, .. } | Event::SpellHit { caster: attacker, victim, damage, .. } => {
                            if (attacker == p || s.owner_for_test(attacker) == p) && targets.contains(&victim) {
                                dealt += damage as u64;
                            }
                        }
                        Event::CastFailed { player, error } if player == p && std::env::var_os("FAILS").is_some() => eprintln!("  {error:?}"),
                        Event::SpellGo { caster, spell, .. } if caster == p && std::env::var_os("FAILS").is_some() => eprintln!("  go {spell}"),
                        _ => {}
                    }
                }
            }
            let hit = s.hit_for_test(p);
            eprintln!(
                "{:8} {:5} {:?}: {:.0}/s ({:.2} hits/s)",
                class.key,
                if pack { "pack" } else { "one" },
                picks,
                dealt as f32 / secs,
                dealt as f32 / secs / hit.max(1.0)
            );
        }
    }
}

#[test]
fn a_base_splits_its_wave_between_its_two_roads() {
    // Every lane walks two roads: straight in, and a long bend round to the east.
    let mut a = arena();
    for l in &mut a.lanes {
        let (x, y) = (l.points[0][0], l.points[0][1]);
        let straight = l.points.clone();
        let bend = vec![[x, y, 0.0], [x + 80.0, y, 0.0], [x * 0.3 + 60.0, y * 0.3, 0.0], [x * 0.06, y * 0.06, 0.0]];
        *l = Lane::split(l.name.clone(), vec![straight, bend]);
    }
    let mut s = Sim::new(test_content(), a, Box::new(Flat(0.0)), 3);
    let p = s.add_player("Ayla", "warrior", Luck::default());
    s.move_player(p, [500.0, 500.0, 0.0], 0.0);
    s.start_now();
    let (mut straight, mut bend) = (0, 0);
    let mut speeds = (Vec::new(), Vec::new());
    for _ in 0..(20 * 20) {
        for e in s.step(0.05) {
            if let Event::MoveTo { path, speed, .. } = e {
                if path.len() == 3 {
                    straight += 1;
                    speeds.0.push(speed);
                } else if path.len() == 4 {
                    bend += 1;
                    speeds.1.push(speed);
                }
            }
        }
    }
    assert!(straight >= 1 && bend >= 1, "both roads get enemies: {straight} straight, {bend} round the bend");
    // The long road is walked faster, so both halves arrive together.
    let avg = |v: &Vec<f32>| v.iter().sum::<f32>() / v.len().max(1) as f32;
    assert!(avg(&speeds.1) > avg(&speeds.0) * 1.2, "{:?} vs {:?}", speeds.0, speeds.1);
}

#[test]
fn the_shop_sells_at_home_by_the_shopkeeper() {
    let mut s = sim();
    let p = s.add_player("Ayla", "mage", Luck::default());
    s.close_all_menus_for_test(p);
    // Away from home: nothing sells.
    s.give_gold_for_test(p, 500);
    s.move_player(p, [400.0, 400.0, 0.0], 0.0);
    assert!(!s.at_shop(p));
    let err = s.buy(p, "protector").unwrap_err();
    assert!(err.contains("Bruni Ironhammer"), "{err}");
    // The shopkeeper stands by the Ancient.
    let vendor = s.entities().into_iter().find(|e| matches!(e.what, What::Npc { npc: NpcKind::Vendor, .. })).expect("a shopkeeper");
    assert!(dist2d(vendor.pos, s.arena().hub) < 12.0);
    s.move_player(p, vendor.pos, 0.0);
    assert!(s.at_shop(p));
    s.give_gold_for_test(p, 0);
    let items = s.shop(p);
    let keys: Vec<&str> = items.iter().map(|i| i.key.as_str()).collect();
    for k in ["protector", "spring", "defender", "fortify", "strengthen", "thornbark"] {
        assert!(keys.contains(&k), "{k} in {keys:?}");
    }
    // A mage's weapon takes the schools mages cast.
    let enchants: Vec<&str> = keys.iter().filter_map(|k| k.strip_prefix("enchant:")).collect();
    assert_eq!(enchants.len(), 3, "{keys:?}");
    for e in ["fire", "frost", "arcane"] {
        assert!(enchants.contains(&e), "{enchants:?}");
    }
    // Fortify needs a defence to fortify.
    assert!(items.iter().find(|i| i.key == "fortify").unwrap().cost.is_none());

    s.give_gold_for_test(p, 2000);
    s.kill_for_test(p);
    assert!(s.respawn_in(p).is_some(), "fallen at home: still the shop's");
    let before = s.entities().into_iter().filter(|e| matches!(e.what, What::Npc { npc: NpcKind::Protector, .. })).count();
    assert_eq!(s.buy(p, "protector").unwrap(), "Treant Protector");
    let after = s.entities().into_iter().filter(|e| matches!(e.what, What::Npc { npc: NpcKind::Protector, .. })).count();
    assert_eq!(after, before + 1, "a protector stands by the Ancient");
    // Now there is something to fortify, and the next protector costs more.
    let items = s.shop(p);
    assert!(items.iter().find(|i| i.key == "fortify").unwrap().cost.is_some());
    s.buy(p, "fortify").unwrap();
    s.buy(p, "strengthen").unwrap();
    assert!(s.buy(p, "enchant:shadow").is_err(), "not offered to a mage");
    assert!(s.buy(p, "nonsense").is_err());
    for it in s.shop(p) {
        assert!(!it.text.contains(';') && !it.text.contains('^') && !it.text.contains('~'), "{}", it.text);
    }
}

#[test]
fn defences_can_be_built_at_a_bridge() {
    let mut a = arena();
    a.posts = vec![htl_post("the north bridge", [40.0, 40.0, 0.0])];
    let mut s = Sim::new(test_content(), a, Box::new(Flat(0.0)), 5);
    let p = s.add_player("Ayla", "warrior", Luck::default());
    s.close_all_menus_for_test(p);
    let standard = s.entities().into_iter().find(|e| matches!(e.what, What::Npc { npc: NpcKind::Post(0), .. })).expect("a standard at the bridge");
    assert!(dist2d(standard.pos, [40.0, 40.0, 0.0]) < 1.0);
    assert_eq!(s.site_of(standard.id), 1);
    let sites = s.shop_sites();
    assert_eq!(sites.len(), 2);
    assert_eq!(sites[1].name, "the north bridge");
    s.give_gold_for_test(p, 5000);
    assert!(s.buy(p, "protector@1").unwrap().contains("north bridge"));
    s.buy(p, "defender@1").unwrap();
    s.buy(p, "protector").unwrap();
    assert!(s.buy(p, "protector@7").is_err());
    let sites = s.shop_sites();
    assert_eq!((sites[0].protectors, sites[1].protectors, sites[1].defenders), (1, 1, 1));
    let treants: Vec<Vec3> = s.entities().into_iter().filter(|e| matches!(e.what, What::Npc { npc: NpcKind::Protector, .. })).map(|e| e.pos).collect();
    assert!(treants.iter().any(|t| dist2d(*t, [40.0, 40.0, 0.0]) < 10.0), "one by the bridge: {treants:?}");
    assert!(treants.iter().any(|t| dist2d(*t, [0.0, 0.0, 0.0]) < 15.0), "one by the Ancient: {treants:?}");
    // A run restart keeps the standard (and clears the defences).
    s.restart();
    assert!(s.entities().into_iter().any(|e| matches!(e.what, What::Npc { npc: NpcKind::Post(0), .. })));
    assert_eq!(s.shop_sites()[1].protectors, 0);
}

fn htl_post(name: &str, pos: Vec3) -> crate::Post {
    crate::Post { name: name.into(), pos, facing: 0.785 }
}

/// An arena whose lanes each split over two roads, each through its own bridge post.
fn bridged_arena() -> Arena {
    let mut a = arena();
    a.posts = vec![htl_post("the north bridge", [30.0, 0.0, 0.0]), htl_post("the south bridge", [-30.0, 0.0, 0.0])];
    for l in &mut a.lanes {
        let (x, y) = (l.points[0][0], l.points[0][1]);
        let north = vec![[x, y, 0.0], [60.0, y * 0.4, 0.0], [30.0, 0.0, 0.0], [6.0, 0.0, 0.0]];
        let south = vec![[x, y, 0.0], [-60.0, y * 0.4, 0.0], [-40.0, y * 0.2, 0.0], [-30.0, 0.0, 0.0], [-6.0, 0.0, 0.0]];
        *l = Lane::split(l.name.clone(), vec![north, south]);
    }
    a
}

#[test]
fn the_next_wave_is_shown_by_front_and_players_can_claim_one() {
    let mut s = Sim::new(test_content(), bridged_arena(), Box::new(Flat(0.0)), 9);
    let p = s.add_player("Ayla", "warrior", Luck::default());
    s.close_all_menus_for_test(p);
    let mut said = Vec::new();
    for _ in 0..4 {
        for e in s.step(0.05) {
            if let Event::Announce { text, .. } = e {
                said.push(text);
            }
        }
    }
    let preview = said.iter().find(|t| t.starts_with("Next, wave 1")).unwrap_or_else(|| panic!("a preview: {said:?}"));
    assert!(preview.contains("north bridge") || preview.contains("south bridge"), "{preview}");
    let fronts = s.fronts();
    assert_eq!(fronts.len(), 3, "the Ancient and two bridges");
    let planned: u32 = fronts.iter().map(|f| f.planned).sum();
    assert!(planned > 0);
    assert!(fronts[1].planned > 0 && fronts[2].planned > 0, "both bridges get some: {fronts:?}");
    // Claiming a front.
    assert_eq!(s.ping(p, Ping::Hold(1)).unwrap(), "Ayla will hold the north bridge.");
    assert_eq!(s.fronts()[1].holders, vec!["Ayla".to_string()]);
    assert!(s.ping(p, Ping::Hold(9)).is_none());
    s.move_player(p, [-29.0, 1.0, 0.0], 0.0);
    assert_eq!(s.ping(p, Ping::Help).unwrap(), "Ayla needs help at the south bridge!");
    // The wave goes as planned: enemies come over both bridges.
    s.start_now();
    for _ in 0..(20 * 6) {
        s.step(0.05);
    }
    let now = s.fronts();
    assert!(now[1].incoming + now[2].incoming > 0, "{now:?}");
}

#[test]
fn enemy_roles_play_their_part() {
    let mut s = sim();
    let p = s.add_player("Ayla", "warrior", Luck::default());
    s.close_all_menus_for_test(p);
    s.give_gold_for_test(p, 5000);
    s.start_now();
    s.clear_wave_for_test();

    // A siegebreaker walks past a player to get at a treant.
    s.buy(p, "protector").unwrap();
    let treant = s.entities().into_iter().find(|e| matches!(e.what, What::Npc { npc: NpcKind::Protector, .. })).unwrap();
    s.move_player(p, [treant.pos[0] + 6.0, treant.pos[1] + 6.0, 0.0], 0.0);
    let siege = s.spawn_for_test("siegebreaker", [treant.pos[0] + 20.0, treant.pos[1] + 20.0, 0.0]);
    let mut hit_treant = false;
    for _ in 0..(20 * 12) {
        for e in s.step(0.05) {
            if let Event::AttackStart { attacker, victim } = e {
                if attacker == siege {
                    assert_ne!(victim, p, "siege ignores the player");
                    hit_treant |= victim == treant.id;
                }
            }
        }
        if hit_treant {
            break;
        }
    }
    assert!(hit_treant, "the siegebreaker goes for the treant");
    s.clear_wave_for_test();

    // A bonecaster shoots from range.
    s.move_player(p, [60.0, -60.0, 0.0], 0.0);
    let archer = s.spawn_for_test("bonecaster", [60.0, -45.0, 0.0]);
    let (mut shots, mut closest) = (0, f32::MAX);
    for _ in 0..(20 * 8) {
        for e in s.step(0.05) {
            if let Event::SpellGo { caster, spell: 686, .. } = e {
                if caster == archer {
                    shots += 1;
                }
            }
        }
        if let Some(a) = s.entities().into_iter().find(|e| e.id == archer) {
            closest = closest.min(dist2d(a.pos, [60.0, -60.0, 0.0]));
        }
    }
    assert!(shots >= 2, "it shoots: {shots}");
    assert!(closest > 8.0, "from range: came within {closest}");
    s.clear_wave_for_test();

    // A dark mender heals a hurt skeleton.
    let skel = s.spawn_for_test("skeleton", [-80.0, 80.0, 0.0]);
    let _mender = s.spawn_for_test("dark_mender", [-82.0, 80.0, 0.0]);
    let (h, m) = s.health_for_test(skel);
    s.set_health_for_test(skel, m / 3);
    for _ in 0..(20 * 4) {
        s.step(0.05);
    }
    let (after, _) = s.health_for_test(skel);
    assert!(after > m / 3, "healed: {} -> {after} (of {m}, was {h})", m / 3);
    s.clear_wave_for_test();

    // A ghoul runner runs straight past a player in its way.
    s.move_player(p, [70.0, 70.0, 0.0], 0.0);
    let runner = s.spawn_for_test("ghoul_runner", [80.0, 80.0, 0.0]);
    for _ in 0..(20 * 3) {
        for e in s.step(0.05) {
            if let Event::AttackStart { attacker, victim } = e {
                assert!(!(attacker == runner && victim == p), "the runner doesn't stop for the player");
            }
        }
    }
    let r = s.entities().into_iter().find(|e| e.id == runner).unwrap();
    assert!(dist2d(r.pos, [0.0, 0.0, 0.0]) < 80.0, "it ran on: {:?}", r.pos);
}

#[test]
fn flankers_take_the_long_road() {
    let mut s = Sim::new(test_content(), bridged_arena(), Box::new(Flat(0.0)), 4);
    let p = s.add_player("Ayla", "warrior", Luck::default());
    s.close_all_menus_for_test(p);
    let longest: Vec<usize> = s
        .arena()
        .lanes
        .iter()
        .map(|l| (0..l.routes.len()).max_by(|&a, &b| crate::geom::path_len(&l.routes[a]).total_cmp(&crate::geom::path_len(&l.routes[b]))).unwrap())
        .collect();
    let mut seen = 0;
    for n in 8..40 {
        for (key, lane, route) in s.plan_for_test(n) {
            if key == "ghoul_runner" {
                assert_eq!(route, longest[lane]);
                seen += 1;
            }
        }
    }
    assert!(seen > 0, "some runners in 30 waves");
}

#[test]
fn holding_the_final_wave_wins_the_run_and_the_waves_go_on() {
    let mut content = test_content();
    content.game.victory_wave = 2;
    let mut s = Sim::new(content, arena(), Box::new(Flat(0.0)), 2);
    let p = s.add_player("Ayla", "warrior", Luck::default());
    s.close_all_menus_for_test(p);
    let mut won = None;
    let mut banners = Vec::new();
    for wave in 1..=3 {
        s.start_now();
        s.clear_wave_for_test();
        for _ in 0..4 {
            for e in s.step(0.05) {
                match e {
                    Event::Victory(r) => won = Some((wave, r)),
                    Event::Splash { style, title, .. } => banners.push((style, title)),
                    _ => {}
                }
            }
        }
        s.close_all_menus_for_test(p);
    }
    let (wave, r) = won.expect("a victory");
    assert_eq!(wave, 2);
    assert!(r.won && r.depth == 2);
    assert!(s.won());
    assert!(banners.iter().any(|(st, t)| *st == Splash::Victory && t == "Victory!"));
    assert!(banners.iter().any(|(_, t)| t == "Final Wave 2"), "{banners:?}");
    assert!(!banners.iter().any(|(_, t)| t == "Wave 2 Cleared"), "the victory banner replaces it");
    assert_eq!(s.status().wave, 3, "the waves go on");
    assert!(banners.iter().filter(|(st, _)| *st == Splash::Victory).count() == 1, "won once");
}

#[test]
fn choosing_a_card_pauses_the_world_until_it_is_chosen() {
    // The real content: the pause is on.
    let mut s = Sim::new(Content::builtin(), arena(), Box::new(Flat(0.0)), 6);
    let p = s.add_player("Ayla", "mage", Luck::default());
    // The starting pick holds the countdown.
    let mut paused_for = None;
    for e in s.step(0.05) {
        if let Event::Paused { choosing } = e {
            paused_for = Some(choosing);
        }
    }
    assert_eq!(paused_for, Some(vec!["Ayla".to_string()]));
    let left = s.status().seconds_left;
    for _ in 0..100 {
        s.step(0.05);
    }
    assert_eq!(s.status().seconds_left, left, "the countdown waits");
    assert_eq!(s.paused_for(), vec!["Ayla".to_string()]);
    // Closing the window doesn't put it off: it comes straight back.
    s.menu_closed(p);
    let reopened = s.step(0.05).iter().any(|e| matches!(e, Event::Menu { player, menu: Some(_) } if *player == p));
    assert!(reopened, "the pick comes back");
    // Another window opened over it (the Ancient's) gives way to the pick too.
    let hub = s.hub_id();
    s.move_player(p, s.arena().hub, 0.0);
    s.interact(p, hub);
    s.step(0.05);
    let m = s.menu(p).expect("a window");
    assert!(m.offer.is_some(), "the pick, not the Ancient's window: {:?}", m.kind);
    // Choosing lets the world go on.
    s.menu_select(p, 0);
    let resumed = s.step(0.05).iter().any(|e| matches!(e, Event::Paused { choosing } if choosing.is_empty()));
    assert!(resumed);
    s.start_now();
    for _ in 0..(20 * 4) {
        s.step(0.05);
    }
    // A level-up mid-wave: enemies stop where they stand, and walk on after the pick.
    let before: Vec<(EntityId, Vec3)> = s.entities().into_iter().filter(|e| matches!(e.what, What::Enemy { .. })).map(|e| (e.id, e.pos)).collect();
    assert!(!before.is_empty());
    s.set_level_for_test(p, 2);
    let ev = s.step(0.05);
    assert!(ev.iter().any(|e| matches!(e, Event::Stop { .. })), "moving enemies stop on screen");
    for _ in 0..60 {
        s.step(0.05);
    }
    let still: Vec<(EntityId, Vec3)> = s.entities().into_iter().filter(|e| matches!(e.what, What::Enemy { .. })).map(|e| (e.id, e.pos)).collect();
    for (id, pos) in &still {
        let was = before.iter().find(|b| b.0 == *id).map(|b| b.1);
        if let Some(was) = was {
            assert!(dist2d(was, *pos) < 1.5, "{id} held still");
        }
    }
    s.menu_select(p, 0);
    let ev = s.step(0.05);
    assert!(ev.iter().any(|e| matches!(e, Event::MoveTo { .. })), "they walk on");
    // A pick left alone is made for the player in the end.
    s.set_level_for_test(p, 3);
    let mut auto = false;
    for _ in 0..(20 * 65) {
        for e in s.step(0.05) {
            if let Event::Announce { text, .. } = e {
                auto |= text.contains("took too long");
            }
        }
    }
    assert!(auto, "the first card is taken for an idle player");
    assert!(s.paused_for().is_empty());
}

#[test]
fn spells_swap_keys() {
    let mut s = sim();
    let p = s.add_player("Ayla", "mage", Luck::default());
    s.close_all_menus_for_test(p);
    s.give_test_card(p, "frostbolt", 0);
    s.give_test_card(p, "fireball", 0);
    s.give_test_card(p, "frost_nova", 0);
    let keys = |s: &Sim| s.spell_bar(p).unwrap().slots.iter().map(|b| b.key.clone()).collect::<Vec<_>>();
    let start = keys(&s);
    assert_eq!(&start[..3], &["frostbolt", "fireball", "frost_nova"]);
    assert!(s.move_spell(p, 0, 2));
    assert_eq!(&keys(&s)[..3], &["frost_nova", "fireball", "frostbolt"]);
    assert!(!s.move_spell(p, 0, 9), "no spell there");
    // Casting follows the new order.
    let bar = s.spell_bar(p).unwrap();
    assert_eq!(bar.slots[2].key, "frostbolt");
    // A spell a talent taught takes the next key, and swaps with the cards' spells.
    s.give_test_talent(p, "presence_of_mind");
    assert_eq!(keys(&s).len(), 4);
    assert!(s.move_spell(p, 3, 0));
    assert_eq!(keys(&s)[0], "presence_of_mind");
    // A spell moves onto an empty key too (key 6, E), leaving its own empty.
    assert!(s.move_spell(p, 1, 5));
    assert_eq!(keys(&s), ["presence_of_mind", "", "frostbolt", "frost_nova", "", "fireball"].map(String::from).to_vec());
    assert!(!s.move_spell(p, 1, 2), "nothing on key 2 to move");
    assert!(!s.move_spell(p, 0, 6), "only six keys");
    // The next spell learned fills the first empty key.
    s.give_test_card(p, "arcane_missiles", 0);
    assert_eq!(keys(&s)[1], "arcane_missiles");
}

/// Six spells fill the bar, whichever way they came: no more spell cards, and no talents that
/// would teach a seventh (upgrades and passive talents still come).
#[test]
fn six_spells_fill_the_bar() {
    let mut s = sim();
    let p = s.add_player("Ayla", "mage", Luck::default());
    s.close_all_menus_for_test(p);
    for k in ["fireball", "frostbolt", "arcane_missiles", "scorch"] {
        s.give_test_card(p, k, 1);
    }
    s.give_test_talent(p, "presence_of_mind");
    s.give_test_talent(p, "arcane_power");
    assert_eq!(s.spell_bar(p).unwrap().slots.len(), 6);
    let class = s.content().class_index("mage").unwrap();
    let teaching: Vec<String> = s.content().kit(class).talents.iter().filter(|t| t.grants.is_some()).map(|t| format!("talent:{}", t.key)).collect();
    let mut upgrades = 0;
    for _ in 0..200 {
        for key in s.roll_for_test(p) {
            assert!(!teaching.contains(&key), "{key} would teach a seventh spell");
            match key.split_once(':') {
                Some(("talent", _)) => {}
                Some(_) => upgrades += 1,
                None => panic!("{key} would take a seventh spell slot"),
            }
        }
    }
    assert!(upgrades > 0);
}

/// How the hero's power and the enemies' toughness move together over a run (run by hand):
/// per wave, the level, a hit against a normal enemy's health, how long the wave took, the
/// Ancient's health and the hero's deaths.
#[test]
#[ignore]
fn power_curve() {
    let waves: u32 = std::env::var("WAVES").ok().and_then(|w| w.parse().ok()).unwrap_or(20);
    let classes: Vec<String> = std::env::var("CLASSES").map(|c| c.split(',').map(String::from).collect()).unwrap_or(vec!["mage".into(), "warrior".into()]);
    for class in classes {
        let mut s = Sim::new(test_content(), bridged_arena(), Box::new(Flat(0.0)), 11);
        let p = s.add_player("Solo", &class, Luck::default());
        let mut bots = vec![Bot::new(p, Policy::Defend)];
        let _ = s.step(0.0);
        s.start_now();
        let mut last_wave = 0;
        let mut wave_started = 0.0f32;
        let mut t = 0.0f32;
        let mut deaths = 0;
        let mut hub_low = 100;
        eprintln!("{class}: wave level  hit  enemyHP  hits/kill  secs  hub%  deaths");
        while t < 60.0 * 60.0 {
            bot_turn(&mut s, &mut bots[0], 0.05);
            for e in s.step(0.05) {
                if let Event::Died { id, .. } = e {
                    if id == p {
                        deaths += 1;
                    }
                }
            }
            t += 0.05;
            let st = s.status();
            hub_low = hub_low.min(st.hub_health * 100 / st.hub_max_health.max(1));
            if st.phase == PhaseName::Wave && st.wave != last_wave {
                last_wave = st.wave;
                wave_started = t;
            }
            if st.phase == PhaseName::Intermission && st.wave == last_wave && wave_started > 0.0 {
                let (hit, hp) = s.power_for_test(p);
                let level = s.player_view(p).map_or(0, |v| v.level);
                eprintln!("{class}: {:>4} {:>5} {:>5.0} {:>8.0} {:>9.1} {:>5.0} {:>5} {:>6}", st.wave, level, hit, hp, hp / hit.max(1.0), t - wave_started, hub_low, deaths);
                wave_started = 0.0;
                hub_low = 100;
                deaths = 0;
                if st.wave >= waves {
                    break;
                }
                s.start_now();
            }
            if st.phase == PhaseName::Defeat {
                eprintln!("{class}: defeat on wave {}", st.wave);
                break;
            }
        }
    }
}

#[test]
fn ordinary_enemies_throw_strikes_you_can_step_out_of() {
    let mut s = sim();
    let p = s.add_player("Ayla", "warrior", Luck::default());
    s.close_all_menus_for_test(p);
    s.start_now();
    s.clear_wave_for_test();
    s.move_player(p, [60.0, -60.0, 0.0], 0.0);
    let ghoul = s.spawn_for_test("ghoul", [60.0, -48.0, 0.0]);
    // It marks the ground under the player.
    let mut marked = None;
    for _ in 0..(20 * 6) {
        s.prime_hp_for_test(p);
        for e in s.step(0.05) {
            if let Event::ZoneStart { caster, pos, .. } = e {
                if caster == ghoul {
                    marked = Some(pos);
                }
            }
        }
        if marked.is_some() {
            break;
        }
    }
    let spot = marked.expect("the ghoul pounces");
    assert!(dist2d(spot, [60.0, -60.0, 0.0]) < 1.5, "at the player's feet");
    // Stepping out of it: no damage from it.
    s.move_player(p, [70.0, -60.0, 0.0], 0.0);
    let mut hurt = false;
    let mut dodged = false;
    for _ in 0..40 {
        for e in s.step(0.05) {
            if let Event::Dodged { player, .. } = &e {
                dodged |= *player == p;
            }
            if let Event::Hit { attacker, victim, .. } = e {
                if attacker == ghoul && victim == p {
                    if let Some(me) = s.entities().into_iter().find(|x| x.id == p) {
                        hurt |= dist2d(me.pos, spot) < 3.5;
                    }
                }
            }
        }
    }
    assert!(!hurt, "out of the circle, out of harm");
    assert!(dodged, "and told so: dodged!");
    s.clear_wave_for_test();
    // A bloat killed before it bursts takes the burst with it.
    s.move_player(p, [-60.0, 60.0, 0.0], 0.0);
    let bloat = s.spawn_for_test("plague_bloat", [-60.0, 64.0, 0.0]);
    let mut started = false;
    for _ in 0..(20 * 3) {
        for e in s.step(0.05) {
            if matches!(e, Event::ZoneStart { caster, .. } if caster == bloat) {
                started = true;
            }
        }
        if started {
            break;
        }
    }
    assert!(started, "the bloat starts to burst");
    s.clear_wave_for_test();
    let mut burst = false;
    for _ in 0..60 {
        for e in s.step(0.05) {
            if matches!(e, Event::Hit { victim, .. } if victim == p) {
                burst = true;
            }
        }
    }
    assert!(!burst, "killed in time, it never bursts");
}

/// A ring is safe inside, by the caster's feet; a player standing where two of its marks overlap
/// is hit once. A line hits along its length and not beside it.
#[test]
fn rings_are_safe_inside_and_lines_beside() {
    let mut s = sim();
    let near = s.add_player("Near", "warrior", Luck::default());
    let on = s.add_player("On", "warrior", Luck::default());
    s.close_all_menus_for_test(near);
    s.close_all_menus_for_test(on);
    s.start_now();
    s.clear_wave_for_test();
    let c = [40.0, 40.0, 0.0];
    let lich = s.spawn_for_test("lich", c);
    s.move_player(near, [c[0] + 2.0, c[1], 0.0], 0.0);
    s.move_player(on, [c[0], c[1] + 12.0, 0.0], 0.0);
    // Wait for the ring: many marks at once.
    let mut ring_lands = None;
    for _ in 0..(20 * 60) {
        s.prime_hp_for_test(near);
        s.prime_hp_for_test(on);
        let evs = s.step(0.05);
        let marks = evs.iter().filter(|e| matches!(e, Event::ZoneStart { caster, .. } if *caster == lich)).count();
        if marks >= 6 {
            ring_lands = Some(());
            break;
        }
        // Keep them where they stand (the lich would walk away to the hub).
        s.move_player(near, [c[0] + 2.0, c[1], 0.0], 0.0);
        s.move_player(on, [c[0], c[1] + 12.0, 0.0], 0.0);
    }
    assert!(ring_lands.is_some(), "the Archlich casts its ring");
    let mut frost: std::collections::HashMap<EntityId, u32> = std::collections::HashMap::new();
    for _ in 0..(20 * 4) {
        s.prime_hp_for_test(near);
        s.prime_hp_for_test(on);
        for e in s.step(0.05) {
            if let Event::SpellHit { caster, victim, spell: 10, .. } = e {
                if caster == lich {
                    *frost.entry(victim).or_default() += 1;
                }
            }
        }
    }
    assert_eq!(frost.get(&near).copied().unwrap_or(0), 0, "inside the ring is safe");
    assert_eq!(frost.get(&on).copied().unwrap_or(0), 1, "on the ring: hit once, however many marks overlap");
    s.clear_wave_for_test();

    // A charger tramples down a line towards the nearest player.
    let c = [-40.0, -40.0, 0.0];
    let (ahead, beside) = ([c[0] + 10.0, c[1], 0.0], [c[0] + 8.0, c[1] + 8.0, 0.0]);
    let steed = s.spawn_for_test("skeletal_steed", c);
    let mut hits: std::collections::HashMap<EntityId, u32> = std::collections::HashMap::new();
    let mut lined = false;
    for _ in 0..(20 * 8) {
        s.prime_hp_for_test(near);
        s.prime_hp_for_test(on);
        s.move_player(near, ahead, 0.0);
        s.move_player(on, beside, 0.0);
        for e in s.step(0.05) {
            match e {
                Event::ZoneStart { caster, .. } if caster == steed => lined = true,
                Event::SpellHit { caster, victim, spell: 2120, .. } if caster == steed => *hits.entry(victim).or_default() += 1,
                _ => {}
            }
        }
    }
    assert!(lined, "the charger tramples");
    assert!(hits.get(&near).copied().unwrap_or(0) >= 1, "in the line: run down");
    assert_eq!(hits.get(&on).copied().unwrap_or(0), 0, "beside the line: safe");
}

/// A boss's loot needs no clicking: its gold and the better of its pieces go straight on.
#[test]
fn boss_loot_goes_straight_to_the_bags() {
    let mut s = sim();
    let p = s.add_player("Ayla", "mage", Luck::default());
    s.close_all_menus_for_test(p);
    s.start_now();
    s.clear_wave_for_test();
    s.move_player(p, [30.0, 30.0, 0.0], 0.0);
    let gold = s.player_view(p).unwrap().gold;
    let worn = s.player_view(p).unwrap().equip.clone();
    let bagged = s.player_view(p).unwrap().pack.len();
    s.spawn_for_test("skeleton_lord", [32.0, 30.0, 0.0]);
    s.clear_wave_for_test();
    let mut lootable = false;
    for _ in 0..10 {
        for e in s.step(0.05) {
            if matches!(e, Event::Lootable { lootable: true, .. }) {
                lootable = true;
            }
        }
    }
    let v = s.player_view(p).unwrap();
    assert!(!lootable, "no corpse to click");
    assert!(v.gold > gold, "the boss's gold");
    assert_eq!(v.equip, worn, "nothing put on by itself");
    assert!(v.pack.len() > bagged, "an epic piece in the bags");
}

/// A necromancer raises skeletons while it fights, a few at a time; a bone construct falls
/// apart into skeletons when it dies.
#[test]
fn summoners_raise_the_dead_and_constructs_split() {
    let mut s = sim();
    let p = s.add_player("Ayla", "warrior", Luck::default());
    s.close_all_menus_for_test(p);
    s.start_now();
    s.clear_wave_for_test();
    let c = [50.0, -50.0, 0.0];
    s.move_player(p, c, 0.0);
    let necro = s.spawn_for_test("necromancer", [c[0] + 10.0, c[1], 0.0]);
    let skel = s.content().enemies.iter().position(|e| e.key == "skeleton").unwrap();
    let skeletons = |s: &Sim| {
        s.entities()
            .into_iter()
            .filter(|e| e.alive && matches!(e.what, What::Enemy { kind, .. } if !kind.boss && kind.index == skel))
            .count()
    };
    let mut most = 0;
    for _ in 0..(20 * 40) {
        s.prime_hp_for_test(p);
        s.move_player(p, c, 0.0);
        s.step(0.05);
        most = most.max(skeletons(&s));
    }
    assert!(most >= 2, "it raises skeletons ({most})");
    assert!(most <= 4, "no more than four at once ({most})");
    let _ = necro;
    s.clear_wave_for_test();
    s.step(0.05);
    assert_eq!(skeletons(&s), 0);
    let construct = s.spawn_for_test("bone_construct", [c[0] + 10.0, c[1], 0.0]);
    s.step(0.05);
    s.kill_for_test(construct);
    s.step(0.05);
    assert_eq!(skeletons(&s), 3, "it falls apart into three");
}

/// A boss calls its dead at half health and rages at a quarter.
#[test]
fn bosses_call_their_dead_then_rage() {
    let mut s = sim();
    let p = s.add_player("Ayla", "warrior", Luck::default());
    s.close_all_menus_for_test(p);
    s.start_now();
    s.clear_wave_for_test();
    let boss = s.spawn_for_test("skeleton_lord", [60.0, 60.0, 0.0]);
    let skel = s.content().enemies.iter().position(|e| e.key == "skeleton").unwrap();
    let skeletons = |s: &Sim| {
        s.entities()
            .into_iter()
            .filter(|e| e.alive && matches!(e.what, What::Enemy { kind, .. } if !kind.boss && kind.index == skel))
            .count()
    };
    s.strike_for_test(boss, 0.3);
    assert_eq!(skeletons(&s), 0, "not yet");
    s.strike_for_test(boss, 0.25);
    assert_eq!(skeletons(&s), 2, "at half health it calls skeletons: two for one hero");
    let mut raged = false;
    s.strike_for_test(boss, 0.3);
    for e in s.step(0.05) {
        if let Event::Announce { text, .. } = e {
            raged |= text.contains("enraged");
        }
    }
    assert!(raged, "at a quarter it rages");
    assert_eq!(skeletons(&s), 2, "only once");
}

/// Who does the killing, wave by wave, when four bots spend all their gold on defences at
/// the Ancient between waves (run by hand): the defences should help, not carry.
#[test]
#[ignore]
fn defence_share_report() {
    let field = if std::env::var_os("BRIDGED").is_some() { bridged_arena() } else { arena() };
    let mut content = test_content();
    content.game.wave_events = std::env::var_os("EVENTS").is_some();
    let mut s = Sim::new(content, field, Box::new(Flat(0.0)), 1);
    let mut bots = four_bots(&mut s, Policy::Defend);
    let _ = s.step(0.0);
    s.start_now();
    let waves: u32 = std::env::var("WAVES").ok().and_then(|w| w.parse().ok()).unwrap_or(12);
    let players: Vec<EntityId> = bots.iter().map(|b| b.id).collect();
    let (mut by_players, mut by_defences, mut by_other) = (0u64, 0u64, 0u64);
    let (mut hub_low, mut deaths) = (100u32, 0u32);
    let mut last_wave = 0;
    let mut t = 0.0;
    while t < 4000.0 {
        for b in bots.iter_mut() {
            bot_turn(&mut s, b, 0.05);
        }
        let views = s.entities();
        for e in s.step(0.05) {
            if let Event::Died { id } = e {
                deaths += u32::from(players.contains(&id));
            }
            let (who, victim, dmg) = match e {
                Event::Hit { attacker, victim, damage, .. } => (attacker, victim, damage),
                Event::SpellHit { caster, victim, damage, .. } => (caster, victim, damage),
                _ => continue,
            };
            if !views.iter().any(|v| v.id == victim && matches!(v.what, What::Enemy { .. })) {
                continue;
            }
            let owner = views.iter().find(|v| v.id == who).map(|v| v.what.clone());
            match owner {
                Some(What::Player { .. }) => by_players += dmg as u64,
                Some(What::Ally { owner, .. }) if players.contains(&owner) => by_players += dmg as u64,
                Some(What::Npc { .. }) | Some(What::Ally { .. }) | Some(What::Hub) => by_defences += dmg as u64,
                _ => by_other += dmg as u64,
            }
        }
        t += 0.05;
        let st = s.status();
        hub_low = hub_low.min(st.hub_health * 100 / st.hub_max_health.max(1));
        if st.phase == PhaseName::Intermission {
            // Spend everything on what's cheapest at the Ancient (NOBUY=1: nothing).
            for &p in players.iter().filter(|_| std::env::var_os("NOBUY").is_none()) {
                go_home(&mut s, p);
                loop {
                    let items = s.shop(p);
                    let best = items.iter().filter(|i| i.group == "Defence" && i.cost.is_some_and(|c| c <= s.player_view(p).unwrap().gold)).min_by_key(|i| i.cost);
                    let Some(k) = best.map(|i| i.key.clone()) else { break };
                    if s.buy(p, &k).is_err() {
                        break;
                    }
                }
            }
            if st.wave != last_wave {
                let total = (by_players + by_defences + by_other).max(1);
                eprintln!(
                    "wave {:2}: players {:3}%  defences {:3}%  other {:3}%   hub low {hub_low:3}%  hero deaths {deaths}",
                    st.wave,
                    by_players * 100 / total,
                    by_defences * 100 / total,
                    by_other * 100 / total
                );
                hub_low = 100;
                deaths = 0;
                by_players = 0;
                by_defences = 0;
                by_other = 0;
                last_wave = st.wave;
            }
            if st.seconds_left > 3 {
                s.start_now();
            }
        }
        if st.phase == PhaseName::Defeat || st.wave > waves {
            eprintln!("{st:?}");
            break;
        }
    }
}

/// Elites of a wave carry a twist; a thorned one hurts what hits it, a vampiric one heals.
#[test]
fn elite_twists_bite() {
    let mut s = sim();
    let p = s.add_player("Ayla", "warrior", Luck::default());
    s.close_all_menus_for_test(p);
    s.start_now();
    s.clear_wave_for_test();
    let c = [30.0, -30.0, 0.0];
    s.move_player(p, c, 0.0);
    let thorned = s.spawn_for_test("golem", [c[0] + 2.0, c[1], 0.0]);
    s.twist_for_test(thorned, 4);
    s.tough_for_test(thorned);
    s.attack(p, thorned);
    let mut stung = false;
    for _ in 0..(20 * 6) {
        s.prime_hp_for_test(p);
        s.move_player(p, c, 0.0);
        for e in s.step(0.05) {
            if let Event::SpellHit { caster, victim, spell: 467, .. } = e {
                stung |= caster == thorned && victim == p;
            }
        }
    }
    assert!(stung, "the thorned elite pays back a hit");
    s.clear_wave_for_test();
    let vamp = s.spawn_for_test("golem", [c[0] + 2.0, c[1], 0.0]);
    s.twist_for_test(vamp, 1);
    s.hurt_for_test(vamp, 0.5);
    let before = s.entities().into_iter().find(|e| e.id == vamp).unwrap().health;
    for _ in 0..(20 * 6) {
        s.prime_hp_for_test(p);
        s.move_player(p, c, 0.0);
        s.step(0.05);
    }
    let after = s.entities().into_iter().find(|e| e.id == vamp).unwrap().health;
    assert!(after > before, "the vampire drinks ({before} -> {after})");
}

/// Arcane Missiles is channelled: its missiles fly over two seconds while the mage stands
/// still, and stepping away loses the rest.
#[test]
fn channels_lose_their_missiles_when_you_move() {
    for moves in [false, true] {
        let mut s = sim();
        let p = s.add_player("Ayla", "mage", Luck::default());
        s.close_all_menus_for_test(p);
        s.give_test_card(p, "arcane_missiles", 0);
        s.set_manual(p, true);
        s.start_now();
        s.clear_wave_for_test();
        let c = [20.0, 20.0, 0.0];
        s.move_player(p, c, 0.0);
        let dummy = s.spawn_for_test("golem", [c[0] + 15.0, c[1], 0.0]);
        s.tough_for_test(dummy);
        s.step(0.05);
        let mut cast = false;
        for _ in 0..200 {
            s.move_player(p, c, 0.0);
            if s.cast_slot(p, 0).is_ok() {
                cast = true;
                break;
            }
            s.step(0.05);
        }
        assert!(cast, "it casts");
        let mut hits = 0;
        for k in 0..60 {
            if moves && k == 10 {
                s.move_player(p, [c[0] - 5.0, c[1], 0.0], 0.0);
            }
            for e in s.step(0.05) {
                if let Event::SpellHit { caster, victim, .. } = e {
                    if caster == p && victim == dummy {
                        hits += 1;
                    }
                }
            }
        }
        if moves {
            assert!(hits < 5, "moving loses the rest ({hits} hit)");
        } else {
            assert_eq!(hits, 5, "standing still, all five land");
        }
    }
}

/// Two heroes side by side stand Shoulder to Shoulder: more damage, less taken.
#[test]
fn heroes_together_stand_shoulder_to_shoulder() {
    let mut s = sim();
    let a = s.add_player("Ayla", "warrior", Luck::default());
    let b = s.add_player("Bo", "priest", Luck::default());
    s.close_all_menus_for_test(a);
    s.close_all_menus_for_test(b);
    s.move_player(a, [10.0, 10.0, 0.0], 0.0);
    s.move_player(b, [60.0, 10.0, 0.0], 0.0);
    s.step(0.05);
    let apart = s.hit_for_test(a);
    s.move_player(b, [15.0, 10.0, 0.0], 0.0);
    let mut shown = false;
    for e in s.step(0.05) {
        if let Event::Aura { unit, spell: 20217, .. } = e {
            shown |= unit == a;
        }
    }
    assert!(shown, "the buff shows");
    let together = s.hit_for_test(a);
    assert!((together / apart - 1.1).abs() < 0.01, "{apart} -> {together}");
}

/// The scoreboard counts what each hero dealt and how often they fell.
#[test]
fn the_scoreboard_counts_damage_and_falls() {
    let mut s = sim();
    let p = s.add_player("Ayla", "warrior", Luck::default());
    s.close_all_menus_for_test(p);
    s.start_now();
    s.clear_wave_for_test();
    let c = [30.0, 30.0, 0.0];
    s.move_player(p, c, 0.0);
    let dummy = s.spawn_for_test("golem", [c[0] + 2.0, c[1], 0.0]);
    s.tough_for_test(dummy);
    s.attack(p, dummy);
    for _ in 0..100 {
        s.prime_hp_for_test(p);
        s.move_player(p, c, 0.0);
        s.step(0.05);
    }
    s.kill_for_test(p);
    let r = s.results_for_test();
    assert!(r[0].dealt > 0, "damage counted");
    assert_eq!(r[0].falls, 1);
}

/// With every hero away, a hit on the Ancient is called out with where it comes from.
#[test]
fn an_unguarded_ancient_calls_for_help() {
    let mut s = sim();
    let p = s.add_player("Ayla", "warrior", Luck::default());
    s.close_all_menus_for_test(p);
    s.start_now();
    s.clear_wave_for_test();
    s.move_player(p, [200.0, 200.0, 0.0], 0.0);
    let hub = s.arena().hub;
    let ghoul = s.spawn_for_test("ghoul", [hub[0] + 3.0, hub[1], 0.0]);
    let mut called = None;
    for _ in 0..(20 * 15) {
        s.prime_hp_for_test(p);
        for e in s.step(0.05) {
            if let Event::Announce { text, .. } = e {
                if text.contains("under attack") {
                    called = Some(text);
                }
            }
        }
        if called.is_some() {
            break;
        }
    }
    let _ = ghoul;
    assert!(called.as_deref().is_some_and(|t| t.contains("from the north")), "{called:?}");
}

/// A wraith pulses death around itself: close is hurt, far is safe.
#[test]
fn wraiths_pulse_death_nearby() {
    let mut s = sim();
    let near = s.add_player("Near", "warrior", Luck::default());
    let far = s.add_player("Far", "warrior", Luck::default());
    s.close_all_menus_for_test(near);
    s.close_all_menus_for_test(far);
    s.start_now();
    s.clear_wave_for_test();
    let c = [-30.0, 40.0, 0.0];
    let w = s.spawn_for_test("wraith", c);
    let mut hit: std::collections::HashMap<EntityId, u32> = Default::default();
    for _ in 0..(20 * 5) {
        s.prime_hp_for_test(near);
        s.prime_hp_for_test(far);
        s.move_player(near, [c[0] + 2.0, c[1], 0.0], 0.0);
        s.move_player(far, [c[0] + 12.0, c[1], 0.0], 0.0);
        for e in s.step(0.05) {
            if let Event::SpellHit { caster, victim, spell: 1395, .. } = e {
                if caster == w {
                    *hit.entry(victim).or_default() += 1;
                }
            }
        }
    }
    assert!(hit.get(&near).copied().unwrap_or(0) >= 3, "close: pulsed ({hit:?})");
    assert_eq!(hit.get(&far).copied().unwrap_or(0), 0, "far: safe");
}

/// Wave 7 is the Swarm (twice as many, half as tough, no elites); wave 12 the Vanguard (all elites).
#[test]
fn some_waves_come_differently() {
    let mut c = test_content();
    c.game.wave_events = true;
    let mut s = Sim::new(c, arena(), Box::new(Flat(0.0)), 7);
    let p = s.add_player("Ayla", "warrior", Luck::default());
    s.close_all_menus_for_test(p);
    let normal = s.plan_kinds_for_test(6);
    let swarm = s.plan_kinds_for_test(7);
    let vanguard = s.plan_kinds_for_test(12);
    let count = |pl: &[(bool, bool)]| pl.iter().filter(|(boss, _)| !boss).count();
    assert!(count(&swarm) as f32 >= count(&normal) as f32 * 1.7, "the Swarm is bigger: {} vs {}", count(&swarm), count(&normal));
    assert!(swarm.iter().all(|(_, elite)| !elite), "no elites in the Swarm");
    assert!(vanguard.iter().filter(|(boss, _)| !boss).all(|(_, elite)| *elite), "the Vanguard is all elites");
    assert!(count(&vanguard) < count(&normal), "and fewer");
}

/// Wrath of the Ancient: once a wave, everything near it rooted and torn.
#[test]
fn the_ancients_wrath_once_a_wave() {
    let mut s = sim();
    let p = s.add_player("Ayla", "warrior", Luck::default());
    s.close_all_menus_for_test(p);
    s.give_gold_for_test(p, 5000);
    assert!(s.buy(p, "wrath").is_err(), "only during a wave");
    s.start_now();
    s.step(0.05);
    let hub = s.arena().hub;
    let near = s.spawn_for_test("golem", [hub[0] + 10.0, hub[1], 0.0]);
    let far = s.spawn_for_test("golem", [hub[0] + 80.0, hub[1], 0.0]);
    let hp = |s: &Sim, id| s.entities().into_iter().find(|e| e.id == id).unwrap().health;
    let (n0, f0) = (hp(&s, near), hp(&s, far));
    s.buy(p, "wrath").expect("bought");
    assert!(hp(&s, near) < n0, "near: torn");
    assert_eq!(hp(&s, far), f0, "far: untouched");
    assert!(s.buy(p, "wrath").is_err(), "once a wave");
}

/// The playtest, replayed: a hunter and a warrior who take bases from wave 3, spend their gold
/// at the Ancient and use the whole break. Per wave: levels, experience (in waves and between),
/// what a hero's hit does, what an enemy has and what its hits do to a hero.
/// `cargo test -p htl-sim --release playtest_report -- --ignored --nocapture` (WAVES=n, CLASSES=a,b)
#[test]
#[ignore]
fn playtest_report() {
    let waves: u32 = std::env::var("WAVES").ok().and_then(|w| w.parse().ok()).unwrap_or(25);
    let classes: Vec<String> =
        std::env::var("CLASSES").map(|c| c.split(',').map(String::from).collect()).unwrap_or(vec!["hunter".into(), "warrior".into()]);
    let mut content = test_content();
    content.game.wave_events = true;
    let seed = std::env::var("SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(5);
    let mut s = Sim::new(content, bridged_arena(), Box::new(Flat(0.0)), seed);
    // The first hero takes the bases, the others hold the Ancient.
    let mut bots: Vec<Bot> = classes
        .iter()
        .enumerate()
        .map(|(i, c)| Bot::new(s.add_player(&format!("P{i}"), c, Luck::default()), if i == 0 && std::env::var_os("DEFEND").is_none() { Policy::Expedition(3) } else { Policy::Defend }))
        .collect();
    let players: Vec<EntityId> = bots.iter().map(|b| b.id).collect();
    let _ = s.step(0.0);
    s.start_now();
    let mut t = 0.0f32;
    let mut last_wave = 0;
    #[derive(Default)]
    struct W {
        xp_wave: u64,
        xp_break: u64,
        kills: u32,
        hero_hits: (u64, u32, u32),
        enemy_hits: (u64, u32, u32),
        enemy_hp: (u64, u32),
        deaths: u32,
        hub_low: u32,
        kinds: std::collections::BTreeMap<String, u32>,
        earned: u64,
        torn: u32,
    }
    let mut last_gold = 0u32;
    let mut w = W { hub_low: 100, ..W::default() };
    eprintln!("wave cyc  lvls        xp(wave/break) kills  heroHit(avg/max)   enemyHP  enemyHit(avg/max) heroHP   deaths hub%");
    while t < 3.0 * 3600.0 {
        for b in bots.iter_mut() {
            bot_turn(&mut s, b, 0.05);
        }
        let views = s.entities();
        let st = s.status();
        let in_wave = st.phase == PhaseName::Wave;
        for e in s.step(0.05) {
            match e {
                Event::Announce { text, .. } if text.contains("torn down") => w.torn += 1,
                Event::Gold { player, gold } if player == players[0] => {
                    if gold > last_gold {
                        w.earned += (gold - last_gold) as u64;
                    }
                    last_gold = gold;
                }
                Event::Xp { player, amount, .. } if player == players[0] => {
                    if in_wave {
                        w.xp_wave += amount as u64
                    } else {
                        w.xp_break += amount as u64
                    }
                }
                Event::Died { id } => {
                    if players.contains(&id) {
                        w.deaths += 1;
                        if std::env::var_os("WHO").is_some() {
                            let killer = views.iter().filter(|v| v.attacking == Some(id)).map(|v| format!("{:?}", v.what)).collect::<Vec<_>>();
                            eprintln!("   {} died at {:?}; on them: {killer:?}", if id == players[0] { "P0" } else { "P1" }, views.iter().find(|v| v.id == id).map(|v| v.pos));
                        }
                    } else if let Some(v) = views.iter().find(|v| v.id == id && matches!(v.what, What::Enemy { elite: false, .. })) {
                        w.kills += 1;
                        if let (Ok(_), What::Enemy { kind, .. }) = (std::env::var("KINDS"), &v.what) {
                            *w.kinds.entry(s.enemy_def(*kind).name.clone()).or_insert(0) += 1;
                        }
                        w.enemy_hp.0 += v.max_health as u64;
                        w.enemy_hp.1 += 1;
                    }
                }
                Event::Hit { attacker, victim, damage, .. } | Event::SpellHit { caster: attacker, victim, damage, .. } => {
                    let from_player = players.contains(&attacker);
                    let on_player = players.contains(&victim);
                    let from_enemy = views.iter().any(|v| v.id == attacker && matches!(v.what, What::Enemy { elite: false, .. }));
                    if from_player && damage > 20000 && std::env::var_os("BIG").is_some() {
                        eprintln!("      big hit {damage}: {e:?}");
                    }
                    if from_player {
                        w.hero_hits.0 += damage as u64;
                        w.hero_hits.1 += 1;
                        w.hero_hits.2 = w.hero_hits.2.max(damage);
                    }
                    if on_player && from_enemy {
                        w.enemy_hits.0 += damage as u64;
                        w.enemy_hits.1 += 1;
                        w.enemy_hits.2 = w.enemy_hits.2.max(damage);
                    }
                }
                _ => {}
            }
        }
        t += 0.05;
        let st = s.status();
        w.hub_low = w.hub_low.min(st.hub_health * 100 / st.hub_max_health.max(1));
        if st.phase == PhaseName::Intermission {
            for &p in &players {
                loop {
                    let items = s.shop(p);
                    let gold = s.player_view(p).unwrap().gold;
                    let best = items.iter().filter(|i| i.group == "Defence" && i.cost.is_some_and(|c| c <= gold)).min_by_key(|i| i.cost);
                    let Some(k) = best.map(|i| i.key.clone()) else { break };
                    if s.buy(p, &k).is_err() {
                        break;
                    }
                }
            }
        }
        if st.phase == PhaseName::Wave && st.wave != last_wave && last_wave > 0 || st.phase == PhaseName::Defeat {
            let lv: Vec<String> = players.iter().map(|&p| s.player_view(p).map_or(0, |v| v.level).to_string()).collect();
            let hp: Vec<String> = players.iter().map(|&p| s.player_view(p).map_or(0, |v| v.stats.max_health).to_string()).collect();
            let avg = |(sum, n, _): (u64, u32, u32)| sum / n.max(1) as u64;
            eprintln!(
                "{:>4} {:>3}  {:<10} {:>7}/{:<7} {:>5}  {:>7}/{:<8} {:>8}  {:>6}/{:<6}  {:<12} {:>3} {:>3}",
                last_wave,
                s.cycle(),
                lv.join(","),
                w.xp_wave,
                w.xp_break,
                w.kills,
                avg(w.hero_hits),
                w.hero_hits.2,
                w.enemy_hp.0 / w.enemy_hp.1.max(1) as u64,
                avg(w.enemy_hits),
                w.enemy_hits.2,
                hp.join(","),
                w.deaths,
                w.hub_low
            );
            if !w.kinds.is_empty() {
                eprintln!("      {:?}", w.kinds);
            }
            if std::env::var_os("TORN").is_some() {
                eprintln!("      defences torn down: {}", w.torn);
            }
            if std::env::var_os("GOLD").is_some() {
                eprintln!("      P0 earned {} gold this wave", w.earned);
            }
            if std::env::var_os("AFFIX").is_some() {
                eprintln!("      affixes {:?}", s.affix_names());
            }
            if std::env::var_os("BUILD").is_some() && last_wave % 5 == 0 {
                for &p in &players {
                    let v = s.player_view(p).unwrap();
                    eprintln!("      {} dmg {:?} crit {:.0}% cards {:?}", v.name, v.stats.damage, v.stats.crit_pct, v.cards);
                    let gear: Vec<String> = v.equip.iter().filter_map(|(_, _, e, _)| s.item_template(*e).map(|t| t.name.clone())).collect();
                    eprintln!("        gear {gear:?}");
                }
            }
            w = W { hub_low: 100, ..W::default() };
            if st.phase == PhaseName::Defeat || last_wave >= waves {
                break;
            }
        }
        if st.phase == PhaseName::Wave {
            last_wave = st.wave;
        }
    }
}

/// A Hearthstone cast that's broken costs nothing; one that gets you home starts the cooldown.
#[test]
fn a_broken_hearthstone_is_ready_again() {
    let mut s = sim();
    let p = s.add_player("Ayla", "mage", Luck::default());
    s.close_all_menus_for_test(p);
    let away = [40.0, 0.0, 0.0];
    s.move_player(p, away, 0.0);
    s.hearth(p);
    let _ = s.step(0.5);
    s.move_player(p, [45.0, 0.0, 0.0], 0.0);
    let ev = s.step(0.1);
    assert!(ev.iter().any(|e| matches!(e, Event::CooldownCleared { player, .. } if *player == p)), "the client's cooldown is lifted");
    // At once, it goes again, and this time it gets home.
    s.hearth(p);
    let mut tells = Vec::new();
    let mut home = false;
    for _ in 0..80 {
        for e in s.step(0.05) {
            match e {
                Event::Tell { text, .. } => tells.push(text),
                Event::Teleport { player, .. } if player == p => home = true,
                _ => {}
            }
        }
    }
    assert!(home, "the second cast gets home: {tells:?}");
    assert!(!tells.iter().any(|t| t.contains("ready in")), "{tells:?}");
    // Now it waits.
    s.move_player(p, away, 0.0);
    s.hearth(p);
    let tells: Vec<String> = s.step(0.05).into_iter().filter_map(|e| if let Event::Tell { text, .. } = e { Some(text) } else { None }).collect();
    assert!(tells.iter().any(|t| t.contains("ready in")), "{tells:?}");
}

/// The damage meter: each hero's damage by source, which they can reset, and the party's
/// standings for the wave.
#[test]
fn the_damage_meter_counts_by_source() {
    let mut s = Sim::new(test_content(), bridged_arena(), Box::new(Flat(0.0)), 3);
    let mut bots: Vec<Bot> = ["hunter", "mage"].iter().enumerate().map(|(i, c)| Bot::new(s.add_player(&format!("M{i}"), c, Luck::default()), Policy::Defend)).collect();
    let (cleared, _) = run_bots(&mut s, &mut bots, 2, 600.0);
    assert!(cleared >= 2);
    // (The next wave has just begun: some of it.)
    for _ in 0..(20 * 25) {
        for b in bots.iter_mut() {
            bot_turn(&mut s, b, 0.05);
        }
        let _ = s.step(0.05);
    }
    let hunter = bots[0].id;
    let m = s.meter(hunter).unwrap();
    assert!(m.total > 0 && m.secs > 0.0);
    assert_eq!(m.rows.iter().map(|r| r.1).sum::<u64>(), m.total, "every point has a source: {:?}", m.rows);
    assert!(m.rows.iter().any(|(l, _)| l == "Auto Shot"), "{:?}", m.rows);
    assert!(m.rows.iter().all(|(l, n)| l != "Other effects" || *n * 10 < m.total), "most damage is named: {:?}", m.rows);
    assert!(m.rows.iter().any(|(l, _)| *l == s.content().pet.name), "the pet's share is the hunter's: {:?}", m.rows);
    let party = s.party_meter();
    let heroes: Vec<_> = party.iter().filter(|r| r.class == "hunter" || r.class == "mage").collect();
    assert_eq!(heroes.len(), 2);
    assert!(heroes[0].damage >= heroes[1].damage && heroes[1].damage > 0, "{party:?}");
    // The defences have lines of their own: the base's treants and the Ancient, and the soldiers.
    assert!(party.iter().any(|r| r.name == "Base defences" && r.damage > 0), "{party:?}");
    assert!(party.windows(2).all(|w| w[0].damage >= w[1].damage), "most first: {party:?}");
    s.reset_meter(hunter);
    let m = s.meter(hunter).unwrap();
    assert_eq!((m.total, m.rows.len()), (0, 0));
    assert!(s.party_meter().iter().any(|r| r.name == "M0" && r.damage > 0), "a reset leaves the party's wave alone");
}

/// Gold has three homes: the base, the soldiers, and the hero's wares.
#[test]
fn gold_trains_heroes_and_drills_soldiers() {
    let mut s = sim();
    let p = s.add_player("Ayla", "warrior", Luck::default());
    s.close_all_menus_for_test(p);
    let _ = s.step(0.0);
    s.start_now();
    let _ = s.step(0.05);
    s.give_gold_for_test(p, 20_000);
    let groups: HashSet<&str> = s.shop(p).iter().map(|i| i.group).collect();
    for g in ["Defence", "The Ancient", "Your soldiers", "Items", "Your weapon"] {
        assert!(groups.contains(g), "the shop sells {g}: {groups:?}");
    }
    // Wares: five on offer; one bought goes on at once, into a slot the sets leave empty.
    let wares: Vec<ShopItem> = s.shop(p).into_iter().filter(|i| i.key.starts_with("item:")).collect();
    assert_eq!(wares.len(), 5, "{wares:?}");
    assert!(wares.iter().all(|w| !w.icon_name.is_empty() && w.cost.is_some()));
    let worn = |s: &Sim| s.player_view(p).unwrap().equip.iter().filter(|(_, _, e, _)| *e >= items::FIRST_WARE_ENTRY).count();
    assert_eq!(worn(&s), 0);
    let name = s.buy(p, "item:0").unwrap();
    assert_eq!(name, wares[0].name);
    assert_eq!(worn(&s), 1, "the ware is worn");
    assert_eq!(s.shop(p).into_iter().filter(|i| i.key.starts_with("item:")).count(), 4, "and gone from the stock");
    s.buy(p, "restock").unwrap();
    let fresh: Vec<String> = s.shop(p).into_iter().filter(|i| i.key.starts_with("item:")).map(|i| i.name).collect();
    assert_eq!(fresh.len(), 5);
    assert!(!fresh.contains(&name), "nothing already owned is offered again: {fresh:?}");
    // Soldiers: Muster lets more defenders be hired; Drill makes them tougher.
    let cap = |s: &Sim| s.shop(p).into_iter().find(|i| i.key == "defender").unwrap().count;
    let base_cap = cap(&s);
    s.buy(p, "muster").unwrap();
    assert_ne!(cap(&s), base_cap, "Muster raises the cap");
    s.buy(p, "defender").unwrap();
    let hp = |s: &Sim| s.entities().iter().filter(|e| matches!(e.what, What::Ally { owner, .. } if owner == s.hub_id())).map(|e| e.max_health).max().unwrap();
    let plain = hp(&s);
    s.buy(p, "drill").unwrap();
    assert!(hp(&s) > plain, "drilled defenders have more health");
}

/// Leaving mid-run and coming back as the same hero picks up where they left off; as another
/// class, or in a new run, it's a fresh start.
#[test]
fn a_hero_who_relogs_comes_back_as_they_left() {
    let mut s = sim();
    let a = s.add_player("Ayla", "mage", Luck::default());
    let b = s.add_player("Bran", "warrior", Luck::default());
    s.close_all_menus_for_test(a);
    s.close_all_menus_for_test(b);
    let _ = s.step(0.0);
    s.start_now();
    let _ = s.step(0.05);
    s.give_test_card(a, "frostbolt", 2);
    s.give_test_card(a, "fireball", 0);
    s.set_level_for_test(a, 7);
    s.give_gold_for_test(a, 321);
    let before = s.player_view(a).unwrap();
    s.remove_player(a);
    for _ in 0..40 {
        let _ = s.step(0.05);
    }
    let back = s.add_player("Ayla", "mage", Luck::default());
    let now = s.player_view(back).unwrap();
    assert_eq!(now.level, 7);
    assert_eq!(now.cards, before.cards, "the same spells and upgrades");
    assert_eq!(now.gold, before.gold);
    assert_eq!(s.spell_bar(back).unwrap().slots.len(), 2);
    // Another class is another hero.
    s.remove_player(back);
    let other = s.add_player("Ayla", "priest", Luck::default());
    assert!(s.player_view(other).unwrap().cards.is_empty());
    // And once everyone has gone, the run is over: nobody is kept.
    s.remove_player(other);
    s.remove_player(b);
    let fresh = s.add_player("Bran", "warrior", Luck::default());
    assert_eq!(s.player_view(fresh).unwrap().level, 1);
}

/// Armour counts and warriors are the tanks: a warrior in plate takes far less of the same
/// blow than a mage in cloth; a Cloak of the Vanguard adds health.
#[test]
fn warriors_take_blows_best() {
    let mut s = sim();
    let w = s.add_player("Tank", "warrior", Luck::default());
    let m = s.add_player("Glass", "mage", Luck::default());
    for p in [w, m] {
        s.close_all_menus_for_test(p);
    }
    let _ = s.step(0.0);
    s.start_now();
    let _ = s.step(0.05);
    // The same set of armour, as each class wears it.
    for (p, class) in [(w, "warrior"), (m, "mage")] {
        let c = s.content().class_index(class).unwrap();
        for piece in 0..8 {
            s.give_test_item(p, items::GearKey { class: c, tier: 0, quality: 2, piece, archetype: 0 }.entry());
        }
    }
    let took = |s: &mut Sim, p: EntityId| s.damage_taken_for_test(p, 1000);
    let (tw, tm) = (took(&mut s, w), took(&mut s, m));
    assert!(tw * 10 < tm * 6, "plate and a warrior's toughness: warrior takes {tw}, mage {tm} of 1000");
    assert!(tw < 450, "a warrior in a full set shrugs off over half: {tw}");
    let hp = s.player_view(m).unwrap().stats.max_health;
    s.give_ware_for_test(m, "vanguard_cloak");
    assert!(s.player_view(m).unwrap().stats.max_health as f32 >= hp as f32 * 1.1, "the Cloak of the Vanguard adds health");
}

/// Each wave held writes a balance report for the server's log: the enemies, and what each hero
/// took and dealt.
#[test]
fn each_wave_writes_a_balance_report() {
    let mut s = Sim::new(test_content(), bridged_arena(), Box::new(Flat(0.0)), 4);
    let mut bots: Vec<Bot> = ["warrior", "hunter"].iter().enumerate().map(|(i, c)| Bot::new(s.add_player(&format!("B{i}"), c, Luck::default()), Policy::Defend)).collect();
    let _ = s.step(0.0);
    s.start_now();
    let mut logs = Vec::new();
    for _ in 0..(20 * 300) {
        for b in bots.iter_mut() {
            bot_turn(&mut s, b, 0.05);
        }
        for e in s.step(0.05) {
            if let Event::Log(l) = e {
                logs.push(l);
            }
        }
        if logs.len() >= 4 {
            break;
        }
    }
    for l in &logs {
        eprintln!("{l}");
    }
    assert!(logs.first().is_some_and(|l| l.starts_with("Wave 1 ") && l.contains("enemies at level")), "{logs:?}");
    assert!(logs.iter().any(|l| l.contains("B0 (warrior") && l.contains("hits")), "{logs:?}");
    assert!(logs.iter().any(|l| l.contains("Base defences dealt")), "{logs:?}");
}

/// One bot of `CLASS` (warrior) alone on the bridged field, buying defences between waves like a
/// player (NOBUY=1: none): per wave, who does the killing, what the hero took (as a share of
/// their health) and their deaths. The signal behind "the warrior never takes damage" and "one
/// soldier holds a wave".
#[test]
#[ignore]
fn solo_class_report() {
    let class = std::env::var("CLASS").unwrap_or("warrior".into());
    let seed: u64 = std::env::var("SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
    let waves: u32 = std::env::var("WAVES").ok().and_then(|w| w.parse().ok()).unwrap_or(20);
    let mut content = test_content();
    if std::env::var_os("NOHOLD").is_some() {
        content.defenses.hold = 0;
    }
    let mut s = Sim::new(content, bridged_arena(), Box::new(Flat(0.0)), seed);
    let mut bot = Bot::new(s.add_player("Solo", &class, Luck::default()), Policy::Defend);
    let me = bot.id;
    let _ = s.step(0.0);
    s.start_now();
    let (mut by_me, mut by_def, mut by_soldiers) = (0u64, 0u64, 0u64);
    let (mut taken, mut hits, mut biggest, mut deaths) = (0u64, 0u32, 0u32, 0u32);
    let mut last_wave = 0;
    let mut t = 0.0;
    while t < 9000.0 {
        bot_turn(&mut s, &mut bot, 0.05);
        let views = s.entities();
        for e in s.step(0.05) {
            match e {
                Event::Died { id } if id == me => deaths += 1,
                Event::Hit { attacker, victim, damage, .. } | Event::SpellHit { caster: attacker, victim, damage, .. } => {
                    if victim == me {
                        taken += damage as u64;
                        hits += 1;
                        biggest = biggest.max(damage);
                    } else if views.iter().any(|v| v.id == victim && matches!(v.what, What::Enemy { .. })) {
                        match views.iter().find(|v| v.id == attacker).map(|v| v.what.clone()) {
                            Some(What::Player { .. }) => by_me += damage as u64,
                            Some(What::Ally { owner, .. }) if owner == me => by_me += damage as u64,
                            Some(What::Ally { .. }) => by_soldiers += damage as u64,
                            Some(_) => by_def += damage as u64,
                            None => {}
                        }
                    }
                }
                _ => {}
            }
        }
        t += 0.05;
        let st = s.status();
        if std::env::var_os("TRACE").is_some() && ((t * 20.0) as u32) % 60 == 0 {
            let me_v = s.entities().into_iter().find(|e| e.id == me).unwrap();
            let hub = s.arena().hub;
            let near = s.entities().iter().filter(|e| matches!(e.what, What::Enemy { .. }) && e.alive && dist2d(e.pos, hub) < 15.0).count();
            let tv = bot.target.and_then(|t| s.entities().into_iter().find(|e| e.id == t)).map(|e| (e.pos, e.alive, e.health, e.max_health, s.enemy_modifier(e.id)));
            eprintln!("t {t:6.1} wave {} {:?} hero at {:?} alive {} hp {} target {:?} {tv:?} casting {} menu {:?} | enemies by hub {near} hub {}", st.wave, st.phase, me_v.pos, me_v.alive, me_v.health, bot.target, s.casting(me), s.menu(me).map(|m| m.kind), st.hub_health);
        }
        if st.phase == PhaseName::Intermission {
            if std::env::var_os("SOLDIERS").is_some() {
                // All in on the soldiers: hire and drill them first, then the rest.
                go_home(&mut s, me);
                loop {
                    let items = s.shop(me);
                    let gold = s.player_view(me).map_or(0, |v| v.gold);
                    let pick = items
                        .iter()
                        .filter(|i| i.cost.is_some_and(|c| c <= gold))
                        .filter(|i| i.key != "march" && (i.group == "Your soldiers" || i.key.contains("defender") || i.group == "Defence"))
                        .min_by_key(|i| (if i.group == "Your soldiers" || i.key.contains("defender") { 0 } else { 1 }, i.cost));
                    let Some(k) = pick.map(|i| i.key.clone()) else { break };
                    if s.buy(me, &k).is_err() {
                        break;
                    }
                }
            } else if std::env::var_os("NOBUY").is_none() {
                shop_defences(&mut s, me);
            }
            if st.wave != last_wave {
                let v = s.entities().into_iter().find(|e| e.id == me).unwrap();
                let hp = v.max_health.max(1) as f32;
                let total = (by_me + by_def + by_soldiers).max(1);
                eprintln!(
                    "wave {:2}: level {:2} hp {:5}  took {:6.0}% of hp in {:4} hits ({:4.1}% a hit, biggest {:4.1}%), deaths {}   damage by hero {:3}% / soldiers {:3}% / treants {:3}%  hub {:3}%",
                    st.wave,
                    v.level,
                    v.max_health,
                    taken as f32 * 100.0 / hp,
                    hits,
                    taken as f32 * 100.0 / hp / hits.max(1) as f32,
                    biggest as f32 * 100.0 / hp,
                    deaths,
                    by_me * 100 / total,
                    by_soldiers * 100 / total,
                    by_def * 100 / total,
                    st.hub_health * 100 / st.hub_max_health.max(1)
                );
                (by_me, by_def, by_soldiers, taken, hits, biggest, deaths) = (0, 0, 0, 0, 0, 0, 0);
                last_wave = st.wave;
            }
            if st.seconds_left > 3 {
                s.start_now();
            }
        }
        if st.phase == PhaseName::Defeat || st.wave > waves {
            eprintln!("{st:?}");
            break;
        }
    }
}

/// Charge runs at the target (the client plays the run) and the blow lands on arrival, not
/// before; the hero ends up just short of it on their own side.
#[test]
fn charge_rushes_in_and_hits_on_arrival() {
    let mut s = sim();
    let p = s.add_player("Ayla", "warrior", Luck::default());
    s.close_all_menus_for_test(p);
    s.set_level_for_test(p, 20);
    s.give_test_card(p, "charge", 1);
    s.set_manual(p, true);
    s.start_now();
    s.clear_wave_for_test();
    let _ = s.step(0.05);
    let me = s.entities().into_iter().find(|e| e.id == p).unwrap().pos;
    let far = s.spawn_boss_for_test(0, [me[0] + 18.0, me[1], 0.0]);
    s.select(p, Some(far));
    let slot = 0;
    let mut ev = Vec::new();
    for _ in 0..40 {
        s.prime_for_test(p);
        if s.cast_slot(p, slot).is_ok() {
            ev = s.step(0.0);
            break;
        }
        let _ = s.step(0.05);
    }
    let rush = ev.iter().find_map(|e| match e {
        Event::Rush { player, from, to, secs } if *player == p => Some((*from, *to, *secs)),
        _ => None,
    });
    let (from, to, secs) = rush.expect("a rush, not a teleport");
    assert!(!ev.iter().any(|e| matches!(e, Event::Teleport { .. })));
    let tpos = s.entities().into_iter().find(|e| e.id == far).unwrap().pos;
    assert!((dist2d(to, tpos) - 2.0).abs() < 0.5 && dist2d(from, to) > 6.0, "to just short of the target: {from:?} -> {to:?}, target at {tpos:?}");
    assert!((secs - dist2d(from, to) / 24.0).abs() < 0.01, "at Charge's pace: {secs}");
    let (before, _) = s.health_for_test(far);
    let early = s.step(0.2);
    assert!(!early.iter().any(|e| matches!(e, Event::SpellHit { victim, .. } | Event::Hit { victim, .. } if *victim == far)), "nothing lands mid-run");
    let mut landed = false;
    for _ in 0..20 {
        let ev = s.step(0.05);
        landed |= ev.iter().any(|e| matches!(e, Event::SpellHit { victim, damage, .. } if *victim == far && *damage > 0));
    }
    assert!(landed, "the blow lands on arrival");
    assert!(s.health_for_test(far).0 < before);
}

/// A soldier holds at most `defenses.hold` enemies; the rest march past it.
#[test]
fn a_soldier_holds_only_a_few_and_the_rest_march_past() {
    let mut s = sim();
    let p = s.add_player("Ayla", "warrior", Luck::default());
    s.close_all_menus_for_test(p);
    s.start_now();
    s.clear_wave_for_test();
    s.give_gold_for_test(p, 10_000);
    s.buy(p, "defender").expect("a soldier");
    s.move_player(p, [500.0, 500.0, 0.0], 0.0);
    let _ = s.step(0.05);
    let soldier = s.entities().into_iter().find(|e| matches!(e.what, What::Ally { pet: false, .. })).expect("the soldier").clone();
    let hold = s.content().defenses.hold as usize;
    assert_eq!(hold, 4);
    for k in 0..9 {
        let a = k as f32 * 0.7;
        s.spawn_for_test("skeleton", [soldier.pos[0] + 6.0 * a.cos(), soldier.pos[1] + 6.0 * a.sin(), 0.0]);
    }
    let mut most = 0;
    for _ in 0..60 {
        let _ = s.step(0.05);
        let on_it = s.entities().iter().filter(|e| matches!(e.what, What::Enemy { .. }) && e.alive && e.attacking == Some(soldier.id)).count();
        most = most.max(on_it);
    }
    assert!(most >= 2 && most <= hold, "{most} on the soldier at once");
}

/// Avatar makes you bigger, so the choice shows.
#[test]
fn avatar_grows_the_hero() {
    let mut s = sim();
    let p = s.add_player("Ayla", "warrior", Luck::default());
    s.close_all_menus_for_test(p);
    assert_eq!(s.player_view(p).unwrap().stats.scale, 1.0);
    s.give_test_legendary(p, "avatar");
    let ev = s.step(0.0);
    assert!((s.player_view(p).unwrap().stats.scale - 1.15).abs() < 0.001);
    assert!(ev.iter().any(|e| matches!(e, Event::Stats { player } if *player == p)), "the server is told");
}

/// A worn ware from an earlier tier can be reforged at the shop: its stats grow to the tier
/// dropping now, and it stays worn. Helms and tabards are wares too, and show on the hero.
#[test]
fn wares_reforge_to_the_current_tier_and_helms_show() {
    let mut s = sim();
    let p = s.add_player("Ayla", "warrior", Luck::default());
    s.close_all_menus_for_test(p);
    s.give_ware_for_test(p, "vanguard_cloak");
    s.give_ware_for_test(p, "stormwind_greathelm");
    s.give_ware_for_test(p, "alliance_tabard");
    let v = s.player_view(p).unwrap();
    let at = |v: &PlayerView, slot: u8| v.equip.iter().find(|(sl, ..)| *sl == slot).map(|x| x.2);
    assert!(at(&v, items::slot::HEAD).is_some_and(|e| e >= items::FIRST_WARE_ENTRY), "a helm in the head slot");
    assert!(at(&v, items::slot::TABARD).is_some(), "a tabard");
    let helm = s.item_template(at(&v, items::slot::HEAD).unwrap()).unwrap();
    assert_eq!((helm.inventory_type, helm.subclass), (1, 4), "a plate helm for a warrior");
    assert!(helm.armor > 0);
    let cloak = at(&v, items::slot::BACK).unwrap();
    let before = s.item_template(cloak).unwrap().stats.iter().map(|x| x.1).sum::<i32>();
    assert!(!s.shop(p).iter().any(|i| i.key.starts_with("reforge:")), "nothing to reforge at tier 1");
    s.set_cycle_for_test(2);
    let card = s.shop(p).into_iter().find(|i| i.key == format!("reforge:{}", items::slot::BACK)).expect("a reforge card for the cloak");
    assert!(card.text.contains("Tier 3"), "{}", card.text);
    s.give_gold_for_test(p, 10_000);
    s.buy(p, &card.key).unwrap();
    let after_entry = at(&s.player_view(p).unwrap(), items::slot::BACK).unwrap();
    assert_ne!(after_entry, cloak);
    let after = s.item_template(after_entry).unwrap().stats.iter().map(|x| x.1).sum::<i32>();
    assert!(after > before, "stronger: {before} -> {after}");
    assert!(s.buy(p, &card.key).is_err(), "only once per tier");
}

/// A boss leaves powers, its own among them; taking one changes the hero (numbers, look, size),
/// and it's never offered again. Escalations offer the class's own legendaries beside the old ones.
#[test]
fn bosses_leave_powers_and_escalations_offer_class_legendaries() {
    let content = test_content();
    assert!(content.powers.len() >= 25, "{} powers", content.powers.len());
    for class in ["warrior", "mage", "priest", "hunter"] {
        let c = content.class_index(class).unwrap();
        let own = |src| content.powers.iter().filter(|p| p.source == src && p.class == Some(c)).count();
        assert!(own(crate::powers::PowerSource::Escalation) >= 2 && own(crate::powers::PowerSource::Boss) >= 3, "{class} has its own powers");
    }
    let mut s = sim();
    let p = s.add_player("Ayla", "warrior", Luck::default());
    s.close_all_menus_for_test(p);
    let _ = s.step(0.0);
    // Gutrender (the second boss) leaves its hunger, and two more.
    s.boss_offer_for_test(p, 1);
    for _ in 0..40 {
        let _ = s.step(0.05);
    }
    let menu = s.menu(p).expect("a pick").clone();
    let offer = menu.offer.expect("cards");
    let keys: Vec<String> = offer.cards.iter().map(|c| c.key.clone()).collect();
    assert_eq!(offer.cards.len(), 3, "{keys:?}");
    assert!(keys.iter().all(|k| k.starts_with("power:")), "{keys:?}");
    assert_eq!(keys[0], "power:gutrenders_hunger", "the boss's own first");
    assert!(offer.cards.iter().all(|c| c.text.kind == "Boss power"));
    let hp = s.player_view(p).unwrap().stats.max_health;
    s.menu_select(p, 0);
    let v = s.player_view(p).unwrap();
    assert!(v.stats.max_health as f32 >= hp as f32 * 1.14, "15% more health: {hp} -> {}", v.stats.max_health);
    assert!((v.stats.scale - 1.12).abs() < 0.01, "bigger");
    assert!(s.held_cards(p).iter().any(|h| h.key == "power:gutrenders_hunger"));
    // Not again.
    s.boss_offer_for_test(p, 1);
    for _ in 0..40 {
        let _ = s.step(0.05);
    }
    let keys: Vec<String> = s.menu(p).unwrap().offer.as_ref().unwrap().cards.iter().map(|c| c.key.clone()).collect();
    assert!(!keys.contains(&"power:gutrenders_hunger".to_string()), "{keys:?}");
    s.close_all_menus_for_test(p);
    // An Escalation: legendaries, from a pool the warrior's own are part of.
    let mut seen = HashSet::new();
    for _ in 0..30 {
        s.legendary_offer_for_test(p);
        for _ in 0..40 {
            let _ = s.step(0.05);
        }
        for c in &s.menu(p).unwrap().offer.as_ref().unwrap().cards {
            seen.insert(c.key.clone());
        }
        s.close_all_menus_for_test(p);
    }
    assert!(seen.contains("power:titans_grip") && seen.contains("power:unbreakable"), "{seen:?}");
    assert!(seen.contains("avatar"), "the old legendaries too: {seen:?}");
    assert!(!seen.contains("power:archmage"), "no mage's powers for a warrior");
}

/// A power's spell patch reaches the spell: Bladestorm makes Whirlwind spin twice more often.
#[test]
fn a_power_patches_its_spells() {
    let mut s = sim();
    let p = s.add_player("Ayla", "warrior", Luck::default());
    s.close_all_menus_for_test(p);
    s.give_test_card(p, "whirlwind", 0);
    let before = s.spell_nums_for_test(p, "whirlwind");
    s.give_test_power(p, "bladestorm");
    let after = s.spell_nums_for_test(p, "whirlwind");
    assert!(after.echo > before.echo + 0.5 && after.radius > before.radius + 2.0, "{before:?} -> {after:?}");
}

fn talent_index(s: &Sim, class: &str, key: &str) -> usize {
    let c = s.content().class_index(class).unwrap();
    s.content().kit(c).talents.iter().position(|t| t.key == key).unwrap_or_else(|| panic!("no talent {key}"))
}

/// Take talents of a tree until it has `points` (any that are open), as level-up cards would.
fn fill_tree(s: &mut Sim, p: EntityId, class: &str, tree: &str, points: u32) {
    let c = s.content().class_index(class).unwrap();
    let kit = s.content().kit(c).clone();
    let t = kit.tree_index(tree).unwrap();
    for _ in 0..30 {
        if s.tree_points(p)[t] >= points {
            return;
        }
        let open = (0..kit.talents.len()).find(|&i| kit.talents[i].tree == t && kit.talents[i].path.is_none() && s.talent_for_test(p, i).is_ok());
        if open.is_none() {
            return;
        }
    }
}

/// Talents that only reshape a spell come only to heroes holding it; combinations need both
/// spells; talents come every other level.
#[test]
fn talents_follow_the_spells_you_hold() {
    let mut s = sim();
    let p = s.add_player("Ayla", "warrior", Luck::default());
    s.close_all_menus_for_test(p);
    let itc = talent_index(&s, "warrior", "improved_thunder_clap");
    fill_tree(&mut s, p, "warrior", "protection", 3);
    assert!(s.talent_for_test(p, itc).is_err(), "no Thunder Clap talents without Thunder Clap");
    s.give_test_card(p, "thunder_clap", 0);
    assert!(s.talent_for_test(p, itc).is_ok(), "with it, yes");
    // Rending Whirl: Whirlwind and Rend, and 6 points in Fury.
    let rw = talent_index(&s, "warrior", "rending_whirl");
    fill_tree(&mut s, p, "warrior", "fury", 6);
    s.give_test_card(p, "whirlwind", 0);
    assert!(s.talent_for_test(p, rw).is_err(), "Whirlwind alone isn't enough");
    s.give_test_card(p, "rend", 0);
    let before = s.spell_nums_for_test(p, "whirlwind").dot;
    s.talent_for_test(p, rw).expect("with both");
    assert!(s.spell_nums_for_test(p, "whirlwind").dot > before + 0.5, "Whirlwind bleeds now");
}

/// A weapon path re-arms the warrior (sword and shield), closes the other paths, and the
/// weapons that drop follow it.
#[test]
fn the_shieldbearer_takes_up_sword_and_shield() {
    let mut s = sim();
    let p = s.add_player("Ayla", "warrior", Luck::default());
    s.close_all_menus_for_test(p);
    let path = talent_index(&s, "warrior", "path_of_the_shieldbearer");
    let other = talent_index(&s, "warrior", "path_of_the_berserker");
    assert!(s.talent_for_test(p, path).is_err(), "a second-tier talent: 3 points first");
    fill_tree(&mut s, p, "warrior", "protection", 3);
    fill_tree(&mut s, p, "warrior", "fury", 3);
    let taken = s.content().defenses.reference_health;
    let _ = taken;
    let hp = s.player_view(p).unwrap().stats.max_health;
    s.talent_for_test(p, path).expect("the path");
    let ev = s.step(0.0);
    let v = s.player_view(p).unwrap();
    let worn = |slot: u8| v.equip.iter().find(|(sl, ..)| *sl == slot).and_then(|x| s.item_template(x.2));
    let main = worn(items::slot::MAIN_HAND).expect("a sword");
    let off = worn(items::slot::OFF_HAND).expect("a shield");
    assert_eq!((main.inventory_type, main.subclass), (21, 7), "a one-handed sword: {}", main.name);
    assert_eq!((off.inventory_type, off.subclass), (14, 6), "a shield: {}", off.name);
    assert!(off.armor > 0);
    assert!(v.stats.max_health > hp, "tougher");
    assert!(ev.iter().any(|e| matches!(e, Event::Visible { player } if *player == p)), "everyone sees the new weapons");
    assert!(s.talent_for_test(p, other).is_err(), "one path a run");
    // Weapons that drop now are swords and shields, never greatswords.
    for _ in 0..30 {
        s.escalation_gear_for_test(p);
        s.close_all_menus_for_test(p);
    }
    let pack: Vec<_> = s.player_view(p).unwrap().pack.iter().filter_map(|x| s.item_template(x.2)).filter(|t| t.item_class == 2 || t.subclass == 6).collect();
    assert!(!pack.is_empty());
    assert!(pack.iter().all(|t| t.inventory_type == 21 || t.inventory_type == 14), "{:?}", pack.iter().map(|t| &t.name).collect::<Vec<_>>());
}

/// Riflemen shoot from behind the line, clerics heal, Promotion turns defenders into Footmen
/// (and holds one more each), and March Orders walk a soldier to another place.
#[test]
fn soldiers_shoot_heal_rise_in_rank_and_march() {
    let mut s = Sim::new(test_content(), bridged_arena(), Box::new(Flat(0.0)), 3);
    let p = s.add_player("Ayla", "warrior", Luck::default());
    s.close_all_menus_for_test(p);
    s.start_now();
    s.clear_wave_for_test();
    s.give_gold_for_test(p, 50_000);
    let hub = s.arena().hub;
    s.move_player(p, [hub[0] + 5.0, hub[1], hub[2]], 0.0);
    s.buy(p, "defender").unwrap();
    s.buy(p, "rifleman").unwrap();
    s.buy(p, "cleric").unwrap();
    let _ = s.step(0.05);
    let soldiers = |s: &Sim| s.entities().into_iter().filter(|e| matches!(e.what, What::Ally { owner, pet: false } if owner == s.hub_id())).collect::<Vec<_>>();
    assert_eq!(soldiers(&s).len(), 3);
    let kinds: Vec<_> = soldiers(&s).iter().filter_map(|e| s.soldier_kind(e.id)).collect();
    assert!(kinds.contains(&SoldierKind::Rifleman) && kinds.contains(&SoldierKind::Cleric) && kinds.contains(&SoldierKind::Defender(0)));
    let rifle = soldiers(&s).into_iter().find(|e| s.soldier_kind(e.id) == Some(SoldierKind::Rifleman)).unwrap();
    // An enemy in rifle range (not in reach of the defender): shot.
    let foe = s.spawn_for_test("skeleton", [rifle.pos[0] + 18.0, rifle.pos[1], 0.0]);
    let mut shot = false;
    for _ in 0..80 {
        for e in s.step(0.05) {
            if matches!(e, Event::SpellGo { caster, spell, .. } if caster == rifle.id && spell == crate::RIFLE_SHOT) {
                shot = true;
            }
        }
    }
    assert!(shot, "the rifleman shoots");
    let _ = foe;
    // The cleric heals a hurt hero near them.
    let cleric = soldiers(&s).into_iter().find(|e| s.soldier_kind(e.id) == Some(SoldierKind::Cleric)).unwrap();
    s.move_player(p, [cleric.pos[0] + 2.0, cleric.pos[1], 0.0], 0.0);
    s.hurt_for_test(p, 0.5);
    let mut healed = false;
    for _ in 0..80 {
        for e in s.step(0.05) {
            if matches!(e, Event::Heal { healer, .. } if healer == cleric.id) {
                healed = true;
            }
        }
    }
    assert!(healed, "the cleric heals");
    // Promotion: the defender becomes a Footman (a new unit), holding one more.
    s.move_player(p, [hub[0] + 5.0, hub[1], hub[2]], 0.0);
    let hold = s.content().defenses.hold;
    s.buy(p, "promote").unwrap();
    let _ = s.step(0.05);
    let def = soldiers(&s).into_iter().find(|e| matches!(s.soldier_kind(e.id), Some(SoldierKind::Defender(_)))).unwrap();
    assert_eq!(s.soldier_kind(def.id), Some(SoldierKind::Defender(1)));
    assert_eq!(s.defender_rank(), 1);
    assert_eq!(s.holds_for_test(def.id), hold + 1);
    // March Orders: one soldier to the first bridge post, on foot.
    assert!(s.site_count() > 1);
    let before = soldiers(&s).iter().map(|e| (e.id, e.pos)).collect::<Vec<_>>();
    s.buy(p, "march@1").expect("an order");
    let _ = s.step(0.05);
    let moved = soldiers(&s).into_iter().find(|e| s.site_defenses(1).2 > 0 && s.site_of_soldier_for_test(e.id) == 1).expect("one sent");
    let start = before.iter().find(|(id, _)| *id == moved.id).unwrap().1;
    let post = s.arena().posts[0].pos;
    let mut last = dist2d(start, post);
    let mut walked = false;
    for _ in 0..200 {
        let _ = s.step(0.05);
        let now = s.entities().into_iter().find(|e| e.id == moved.id).unwrap().pos;
        let d = dist2d(now, post);
        assert!(d <= last + 0.5, "no teleport, no wandering back");
        walked |= d < last - 0.01;
        last = d;
    }
    assert!(walked, "it walks");
    assert!(dist2d(start, post) - last > 5.0, "and gets on its way: {} -> {last}", dist2d(start, post));
}
