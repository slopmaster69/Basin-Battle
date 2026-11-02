//! Elwynn's creatures: packs that notice you and call their neighbours, readable wind-ups that
//! commit to a direction, a recovery to punish, and a leash home.

use super::*;
use crate::land::clear_line;

/// A creature's missile that splashes at least this wide is a lobbed throw (a pot, a bomb): its
/// landing is marked from the wind-up's start.
pub(crate) const LOB_RADIUS: f32 = 2.5;
/// The least a lobbed throw spends in the air, however near it lands.
pub(crate) const LOB_MIN_FLY: f32 = 0.7;

/// How long a lobbed throw is in the air over `dist` yards.
pub(crate) fn lob_flight(dist: f32, speed: f32) -> f32 {
    (dist / speed.max(1.0)).max(LOB_MIN_FLY)
}

/// The real ground look that warns of a splash by its look (`look`, else its missile's `spell`):
/// Heated Ground for fire (a fireball's, a bomb's, Cookie's hot stew), Shadow Fissure for shadow,
/// Consecration for holy, Frost Trap's frost for frost, Plague Cloud for acid and poison, a dust
/// cloud for rocks and quakes. 0: none known (the drawn circle).
pub(crate) fn ground_look(spell: u32, look: u32) -> u32 {
    let of = |id: u32| match id {
        // Fire: Fire Nova, Geyser (hot stew), Sapper Explode, Fireball, Firebolt, Throw Dark Iron
        // Ale, dynamite, Lightning Bolt, Molten Metal, Heated Ground itself.
        10604 | 10987 | 3204 | 133 | 3110 | 14814 | 7978 | 4062 | 403 | 5213 | 22191 => 22191,
        // Shadow: Void Blast, Spirit Burst, Shadow Bolt, Shadow Fissure.
        27812 | 24050 | 686 | 27810 => 27810,
        // Holy: the resurrection light, Consecration.
        24171 | 20116 | 26573 => 20116,
        // Frost: Frost Blast, Frostbolt, Frost Trap.
        27808 | 116 | 13810 => 13810,
        // Acid and poison: Corrosive Acid, Irritating Spores, Venom Spit, Plague Cloud.
        8245 | 22786 | 6917 | 3256 => 3256,
        // Earth: Rocky Ground, Patch, Boulder, Throw Rock, Dust Cloud.
        26271 | 10860 | 9483 | 23065 | 7272 => 7272,
        _ => 0,
    };
    match of(look) {
        0 => of(spell),
        g => g,
    }
}

/// How long the ring a creature's turn throws out is marked before it lands.
const PHASE_SHOVE_WARN: f32 = 0.7;

/// What a creature is fighting: a hero, or (in the settlement attack) a station.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Foe {
    Hero(Id),
    Obj(Id),
}

#[derive(Debug, Clone, PartialEq)]
pub enum MobState {
    Idle { wander_at: f32 },
    Chase { foe: Foe },
    /// Winding up a blow at `foe`, `t` seconds in; its mark on the ground; whether the swing has
    /// been shown.
    Windup { foe: Foe, t: f32, mark: Option<Id>, shown: bool },
    Recover { foe: Foe, until: f32 },
    /// Back home, ignoring everything until there.
    Return,
    /// Fallen: the body stays a moment.
    Dead { gone_at: f32 },
}

impl MobState {
    pub fn target(&self) -> Option<Id> {
        match self {
            MobState::Chase { foe: Foe::Hero(h) } | MobState::Windup { foe: Foe::Hero(h), .. } | MobState::Recover { foe: Foe::Hero(h), .. } => Some(*h),
            _ => None,
        }
    }
    pub fn foe(&self) -> Option<Foe> {
        match self {
            MobState::Chase { foe } | MobState::Windup { foe, .. } | MobState::Recover { foe, .. } => Some(*foe),
            _ => None,
        }
    }
}

/// The swing's animation starts this long before the blow lands, so the strike in the animation
/// meets the moment it counts.
const ANIM_LEAD: f32 = 0.3;
/// The reel of a staggered creature or hero (`Emotes.dbc` 64, Stun).
pub const STUN_EMOTE: u32 = 64;
/// A roar (EMOTE_ONESHOT_ROAR).
pub(crate) const ROAR_EMOTE: u32 = 15;
/// A creature's attack clips (`AnimationData`): bare-handed, and with a weapon.
const ATTACK_UNARMED: u16 = 16;
const ATTACK_1H: u16 = 17;
const ATTACK_2H: u16 = 18;
const ATTACK_2HL: u16 = 19;
const ATTACK_PIERCE: u16 = 85;
/// A lunge carries a creature forward over the last this-much of its wind-up.
const LUNGE_SECS: f32 = 0.25;
/// At most this many fighters press one hero at once; the rest circle at about this far, a step
/// round at a time.
const MAX_ATTACKERS: usize = 2;
/// An opening this near draws a creature in at once (past its turn).
pub(crate) const PRESS_RANGE: f32 = 7.0;
const CIRCLE_RADIUS: f32 = 3.5;
const CIRCLE_STEP: f32 = 0.6;
/// Stepping back out of a heavy blow: this far, over this long, with the Dodge clip.
const BACKSTEP: f32 = 3.0;
const BACKSTEP_SECS: f32 = 0.35;
const DODGE_ANIM: u16 = 30;
/// A lunge lands this far short of its foe (not on top of them).
const LUNGE_SHORT: f32 = 1.8;
/// A camp whose fighters are due back within this long isn't taken by killing the first back.
const CAMP_RETURNING: f32 = 60.0;
/// A broken posture: reeling this long, open to a riposte.
const POSTURE_BREAK: f32 = 2.2;
/// After a break it stands back up: its posture builds again only this long after the reel ends
/// (else a group fills it again while it reels and it never stands).
const POSTURE_GRACE: f32 = 4.0;
/// Heroes this near a creature's fight are in it.
const FIGHT_RANGE: f32 = 25.0;
/// A hero healing this near a foe on them draws its attack this soon.
const PUNISH_RANGE: f32 = 9.0;
const PUNISH_SECS: f32 = 0.25;
/// A caster this much of its distance from its foe is crowded, and backs off this far a step.
const CROWDED: f32 = 0.45;
const BACK_OFF: f32 = 3.0;
/// Seconds between a creature's decisions while it walks.
const THINK: f32 = 0.25;
/// A creature this near home after a chase is back.
const HOME: f32 = 2.0;
/// ...and on home's own floor: not this far above or below it with a floor between (a deck over a
/// hold is another floor).
const HOME_RISE: f32 = 3.0;
/// The steepest a creature's walk goes up or down (rise over run), as the land's own walks.
const WALK_SLOPE: f32 = 1.2;
/// And a body thrown or pulled along the ground (a stair at a time: `slide_end`'s steps).
const SLIDE_SLOPE: f32 = 3.75;
/// A drop this much more than a walk makes over the ground it covered is a fall off its floor
/// (through a gap in a deck, over its edge, to the hold below).
const FALL: f32 = 1.5;
/// Below, a creature that drops more than this at once has fallen through its floor.
const FELL_THROUGH: f32 = 4.0;
/// A chase goes this far past the leash after a foe still in it (the leash is how far from home
/// the foe may be, so a hero shooting from just past it still draws the fight to them), and the
/// creature itself never more than twice that past.
const LEASH_SLACK: f32 = 6.0;
/// A foe it can find no way to for this long is given up on.
const LOST_SECS: f32 = 4.0;
/// Creatures round a foe keep about this far apart, spreading round in steps this wide.
const SPACING: f32 = 1.6;
/// The heroes this near a champion as it falls learn its card.
pub const TEACH_RANGE: f32 = 40.0;
/// A creature at rest farther than this from every hero doesn't think (nor wander).
pub const SLEEP_RANGE: f32 = 160.0;
const SPREAD_STEP: f32 = 0.6;
/// No height given: the top floor.
const TOP: f32 = 1.0e4;

impl Survival {
    /// Whether a creature at `pos` is on another floor than its home's: well above or below it,
    /// with a floor between them (under the deck its home is on, or up on one over it). Ground
    /// with no floors over floors has none.
    fn off_home_floor(&self, pos: Vec3, home: Vec3) -> bool {
        let (lo, hi) = if pos[2] < home[2] { (pos[2], home[2]) } else { (home[2], pos[2]) };
        // (The higher one's own floor counts as between.)
        hi - lo > HOME_RISE && (self.land.slab_between(pos[0], pos[1], lo, hi + 0.1) || self.land.slab_between(home[0], home[1], lo, hi + 0.1))
    }

    /// A creature at `pos` is home: near it, on its floor.
    fn at_home(&self, pos: Vec3, home: Vec3) -> bool {
        dist2d(pos, home) <= HOME && !self.off_home_floor(pos, home)
    }

    /// A foe's health: its own times `combat.foe_health` (a critter, a dummy or a totem as it is).
    pub(crate) fn base_health(&self, def: usize) -> u32 {
        let d = &self.content.mobs[def];
        let share = if d.critter || d.inert() { 1.0 } else { self.content.combat.foe_health };
        // A wager's level, and the dungeon's tier, below.
        let below = if d.critter || d.inert() { 1.0 } else { self.wager_scale(def).0 * self.tier_scale().0 };
        (d.health as f32 * share * below).round().max(1.0) as u32
    }

    /// Whether a creature grows with the heroes in its fight: not below (every hero in the mines
    /// counts there), not in the arena (it sends more foes instead), not a critter, a dummy or a
    /// totem.
    fn grows_with_fight(&self, mid: Id) -> bool {
        let Some(m) = self.mobs.get(&mid) else { return false };
        let d = &self.content.mobs[m.def];
        !self.is_below() && !self.is_arena_foe(mid) && !d.critter && !d.inert()
    }

    /// Heroes in a creature's fight now: those standing near it, and any on it lately from
    /// farther off.
    pub(crate) fn heroes_in_fight(&self, mid: Id) -> u32 {
        let now = self.clock;
        let Some(m) = self.mobs.get(&mid) else { return 1 };
        let pos = m.pos;
        let n = self
            .players
            .values()
            .filter(|p| p.health > 0 && ((dist2d(p.pos, pos) <= FIGHT_RANGE && (p.pos[2] - pos[2]).abs() < 10.0) || p.foe.is_some_and(|(f, at)| f == mid && now - at <= 6.0)))
            .count();
        n.max(1) as u32
    }

    /// A creature takes in the heroes who've joined its fight: `combat.hero_health` more health
    /// for each past the first (what it has taken stays taken), and harder blows. It never
    /// shrinks until it's home again.
    pub(crate) fn join_fight(&mut self, mid: Id) {
        if !self.grows_with_fight(mid) {
            return;
        }
        let n = self.heroes_in_fight(mid);
        let m = &self.mobs[&mid];
        if n <= m.fighters || m.health == 0 {
            return;
        }
        let base = self.base_health(m.def) as f32;
        let max = (base * (1.0 + self.content.combat.hero_health * (n - 1) as f32)).round() as u32;
        let m = self.mobs.get_mut(&mid).unwrap();
        m.fighters = n;
        if max > m.max_health {
            m.health += max - m.max_health;
            m.max_health = max;
            let (health, max) = (m.health, m.max_health);
            self.events.push(Event::Health { id: mid, health, max });
        }
    }

    /// How hard a creature's blows land: `combat.foe_damage` times its own, and `hero_damage`
    /// more for each hero past the first in its fight (below, each hero in the mines).
    pub(crate) fn foe_blow_share(&self, mid: Id) -> f32 {
        let c = &self.content.combat;
        let Some(m) = self.mobs.get(&mid) else { return c.foe_damage };
        let n = if self.is_below() { self.players.len().max(1) as u32 } else { m.fighters };
        let siege = if self.is_siege_foe(mid) { self.siege_def().map_or(1.0, |d| d.foe_damage) } else { 1.0 };
        c.foe_damage * siege * (1.0 + c.hero_damage * n.saturating_sub(1) as f32)
    }

    pub(crate) fn place_packs(&mut self) {
        for k in 0..self.content.packs.len() {
            if self.pack_wanted(k) {
                self.place_pack(k);
            }
        }
    }

    /// One pack laid out at its spot.
    pub(crate) fn place_pack(&mut self, k: usize) {
        {
            let pack = self.content.packs[k].clone();
            let def = self.content.mob(&pack.mob).unwrap();
            // A wanderer comes at one of its spots.
            let mid = self.pack_spot(k);
            for n in 0..pack.count {
                let a = n as f32 / pack.count.max(1) as f32 * std::f32::consts::TAU + self.rng.gen::<f32>() * 0.6;
                let r = if pack.count > 1 { pack.radius * (0.4 + 0.6 * self.rng.gen::<f32>()) } else { 0.0 };
                // Not in a trunk nor a wall: round the circle, then nearer its middle.
                let z = pack.z.unwrap_or(TOP);
                let at = (0..12)
                    .map(|t| {
                        // (One alone at its spot steps out from it, a little farther each try.)
                        let (aa, rr) = (a + t as f32 * 0.9, r * (1.0 - t as f32 / 12.0) + t as f32 * 0.6);
                        self.ground_near(mid[0] + aa.cos() * rr, mid[1] + aa.sin() * rr, z)
                    })
                    .find(|p| {
                        !self.land.blocked_near(p[0], p[1], p[2])
                            // One down in a cave stays on its own floor (not the hill's slope over it).
                            && pack.z.is_none_or(|want| (p[2] - want).abs() < 3.0)
                            && (self.is_below() || pack.z.is_some() || !self.steep_at(p[0], p[1], 2.4) && !self.up_on_something(p[0], p[1]))
                    })
                    .unwrap_or_else(|| if pack.z.is_some() { self.ground_near(mid[0], mid[1], z) } else { self.ground_near(mid[0] + a.cos() * r, mid[1] + a.sin() * r, z) });
                self.spawn_mob(def, at, Some(k), false);
            }
        }
    }

    /// Where a pack stands this time: its spot, or one of its `spots` at random.
    pub(crate) fn pack_spot(&mut self, k: usize) -> [f32; 2] {
        let p = &self.content.packs[k];
        if p.spots.is_empty() {
            return p.at;
        }
        let i = self.rng.gen_range(0..p.spots.len());
        self.content.packs[k].spots[i]
    }

    pub(crate) fn spawn_mob(&mut self, def: usize, at: Vec3, pack: Option<usize>, raider: bool) -> Id {
        let id = self.alloc();
        let health = self.base_health(def);
        let facing = self.rng.gen::<f32>() * std::f32::consts::TAU;
        self.mobs.insert(
            id,
            Mob {
                def,
                rally: 0,
                pos: at,
                facing,
                home: at,
                health,
                max_health: health,
                state: MobState::Idle { wander_at: self.clock + 3.0 + self.rng.gen::<f32>() * 6.0 },
                ready_at: 0.0,
                pack,
                raider,
                stagger_until: 0.0,
                thrown_until: 0.0,
                riposte: false,
                special: false,
                pattern: None,
                lunged: false,
                guarding: false,
                circle: if id % 2 == 0 { 1.0 } else { -1.0 },
                sidle: None,
                enraged: false,
                phase: 0,
                rise_at: None,
                lob: None,
                posture: 0,
                posture_from: 0.0,
                posture_guard: 0.0,
                path: Vec::new(),
                speed: 0.0,
                think: 0.0,
                hidden: false,
                lost_at: None,
                plan_to: None,
                form_of: None,
                panic_stride: 0.0,
                voiced: false,
                backstabs: Vec::new(),
                turn_on: None,
                fighters: 1,
                wary_until: 0.0,
                threat: Vec::new(),
                retarget_at: 0.0,
                drawn_off: None,
            },
        );
        // An ambusher lies unseen until a hero comes near (or its chest is opened).
        let hidden = pack.is_some_and(|k| self.content.packs[k].hidden > 0.0 || !self.content.packs[k].wake.is_empty());
        if hidden {
            self.mobs.get_mut(&id).unwrap().hidden = true;
        } else {
            self.events.push(Event::MobSpawned { id });
        }
        id
    }

    /// For testing (debug on): a creature a few strides in front of the hero, on them at once.
    pub fn summon(&mut self, id: Id, key: &str) -> Result<String, String> {
        let def = self.content.mob(key).ok_or_else(|| {
            let keys: Vec<&str> = self.content.mobs.iter().map(|m| m.key.as_str()).collect();
            format!("No creature {key}: try {}.", keys.join(", "))
        })?;
        let p = self.players.get(&id).ok_or("No hero.")?;
        let at = self.ground_near(p.pos[0] + p.facing.cos() * 8.0, p.pos[1] + p.facing.sin() * 8.0,
            p.pos[2],
        );
        let m = self.spawn_mob(def, at, None, false);
        self.engage(m, Foe::Hero(id));
        Ok(self.content.mobs[def].name.clone())
    }

    /// Patrols move on: when a pack's wait at a point is up, those of it at rest walk to the next
    /// (each keeping its place in the pack), and it's home for them meanwhile.
    pub(crate) fn step_patrols(&mut self) {
        let now = self.clock;
        for k in 0..self.content.packs.len() {
            let pack = &self.content.packs[k];
            if pack.route.is_empty() {
                continue;
            }
            let (at, next_at) = self.patrols.get(&k).copied().unwrap_or((0, 0.0));
            if now < next_at {
                continue;
            }
            // The route there and back: the start, the points, then back again.
            let mut stops = vec![pack.at];
            stops.extend(pack.route.iter().copied());
            let n = stops.len();
            let cycle = 2 * (n - 1);
            let step = (at + 1) % cycle;
            let i = if step < n { step } else { cycle - step };
            let to = stops[i];
            let wait = pack.wait;
            let members: Vec<Id> = self
                .mobs
                .iter()
                .filter(|(_, m)| m.pack == Some(k) && m.health > 0 && matches!(m.state, MobState::Idle { .. }))
                .map(|(id, _)| *id)
                .collect();
            // A pack in a fight waits for it to end.
            let fighting = self.mobs.values().any(|m| m.pack == Some(k) && m.health > 0 && m.state.foe().is_some());
            if fighting || members.is_empty() {
                self.patrols.insert(k, (at, now + 2.0));
                continue;
            }
            let mut longest = 0.0f32;
            for (j, id) in members.iter().enumerate() {
                let a = j as f32 * 2.4;
                let r = if members.len() > 1 { 1.8 } else { 0.0 };
                let goal = self.ground_near(to[0] + a.cos() * r, to[1] + a.sin() * r,
                    self.mobs[id].pos[2],
                );
                let m = &self.mobs[id];
                let (from, speed) = (m.pos, self.content.mobs[m.def].walk_speed.max(1.0));
                longest = longest.max(dist2d(from, goal) / speed);
                let m = self.mobs.get_mut(id).unwrap();
                m.home = goal;
                m.state = MobState::Idle { wander_at: f32::MAX };
                self.go(*id, goal, speed);
            }
            self.patrols.insert(k, (step, now + longest * 1.3 + wait));
        }
    }

    pub(crate) fn step_respawns(&mut self) {
        let now = self.clock;
        let (due, keep): (Vec<_>, Vec<_>) = std::mem::take(&mut self.respawns).into_iter().partition(|r| r.3 <= now);
        self.respawns = keep;
        for (def, home, pack, _) in due {
            // Not in front of a hero's nose: wait while one is near.
            if self.players.values().any(|p| dist2d(p.pos, home) < 25.0) {
                self.respawns.push((def, home, pack, now + 10.0));
                continue;
            }
            // A wanderer comes back somewhere else.
            let home = match pack.filter(|k| !self.content.packs[*k].spots.is_empty()) {
                Some(k) => {
                    let at = self.pack_spot(k);
                    self.ground(at[0], at[1])
                }
                None => home,
            };
            self.spawn_mob(def, home, pack, false);
            // A rare back is news: where it's been seen.
            if pack.is_some_and(|k| self.content.packs[k].rare) && !self.is_below() {
                let name = self.content.mobs[def].name.clone();
                let near = self.place_near([home[0], home[1]]).map_or(String::new(), |p| format!(" near {p}"));
                self.announce(format!("{name} has been seen again{near}."));
            }
        }
    }

    /// `mid` turns on `foe`, and its neighbours within its pack's link come too.
    pub(crate) fn engage(&mut self, mid: Id, foe: Foe) {
        let mut queue = vec![mid];
        let mut seen = vec![mid];
        while let Some(m) = queue.pop() {
            let Some(mob) = self.mobs.get_mut(&m) else { continue };
            if mob.health == 0 {
                continue;
            }
            // A harmless creature runs instead.
            if self.content.mobs[mob.def].critter {
                if let Foe::Hero(h) = foe {
                    self.flee_from(m, h);
                }
                continue;
            }
            let Some(mob) = self.mobs.get_mut(&m) else { continue };
            if (matches!(mob.state, MobState::Idle { .. } | MobState::Return) || m == mid)
                && !matches!(mob.state, MobState::Windup { .. } | MobState::Recover { .. })
            {
                // A patrol's leash runs from where it was caught, not from where it was bound.
                if matches!(mob.state, MobState::Idle { .. }) && mob.pack.is_some_and(|k| !self.content.packs[k].route.is_empty()) {
                    mob.home = mob.pos;
                }
                mob.state = MobState::Chase { foe };
                mob.think = 0.0;
                // Its voice as it turns on them (once until it's home again).
                let sound = self.content.mobs[mob.def].aggro_sound;
                if m == mid && sound != 0 && !mob.voiced {
                    mob.voiced = true;
                    self.events.push(Event::Sound { id: m, sound });
                }
            }
            let (pos, link) = (mob.pos, mob.pack.map_or(0.0, |k| self.content.packs[k].link));
            if link <= 0.0 {
                continue;
            }
            let near: Vec<Id> = self
                .mobs
                .iter()
                .filter(|(id, o)| !seen.contains(id) && o.health > 0 && !o.raider && !o.hidden && matches!(o.state, MobState::Idle { .. }) && dist2d(o.pos, pos) <= link)
                .map(|(id, _)| *id)
                .collect();
            for n in near {
                seen.push(n);
                queue.push(n);
            }
        }
    }

    /// A hero's weapon (or a raid's tower, the spikes) hurts a creature.
    pub(crate) fn hurt_mob(&mut self, mid: Id, by: Id, damage: u32) {
        self.hurt_mob_as(mid, by, damage, "Melee");
    }

    /// A creature hurt by `by`: what took it (`label`) for a hero's damage meter.
    pub(crate) fn hurt_mob_as(&mut self, mid: Id, by: Id, damage: u32, label: &str) {
        let now = self.clock;
        // A frozen creature's ice takes it first; a panic or an incapacitate may end.
        let damage = self.cc_on_damage(mid, damage);
        let Some(m) = self.mobs.get_mut(&mid) else { return };
        if m.health == 0 {
            return;
        }
        let before = m.health;
        m.health = m.health.saturating_sub(damage);
        let (h, max) = (m.health, m.max_health);
        self.events.push(Event::Health { id: mid, health: h, max });
        self.meter_dealt(by, mid, before - h, label);
        self.note_threat(mid, by, damage);
        if let Some(p) = self.players.get_mut(&by) {
            p.combat_at = now;
        }
        if h == 0 {
            self.mob_died(mid, by);
            return;
        }
        // A champion's posture: enough damage in a short while breaks its stance. It takes as much
        // more for each hero in its fight as its health does, and none builds while it reels
        // from a break or just after.
        let limit = self.content.mobs[self.mobs[&mid].def].posture;
        let fighters = self.mobs[&mid].fighters.max(1);
        let limit = (limit as f32 * (1.0 + self.content.combat.hero_health * (fighters - 1) as f32)).round() as u32;
        if limit > 0 && self.players.contains_key(&by) && now >= self.mobs[&mid].posture_guard {
            let window = self.content.mobs[self.mobs[&mid].def].posture_secs;
            let m = self.mobs.get_mut(&mid).unwrap();
            if now - m.posture_from > window {
                m.posture = 0;
                m.posture_from = now;
            }
            let heft = self.players.get(&by).map_or(1.0, |p| self.weapon_of(p).posture).max(1.0);
            // Struck just after its own blow, it's open: that counts more.
            let m = self.mobs.get_mut(&mid).unwrap();
            let open = matches!(m.state, MobState::Recover { .. }) && m.stagger_until <= now;
            let heft = if open { heft * self.content.combat.recovery_posture.max(1.0) } else { heft };
            m.posture += (damage as f32 * heft).round() as u32;
            if m.posture >= limit {
                m.posture = 0;
                m.posture_from = now;
                m.posture_guard = now + POSTURE_BREAK + POSTURE_GRACE;
                self.stagger_mob(mid, POSTURE_BREAK, true);
                self.events.push(Event::Flash { player: by, text: "Stance broken!".into() });
                self.events.push(Event::Visual { id: mid, kit: combat::KIT_DUST, by: None });
                return;
            }
        }
        if self.due_enrage(mid) && self.mobs[&mid].stagger_until <= now {
            self.enrage(mid, by);
            return;
        }
        // Struck by a hero (not a tower or the spikes): it turns on them.
        let m = &self.mobs[&mid];
        let hero = self.players.contains_key(&by);
        if hero && (matches!(m.state, MobState::Idle { .. } | MobState::Return) || (m.raider && matches!(m.state.foe(), Some(Foe::Obj(_))))) {
            // Mid-blow at a station (a Hearth, a wall): the blow lands, and then it's theirs.
            if matches!(m.state, MobState::Windup { .. } | MobState::Recover { .. }) {
                self.mobs.get_mut(&mid).unwrap().drawn_off = Some(by);
            }
            self.engage(mid, Foe::Hero(by));
        }
    }

    /// A harmless creature bolts away from a hero, and settles where it stops.
    pub(crate) fn flee_from(&mut self, mid: Id, from: Id) {
        let Some(h) = self.players.get(&from).map(|p| p.pos) else { return };
        let Some(m) = self.mobs.get(&mid) else { return };
        let away = facing(h, m.pos) + (self.rng.gen::<f32>() - 0.5);
        // Away to open ground: a little more to one side, then the other, if a trunk is there.
        let at = m.pos;
        let to = [0.0f32, 0.5, -0.5, 1.0, -1.0, 1.5, -1.5]
            .iter()
            .map(|turn| self.ground_near(at[0] + (away + turn).cos() * 14.0, at[1] + (away + turn).sin() * 14.0, at[2]))
            .find(|p| !self.land.blocked_near(p[0], p[1], p[2]))
            .unwrap_or(at);
        let speed = self.content.mobs[m.def].run_speed;
        let now = self.clock;
        let m = self.mobs.get_mut(&mid).unwrap();
        m.home = to;
        m.state = MobState::Idle { wander_at: now + 8.0 };
        self.go(mid, to, speed);
    }

    /// Whether a creature's next phase is due (below its share, and not had yet).
    pub(crate) fn due_enrage(&self, mid: Id) -> bool {
        self.mobs.get(&mid).is_some_and(|m| {
            m.health > 0 && self.content.mobs[m.def].phases.get(m.phase).is_some_and(|e| m.health as f32 <= e.at * m.max_health as f32)
        })
    }

    /// The phase a creature is in, if one has begun.
    pub(crate) fn phase_now(&self, mid: Id) -> Option<&crate::content::EnrageDef> {
        let m = self.mobs.get(&mid)?;
        m.phase.checked_sub(1).and_then(|k| self.content.mobs[m.def].phases.get(k))
    }

    /// A creature reels for `secs`: a wind-up in hand is lost, it stands and does nothing, and (from
    /// a parry) takes more from every blow meanwhile.
    pub(crate) fn stagger_mob(&mut self, mid: Id, secs: f32, riposte: bool) {
        let now = self.clock;
        let Some(m) = self.mobs.get(&mid) else { return };
        if m.health == 0 {
            return;
        }
        // A cast cut off ends (the step's own: a fighter's hex too).
        let ranged = self.attack_now(mid).ranged;
        let m = &self.mobs[&mid];
        let (mark, casting) = match m.state {
            MobState::Windup { mark, .. } => (mark, ranged),
            _ => (None, false),
        };
        let foe = m.state.foe();
        if let Some(mk) = mark {
            self.unmark(mk);
        }
        if let Some((_, mk)) = self.mobs.get_mut(&mid).and_then(|m| m.lob.take()) {
            self.unmark(mk);
        }
        if casting {
            self.events.push(Event::CastEnd { caster: mid });
        }
        let m = self.mobs.get_mut(&mid).unwrap();
        // A second stagger doesn't cut a longer one short.
        let until = m.stagger_until.max(now + secs);
        let secs = until - now;
        m.riposte = riposte || (m.riposte && m.stagger_until > now);
        m.stagger_until = until;
        m.special = false;
        m.pattern = None;
        self.show_guard(mid, false);
        let m = self.mobs.get_mut(&mid).unwrap();
        m.ready_at = m.ready_at.max(now + secs);
        if let Some(f) = foe {
            m.state = MobState::Recover { foe: f, until: now + secs };
        }
        self.halt(mid);
        self.events.push(Event::Emote { id: mid, emote: STUN_EMOTE });
        // Reeling back from the blow that did it, in front of it.
        let m = &self.mobs[&mid];
        let from = [m.pos[0] + m.facing.cos(), m.pos[1] + m.facing.sin(), m.pos[2]];
        self.events.push(Event::React { id: mid, anim: combat::STUN_ANIM, from, lean: 0.3, hold: secs, fit: 0.0 });
    }

    /// Its next phase: whatever it was doing, the phase's pattern at once, and its pace and might
    /// from now.
    fn enrage(&mut self, mid: Id, by: Id) {
        let Some(m) = self.mobs.get_mut(&mid) else { return };
        m.enraged = true;
        let k = m.phase;
        m.phase += 1;
        let def = m.def;
        let foe = m.state.foe().unwrap_or(Foe::Hero(by));
        let winding = matches!(m.state, MobState::Windup { .. });
        if let MobState::Windup { mark: Some(mk), .. } = m.state {
            self.unmark(mk);
        }
        if winding && self.attack_now(mid).ranged {
            self.events.push(Event::CastEnd { caster: mid });
        }
        let m = self.mobs.get_mut(&mid).unwrap();
        m.stagger_until = 0.0;
        m.riposte = false;
        m.special = false;
        m.pattern = None;
        let Some(e) = self.content.mobs[def].phases.get(k).cloned() else { return };
        // Its turn shown: a flash on it, a burst at its feet, heroes close thrown down.
        if e.kit != 0 {
            self.events.push(Event::Visual { id: mid, kit: e.kit, by: None });
        }
        let at = self.mobs[&mid].pos;
        if e.look != 0 {
            self.events.push(Event::Missile { caster: mid, spell: e.look, target: Target::Point(at) });
        }
        // A blast all round as it turns, marked a moment ahead: roll out or be thrown down.
        if e.shove > 0.0 {
            let now = self.clock;
            // Its own look's ground (fire, shadow...), else dust kicked up.
            let ground = match ground_look(0, e.look) {
                0 => 7272,
                g => g,
            };
            let mark = self.mark_ground(at, e.shove, PHASE_SHOVE_WARN, ground);
            let damage = (self.content.mobs[def].attack.damage as f32 * 0.5).round() as u32;
            self.missiles.push(combat::Missile { owner: mid, from: at, at, radius: e.shove, damage, blockable: false, heavy: true, land_at: now + PHASE_SHOVE_WARN, mark, look: 0 });
        }
        if !e.text.is_empty() {
            let name = self.content.mobs[def].name.clone();
            self.announce(format!("{name}: {}", e.text));
        }
        // Help called from the dark, on its foe at once.
        let at = self.mobs[&mid].pos;
        for (n, k) in e.summon.iter().enumerate() {
            let Some(sd) = self.content.mob(k) else { continue };
            let a = n as f32 * std::f32::consts::PI + 0.8;
            let p = self.ground_near(at[0] + a.cos() * 5.0, at[1] + a.sin() * 5.0, at[2]);
            let s = self.spawn_mob(sd, p, None, false);
            if let Some(m) = self.mobs.get_mut(&s) {
                m.home = at;
            }
            self.events.push(Event::Visual { id: s, kit: combat::KIT_DUST, by: None });
            self.engage(s, foe);
        }
        self.events.push(Event::Emote { id: mid, emote: ROAR_EMOTE });
        let sound = if e.sound != 0 { e.sound } else { self.content.mobs[def].enrage_sound };
        if sound != 0 {
            self.events.push(Event::Sound { id: mid, sound });
        }
        let k = self.content.mobs[def].patterns.iter().position(|p| p.name == e.pattern);
        match (k, self.foe_pos(foe)) {
            (Some(k), Some(to)) => {
                self.mobs.get_mut(&mid).unwrap().pattern = Some((k, 0));
                self.windup(mid, foe, to);
            }
            _ => {
                let m = self.mobs.get_mut(&mid).unwrap();
                m.state = MobState::Chase { foe };
                m.think = 0.0;
            }
        }
    }

    /// Whether what a creature's death wakes (`then_needs`) is so: a secret or door below opened,
    /// a boss beaten for good, or an item of that key in its killer's bags.
    pub(crate) fn then_allowed(&self, needs: &str, killer: Id) -> bool {
        if needs.is_empty() {
            return true;
        }
        let below = self.below.as_ref().is_some_and(|b| b.open.contains(needs) || b.slain.contains(needs));
        let carried = self.content.item(needs).is_some_and(|i| self.players.get(&killer).is_some_and(|p| p.bag.get(&i).copied().unwrap_or(0) > 0));
        below || carried
    }

    /// What stepped out of a fallen creature (`then`) shows itself and turns on `on`: its flash,
    /// a ground look, its line.
    pub(crate) fn rise(&mut self, s: Id, on: Id, of_def: usize) {
        let of = self.content.mobs[of_def].clone();
        let Some(m) = self.mobs.get_mut(&s) else { return };
        m.rise_at = None;
        let (at, nd) = (m.pos, m.def);
        if std::mem::take(&mut m.hidden) {
            self.events.push(Event::MobSpawned { id: s });
        }
        // The body it rose from goes as it stands.
        let bodies: Vec<Id> = self.mobs.iter().filter(|(_, x)| of.then_after > 0.0 && matches!(x.state, MobState::Dead { .. }) && x.def == of_def && dist2d(x.pos, at) < 2.0).map(|(id, _)| *id).collect();
        for b in bodies {
            self.mobs.remove(&b);
            self.events.push(Event::MobGone { id: b });
        }
        let kit = if of.then_kit != 0 { of.then_kit } else { combat::KIT_DUST };
        self.events.push(Event::Visual { id: s, kit, by: None });
        if of.then_look != 0 {
            self.events.push(Event::Missile { caster: s, spell: of.then_look, target: Target::Point(at) });
        }
        if !of.then_text.is_empty() {
            let name = self.content.mobs[nd].name.clone();
            self.announce(format!("{name}: {}", of.then_text));
        }
        if of.then_after > 0.0 {
            self.events.push(Event::Emote { id: s, emote: ROAR_EMOTE });
            let sound = self.content.mobs[nd].aggro_sound;
            if sound != 0 {
                self.events.push(Event::Sound { id: s, sound });
            }
        }
        // On whoever felled the last form, else the nearest hero standing.
        let foe = Some(on).filter(|h| self.players.get(h).is_some_and(|p| p.health > 0)).or_else(|| {
            self.players.iter().filter(|(_, p)| p.health > 0).min_by(|a, b| dist2d(a.1.pos, at).total_cmp(&dist2d(b.1.pos, at))).map(|(h, _)| *h)
        });
        if let Some(h) = foe {
            self.engage(s, Foe::Hero(h));
        }
    }

    /// Secret forms whose time has come rise.
    fn step_rises(&mut self) {
        let now = self.clock;
        let due: Vec<(Id, Id, usize)> = self.mobs.iter().filter_map(|(id, m)| m.rise_at.filter(|(t, _, _)| *t <= now).map(|(_, on, of)| (*id, on, of))).collect();
        for (s, on, of) in due {
            self.rise(s, on, of);
        }
    }

    fn mob_died(&mut self, mid: Id, killer: Id) {
        let now = self.clock;
        self.death_mark_reset(mid);
        if matches!(self.mobs.get(&mid).map(|m| &m.state), Some(MobState::Windup { .. })) && self.attack_now(mid).ranged {
            self.events.push(Event::CastEnd { caster: mid });
        }
        let Some(m) = self.mobs.get_mut(&mid) else { return };
        if let MobState::Windup { mark: Some(mk), .. } = m.state {
            self.unmark(mk);
        }
        if let Some((_, mk)) = self.mobs.get_mut(&mid).and_then(|m| m.lob.take()) {
            self.unmark(mk);
        }
        self.show_guard(mid, false);
        let m = self.mobs.get_mut(&mid).unwrap();
        m.state = MobState::Dead { gone_at: now + 4.0 };
        // It falls where it is: whatever walk, lunge or step it was in stops with it.
        let walking = !m.path.is_empty();
        m.path.clear();
        m.sidle = None;
        let (at, face) = (m.pos, m.facing);
        if walking {
            self.events.push(Event::MobStop { id: mid, pos: at, facing: face });
        }
        let m = self.mobs.get_mut(&mid).unwrap();
        let (def, home, pack, raider) = (m.def, m.home, m.pack, m.raider);
        self.events.push(Event::Died { id: mid });
        // Those locked on to it move on to the next foe in front of them (or let go).
        let locked: Vec<Id> = self.players.iter().filter(|(_, p)| p.lock == Some(mid)).map(|(h, _)| *h).collect();
        for h in locked {
            self.players.get_mut(&h).unwrap().lock = None;
            self.toggle_lock(h);
        }
        let key = self.content.mobs[def].key.clone();
        let sound = self.content.mobs[def].death_sound;
        if sound != 0 {
            self.events.push(Event::Sound { id: mid, sound });
        }
        if !self.content.mobs[def].inert() {
            self.gain_valor(killer, self.content.valor.kill);
        }
        // Siege: gold for the hero who felled it, and nothing else of Elwynn's (trophies, cards,
        // drops, the dead coming back).
        if self.is_siege() {
            self.siege_kill(mid, killer);
            self.events.push(Event::Dirty);
            return;
        }
        // The first of its kind the camp kills leaves its trophy in the chest (from below, by the
        // book).
        let first = self.camp.counts.get(&format!("kill:{key}")).copied().unwrap_or(0) == 0;
        self.forward_count("kill", &key, 1);
        if first && !self.content.mobs[def].trophy.is_empty() {
            let trophy = self.content.mobs[def].trophy.clone();
            let mut names = Vec::new();
            let mut items = BTreeMap::new();
            for (k, n) in &trophy {
                let item = self.content.item(k).unwrap();
                *items.entry(item).or_default() += n;
                names.push(self.content.items[item].name.clone());
            }
            if self.is_below() {
                self.trophy_to_camp(items);
            } else {
                for (i, n) in items {
                    *self.storage.entry(i).or_default() += n;
                }
            }
            self.events.push(Event::Storage);
            let name = self.content.mobs[def].name.clone();
            self.events.push(Event::Banner { player: None, title: format!("{name} Falls"), sub: format!("Into the chest: {}", names.join(", ")) });
            self.announce(format!("{name} is dead. In the camp's chest: {}.", names.join(", ")));
        }
        let d = self.content.mobs[def].clone();
        let killer_name = self.players.get(&killer).map(|p| p.name.clone());
        // A class's champion: every hero who fought it (near it as it fell) learns its cards.
        if !d.teaches.is_empty() && !self.is_below() {
            let at = self.mobs[&mid].pos;
            let fought: Vec<Id> = self.players.iter().filter(|(h, p)| **h == killer || dist2d(p.pos, at) <= TEACH_RANGE).map(|(h, _)| *h).collect();
            self.teach(&fought, &d.teaches, &format!("{} has fallen", d.name));
        }
        if pack.is_some_and(|k| self.content.packs[k].rare) {
            // A rare creature's fall is news.
            let by = killer_name.map_or(String::new(), |n| format!(" to {n}"));
            self.events.push(Event::Banner { player: None, title: format!("{} Falls", d.name), sub: String::new() });
            self.announce(format!("{} has fallen{by}.", d.name));
        }
        // What steps out of it (a shredder's driver): in its place and its pack, so the pack
        // stands until that falls too.
        // A secret form (`then_after`) lies unseen where it fell and rises in a while: the fight
        // seemed won. `then_needs` keeps it for those who found what wakes it.
        let rises = self.content.mob(&d.then).filter(|_| self.then_allowed(&d.then_needs, killer));
        if let Some(nd) = rises {
            let at = self.mobs[&mid].pos;
            let s = self.spawn_mob(nd, at, pack, raider);
            if self.is_arena_foe(mid) {
                self.arena.foes.push(s);
            }
            let max = if self.is_below() { self.foe_health(nd) } else { self.base_health(nd) };
            let waits = d.then_after > 0.0;
            // Its first form (the one put back at a rest), down a chain of them.
            let root = self.mobs[&mid].form_of.unwrap_or(def);
            if let Some(m) = self.mobs.get_mut(&s) {
                m.health = max;
                m.max_health = max;
                m.home = home;
                m.facing = face;
                m.form_of = Some(root);
                if waits {
                    m.hidden = true;
                    m.rise_at = Some((now + d.then_after, killer, def));
                } else if std::mem::take(&mut m.hidden) {
                    self.events.push(Event::MobSpawned { id: s });
                }
            }
            if waits {
                // (Not there to be seen yet.)
                self.events.retain(|e| !matches!(e, Event::MobSpawned { id } if *id == s));
            } else {
                self.rise(s, killer, def);
            }
        }
        // A pack's creatures come back; one called up for testing doesn't. Below, they come
        // back when a hero rests (and a boss never does).
        if self.is_below() {
            let at = self.mobs[&mid].pos;
            let form = self.mobs[&mid].form_of.is_some();
            self.below_kill(def, home, at, pack, raider, form);
        } else if !raider && pack.is_some() {
            self.respawns.push((def, home, pack, now + d.respawn));
        }
        // What it drops goes straight into the killer's bags.
        if self.players.contains_key(&killer) {
            let mut got = Vec::new();
            for (k, range) in &d.drops {
                let item = self.content.item(k).unwrap();
                let n = self.roll(*range);
                if n == 0 {
                    continue;
                }
                let left = self.give(killer, item, n);
                got.push(format!("{} {}", n - left, self.content.items[item].name));
                if left > 0 {
                    self.tell(killer, format!("Your bags are full: {left} {} left behind.", self.content.items[item].name));
                }
            }
            if !got.is_empty() {
                self.tell(killer, format!("Looted: {}.", got.join(", ")));
            }
        }
        self.events.push(Event::Dirty);
        if self.is_arena_foe(mid) {
            self.arena_news();
        } else if raider {
            self.raider_down();
        }
        // The last of a Horde camp: it's taken.
        if let Some(k) = pack {
            let camp = self.content.packs[k].camp.clone();
            if !camp.is_empty() {
                // Taken when none stand and none are about to come back (the first of them back
                // after a hold, picked off, don't make it taken again).
                let content = &self.content;
                let standing = self.mobs.values().any(|m| m.health > 0 && m.pack.is_some_and(|p| content.packs[p].camp == camp));
                let returning = self.respawns.iter().any(|r| r.2.is_some_and(|p| content.packs[p].camp == camp) && r.3 < now + CAMP_RETURNING);
                if !standing && !returning {
                    self.take_camp(&camp);
                }
            }
        }
    }

    /// Every fighter of a Horde camp dead: it's the heroes' for a while (none come back there
    /// meanwhile), and the first time it pays into the chest.
    fn take_camp(&mut self, key: &str) {
        let Some(c) = self.content.camps.iter().find(|c| c.key == key).cloned() else { return };
        let until = self.clock + c.hold;
        let content = &self.content;
        for r in self.respawns.iter_mut() {
            if r.2.is_some_and(|p| content.packs[p].camp == key) {
                r.3 = r.3.max(until);
            }
        }
        let first = self.camp.counts.get(&format!("clear:{key}")).copied().unwrap_or(0) == 0;
        self.count("clear", key, 1);
        let mins = (c.hold / 60.0).round() as u32;
        let mut sub = format!("The Horde won't be back for {mins} minutes");
        if first && !c.reward.is_empty() {
            for (k, n) in &c.reward {
                let item = self.content.item(k).unwrap();
                *self.storage.entry(item).or_default() += n;
            }
            self.events.push(Event::Storage);
            sub = format!("{sub}. Into the chest: {}", self.cost_text(&c.reward));
        }
        self.events.push(Event::Banner { player: None, title: c.title.clone(), sub });
        self.announce(format!("The Horde is driven from {}! None come back there for {mins} minutes.", c.name));
    }

    pub(crate) fn step_mobs(&mut self, dt: f32) {
        self.step_rises();
        let now = self.clock;
        let ids: Vec<Id> = self.mobs.keys().copied().collect();
        let heroes: Vec<Vec3> = self.players.values().map(|p| p.pos).collect();
        for id in ids {
            // Gone already this step (a totem broken, a creature another one's turn removed).
            if !self.mobs.contains_key(&id) {
                continue;
            }
            // Bodies go.
            if let Some(MobState::Dead { gone_at }) = self.mobs.get(&id).map(|m| m.state.clone()) {
                if now >= gone_at {
                    self.mobs.remove(&id);
                    self.events.push(Event::MobGone { id });
                }
                continue;
            }
            // A totem or a dummy only stands there; a secret form waits to rise.
            if self.content.mobs[self.mobs[&id].def].inert() || self.mobs[&id].rise_at.is_some() {
                continue;
            }
            // Heroes come to its fight: it grows with them.
            if self.mobs[&id].state.foe().is_some() {
                self.join_fight(id);
            }
            // Far from every hero, a creature at rest sleeps (the land is wide).
            if heroes.iter().all(|p| dist2d(*p, self.mobs[&id].pos) > SLEEP_RANGE) {
                let m = &self.mobs[&id];
                if matches!(m.state, MobState::Idle { .. }) && m.path.is_empty() {
                    continue;
                }
            }
            // Rooted or stunned: no walking; stunned, frozen or panicked: no thinking (a panic runs
            // by itself).
            let thrown = self.mobs[&id].thrown_until > now;
            if self.held_still(id) && !thrown {
                if !self.mobs[&id].path.is_empty() {
                    self.halt(id);
                }
            } else {
                self.walk(id, dt);
            }
            if self.disabled(id) {
                continue;
            }
            // A second wind that was due while it reeled comes once it stands again.
            if self.due_enrage(id) && self.mobs[&id].stagger_until <= now {
                if self.mobs[&id].state.foe().is_some() {
                    self.enrage(id, 0);
                    continue;
                }
            }
            self.think(id, dt);
        }
    }

    /// Move along the current path.
    fn walk(&mut self, id: Id, dt: f32) {
        let below = self.is_below();
        let Some(m) = self.mobs.get_mut(&id) else { return };
        if m.path.is_empty() || m.speed <= 0.0 {
            return;
        }
        let start = m.pos;
        let mut left = m.speed * dt;
        while left > 0.0 && !m.path.is_empty() {
            let next = m.path[0];
            let d = dist2d(m.pos, next);
            if d <= left {
                m.pos = next;
                left -= d;
                m.path.remove(0);
            } else {
                let t = left / d;
                m.pos = [m.pos[0] + (next[0] - m.pos[0]) * t, m.pos[1] + (next[1] - m.pos[1]) * t, m.pos[2]];
                m.facing = facing(m.pos, next);
                left = 0.0;
            }
        }
        let walked = m.speed * dt - left;
        let (x, y, was) = (m.pos[0], m.pos[1], m.pos[2]);
        let leg_end = m.path.first().map(|p| p[2]);
        let mut z = self.land.height_near(x, y, was);
        // Below, partway along a leg that ends on the floor it's on, it doesn't drop to the floor
        // under it: the way was looked over every half yard and found whole, and a gap between two
        // of those looks (a slit in a deck's planks, a corner of its edge) is stepped over, not
        // fallen through to the hold, where it would stand under its foe with no way back up.
        if below && leg_end.is_some_and(|e| e - z > FALL) && was - z > FALL + WALK_SLOPE * walked {
            z = was;
        }
        // Stepping round a foe: turned to them, not the way it goes.
        let face = m.sidle.and_then(|h| self.players.get(&h)).map(|p| p.pos);
        let m = self.mobs.get_mut(&id).unwrap();
        m.pos[2] = z;
        if let Some(at) = face {
            m.facing = facing(m.pos, at);
        }
        if m.path.is_empty() {
            m.sidle = None;
        }
        // Below, fallen through its floor however it came to (more of a drop at once than any walk
        // or slide makes): back where it stood, in whatever fight it was in; on its way home, home.
        if below && start[2] - z > FELL_THROUGH.max(FALL + SLIDE_SLOPE * walked) {
            let to = if matches!(m.state, MobState::Return) { m.home } else { start };
            self.set_back(id, to, "fell through the floor");
        }
    }

    /// Below: a creature fallen through its floor, or cut off from its foe and its home alike, set
    /// down at `at` at once (everyone sees it there), its walk dropped and its fight kept. The log
    /// says so, so that the next time can be looked into.
    fn set_back(&mut self, id: Id, at: Vec3, why: &str) {
        let Some(m) = self.mobs.get_mut(&id) else { return };
        let was = m.pos;
        m.pos = at;
        m.path.clear();
        m.speed = 0.0;
        m.sidle = None;
        m.plan_to = None;
        m.lost_at = None;
        let (face, def) = (m.facing, m.def);
        self.events.push(Event::MobStop { id, pos: at, facing: face });
        let line = format!(
            "{} {why} at ({:.1}, {:.1}, {:.1}): put back at ({:.1}, {:.1}, {:.1})",
            self.content.mobs[def].name, was[0], was[1], was[2], at[0], at[1], at[2]
        );
        self.events.push(Event::Log(line));
    }

    /// Whether a creature can't get home from where it stands: not there (on home's floor), no
    /// straight walk and no way round. Only a land that searches can tell.
    fn cut_off(&self, id: Id) -> bool {
        let Some(m) = self.mobs.get(&id) else { return false };
        let (pos, home) = (m.pos, m.home);
        if !self.land.plans() || self.at_home(pos, home) {
            return false;
        }
        !self.land.walk_clear(pos, home) && self.land.path(pos, home).is_none_or(|p| p.is_empty())
    }

    fn go(&mut self, id: Id, to: Vec3, speed: f32) {
        // Snared: slower, as everyone sees it.
        let speed = speed * self.speed_share(id);
        let Some(m) = self.mobs.get(&id) else { return };
        let from = m.pos;
        // Straight only where a walker could: nothing solid in the way (a trunk, a wall) and no
        // slope too steep (a cliff); else round by the land's own way.
        // (A step away is straight too, but only on its own floor: a spot under the deck it stands
        // on, a step off, is no step down.)
        let straight = self.land.walk_clear(from, to) || (dist2d(from, to) < 0.8 && (to[2] - from[2]).abs() < FALL);
        // On a searched way to about the same place, at the same pace: it keeps to it (a search
        // is dear; a chase asks again every quarter second).
        if !straight && !m.path.is_empty() && (m.speed - speed).abs() < 0.01 && m.plan_to.is_some_and(|p| dist2d(p, to) <= (0.08 * dist2d(from, to)).max(1.5)) {
            return;
        }
        let path = if straight {
            vec![to]
        } else {
            match self.land.path(from, to) {
                Some(p) if !p.is_empty() => p,
                // Below, on its way home with no way there from another floor (under its floor,
                // cut off): put back home, not walked through the rock to it (nor stopped under it,
                // short of it). On home's floor a failed search walks on as before.
                _ if matches!(m.state, MobState::Return) && self.is_below() && self.land.plans() && self.off_home_floor(from, to) => {
                    self.set_back(id, to, "found no way home");
                    return;
                }
                // No way there: a chase stands rather than walking through the wall or up the cliff
                // (and gives up if it stays like this); a raider on its march, one on its way
                // home, or on a land that doesn't search, goes straight.
                _ if m.raider || matches!(m.state, MobState::Return) || !self.land.plans() => vec![to],
                _ => {
                    let now = self.clock;
                    let m = self.mobs.get_mut(&id).unwrap();
                    m.lost_at.get_or_insert(now);
                    self.halt(id);
                    return;
                }
            }
        };
        let m = self.mobs.get_mut(&id).unwrap();
        m.lost_at = None;
        m.plan_to = (!straight).then_some(to);
        m.path = path.clone();
        m.speed = speed;
        m.sidle = None;
        let mut shown = vec![from];
        shown.extend(path);
        self.events.push(Event::MobMove { id, path: shown, speed, face: None });
    }

    pub(crate) fn sidle_to(&mut self, id: Id, to: Vec3, speed: f32, foe: Id) {
        self.sidle(id, to, speed, foe);
    }

    /// Walk to `to` facing `foe` the while (circling, stepping back), in a straight line.
    fn sidle(&mut self, id: Id, to: Vec3, speed: f32, foe: Id) {
        let speed = speed * self.speed_share(id);
        let Some(m) = self.mobs.get(&id) else { return };
        let from = m.pos;
        if !self.land.walk_clear(from, to) || self.land.blocked_near(to[0], to[1], to[2]) {
            return;
        }
        let m = self.mobs.get_mut(&id).unwrap();
        m.path = vec![to];
        m.plan_to = None;
        m.speed = speed;
        m.sidle = Some(foe);
        self.events.push(Event::MobMove { id, path: vec![from, to], speed, face: Some(foe) });
    }

    /// Hero `hero`'s foes in the middle of an attack on them (winding up or between blows).
    fn attackers_on(&self, hero: Id, except: Id) -> usize {
        self.mobs
            .iter()
            // Up close: an archer or a caster loosing from afar isn't one of the two.
            .filter(|(id, m)| {
                **id != except
                    && m.health > 0
                    && !self.attack_now(**id).ranged
                    && match m.state {
                        MobState::Windup { foe: Foe::Hero(h), .. } => h == hero,
                        MobState::Recover { foe: Foe::Hero(h), .. } => h == hero && m.pattern.is_some(),
                        _ => false,
                    }
            })
            .count()
    }

    /// Step round the foe at about `radius`, facing them (waiting a turn, or keeping its distance).
    fn circle_foe(&mut self, id: Id, hero: Id, to: Vec3, radius: f32) {
        let Some(m) = self.mobs.get(&id) else { return };
        if !m.path.is_empty() {
            return;
        }
        let d = self.content.mobs[m.def].clone();
        let (pos, dir) = (m.pos, m.circle);
        // Toward their back, the shorter way round.
        let dir = self.flank_way(id, hero).unwrap_or(dir);
        let bearing = facing(to, pos) + dir * CIRCLE_STEP;
        let goal = self.ground_near(to[0] + bearing.cos() * radius, to[1] + bearing.sin() * radius,
            to[2],
        );
        self.sidle(id, goal, d.walk_speed.max(1.5), hero);
        // Now and then the other way round.
        if self.rng.gen::<f32>() < 0.15 {
            self.mobs.get_mut(&id).unwrap().circle = -dir;
        }
    }

    /// A hero heals in front of their foes: those close and on them come at once (a heal is an
    /// opening, as in the souls games).
    pub(crate) fn punish_healing(&mut self, hero: Id) {
        let Some(p) = self.players.get(&hero) else { return };
        let (pos, soon) = (p.pos, self.clock + PUNISH_SECS);
        for m in self.mobs.values_mut() {
            if m.health > 0 && m.state.foe() == Some(Foe::Hero(hero)) && matches!(m.state, MobState::Chase { .. }) && dist2d(m.pos, pos) <= PUNISH_RANGE {
                m.ready_at = m.ready_at.min(soon);
            }
        }
    }

    /// A shield-bearer's shield goes up while it closes on a foe or waits between blows.
    fn show_guard(&mut self, id: Id, up: bool) {
        let Some(m) = self.mobs.get_mut(&id) else { return };
        let has = self.content.mobs[m.def].shield != 0 && self.content.mobs[m.def].block > 0.0;
        let up = up && has;
        if m.guarding != up {
            m.guarding = up;
            self.events.push(Event::MobGuard { id, up });
        }
    }

    /// A hero winds up a heavy blow at the fighters in front of them: the quick ones may step back
    /// out of it.
    pub(crate) fn evade_heavy(&mut self, hero: Id) {
        let Some(p) = self.players.get(&hero) else { return };
        let (pos, f) = (p.pos, p.facing);
        let near: Vec<Id> = self
            .mobs
            .iter()
            .filter(|(_, m)| {
                m.health > 0
                    && self.content.mobs[m.def].evade > 0.0
                    && matches!(m.state, MobState::Chase { .. })
                    && m.stagger_until <= self.clock
                    && dist2d(m.pos, pos) <= 4.5
                    && angle_between(f, facing(pos, m.pos)) <= 60.0
            })
            .map(|(id, _)| *id)
            .collect();
        for id in near {
            let chance = self.content.mobs[self.mobs[&id].def].evade;
            if self.rng.gen::<f32>() >= chance {
                continue;
            }
            let m = &self.mobs[&id];
            let away = facing(pos, m.pos);
            let to = self.ground_near(m.pos[0] + away.cos() * BACKSTEP, m.pos[1] + away.sin() * BACKSTEP,
                m.pos[2],
            );
            self.sidle(id, to, BACKSTEP / BACKSTEP_SECS, hero);
            self.events.push(Event::React { id, anim: DODGE_ANIM, from: pos, lean: 0.0, hold: 0.0, fit: BACKSTEP_SECS + 0.15 });
            // Its next attack waits a moment after.
            let m = self.mobs.get_mut(&id).unwrap();
            m.ready_at = m.ready_at.max(self.clock + BACKSTEP_SECS);
        }
    }

    pub(crate) fn halt_mob(&mut self, id: Id) {
        self.halt(id);
    }

    /// A panicked creature runs a stride, out of control.
    pub(crate) fn flee_to(&mut self, id: Id, to: Vec3, speed: f32) {
        let Some(m) = self.mobs.get_mut(&id) else { return };
        let from = m.pos;
        m.path = vec![to];
        m.plan_to = None;
        m.speed = speed;
        m.sidle = None;
        m.facing = facing(from, to);
        self.events.push(Event::MobMove { id, path: vec![from, to], speed, face: None });
    }

    /// Thrown to `to` over `air` seconds: it lands there whatever else the blow did, turned the
    /// while to `face` (whoever threw it), so a knockback drives it back rather than turning it to
    /// run.
    pub(crate) fn throw_mob(&mut self, id: Id, to: Vec3, air: f32, face: Option<Id>) {
        let now = self.clock;
        let Some(m) = self.mobs.get_mut(&id) else { return };
        let from = m.pos;
        let speed = dist2d(from, to) / air.max(0.05);
        m.path = vec![to];
        m.plan_to = None;
        m.speed = speed;
        m.thrown_until = now + air;
        let face = face.filter(|f| self.players.contains_key(f));
        let m = self.mobs.get_mut(&id).unwrap();
        m.sidle = face;
        if let Some(at) = face.and_then(|f| self.players.get(&f)).map(|p| p.pos) {
            m.facing = facing(from, at);
        } else {
            m.facing = facing(from, to);
        }
        self.events.push(Event::MobMove { id, path: vec![from, to], speed, face });
    }

    fn halt(&mut self, id: Id) {
        let now = self.clock;
        let Some(m) = self.mobs.get_mut(&id) else { return };
        // In the air it comes down where it was thrown: nothing stops it short.
        if m.thrown_until > now && !m.path.is_empty() {
            return;
        }
        let was_moving = !m.path.is_empty();
        m.path.clear();
        m.speed = 0.0;
        if was_moving {
            let (pos, facing) = (m.pos, m.facing);
            self.events.push(Event::MobStop { id, pos, facing });
        }
    }

    fn foe_pos(&self, foe: Foe) -> Option<Vec3> {
        match foe {
            Foe::Hero(h) => self.players.get(&h).filter(|p| p.health > 0).map(|p| p.pos),
            Foe::Obj(o) => self.objs.get(&o).filter(|o| matches!(o.kind, ObjKind::Station { damaged: false, .. })).map(|o| o.pos),
        }
    }

    /// Whether another standing creature is within `r` of `at`.
    fn crowded(&self, id: Id, at: Vec3, r: f32) -> bool {
        self.mobs.iter().any(|(o, m)| *o != id && m.health > 0 && !m.hidden && dist2d(m.pos, at) < r)
    }

    /// Where to stand `r` from a foe at `foe`, coming from `from`: on its own side of them, or
    /// round a little either way when another already stands there (they spread out round the
    /// foe instead of stacking in one body).
    fn free_spot(&self, id: Id, foe: Vec3, from: Vec3, r: f32) -> Vec3 {
        let bearing = facing(foe, from);
        // Failing a free one: one nobody's in but open ground (not in a trunk), else the first.
        let (mut first, mut open) = (None, None);
        for k in [0.0f32, 1.0, -1.0, 2.0, -2.0, 3.0, -3.0] {
            let b = bearing + k * SPREAD_STEP;
            let at = self.ground_near(foe[0] + b.cos() * r, foe[1] + b.sin() * r, foe[2]);
            if first.is_none() {
                first = Some(at);
            }
            let clear = !self.land.blocked_near(at[0], at[1], at[2]);
            if clear && open.is_none() {
                open = Some(at);
            }
            if clear && !self.crowded(id, at, SPACING) {
                return at;
            }
        }
        open.or(first).unwrap_or(from)
    }

    /// Whether a chase from `home` has gone too far: the foe beyond the leash (with some slack),
    /// or the creature itself well beyond it.
    pub(crate) fn leashed(&self, home: Vec3, pos: Vec3, foe: Vec3, leash: f32) -> bool {
        dist2d(foe, home) > leash + LEASH_SLACK || dist2d(pos, home) > leash + 2.0 * LEASH_SLACK
    }

    /// The hero it turns to when its foe falls: one still in its fight (in reach, in its leash,
    /// not hidden from it), the one it has most against first, then the nearest.
    pub(crate) fn next_in_fight(&self, id: Id) -> Option<Id> {
        let m = self.mobs.get(&id)?;
        let leash = self.leash_of(m.def);
        self.players
            .iter()
            .filter(|(h, p)| {
                p.health > 0
                    && !self.unseen(**h)
                    && dist2d(p.pos, m.pos) <= FIGHT_RANGE
                    && (p.pos[2] - m.pos[2]).abs() < 10.0
                    && (m.raider || !self.leashed(m.home, m.pos, p.pos, leash))
            })
            .map(|(h, p)| (*h, self.threat_on(m, *h), dist2d(p.pos, m.pos)))
            .max_by(|a, b| a.1.total_cmp(&b.1).then(b.2.total_cmp(&a.2)))
            .map(|(h, ..)| h)
    }

    /// A hero is gone from the fight (fallen, or left): each creature on them turns on the next
    /// hero in its fight; with none, it goes home (a raider, on to its next foe).
    pub(crate) fn foe_gone(&mut self, hero: Id) {
        let on: Vec<Id> = self.mobs.iter().filter(|(_, m)| m.state.target() == Some(hero)).map(|(m, _)| *m).collect();
        for mid in on {
            if let Some(h) = self.next_in_fight(mid).filter(|h| *h != hero) {
                let now = self.clock;
                let m = self.mobs.get_mut(&mid).unwrap();
                m.state = MobState::Chase { foe: Foe::Hero(h) };
                m.pattern = None;
                m.lost_at = None;
                m.ready_at = m.ready_at.max(now + 0.6);
                continue;
            }
            let raider = self.mobs[&mid].raider;
            self.mobs.get_mut(&mid).unwrap().state = MobState::Return;
            if raider {
                self.raider_next(mid);
            }
        }
    }

    /// It lets its foe go and heads home.
    fn give_up(&mut self, id: Id) {
        self.show_guard(id, false);
        let Some(m) = self.mobs.get_mut(&id) else { return };
        m.state = MobState::Return;
        m.pattern = None;
        m.special = false;
        m.lost_at = None;
        let (home, speed) = (m.home, self.content.mobs[m.def].run_speed);
        self.halt(id);
        self.go(id, home, speed);
    }

    /// A creature heading home from a hero who drew it past its leash: their blows and shots
    /// find nothing (it evades, as 1.12's do) until it's home or they come back within it.
    pub(crate) fn evading(&self, mid: Id, by: Id) -> bool {
        let (Some(m), Some(p)) = (self.mobs.get(&mid), self.players.get(&by)) else { return false };
        !m.raider && matches!(m.state, MobState::Return) && dist2d(p.pos, m.home) > self.leash_of(m.def) + LEASH_SLACK
    }

    fn think(&mut self, id: Id, dt: f32) {
        let now = self.clock;
        let Some(m) = self.mobs.get(&id) else { return };
        if m.hidden {
            return;
        }
        let d = self.content.mobs[m.def].clone();
        let a = self.attack_now(id);
        let state = m.state.clone();
        let (pos, home, raider) = (m.pos, m.home, m.raider);
        match state {
            MobState::Dead { .. } => {}
            MobState::Idle { wander_at } => {
                if raider {
                    self.raider_next(id);
                    return;
                }
                // Below, at rest on another floor than its home's (under the deck it lives on,
                // where no hero sees it and it sees none): it goes home, by a way there or put back.
                if self.is_below() && self.mobs[&id].path.is_empty() && self.off_home_floor(pos, home) {
                    self.mobs.get_mut(&id).unwrap().state = MobState::Return;
                    self.go(id, home, d.run_speed);
                    return;
                }
                // Notice a hero in sight.
                if d.aggro > 0.0 {
                    let seen = self
                        .players
                        .iter()
                        .filter(|(h, p)| p.health > 0 && dist2d(p.pos, pos) <= d.aggro && !self.unseen_by(id, **h))
                        .filter(|(_, p)| clear_line(self.land.as_ref(), pos, p.pos))
                        .min_by(|a, b| dist2d(a.1.pos, pos).total_cmp(&dist2d(b.1.pos, pos)))
                        .map(|(id, _)| *id);
                    if let Some(h) = seen {
                        self.engage(id, Foe::Hero(h));
                        return;
                    }
                }
                if now >= wander_at {
                    let ang = self.rng.gen::<f32>() * std::f32::consts::TAU;
                    let r = self.rng.gen::<f32>() * 4.0;
                    let to = self.ground_near(home[0] + ang.cos() * r, home[1] + ang.sin() * r, home[2]);
                    if !self.land.blocked_near(to[0], to[1], to[2]) {
                        self.go(id, to, d.walk_speed);
                    }
                    let next = now + 4.0 + self.rng.gen::<f32>() * 8.0;
                    self.mobs.get_mut(&id).unwrap().state = MobState::Idle { wander_at: next };
                }
            }
            MobState::Return => {
                // (Home on its own floor: not stopped short under the deck it lives on.)
                if self.at_home(pos, home) {
                    self.halt(id);
                    // Its fight is over: back to its own health.
                    let own = (self.grows_with_fight(id) && self.mobs[&id].fighters > 1).then(|| self.base_health(self.mobs[&id].def));
                    let m = self.mobs.get_mut(&id).unwrap();
                    if let Some(own) = own {
                        m.max_health = own;
                    }
                    m.fighters = 1;
                    m.health = m.max_health;
                    // Whole again: its second wind and its posture with it, its voice too.
                    m.enraged = false;
                    m.phase = 0;
                    m.voiced = false;
                    m.posture = 0;
                    let (h, mx) = (m.health, m.max_health);
                    m.state = MobState::Idle { wander_at: now + 3.0 };
                    self.events.push(Event::Health { id, health: h, max: mx });
                } else if self.mobs[&id].path.is_empty() {
                    self.go(id, home, d.run_speed);
                }
            }
            MobState::Chase { foe } => {
                // Whom it would rather fight, now and then.
                let foe = if self.retarget(id) {
                    match self.mobs[&id].state {
                        MobState::Chase { foe } => foe,
                        _ => foe,
                    }
                } else {
                    foe
                };
                // Stabbed in the back with others on it: it wheels round on that one now.
                let foe = match self.mobs.get_mut(&id).and_then(|m| m.turn_on.take()) {
                    Some(h) if self.players.get(&h).is_some_and(|p| p.health > 0) => {
                        let f = Foe::Hero(h);
                        let m = self.mobs.get_mut(&id).unwrap();
                        m.state = MobState::Chase { foe: f };
                        m.ready_at = m.ready_at.min(now + 0.2);
                        let at = self.players[&h].pos;
                        let face = facing(pos, at);
                        self.mobs.get_mut(&id).unwrap().facing = face;
                        self.events.push(Event::MobStop { id, pos, facing: face });
                        f
                    }
                    _ => foe,
                };
                let Some(to) = self.foe_pos(foe) else {
                    self.show_guard(id, false);
                    // Its foe fell (or left): the next hero still in its fight, if there is one.
                    if let Some(h) = self.next_in_fight(id) {
                        let m = self.mobs.get_mut(&id).unwrap();
                        m.state = MobState::Chase { foe: Foe::Hero(h) };
                        m.lost_at = None;
                        return;
                    }
                    if raider {
                        self.raider_next(id);
                    } else {
                        self.mobs.get_mut(&id).unwrap().state = MobState::Return;
                        self.halt(id);
                    }
                    return;
                };
                let lost = self.mobs[&id].lost_at.is_some_and(|t| now - t > LOST_SECS);
                // Below, with no way to its foe nor home (fallen under its floor): put back home,
                // still in the fight, not given up and left where nobody can reach it.
                if lost && !raider && self.is_below() && self.cut_off(id) {
                    self.set_back(id, home, "found no way to its foe nor home");
                    return;
                }
                if !raider && (lost || self.leashed(home, pos, to, self.leash_of(self.mobs[&id].def))) {
                    self.give_up(id);
                    return;
                }
                let size = match foe {
                    Foe::Hero(_) => 0.4,
                    Foe::Obj(o) => self.objs.get(&o).and_then(|o| match o.kind {
                        ObjKind::Station { def, .. } => Some(self.content.stations[def].footprint),
                        _ => None,
                    }).unwrap_or(1.0),
                };
                let dist = dist2d(pos, to);
                let reach = if a.ranged { a.range } else { a.range + size - 0.3 };
                let sees = !a.ranged || clear_line(self.land.as_ref(), pos, to);
                let ready = now >= self.mobs[&id].ready_at;
                // A caster's patterns: a blast up close, a volley from afar.
                if a.ranged && !d.patterns.is_empty() && ready && sees {
                    if let Some(k) = self.pick_pattern(id, dist, size) {
                        self.mobs.get_mut(&id).unwrap().pattern = Some((k, 0));
                        self.windup(id, foe, to);
                        return;
                    }
                }
                // A caster crowded between casts backs off, facing its foe.
                if a.ranged && !ready && d.keep_away > 0.0 && dist < d.keep_away * CROWDED {
                    if let Foe::Hero(h) = foe {
                        if self.mobs[&id].path.is_empty() {
                            let away = facing(to, pos);
                            let goal = self.ground_near(pos[0] + away.cos() * BACK_OFF, pos[1] + away.sin() * BACK_OFF,
                                pos[2],
                            );
                            self.sidle(id, goal, d.walk_speed.max(2.5), h);
                        }
                        return;
                    }
                }
                if !a.ranged && !d.patterns.is_empty() {
                    let close = dist <= CIRCLE_RADIUS + 1.5;
                    self.show_guard(id, close);
                    let mut busy = false;
                    let mut ready = ready;
                    if let Foe::Hero(h) = foe {
                        // An opening near it: it comes at once, past its turn.
                        let press = self.presses(id, h, dist);
                        ready = ready || press;
                        // Two at a time: the rest circle and wait their turn (and those farther
                        // off don't leap in meanwhile; they come up to wait).
                        busy = !press && self.attackers_on(h, id) >= MAX_ATTACKERS;
                        if busy && dist <= CIRCLE_RADIUS + 3.0 {
                            self.circle_foe(id, h, to, CIRCLE_RADIUS + 1.0);
                            return;
                        }
                        // Between attacks, up close: it keeps its distance, circling, until ready.
                        if !ready && close {
                            self.circle_foe(id, h, to, CIRCLE_RADIUS);
                            return;
                        }
                    }
                    // A souls-like foe: a string of blows that suits the distance (a lunge from
                    // farther off, a combo up close).
                    if ready {
                        if let Some(k) = self.pick_pattern(id, dist, size) {
                            // A fighter's own missile wants a clear line, like a caster's; it may
                            // loose one while two others are at the hero (it isn't one of the two).
                            let shot = d.patterns[k].steps.first().and_then(|st| st.ranged) == Some(true);
                            if (shot || !busy) && (!shot || clear_line(self.land.as_ref(), pos, to)) {
                                self.mobs.get_mut(&id).unwrap().pattern = Some((k, 0));
                                self.show_guard(id, false);
                                self.windup(id, foe, to);
                                return;
                            }
                        }
                    }
                } else if dist <= reach && sees && (ready || matches!(foe, Foe::Hero(h) if self.presses(id, h, dist))) {
                    self.windup(id, foe, to);
                    return;
                }
                // Close in (a caster stops at its distance).
                let m = self.mobs.get_mut(&id).unwrap();
                m.think -= dt;
                if m.think > 0.0 && !m.path.is_empty() {
                    return;
                }
                m.think = THINK;
                // A caster that can't see its foe from where it stands comes round to where it can.
                let stop = if a.ranged && sees {
                    (d.keep_away).min(a.range * 0.8)
                } else if a.ranged {
                    3.0
                } else {
                    (reach - 0.4).max(0.5)
                };
                // Standing in another's body at its foe: it steps round to a place of its own.
                if dist <= stop && !a.ranged && self.mobs[&id].path.is_empty() && self.crowded(id, pos, SPACING * 0.7) {
                    if let Foe::Hero(h) = foe {
                        let goal = self.free_spot(id, to, pos, dist.max(1.5));
                        if dist2d(goal, pos) > 0.5 {
                            self.sidle(id, goal, d.walk_speed.max(2.5), h);
                            return;
                        }
                    }
                }
                if dist <= stop {
                    let face = facing(pos, to);
                    let m = self.mobs.get_mut(&id).unwrap();
                    m.facing = face;
                    m.lost_at = None;
                    self.halt(id);
                    if dist <= reach {
                        // In reach but waiting on the cooldown: face them.
                        self.events.push(Event::MobStop { id, pos, facing: face });
                    }
                } else {
                    let goal = self.free_spot(id, to, pos, stop * 0.8);
                    self.go(id, goal, d.run_speed);
                }
            }
            MobState::Windup { foe, t, mark, shown } => {
                let t2 = t + dt;
                // Turn towards the foe, but not in the last moment: a dodge then is a dodge.
                if t2 < a.windup - a.lock {
                    if let Some(to) = self.foe_pos(foe) {
                        let want = facing(pos, to);
                        let m = self.mobs.get_mut(&id).unwrap();
                        let diff = (want - m.facing + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                        let max = a.turn_rate.to_radians() * dt;
                        m.facing = (m.facing + diff.clamp(-max, max)).rem_euclid(std::f32::consts::TAU);
                    }
                }
                // A lunge: the last moment of the wind-up carries it forward into the blow.
                let lunge = self.pattern_step(id).map_or(0.0, |s| s.lunge);
                // No farther than to land in front of the foe.
                let lunge = match self.foe_pos(foe) {
                    Some(at) => lunge.min((dist2d(self.mobs[&id].pos, at) - LUNGE_SHORT).max(0.0)),
                    None => lunge,
                };
                if lunge > 0.0 && !self.mobs[&id].lunged && t2 >= a.windup - LUNGE_SECS {
                    let m = self.mobs.get_mut(&id).unwrap();
                    m.lunged = true;
                    let (p0, f) = (m.pos, m.facing);
                    let to = self.ground_near(p0[0] + f.cos() * lunge, p0[1] + f.sin() * lunge, p0[2]);
                    if self.land.walk_clear(p0, to) {
                        self.go(id, to, lunge / LUNGE_SECS);
                    }
                }
                let mut shown = shown;
                if !a.ranged && !shown && t2 >= a.windup - ANIM_LEAD {
                    shown = true;
                    let (pos, f) = (self.mobs[&id].pos, self.mobs[&id].facing);
                    self.events.push(Event::MobStop { id, pos, facing: f });
                    self.events.push(Event::Swing { attacker: id, victim: None, damage: 0, outcome: Outcome::Miss, anim: true, style: Blow::Light });
                }
                if t2 >= a.windup {
                    if let Some(mk) = mark {
                        self.unmark(mk);
                    }
                    self.release(id, foe);
                    self.halt(id);
                    let steps = self.mobs[&id].pattern.and_then(|(k, _)| self.content.mobs[self.mobs[&id].def].patterns.get(k)).map_or(0, |p| p.steps.len());
                    let m = self.mobs.get_mut(&id).unwrap();
                    m.state = MobState::Recover { foe, until: now + a.recovery };
                    m.special = false;
                    m.lunged = false;
                    match m.pattern {
                        // The next blow of the string comes after this one's recovery.
                        Some((k, s)) if s + 1 < steps => m.pattern = Some((k, s + 1)),
                        _ => {
                            m.pattern = None;
                            let cd = self.string_cooldown(a.cooldown);
                            self.mobs.get_mut(&id).unwrap().ready_at = now + cd;
                        }
                    }
                } else {
                    self.mobs.get_mut(&id).unwrap().state = MobState::Windup { foe, t: t2, mark, shown };
                }
            }
            MobState::Recover { foe, until } => {
                if now >= until {
                    // Drawn off a station by a hero who struck it: no more of the string there.
                    if let (Foe::Obj(_), Some(h)) = (foe, self.mobs[&id].drawn_off) {
                        let m = self.mobs.get_mut(&id).unwrap();
                        m.drawn_off = None;
                        if self.players.get(&h).is_some_and(|p| p.health > 0) {
                            let m = self.mobs.get_mut(&id).unwrap();
                            m.pattern = None;
                            m.state = MobState::Chase { foe: Foe::Hero(h) };
                            m.think = 0.0;
                            return;
                        }
                    }
                    match (self.mobs[&id].pattern, self.foe_pos(foe)) {
                        // Mid-string: the next blow, turned on the foe wherever they've got to.
                        (Some(_), Some(to)) => self.windup(id, foe, to),
                        _ => {
                            let m = self.mobs.get_mut(&id).unwrap();
                            let strung = m.pattern.take().is_some();
                            m.state = MobState::Chase { foe };
                            // A string done: now and then it gives ground.
                            if let (false, Foe::Hero(h)) = (strung, foe) {
                                if !self.content.mobs[self.mobs[&id].def].patterns.is_empty() {
                                    self.maybe_give_ground(id, h);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// The blow a creature is set on: its special one while it winds that up, else its usual.
    pub(crate) fn attack_now(&self, id: Id) -> crate::content::AttackDef {
        let Some(m) = self.mobs.get(&id) else { return self.content.mobs[0].attack.clone() };
        let d = &self.content.mobs[m.def];
        let mut a = match (self.pattern_step(id), &d.special, m.special) {
            (Some(step), _, _) => step.over(&d.attack),
            (None, Some(s), true) => s.over(&d.attack),
            _ => d.attack.clone(),
        };
        // Quicker (and harder) in its later phases.
        if let Some(e) = self.phase_now(id) {
            a.windup *= e.speed;
            a.recovery *= e.speed;
            a.cooldown *= e.speed;
            a.damage = (a.damage as f32 * e.might).round() as u32;
        }
        // A wager's level, its pack's rally, and the dungeon's tier.
        let frenzy = if self.wager_has("frenzied") && (m.health as f32) < m.max_health as f32 * super::wager::FRENZY_BELOW { super::wager::FRENZY_MIGHT } else { 1.0 };
        let w = self.wager_scale(m.def).1 * (1.0 + 0.1 * m.rally as f32) * self.tier_scale().1 * frenzy * self.quickening(id);
        if w != 1.0 {
            a.damage = (a.damage as f32 * w).round() as u32;
        }
        a
    }

    /// The step of its pattern a creature is on, if it's in one.
    pub(crate) fn pattern_step(&self, id: Id) -> Option<&crate::content::SpecialDef> {
        let m = self.mobs.get(&id)?;
        let (k, s) = m.pattern?;
        self.content.mobs[m.def].patterns.get(k)?.steps.get(s)
    }

    /// The clip a creature swings with by default: by what it holds.
    fn attack_clip(&self, def: usize) -> u16 {
        let d = &self.content.mobs[def];
        match d.grip.as_str() {
            "2h" | "staff" => ATTACK_2H,
            "polearm" => ATTACK_2HL,
            "dagger" => ATTACK_PIERCE,
            "1h" => ATTACK_1H,
            _ if d.weapon == 0 => ATTACK_UNARMED,
            _ => ATTACK_1H,
        }
    }

    /// One of a creature's patterns that suits a foe `dist` off (`size` its body), weighted.
    pub(crate) fn pick_pattern(&mut self, id: Id, dist: f32, size: f32) -> Option<usize> {
        let m = self.mobs.get(&id)?;
        let d = &self.content.mobs[m.def];
        // A raised guard in front of it: it reaches for the blow no guard stops.
        let guarded = m.state.target().is_some_and(|h| self.hero_guarding(h));
        let breaks = self.content.ai.guard_break.max(0.0);
        let fits: Vec<(usize, f32)> = d
            .patterns
            .iter()
            .enumerate()
            .filter(|(_, p)| p.weight > 0.0 && m.health as f32 <= p.below * m.max_health as f32)
            .filter_map(|(k, p)| {
                let first = p.steps.first()?;
                let reach = first.range.unwrap_or(d.attack.range) + size - 0.3 + first.lunge;
                let max = if p.max_range > 0.0 { p.max_range } else { reach };
                let w = if guarded && p.steps.iter().any(|s| s.blockable == Some(false) || s.heavy == Some(true)) { p.weight * breaks } else { p.weight };
                (dist >= p.min_range && dist <= max).then_some((k, w.max(0.0)))
            })
            .collect();
        let total: f32 = fits.iter().map(|f| f.1).sum();
        if fits.is_empty() || total <= 0.0 {
            return None;
        }
        let mut roll = self.rng.gen::<f32>() * total;
        for (k, w) in &fits {
            if roll < *w {
                return Some(*k);
            }
            roll -= w;
        }
        fits.last().map(|f| f.0)
    }

    pub(crate) fn windup(&mut self, id: Id, foe: Foe, to: Vec3) {
        self.halt(id);
        if let Some(m) = self.mobs.get_mut(&id) {
            m.lost_at = None;
        }
        // A throw it never let go of (knocked out of it, gone home): its ring goes.
        if let Some((_, mk)) = self.mobs.get_mut(&id).and_then(|m| m.lob.take()) {
            self.unmark(mk);
        }
        self.show_guard(id, false);
        let def = self.mobs[&id].def;
        // Now and then its special blow, telegraphed with a flash; in a pattern, the step's own.
        let in_pattern = self.mobs[&id].pattern.is_some();
        let special = if in_pattern {
            self.pattern_step(id).cloned()
        } else {
            self.content.mobs[def].special.clone().filter(|s| self.rng.gen::<f32>() < s.chance)
        };
        self.mobs.get_mut(&id).unwrap().special = special.is_some() && !in_pattern;
        self.mobs.get_mut(&id).unwrap().lunged = false;
        let a = self.attack_now(id);
        let m = &self.mobs[&id];
        let a = &a;
        let pos = m.pos;
        let face = facing(pos, to);
        self.mobs.get_mut(&id).unwrap().facing = face;
        self.events.push(Event::MobStop { id, pos, facing: face });
        // The telegraph: where the blow will land, on the ground, until it does (and on the
        // clients that draw it, the blow itself: its true reach and arc, and when it comes).
        let hero = match foe {
            Foe::Hero(h) => Some(h),
            _ => None,
        };
        let tele = Telegraph { mob: id, reach: a.range + 0.4, arc: a.arc, secs: a.windup, blockable: a.blockable, foe: hero, width: 0.0, at: None, hero: false };
        let mark = if a.ranged {
            self.events.push(Event::Cast { caster: id, spell: a.spell, ms: (a.windup * 1000.0) as u32 });
            // A lobbed throw that splashes: where it will come down is chosen now and marked
            // through the whole wind-up and flight, so the ring is there to read and step out of,
            // however near the thrower stands.
            if a.radius >= LOB_RADIUS {
                let at = self.ground_near(to[0], to[1], to[2]);
                let fly = lob_flight(dist2d(pos, at), a.speed);
                let mk = self.mark_ground(at, a.radius, a.windup + fly + 0.3, ground_look(a.spell, a.look));
                self.mobs.get_mut(&id).unwrap().lob = Some((at, mk));
            }
            None
        } else {
            // A blow all round marks a ring around it; otherwise, the ground in front.
            if a.arc >= 300.0 {
                Some(self.mark_as(pos, a.range, a.windup + 0.2, Some(tele)))
            } else {
                // Where a lunge carries it, as far as the foe.
                let lunge = special.as_ref().map_or(0.0, |s| s.lunge).min((dist2d(pos, to) - LUNGE_SHORT).max(0.0));
                let ahead = a.range * 0.55 + lunge;
                let center = self.ground_near(pos[0] + face.cos() * ahead, pos[1] + face.sin() * ahead,
                    pos[2],
                );
                Some(self.mark_as(center, a.range * 0.6, a.windup + 0.2, Some(tele)))
            }
        };
        if a.emote != 0 {
            self.events.push(Event::Emote { id, emote: a.emote });
        }
        // A blow no guard stops: its foe is warned (dodge it), and with a blow all round, everyone
        // it would reach.
        if !a.blockable {
            let ring = !a.ranged && a.arc >= 300.0;
            let near: Vec<Id> = if ring {
                self.players.iter().filter(|(_, p)| p.health > 0 && dist2d(p.pos, pos) <= a.range + 2.0).map(|(h, _)| *h).collect()
            } else {
                Vec::new()
            };
            if let Foe::Hero(h) = foe {
                if !near.contains(&h) {
                    self.events.push(Event::Danger { player: h, secs: a.windup, from: Some(id) });
                }
            }
            for h in near {
                self.events.push(Event::Danger { player: h, secs: a.windup, from: Some(id) });
            }
        }
        if let Some(kit) = special.as_ref().map(|s| s.kit).filter(|k| *k != 0) {
            self.events.push(Event::Visual { id, kit, by: None });
        }
        // The swing itself, for the clients that play clips: wound up over the whole wind-up (the
        // blade lands as the blow does), with one of the model's takes on its attack.
        if !a.ranged {
            let clip = special.as_ref().and_then(|s| s.clip).unwrap_or_else(|| self.attack_clip(def));
            let take = match &special {
                Some(s) => s.take,
                None => self.rng.gen_range(0..2),
            };
            let from = special.as_ref().and_then(|s| s.from.clone()).or_else(|| self.content.mobs[def].clips_from.clone());
            self.events.push(Event::MobWindup { id, clip, take, windup: a.windup, recovery: a.recovery, blockable: a.blockable, from });
        }
        self.mobs.get_mut(&id).unwrap().state = MobState::Windup { foe, t: 0.0, mark, shown: false };
    }

    /// The blow lands (or the missile flies): every hero in the arc and in reach of the creature's
    /// committed facing, or a station in the way.
    fn release(&mut self, id: Id, foe: Foe) {
        let now = self.clock;
        let a = self.attack_now(id);
        let m = &self.mobs[&id];
        let (pos, f) = (m.pos, m.facing);
        if a.ranged {
            self.events.push(Event::CastEnd { caster: id });
            let lob = self.mobs.get_mut(&id).and_then(|m| m.lob.take());
            let (at, fly, mark) = match lob {
                // Thrown where it was marked at the wind-up's start, its ring still there.
                Some((at, mark)) => (at, lob_flight(dist2d(pos, at), a.speed), mark),
                None => {
                    let Some(to) = self.foe_pos(foe) else { return };
                    let at = self.ground_near(to[0], to[1], to[2]);
                    let fly = dist2d(pos, at) / a.speed.max(1.0);
                    // A splash's ground warned by its spell's own look; a bolt's small ring as drawn.
                    let ground = if a.radius >= 2.0 { ground_look(a.spell, a.look) } else { 0 };
                    (at, fly, self.mark_ground(at, a.radius, fly + 0.3, ground))
                }
            };
            self.events.push(Event::Missile { caster: id, spell: a.spell, target: Target::Point(at) });
            self.missiles.push(combat::Missile {
                look: a.look,
                owner: id,
                from: pos,
                at,
                radius: a.radius,
                damage: a.damage,
                blockable: a.blockable,
                heavy: a.heavy,
                land_at: now + fly,
                mark,
            });
            return;
        }
        // A heavy special blow shakes the ground where it lands.
        if self.mobs[&id].special && a.heavy {
            self.events.push(Event::Visual { id, kit: combat::KIT_DUST, by: None });
        }
        // Its look on the ground as it lands (a stomp's shockwave, a nova's burst), at its feet
        // for a blow all round, else where its reach ends.
        if a.look != 0 {
            let at = if a.arc >= 300.0 { pos } else { [pos[0] + f.cos() * a.range * 0.6, pos[1] + f.sin() * a.range * 0.6, pos[2]] };
            self.events.push(Event::Missile { caster: id, spell: a.look, target: Target::Point(at) });
        }
        let heroes: Vec<(Id, Vec3)> = self.players.iter().filter(|(_, p)| p.health > 0).map(|(id, p)| (*id, p.pos)).collect();
        let mut any = false;
        for (h, hp) in heroes {
            let dist = dist2d(pos, hp);
            if dist <= a.range + 0.4 && (angle_between(f, facing(pos, hp)) <= a.arc / 2.0 || dist < 0.8) {
                any = true;
                // Settled once the hero's round trip is up: out of it by then (as they saw it), missed.
                let area = Some(combat::Area::Arc { pos, facing: f, range: a.range, arc: a.arc });
                self.strike_fair(id, h, a.damage, pos, a.blockable, a.heavy, a.blockable, Blow::Light, area, combat::Follow::None);
            }
        }
        if let Foe::Obj(o) = foe {
            if let Some(op) = self.objs.get(&o).map(|o| o.pos) {
                let size = match self.objs[&o].kind {
                    ObjKind::Station { def, .. } => self.content.stations[def].footprint,
                    _ => 1.0,
                };
                if dist2d(pos, op) <= a.range + size {
                    any = true;
                    self.events.push(Event::Swing { attacker: id, victim: None, damage: 0, outcome: Outcome::Miss, anim: false, style: Blow::Light });
                    self.hurt_station(o, a.damage);
                }
            }
        }
        if !any {
            if let Foe::Hero(h) = foe {
                // Stepped out of it: show the whiff on them.
                self.events.push(Event::Swing { attacker: id, victim: Some(h), damage: 0, outcome: Outcome::Miss, anim: false, style: Blow::Light });
            }
        }
    }
}
