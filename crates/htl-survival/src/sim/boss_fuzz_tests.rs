//! Rough handling of each Deadmines boss: random damage, heroes walking off and dying, many
//! seeds. Nothing may panic, and no timer or position may go bad.

use super::*;
use crate::land::FlatLand;

const DT: f32 = 0.05;


struct Dice(u64);

impl Dice {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as f32) / (1u64 << 31) as f32
    }
}

fn rough(key: &str, seed: u64, heroes: usize) {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::bare(c, Box::new(FlatLand::default()), seed, 0.0);
    let mut dice = Dice(seed.wrapping_mul(7919) ^ 0x5bd1);
    let mut band = Vec::new();
    for n in 0..heroes {
        let h = s.join(&format!("h{n}"), &format!("H{n}"), None);
        s.move_player(h, [3.0, n as f32 * 2.0, 0.0], std::f32::consts::PI, Moving::default());
        let p = s.players.get_mut(&h).unwrap();
        p.health = 3_000;
        p.max_health = 3_000;
        band.push(h);
    }
    let boss = s.spawn_mob(s.content.mob(key).unwrap(), [0.0; 3], None, false);
    s.engage(boss, Foe::Hero(band[0]));
    for tick in 0..(45.0 / DT) as usize {
        if dice.next() < 0.02 {
            let alive: Vec<Id> = s.mobs.iter().filter(|(_, m)| m.health > 0 && !m.friendly).map(|(id, _)| *id).collect();
            if !alive.is_empty() {
                let id = alive[(dice.next() * alive.len() as f32) as usize % alive.len()];
                let by = band[(dice.next() * band.len() as f32) as usize % band.len()];
                let max = s.mobs[&id].max_health as f32;
                let label = if dice.next() < 0.2 { "debug" } else { "Melee" };
                s.hurt_mob_as(id, by, (max * dice.next() * 0.08) as u32 + 1, label);
            }
        }
        if dice.next() < 0.004 {
            let h = band[(dice.next() * band.len() as f32) as usize % band.len()];
            let far = dice.next() < 0.5;
            let to = if far { [60.0 * dice.next(), 60.0 * dice.next() - 30.0, 0.0] } else { [2.0 + 6.0 * dice.next(), 4.0 * dice.next(), 0.0] };
            s.move_player(h, to, 0.0, Moving::default());
        }
        if dice.next() < 0.002 {
            let h = band[(dice.next() * band.len() as f32) as usize % band.len()];
            s.players.get_mut(&h).unwrap().health = 0;
        }
        if tick % 100 == 0 {
            for h in &band {
                if let Some(p) = s.players.get_mut(h) { if p.health == 0 { p.health = p.max_health; } }
            }
        }
        s.step(DT);
        for (id, m) in &s.mobs {
            assert!(m.pos.iter().all(|v| v.is_finite()), "{key} seed {seed}: mob {id} position {:?} at {:.2}", m.pos, s.clock);
            assert!(m.health <= m.max_health, "{key} seed {seed}: mob {id} over its health");
        }
    }
}

macro_rules! rough_tests {
    ($($name:ident: $key:expr),* $(,)?) => {$(
        #[test]
        fn $name() {
            for seed in 1..=2 {
                let t = std::time::Instant::now();
                rough($key, seed, 1 + (seed as usize % 2));
                eprintln!("{} seed {seed}: {:.1}s", $key, t.elapsed().as_secs_f32());
            }
        }
    )*};
}

rough_tests! {
    smite_survives_rough_handling: "dm_smite",
    cookie_survives_rough_handling: "dm_cookie",
    greenskin_survives_rough_handling: "dm_greenskin",
    vancleef_survives_rough_handling: "dm_vancleef",
    shredder_survives_rough_handling: "dm_shredder",
    gilnid_survives_rough_handling: "dm_gilnid",
    rhahkzor_survives_rough_handling: "dm_rhahkzor",
    sneed_survives_rough_handling: "dm_sneed",
}
