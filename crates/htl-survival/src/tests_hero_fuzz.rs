//! A fast fuzz of the heroes themselves, modelled on `boss_fuzz_tests`: every class, one seed,
//! ten simulated seconds, two heroes pressing anything at random (every ability key and its EX,
//! attack, guard, dodge, bash, kick, cancel), aiming at random, being killed, leaving and
//! rejoining mid-fight. Nothing may panic and every position must stay finite. It runs in a few
//! seconds in a debug build, so it can stay in the default test run.

use crate::content::{Content, SCHOOLS, SLOTS};
use crate::land::FlatLand;
use crate::sim::*;

const DT: f32 = 0.05;
const SECS: f32 = 10.0;

struct Dice(u64);

impl Dice {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as f32) / (1u64 << 31) as f32
    }

    fn below(&mut self, n: usize) -> usize {
        ((self.next() * n as f32) as usize).min(n - 1)
    }
}

fn arm(s: &mut Survival, id: Id) {
    let p = s.players.get_mut(&id).unwrap();
    p.max_health = 5_000;
    p.health = 5_000;
    s.set_cooldowns(id, true);
    s.set_valor(id, 100.0);
}

/// A hero of `class` with its five cards on their keys, placed `n` paces from the first.
fn hero(s: &mut Survival, class: &str, n: usize) -> Id {
    let id = s.join(&format!("{class}{n}"), &format!("F{class}{n}"), None);
    s.move_player(id, [0.0, n as f32 * 2.0, 0.0], 0.0, Moving::default());
    let _ = s.step(0.0);
    s.players.get_mut(&id).unwrap().class = class.to_string();
    let cards: Vec<(String, String)> = s.content.abilities.iter().filter(|a| a.school == class && a.on_board()).map(|a| (a.slot.clone(), a.key.clone())).collect();
    for (slot, key) in cards {
        s.players.get_mut(&id).unwrap().owned.insert(key.clone());
        let _ = s.slot_ability(id, &slot, &key);
    }
    arm(s, id);
    id
}

fn foes(s: &mut Survival, dice: &mut Dice) {
    for k in 0..3 {
        let def = s.content.mob("grunt").unwrap();
        let at = [6.0 + 3.0 * dice.next(), (k as f32 - 1.0) * 3.0, 0.0];
        s.spawn_mob(def, at, None, false);
    }
}

fn press(s: &mut Survival, dice: &mut Dice, id: Id) {
    let input = match dice.below(14) {
        0..=5 => Input::Ability(dice.below(SLOTS.len()) as u8, dice.next() < 0.4),
        6 => Input::Attack,
        7 => Input::AttackUp,
        8 => Input::Guard,
        9 => Input::GuardUp,
        10 => Input::Dodge,
        11 => Input::Bash,
        12 => Input::Kick,
        _ => {
            if dice.next() < 0.5 {
                Input::Cancel
            } else {
                Input::Kick
            }
        }
    };
    s.input(id, input);
}

fn rough(class: &str, seed: u64) {
    let mut c = Content::dev();
    c.pvp = Default::default();
    c.dev.full_tree = true;
    let mut s = Survival::new(c, Box::new(FlatLand::default()), seed, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    let mut dice = Dice(seed.wrapping_mul(7919) ^ 0x5bd1 ^ class.len() as u64);
    let mut band: Vec<Id> = (0..2).map(|n| hero(&mut s, class, n)).collect();
    foes(&mut s, &mut dice);
    for tick in 0..(SECS / DT) as usize {
        for k in 0..band.len() {
            let h = band[k];
            if dice.next() < 0.5 {
                press(&mut s, &mut dice, h);
            }
            if dice.next() < 0.2 {
                s.set_aim_line(h, dice.next() * 6.3, dice.next() * 1.4 - 0.7);
            }
            if dice.next() < 0.05 {
                s.set_aim_point(h, [10.0 * dice.next(), 6.0 * dice.next() - 3.0, 0.0]);
            }
            if dice.next() < 0.05 {
                let to = [8.0 * dice.next() - 2.0, 8.0 * dice.next() - 4.0, 0.0];
                s.move_player(h, to, dice.next() * 6.3, Moving { forward: dice.next() < 0.5, ..Default::default() });
            }
            // Killed.
            if dice.next() < 0.01 {
                if let Some(p) = s.players.get_mut(&h) {
                    p.health = 0;
                }
            }
            // Gone and back mid-fight: the same hero, a new id.
            if dice.next() < 0.02 {
                let account = s.players.get(&h).map(|p| p.account.clone()).unwrap_or_default();
                if let Some(save) = s.leave(h) {
                    let n = s.join(&account, &format!("R{tick}{k}"), Some(&save));
                    arm(&mut s, n);
                    band[k] = n;
                }
            }
        }
        if tick % 20 == 0 {
            for h in band.clone() {
                arm(&mut s, h);
            }
            if !s.mobs.values().any(|m| m.health > 0) {
                foes(&mut s, &mut dice);
            }
        }
        s.step(DT);
        for (id, p) in &s.players {
            assert!(p.pos.iter().all(|v| v.is_finite()), "{class} seed {seed}: hero {id} position {:?} at {:.2}", p.pos, s.clock);
            assert!(p.facing.is_finite(), "{class} seed {seed}: hero {id} facing at {:.2}", s.clock);
        }
        for (id, m) in &s.mobs {
            assert!(m.pos.iter().all(|v| v.is_finite()), "{class} seed {seed}: mob {id} position {:?} at {:.2}", m.pos, s.clock);
        }
    }
}

macro_rules! rough_tests {
    ($($name:ident: $class:expr),* $(,)?) => {$(
        #[test]
        fn $name() {
            assert!(SCHOOLS.contains(&$class), "{} is a class", $class);
            let t = std::time::Instant::now();
            rough($class, 1);
            eprintln!("{} hero fuzz: {:.1}s", $class, t.elapsed().as_secs_f32());
        }
    )*};
}

rough_tests! {
    warrior_survives_rough_hands: "warrior",
    paladin_survives_rough_hands: "paladin",
    hunter_survives_rough_hands: "hunter",
    rogue_survives_rough_hands: "rogue",
    priest_survives_rough_hands: "priest",
    shaman_survives_rough_hands: "shaman",
    mage_survives_rough_hands: "mage",
    warlock_survives_rough_hands: "warlock",
    druid_survives_rough_hands: "druid",
}

/// A class added to the game must be added above.
#[test]
fn the_fuzz_covers_every_class() {
    assert_eq!(SCHOOLS.len(), 9, "{SCHOOLS:?}: add the new class to rough_tests!");
}
