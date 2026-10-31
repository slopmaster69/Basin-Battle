//! Heroes and defences working together: what a hero does to an enemy or a defence changes how
//! the defences fight.
//!
//! - A hunter's mark: every defence in reach turns on the marked enemy and hits it harder.
//! - A frozen enemy (a mage's frost): defences hit it harder.
//! - A priest's heal near hired defenders mends them too, and blesses them: they hit harder.
//! - A warrior's Charge: defenders nearby rally to the target, and hit it harder.

use super::*;

/// Defences against a marked enemy, against a frozen one, and from a blessed defender, and
/// defenders against the enemy they rallied to.
pub(super) const MARKED_BONUS: f32 = 0.25;
pub(super) const FROZEN_BONUS: f32 = 0.30;
pub(super) const BLESSED_BONUS: f32 = 0.30;
pub(super) const RALLY_BONUS: f32 = 0.20;
/// How long a blessing and a rally last, and how far each reaches.
pub(super) const BLESS_SECS: f32 = 8.0;
pub(super) const BLESS_REACH: f32 = 12.0;
pub(super) const RALLY_SECS: f32 = 8.0;
pub(super) const RALLY_REACH: f32 = 35.0;
/// The buff a blessed defender shows.
const BLESS_AURA: u32 = 1243;
/// The shout a rally plays on the warrior.
const RALLY_SHOUT: u32 = 6673;

/// How near a hero must stand for a defence to feel their Chain of the Wardens.
const WARDEN_REACH: f32 = 20.0;

impl Sim {
    /// The enemy the defences should turn on near `pos`: a marked or rallied-to one in `range`,
    /// nearest first.
    pub(super) fn defence_focus(&self, pos: Vec3, range: f32) -> Option<(EntityId, f32)> {
        let now = self.clock;
        self.enemies_near(pos, range).into_iter().find(|(id, _)| {
            self.entities.get(id).and_then(|e| e.enemy()).is_some_and(|en| en.aff.mark.is_some() || en.aff.rallied_until > now)
        })
    }

    /// How much harder a defence's hit lands on `victim`.
    pub(super) fn defence_bonus(&self, attacker: EntityId, victim: EntityId) -> f32 {
        let now = self.clock;
        let Some(en) = self.entities.get(&victim).and_then(|e| e.enemy()) else { return 1.0 };
        let mut m = 1.0;
        if en.aff.mark.is_some() {
            m += MARKED_BONUS;
        }
        if self.frozen(victim) {
            m += FROZEN_BONUS;
        }
        // A Chain of the Wardens: the strongest one worn within reach of this defence.
        if let Some(at) = self.entities.get(&attacker).map(|e| e.pos) {
            let aura = self
                .entities
                .values()
                .filter(|e| e.alive() && dist2d(e.pos, at) <= WARDEN_REACH)
                .filter_map(|e| e.player().map(|p| p.stats.defence_aura))
                .fold(0.0f32, f32::max);
            m += aura;
        }
        if let Some(Body::Ally(a)) = self.entities.get(&attacker).map(|e| &e.body) {
            if a.blessed_until > now {
                m += BLESSED_BONUS;
            }
            if en.aff.rallied_until > now {
                m += RALLY_BONUS;
            }
        }
        m
    }

    /// A priest's heal landed on `target`: the hired defenders by it are mended as much, and
    /// blessed.
    pub(super) fn bless_defenders(&mut self, healer: EntityId, target: EntityId, spell: u32, amount: u32) {
        let priest = self.entities.get(&healer).and_then(|e| e.player()).is_some_and(|p| self.content.classes[p.class].key == "priest");
        if !priest || amount == 0 {
            return;
        }
        let Some(at) = self.entities.get(&target).map(|e| e.pos) else { return };
        let now = self.clock;
        let near: Vec<EntityId> = self
            .defenses
            .defenders
            .iter()
            .map(|d| d.0)
            .filter(|d| self.entities.get(d).is_some_and(|e| e.alive() && dist2d(e.pos, at) <= BLESS_REACH))
            .collect();
        for d in near {
            let Some(e) = self.entities.get_mut(&d) else { continue };
            let fresh = matches!(&e.body, Body::Ally(a) if a.blessed_until <= now);
            if let Body::Ally(a) = &mut e.body {
                a.blessed_until = now + BLESS_SECS;
            }
            let healed = amount.min(e.max_health - e.health);
            e.health += healed;
            let (health, max_health) = (e.health, e.max_health);
            if healed > 0 {
                self.events.push(Event::Heal { healer, target: d, spell, amount: healed, crit: false });
                self.events.push(Event::Health { id: d, health, max_health });
            }
            if fresh {
                self.events.push(Event::Aura { unit: d, slot: 0, spell: BLESS_AURA, stacks: 1, secs: BLESS_SECS });
            }
        }
    }

    /// A warrior charged `target`: the defenders in reach rally to it.
    pub(super) fn rally_defenders(&mut self, warrior: EntityId, target: EntityId) {
        let Some(at) = self.entities.get(&target).map(|e| e.pos) else { return };
        let any = self.defenses.defenders.iter().any(|d| self.entities.get(&d.0).is_some_and(|e| e.alive() && dist2d(e.pos, at) <= RALLY_REACH));
        if !any {
            return;
        }
        let until = self.clock + RALLY_SECS;
        if let Some(en) = self.entities.get_mut(&target).and_then(|e| e.enemy_mut()) {
            en.aff.rallied_until = until;
        }
        self.events.push(Event::SpellGo { caster: warrior, spell: RALLY_SHOUT, target: SpellTarget::Unit(warrior), hits: vec![] });
    }

    /// A blessed defender's buff runs out.
    pub(super) fn step_blessings(&mut self) {
        let now = self.clock;
        let done: Vec<EntityId> = self
            .defenses
            .defenders
            .iter()
            .map(|d| d.0)
            .filter(|d| matches!(self.entities.get(d).map(|e| &e.body), Some(Body::Ally(a)) if a.blessed_until > 0.0 && a.blessed_until <= now))
            .collect();
        for d in done {
            if let Some(Body::Ally(a)) = self.entities.get_mut(&d).map(|e| &mut e.body) {
                a.blessed_until = 0.0;
            }
            self.events.push(Event::Aura { unit: d, slot: 0, spell: 0, stacks: 0, secs: 0.0 });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::spells::Mark;
    use super::*;
    use crate::{Arena, BaseSite, Content, Flat, Lane, Luck};

    fn field() -> Arena {
        let lane = |name: &str, x: f32, y: f32| Lane::new(name, vec![[x, y, 0.0], [x * 0.5, y * 0.5, 0.0], [x * 0.06, y * 0.06, 0.0]]);
        Arena {
            map_id: 529,
            area_id: 3358,
            hub: [0.0, 0.0, 0.0],
            hub_facing: 0.0,
            player_spawn: [8.0, 0.0, 0.0],
            player_facing: 3.14,
            posts: Vec::new(),
            map_rect: None,
            lanes: vec![lane("the south-west", -90.0, -90.0), lane("the north-east", 90.0, 90.0)],
            bases: vec![BaseSite { name: "the Farm".into(), pos: [-140.0, -140.0, 0.0], lane: 0 }],
        }
    }

    #[test]
    fn heroes_steer_the_defences() {
        let mut c = Content::builtin();
        c.game.pause_for_picks = false;
        let mut s = Sim::new(c, field(), Box::new(Flat(0.0)), 9);
        let priest = s.add_player("Mend", "priest", Luck::default());
        let warrior = s.add_player("Rush", "warrior", Luck::default());
        let _ = s.step(0.0);
        s.start_now();
        let _ = s.step(0.05);
        s.give_gold_for_test(priest, 5000);
        s.buy(priest, "defender").expect("a defender");
        let defender = s.defenses.defenders[0].0;
        let dpos = s.entities[&defender].pos;
        // Two enemies: one close, one a little further off that a hunter has marked.
        let near = s.alloc();
        s.spawn_enemy(near, EnemyKind { boss: false, index: 0 }, false, None, [dpos[0] + 2.5, dpos[1], 0.0], 0.0, 50.0, 0.0);
        let far = s.alloc();
        s.spawn_enemy(far, EnemyKind { boss: false, index: 0 }, false, None, [dpos[0] + 14.0, dpos[1], 0.0], 0.0, 50.0, 0.0);
        if let Some(en) = s.entities.get_mut(&far).and_then(|e| e.enemy_mut()) {
            en.aff.mark = Some(Mark { owner: warrior, bonus: 0.1, crit: 0.0, pet_bonus: 0.0, jump: false, reset: false });
        }
        assert!(s.defence_bonus(defender, far) >= 1.0 + MARKED_BONUS - 1e-6);
        assert_eq!(s.defence_bonus(defender, near), 1.0);
        for _ in 0..40 {
            let _ = s.step(0.05);
        }
        let engaged = match &s.entities[&defender].body {
            Body::Ally(a) => a.engaged,
            _ => None,
        };
        assert_eq!(engaged, Some(far), "the defender turns on the marked enemy, not the nearer one");
        // A priest's heal by the defender blesses it.
        let at = s.entities[&defender].pos;
        s.entities.get_mut(&priest).unwrap().pos = [at[0] + 3.0, at[1], 0.0];
        s.entities.get_mut(&priest).unwrap().health = 10;
        s.heal(priest, priest, 2061, 200, false);
        assert!(matches!(&s.entities[&defender].body, Body::Ally(a) if a.blessed_until > s.clock));
        assert!(s.defence_bonus(defender, near) >= 1.0 + BLESSED_BONUS - 1e-6);
        // A warrior's charge rallies the defenders to its target.
        s.rally_defenders(warrior, near);
        assert!(s.entities[&near].enemy().unwrap().aff.rallied_until > s.clock);
        assert!(s.defence_bonus(defender, near) >= 1.0 + BLESSED_BONUS + RALLY_BONUS - 1e-6);
    }
}

#[cfg(test)]
mod roles {
    use super::super::Goal;
    use super::*;
    use crate::{Content, Flat, Luck};

    /// A sapper runs past everything to the Ancient and blows up there, for nobody's gain; a
    /// herald drives the enemies near it on.
    #[test]
    fn sappers_blow_up_the_ancient_and_heralds_drive_the_pack() {
        let mut c = Content::builtin();
        c.game.pause_for_picks = false;
        let mut s = Sim::new(c, super::tests_field(), Box::new(Flat(0.0)), 4);
        let p = s.add_player("Watch", "mage", Luck::default());
        let _ = s.step(0.0);
        s.start_now();
        let _ = s.step(0.05);
        let sapper = s.content.enemy_index("sapper").expect("a sapper");
        let id = s.alloc();
        s.spawn_enemy(id, EnemyKind { boss: false, index: sapper }, false, None, [30.0, 0.0, 0.0], 3.14, 1.0, 1.0);
        if let Some(en) = s.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
            en.goal = Goal::Hub;
        }
        // The hero stands far off: no help, and no sharing in its death either.
        s.entities.get_mut(&p).unwrap().pos = [-60.0, 0.0, 0.0];
        let hub = s.hub;
        let (before, max) = (s.entities[&hub].health, s.entities[&hub].max_health);
        let gold = s.entities[&p].player().unwrap().gold;
        let mut blew = false;
        for _ in 0..(20 * 20) {
            for e in s.step(0.05) {
                if matches!(e, Event::Announce { ref text, .. } if text.contains("blew up")) {
                    blew = true;
                }
            }
            if blew {
                break;
            }
        }
        assert!(blew, "the sapper reaches the Ancient and blows up");
        let lost = before - s.entities[&hub].health;
        assert!(lost as f32 >= max as f32 * 0.04, "it tears a share off the Ancient ({lost} of {max})");
        assert_eq!(s.entities[&p].player().unwrap().gold, gold, "nobody is paid for it");
        // A herald's cry.
        let herald = s.content.enemy_index("herald").expect("a herald");
        let h = s.alloc();
        s.spawn_enemy(h, EnemyKind { boss: false, index: herald }, false, None, [40.0, 40.0, 0.0], 0.0, 1.0, 1.0);
        assert!(s.heralded([45.0, 40.0, 0.0]));
        assert!(!s.heralded([80.0, 40.0, 0.0]));
    }

    /// A Plague Butcher's cleave takes a share of a hero's health, whatever their level; a hero
    /// who steps out takes none of it.
    #[test]
    fn switching_targets_does_not_reset_the_swing() {
        let mut c = Content::builtin();
        c.game.pause_for_picks = false;
        let mut s = Sim::new(c, super::tests_field(), Box::new(Flat(0.0)), 6);
        let p = s.add_player("Click", "warrior", Luck::default());
        let _ = s.step(0.0);
        s.start_now();
        let _ = s.step(0.05);
        let at = [-40.0, 30.0, 0.0];
        s.entities.get_mut(&p).unwrap().pos = at;
        let (a, b) = (s.alloc(), s.alloc());
        s.spawn_enemy(a, EnemyKind { boss: false, index: 0 }, false, None, [at[0] + 1.5, at[1], 0.0], 0.0, 500.0, 0.0);
        s.spawn_enemy(b, EnemyKind { boss: false, index: 0 }, false, None, [at[0], at[1] + 1.5, 0.0], 0.0, 500.0, 0.0);
        let secs = s.entities[&p].player().unwrap().stats.attack_secs;
        // Ten seconds of clicking from one to the other every step, and stopping and starting.
        let mut swings = 0;
        for i in 0..200 {
            match i % 3 {
                0 => s.attack(p, a),
                1 => s.attack(p, b),
                _ => {
                    s.stop_attack(p);
                    s.attack(p, a);
                }
            }
            for e in s.step(0.05) {
                if matches!(e, Event::Hit { attacker, .. } if attacker == p) {
                    swings += 1;
                }
            }
            s.entities.get_mut(&p).unwrap().pos = at;
        }
        let most = (10.0 / secs).ceil() as u32 + 2;
        assert!(swings <= most && swings >= 2, "{swings} swings in 10 seconds at {secs}s a swing");
    }

    #[test]
    fn the_pet_hearths_home_with_its_hunter() {
        let mut c = Content::builtin();
        c.game.pause_for_picks = false;
        let mut s = Sim::new(c, super::tests_field(), Box::new(Flat(0.0)), 7);
        let h = s.add_player("Hearth", "hunter", Luck::default());
        let _ = s.step(0.0);
        s.start_now();
        for _ in 0..20 {
            let _ = s.step(0.05);
        }
        let pet = s.pet_of(h).expect("a pet");
        let away = [-60.0, 50.0, 0.0];
        s.entities.get_mut(&h).unwrap().pos = away;
        s.entities.get_mut(&pet).unwrap().pos = [away[0] + 2.0, away[1], 0.0];
        s.hearth(h);
        for _ in 0..(20 * 15) {
            let _ = s.step(0.05);
            if dist2d(s.entities[&h].pos, s.arena.player_spawn) < 1.0 {
                break;
            }
        }
        assert!(dist2d(s.entities[&h].pos, s.arena.player_spawn) < 1.0, "the hunter got home");
        assert!(dist2d(s.entities[&pet].pos, s.arena.player_spawn) < 4.0, "and the pet with them: {:?}", s.entities[&pet].pos);
    }

    #[test]
    fn long_falls_hurt() {
        let mut c = Content::builtin();
        c.game.pause_for_picks = false;
        let mut s = Sim::new(c, super::tests_field(), Box::new(Flat(0.0)), 8);
        let p = s.add_player("Leap", "mage", Luck::default());
        let _ = s.step(0.0);
        s.start_now();
        let _ = s.step(0.05);
        let max = s.entities[&p].max_health;
        assert_eq!(s.fall(p, 12.0), 0, "a jump or a short drop is free");
        let hurt = s.fall(p, 50.0);
        assert!((hurt as f32 / max as f32 - 0.395).abs() < 0.01, "50 yards takes about 40%: {hurt} of {max}");
        s.fall(p, 100.0);
        assert!(!s.entities[&p].alive(), "a hundred yards is the end");
    }

    #[test]
    fn wares_change_how_a_hero_plays() {
        let mut c = Content::builtin();
        c.game.pause_for_picks = false;
        let mut s = Sim::new(c, super::tests_field(), Box::new(Flat(0.0)), 11);
        let h = s.add_player("Wares", "hunter", Luck::default());
        let m = s.add_player("Glass", "mage", Luck::default());
        let _ = s.step(0.0);
        s.start_now();
        let _ = s.step(0.05);
        // Only a class's own wares are offered to it.
        for _ in 0..20 {
            s.restock(m);
            for &e in &s.entities[&m].player().unwrap().stock {
                let (w, _, _) = crate::items::decode_ware(e).unwrap();
                assert!(s.content.wares[w].class.is_none_or(|c| c == s.entities[&m].player().unwrap().class));
            }
        }
        // Rings come in pairs: two worn, the third into the bags.
        for key in ["band_of_thorns", "misers_band", "quick_hands"] {
            s.give_ware_for_test(h, key);
        }
        let p = s.entities[&h].player().unwrap();
        assert!(p.equip[crate::items::slot::FINGER1 as usize].is_some() && p.equip[crate::items::slot::FINGER2 as usize].is_some());
        assert_eq!(p.pack.iter().flatten().filter(|i| i.entry >= crate::items::FIRST_WARE_ENTRY).count(), 1);
        assert!(p.stats.thorns >= 0.4 && p.stats.gold_find >= 0.3);
        // A Miser's Band: more gold from everything earned.
        let g0 = s.entities[&h].player().unwrap().gold;
        s.earn_gold(h, 100);
        assert_eq!(s.entities[&h].player().unwrap().gold - g0, 130);
        // A Trapper's Kit: traps recharge faster.
        let trap = s.content.kit(s.entities[&h].player().unwrap().class).spell_index("freezing_trap").unwrap();
        if !s.entities[&h].player().unwrap().spells.iter().any(|o| o.spell == trap) {
            s.learn_spell(h, trap, false);
        }
        let cd = |s: &Sim| s.entities[&h].player().unwrap().spells.iter().find(|o| o.spell == trap).unwrap().nums.cooldown;
        let before = cd(&s);
        s.give_ware_for_test(h, "trappers_kit");
        assert!((cd(&s) - before * 0.7).abs() < 0.01, "{before} -> {}", cd(&s));
        // The Amulet of the Glass Cannon: more damage out, more in.
        let took = |s: &mut Sim| s.damage_taken_for_test(m, 1000);
        let (hit0, dmg0) = (took(&mut s), s.entities[&m].player().unwrap().stats.damage);
        let full = s.entities[&m].max_health;
        s.entities.get_mut(&m).unwrap().health = full;
        s.give_ware_for_test(m, "glass_cannon");
        s.entities.get_mut(&m).unwrap().health = full;
        let (hit1, dmg1) = (took(&mut s), s.entities[&m].player().unwrap().stats.damage);
        assert!(hit1 as f32 > hit0 as f32 * 1.15 && dmg1 > dmg0 * 1.2, "{hit0}->{hit1}, {dmg0}->{dmg1}");
        // A Heartwood Locket: the hero's damage mends the Ancient.
        s.give_ware_for_test(h, "heartwood_locket");
        let hub = s.hub;
        let max = s.entities[&hub].max_health;
        s.entities.get_mut(&hub).unwrap().health = max / 2;
        let foe = s.alloc();
        s.spawn_enemy(foe, EnemyKind { boss: false, index: 0 }, false, None, [-50.0, 40.0, 0.0], 0.0, 500.0, 0.0);
        s.deal(h, foe, 1000, false, super::super::enemies::Via::Melee);
        assert_eq!(s.entities[&hub].health, max / 2 + 50);
        // A Chain of the Wardens: defences near the hero hit harder.
        s.give_gold_for_test(m, 5000);
        s.buy(m, "defender").unwrap();
        let d = s.defenses.defenders[0].0;
        let at = s.entities[&d].pos;
        let foe_bonus = |s: &Sim| s.defence_bonus(d, foe);
        let plain = foe_bonus(&s);
        s.entities.get_mut(&m).unwrap().player_mut().unwrap().equip[crate::items::slot::NECK as usize] = None;
        s.give_ware_for_test(m, "wardens_chain");
        s.entities.get_mut(&m).unwrap().pos = [at[0] + 5.0, at[1], 0.0];
        assert!((foe_bonus(&s) - plain - 0.3).abs() < 1e-4);
        s.entities.get_mut(&m).unwrap().pos = [at[0] + 50.0, at[1], 0.0];
        assert!((foe_bonus(&s) - plain).abs() < 1e-4, "out of reach, no bonus");
    }

    #[test]
    fn guardians_keep_pace_with_the_waves() {
        let mut c = Content::builtin();
        c.game.pause_for_picks = false;
        let mut s = Sim::new(c, super::tests_field(), Box::new(Flat(0.0)), 5);
        let _p = s.add_player("Wait", "mage", Luck::default());
        let _ = s.step(0.0);
        s.start_now();
        let _ = s.step(0.05);
        s.run.wave = 3;
        s.open_bases();
        let g = s.bases[0].guardians[0];
        let (max3, level3, dmg3) = (s.entities[&g].max_health, s.entities[&g].level, s.entities[&g].enemy().unwrap().damage_mult);
        let h3 = s.health_mult();
        // Wounded, then three waves go by with the base left alone.
        s.entities.get_mut(&g).unwrap().health = max3 / 2;
        s.run.wave = 6;
        s.rescale_guardians();
        let e = &s.entities[&g];
        let grown = e.max_health as f32 / max3 as f32;
        assert!((grown - s.health_mult() / h3).abs() < 0.01 && grown > 1.3, "health grew x{grown}");
        assert!((e.health as f32 / e.max_health as f32 - 0.5).abs() < 0.01, "it keeps its wounds");
        assert!(e.level > level3 && e.enemy().unwrap().damage_mult > dmg3);
        // A claimed base's guardians are long gone; a cleared one isn't rescaled either.
        s.bases[0].state = BaseState::Cleared;
        let before = s.entities[&g].max_health;
        s.run.wave = 9;
        s.rescale_guardians();
        assert_eq!(s.entities[&g].max_health, before);
    }

    #[test]
    fn a_butchers_cleave_takes_a_share_of_health() {
        let mut c = Content::builtin();
        c.game.pause_for_picks = false;
        let mut s = Sim::new(c, super::tests_field(), Box::new(Flat(0.0)), 4);
        let p = s.add_player("Stay", "mage", Luck::default());
        let _ = s.step(0.0);
        s.start_now();
        let _ = s.step(0.05);
        s.set_level_for_test(p, 20);
        let at = [-40.0, 30.0, 0.0];
        s.entities.get_mut(&p).unwrap().pos = at;
        let max = s.entities[&p].max_health;
        s.entities.get_mut(&p).unwrap().health = max;
        let butcher = s.content.enemy_index("butcher").expect("a butcher");
        let id = s.alloc();
        // Feeble swings (damage x0), so only the cleave hurts.
        s.spawn_enemy(id, EnemyKind { boss: false, index: butcher }, false, None, [at[0] + 2.0, at[1], 0.0], 0.0, 50.0, 0.0);
        let mut cleaved = None;
        for _ in 0..(20 * 15) {
            for e in s.step(0.05) {
                if let Event::SpellHit { victim, damage, .. } = e {
                    if victim == p && damage > 0 {
                        cleaved = Some(damage);
                    }
                }
            }
            if cleaved.is_some() {
                break;
            }
            s.entities.get_mut(&p).unwrap().pos = at;
        }
        let dmg = cleaved.expect("the cleave lands on whoever stays");
        assert!(dmg as f32 >= max as f32 * 0.3 && dmg as f32 <= max as f32 * 0.41, "{dmg} of {max}");
    }
}

#[cfg(test)]
pub(super) fn tests_field() -> crate::Arena {
    use crate::{Arena, BaseSite, Lane};
    let lane = |name: &str, x: f32, y: f32| Lane::new(name, vec![[x, y, 0.0], [x * 0.5, y * 0.5, 0.0], [x * 0.06, y * 0.06, 0.0]]);
    Arena {
        map_id: 529,
        area_id: 3358,
        hub: [0.0, 0.0, 0.0],
        hub_facing: 0.0,
        player_spawn: [8.0, 0.0, 0.0],
        player_facing: 3.14,
        posts: Vec::new(),
        map_rect: None,
        lanes: vec![lane("the south-west", -90.0, -90.0), lane("the north-east", 90.0, 90.0)],
        bases: vec![BaseSite { name: "the Farm".into(), pos: [-140.0, -140.0, 0.0], lane: 0 }],
    }
}
