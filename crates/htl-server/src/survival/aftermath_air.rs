//! The air of the finished Deadmines (change E of the 9 Nov review): dust still settling off the
//! piles and grit trickling down them, and the timber creaking. All of it native: the dust is the
//! low cloud a charge raises on landing (spell visual kit 44, `DustCloud_Land`), played now and
//! then on a pile's soil skirt and small stones; the grit is the ore-on-stone clatter of putting
//! down rocks (sound kit 1211) with some of those puffs; the creak is the Deadmines' own wooden
//! door (3078) and the haunted windmill's strained timber (1650), from the snapped beams and
//! posts. Nothing changes the world: these are packets to whoever is inspecting.

use std::time::Instant;

use super::*;

/// `DustCloud_Land.mdx`: a low ground cloud that spreads and settles in a couple of seconds.
pub(super) const KIT_DUST: u32 = 44;
/// `PutDownRocks_Ore`: stones landing on stones.
pub(super) const SOUND_GRIT: u32 = 1211;
/// `DeadmineDoor01Open` and `WindMillHauntedStandVar1`: timber taking strain.
pub(super) const SOUND_CREAK: [u32; 2] = [3078, 1650];
/// Each pile raises its dust this often, give or take a third (seconds).
const DUST_EVERY: f32 = 7.0;
/// A beam creaks this often, give or take a half.
const CREAK_EVERY: f32 = 16.0;
/// No more than this many dust sources (the client draws each puff for a few seconds).
const MAX_DUST: usize = 90;

/// Where the air comes from, and when each next does something.
pub(crate) struct Air {
    started: Instant,
    /// (prop index, seconds from the start of the next puff), on skirts and small stones.
    dust: Vec<(usize, f32)>,
    /// (prop index, next creak), on beams.
    creaks: Vec<(usize, f32)>,
    count: u32,
}

/// A fixed spread in [0, 1): the same every preview, different for each source and turn.
fn spread(a: usize, b: u32) -> f32 {
    let mut h = (a as u32).wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA77) ^ 0x2545_F491;
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    (h % 10_000) as f32 / 10_000.0
}

impl Air {
    /// The sources of a composed ruin: every pile's soil skirts and its smallest stones (up to three
    /// a pile), and every pile's beams. The views' foreground and the drowned cove raise none.
    pub(super) fn new(props: &[Prop], started: Instant) -> Self {
        let mut dust = Vec::new();
        let mut per_site: BTreeMap<&str, usize> = BTreeMap::new();
        for (i, p) in props.iter().enumerate() {
            if p.site == "Rubble apron" || p.site == "Juggernaut collapse basin" { continue; }
            let small_stone = p.model == ROCK && !p.hangs() && p.scale < 0.3;
            if p.model != SKIRT && !small_stone { continue; }
            let n = per_site.entry(p.site).or_default();
            if *n >= 3 { continue; }
            *n += 1;
            dust.push((i, DUST_EVERY * spread(i, 0)));
        }
        dust.truncate(MAX_DUST);
        let creaks = props.iter().enumerate().filter(|(_, p)| p.model == BEAM && p.site != "Rubble apron")
            .map(|(i, _)| (i, CREAK_EVERY * spread(i, 1))).collect();
        Self { started, dust, creaks, count: 0 }
    }

    /// What is due by now: each source's packet, and its next turn.
    pub(super) fn due(&mut self, props: &[Prop], now: f32) -> Vec<Packet> {
        let mut out = Vec::new();
        for (i, at) in self.dust.iter_mut() {
            if *at > now { continue; }
            self.count += 1;
            let guid = prop_guid(*i, props[*i].model);
            out.push(packets::play_spell_visual(guid, KIT_DUST));
            // About a third of the puffs come with the sound of grit landing.
            if spread(*i, self.count) < 0.35 { out.push(packets::play_object_sound(guid, SOUND_GRIT)); }
            *at += DUST_EVERY * (0.67 + 0.67 * spread(*i, self.count + 7));
        }
        for (i, at) in self.creaks.iter_mut() {
            if *at > now { continue; }
            self.count += 1;
            let sound = SOUND_CREAK[(spread(*i, self.count) * 2.0) as usize % 2];
            out.push(packets::play_object_sound(prop_guid(*i, props[*i].model), sound));
            *at += CREAK_EVERY * (0.5 + spread(*i, self.count + 3));
        }
        out
    }
}

impl SurvivalGame {
    /// The preview's air, once a tick: what's due goes to everyone in the world.
    pub(crate) fn aftermath_air(&mut self) {
        let Some(a) = self.aftermath.as_mut() else { return };
        let now = a.air.started.elapsed().as_secs_f32();
        let due = a.air.due(&a.props, now);
        for p in due { self.broadcast(&p); }
    }
}
