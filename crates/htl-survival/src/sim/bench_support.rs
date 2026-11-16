//! A few hooks for the benchmarks outside this crate (`htl-server`'s `perf_tests`): a crowd set on
//! the heroes, without the rest of a wave's machinery.

use super::*;
use super::mobs::Foe;

impl Survival {
    /// Benchmarks only: no creatures and no respawns, so a scene holds only what it's given.
    #[doc(hidden)]
    pub fn bench_clear_creatures(&mut self) {
        self.mobs.clear();
        self.respawns.clear();
    }

    /// Benchmarks only: `n` creatures (the `keys` in turn, ringed round `centre` 8 to 16 yards
    /// out) set on the `band`'s heroes in turn; `raider`s count in the raid's wave. Returns their ids.
    #[doc(hidden)]
    pub fn bench_crowd(&mut self, keys: &[&str], n: usize, raider: bool, band: &[Id], centre: Vec3) -> Vec<Id> {
        let mut ids = Vec::new();
        for i in 0..n {
            let Some(def) = self.content.mob(keys[i % keys.len()]) else { continue };
            let a = i as f32 * 0.7;
            let r = 8.0 + (i % 5) as f32 * 2.0;
            let id = self.spawn_mob(def, [centre[0] + a.cos() * r, centre[1] + a.sin() * r, centre[2]], None, raider);
            if raider {
                self.raid.raiders.push(id);
            }
            self.engage(id, Foe::Hero(band[i % band.len()]));
            ids.push(id);
        }
        ids
    }

    /// Benchmarks only: creature `key` at `at`, set on `hero`.
    #[doc(hidden)]
    pub fn bench_boss(&mut self, key: &str, hero: Id, at: Vec3) -> Option<Id> {
        let def = self.content.mob(key)?;
        let id = self.spawn_mob(def, at, None, false);
        self.engage(id, Foe::Hero(hero));
        Some(id)
    }
}
