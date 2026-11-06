//! Playing the rules' events out as packets.

use htl_survival::{Blow, Event, Id, Outcome, Target};

use super::draw::{creature_guid, CREATURE_ENTRY};
use super::*;
use crate::packets::Aim;

/// One-off gestures (`Emotes.dbc`): a one-handed swing (armed creatures' blows play as this,
/// before the result lands; a hero's is their weapon's own), an unarmed one (a boar's tusks), the
/// reel of a broken guard, and the shield's block.
const EMOTE_ATTACK_UNARMED: u32 = 35;
const EMOTE_ATTACK_1H: u32 = 36;
const EMOTE_STUN: u32 = htl_survival::mobs::STUN_EMOTE;
const EMOTE_SHIELD_BLOCK: u32 = 43;
/// The guard's held pose: the shield up (ShieldBlock), or a two-handed weapon at the ready.
pub(crate) const POSE_SHIELD: u32 = 43;
const POSE_TWO_HANDED: u32 = 375;
/// A ward held up: the spell-precast stance (the arm raised, channelling), not a weapon's guard.
const POSE_WARD: u32 = 193;
const ARROW_DISPLAY: u32 = packets::ARROW_DISPLAY;
/// The hunter's shots whose own look has no missile model: flown from a spot, they carry the
/// arrow (as Basin Battle's server sends them), or the client flies nothing to see. (10346: Fire
/// Shot's, Kill Shot's arrow, carried the same way.)
const ARROW_SPELLS: [u32; 4] = [2643, 3044, 10346, 19434];
/// Where a missile coming home is aimed: this far over the feet of whoever threw it (chest high
/// on a dwarf, the hands on a human), so it flies to the body, not the ground.
const HOME_HEIGHT: f32 = 1.2;
/// A stand-in's cast goes out this many ticks after its body (the client places a body only after
/// the frame that made it: cast in that frame, the missile would leave from the world's middle).
const STAND_IN_CAST_TICKS: u32 = 1;
/// A stand-in stays at least this many ticks past its cast, however soon the rules are done with
/// it: the client holds a missile on its caster until the cast's release (up to a throw's
/// animation), and a caster gone before then takes its missile with it. And at most this many,
/// whatever the rules say.
const STAND_IN_LINGER_TICKS: u32 = 30;
const STAND_IN_MOST_TICKS: u32 = 600;

/// An unseen creature standing at a spot to fly a missile from there (see
/// [`SurvivalGame::stand_in_create`]): its guid, its casts still to go out (its missile, and any
/// look cast from it in the same tick, as a Bank Shot's flash), how many ticks it's stood, and
/// whether the rules are done with it (their `ObjGone`).
pub(crate) struct StandIn {
    guid: u64,
    go: Vec<Packet>,
    ticks: u32,
    gone: bool,
}
/// The swings' trails (`AARRGGBB`): a faint pale streak, a heavy blow's ember.
const LIGHT_TRAIL: &str = "50F2EEE4";
const HEAVY_TRAIL: &str = "78FF9A3C";
/// A creature's swing, its last stroke into the blow (`ac9`): amber for a blow a guard stops, red
/// for one it doesn't (as its blow's telegraph). The client starts it this long before the impact
/// (milliseconds) and lets it fade just after.
const FOE_TRAIL: &str = "70FFC24A";
const FOE_TRAIL_UNBLOCKABLE: &str = "A0FF2A1A";
pub(crate) const FOE_TRAIL_MS: u32 = 320;
/// The client level (`.es hello … acN`) that draws creature trails that way.
pub(crate) const FOE_TRAILS_LEVEL: u8 = 9;
/// `SpellMissInfo`: how a shot came out when it didn't land.
const MISS_MISS: u8 = 1;
const MISS_DODGE: u8 = 3;
const MISS_PARRY: u8 = 4;
const MISS_BLOCK: u8 = 5;
/// A tick of damage over time shows as this spell's (Immolate's burn).
const DOT_SPELL: u32 = 348;
/// From this client level a heal on a friend floats over their head (patch 0056), so the healer is
/// sent the heal log instead of the add-on's `HO` line.
pub(crate) const HEAL_FLOAT_LEVEL: u8 = 28;

impl SurvivalGame {
    pub(crate) fn play(&mut self, events: Vec<Event>) {
        self.step_stand_ins();
        for e in events {
            self.event(e);
        }
    }

    /// Each tick: the stand-ins made last tick cast their missiles, and those done with go.
    fn step_stand_ins(&mut self) {
        let mut cast = Vec::new();
        let mut done = Vec::new();
        for (id, s) in self.stand_ins.iter_mut() {
            s.ticks += 1;
            if s.ticks >= STAND_IN_CAST_TICKS {
                cast.append(&mut s.go);
            }
            if (s.gone && s.ticks >= STAND_IN_CAST_TICKS + STAND_IN_LINGER_TICKS) || s.ticks >= STAND_IN_MOST_TICKS {
                done.push((*id, s.guid));
            }
        }
        for go in cast {
            self.broadcast(&go);
        }
        for (id, guid) in done {
            self.stand_ins.remove(&id);
            self.unbind(id);
            self.broadcast(&packets::destroy_object(guid));
        }
    }

    fn is_player(&self, id: Id) -> bool {
        self.sim.players.contains_key(&id)
    }

    fn event(&mut self, e: Event) {
        match e {
            Event::MobSpawned { id } => {
                let Some(def) = self.sim.mobs.get(&id).map(|m| m.def) else { return };
                self.bind(id, creature_guid(CREATURE_ENTRY + def as u32, id));
                if let Some(p) = self.mob_create(id) {
                    self.broadcast(&p);
                }
                // Its auras went with it: the next change clears what's no longer on it.
                let n = self.sim.auras_of(id).len().min(16);
                if n > 0 {
                    self.auras_shown.insert(id, n);
                }
            }
            Event::MobGone { id } => {
                if let Some(g) = self.unbind(id) {
                    self.broadcast(&packets::destroy_object(g));
                }
            }
            Event::MobMove { id, path, speed, face } => {
                let Some(guid) = self.guid_of(id) else { return };
                // Circling a foe: a walk, turned to them the while.
                if let Some(t) = face.and_then(|f| self.guid_of(f)).filter(|_| path.len() >= 2 && speed > 0.0) {
                    let len = htl_sim::geom::path_len(&path);
                    let ms = ((len / speed) * 1000.0).round().max(1.0) as u32;
                    self.spline_id = self.spline_id.wrapping_add(1);
                    self.broadcast(&packets::monster_move_facing(guid, self.spline_id, &path, ms, t));
                    return;
                }
                if let Some(p) = self.move_packet(guid, &path, speed) {
                    self.broadcast(&p);
                }
            }
            Event::MobGuard { id, up } => {
                // The shield raised: the guard's stance (the hero's own guard pose).
                let Some(g) = self.guid_of(id) else { return };
                let mut f = Fields::new();
                f.set(idx::UNIT_NPC_EMOTESTATE, if up { POSE_SHIELD } else { 0 });
                self.broadcast(&packets::update_object(&[], &[(g, &f)]));
            }
            Event::MobStop { id, pos, facing } => {
                let Some(guid) = self.guid_of(id) else { return };
                self.spline_id = self.spline_id.wrapping_add(1);
                self.broadcast(&packets::monster_face(guid, self.spline_id, pos, facing));
            }
            Event::Swing { attacker, victim, damage, outcome, anim, style } => {
                let Some(a) = self.guid_of(attacker) else { return };
                if anim {
                    // The swing itself: its look (a heavy attack's own), with no number.
                    let emote = match (self.sim.players.get(&attacker), self.sim.mobs.get(&attacker)) {
                        (Some(p), _) if style == Blow::Heavy => self.sim.weapon_of(p).heavy_look(),
                        (Some(p), _) => self.sim.weapon_of(p).emote,
                        (_, Some(m)) if self.sim.content.mobs[m.def].weapon == 0 => EMOTE_ATTACK_UNARMED,
                        _ => EMOTE_ATTACK_1H,
                    };
                    let p = packets::emote(a, emote);
                    // A hero's swing, to a client that plays clips itself: the clip, timed to land
                    // when the blow does, with a trail over it. Everyone else gets the emote.
                    // The blow as committed: its clip and take, its impact (mid-way through its active
                    // moment) and the recovery after, how far it lunges, and its kind.
                    let clip = self.sim.players.get(&attacker).and_then(|p| match &p.action {
                        htl_survival::Action::Swing { strike, .. } => {
                            // A leap out of a Corey roll lands as heavily as a heavy attack looks.
                            let heavy = strike.heavy() || strike.shove() || strike.kind == htl_survival::StrikeKind::Leap;
                            let trail = if heavy { HEAVY_TRAIL } else { LIGHT_TRAIL };
                            let impact = strike.windup + strike.active * 0.5;
                            let ms = ((strike.windup + strike.active + 0.15) * 1000.0) as u32;
                            let kind = match strike.kind {
                                htl_survival::StrikeKind::Light => "light",
                                htl_survival::StrikeKind::Heavy => "heavy",
                                htl_survival::StrikeKind::Running => "running",
                                htl_survival::StrikeKind::Roll => "roll",
                                htl_survival::StrikeKind::Leap => "leap",
                                htl_survival::StrikeKind::Bash => "bash",
                                htl_survival::StrikeKind::Kick => "kick",
                            };
                            let from = strike.from.as_deref().map(clean).unwrap_or_default();
                            Some((strike.clip, strike.take, impact, strike.total() - impact, strike.lunge, strike.step, trail, ms, kind, from))
                        }
                        _ => None,
                    });
                    let mine = self.conn_of(attacker);
                    // A creature's swing already came to the clip-playing clients with its wind-up.
                    let creature = self.sim.mobs.contains_key(&attacker);
                    let conns: Vec<(ConnId, bool)> = self.in_world().map(|(c, _, w)| (*c, w.anim)).collect();
                    for (c, anim_client) in conns {
                        if creature && anim_client {
                            continue;
                        }
                        match &clip {
                            Some((clip, take, impact, recovery, lunge, step, trail, ms, kind, from)) if anim_client => {
                                let me = u8::from(mine == Some(c));
                                self.ui_line(
                                    c,
                                    &format!("AN\t{a:#X}\t{clip}\t{impact:.3}\t{trail}\t{ms}\t{me}\t{take}\t{recovery:.3}\t{lunge:.2}\t{kind}\t{step}\t{from}"),
                                );
                            }
                            _ => self.send(c, p.clone()),
                        }
                    }
                    return;
                }
                let Some(v) = victim.and_then(|v| self.guid_of(v)) else { return };
                let p = match style {
                    // A burn's or a drain's tick: a spell's number, no swing.
                    Blow::Dot => packets::spell_damage(v, a, DOT_SPELL, damage, 0, false, false),
                    // An arrow lands as a shot, not a swing: no melee look on the archer (whose
                    // client would put the bow away for the axe), a ranged flinch on the struck.
                    Blow::Arrow(spell) => match outcome {
                        Outcome::Hit | Outcome::GuardBroken => packets::spell_damage(v, a, spell, damage, 0, false, false),
                        Outcome::Blocked => packets::spell_log_miss(a, spell, v, MISS_BLOCK),
                        Outcome::Dodged => packets::spell_log_miss(a, spell, v, MISS_DODGE),
                        Outcome::Parried => packets::spell_log_miss(a, spell, v, MISS_PARRY),
                        Outcome::Miss => packets::spell_log_miss(a, spell, v, MISS_MISS),
                    },
                    _ => {
                        // A heavy attack's hit shows as a critical: the bigger number and flinch; a heavy
                        // creature's as a crushing blow (the big spurt, the cry).
                        let crit = style == Blow::Heavy;
                        let crushing = self.sim.mobs.get(&attacker).is_some_and(|m| self.sim.content.mobs[m.def].attack.heavy);
                        match outcome {
                            Outcome::Hit | Outcome::GuardBroken => packets::swing_update_with(a, v, damage, packets::VICTIM_HIT, 0, false, true, crit, crushing),
                            Outcome::Blocked => packets::swing_update(a, v, 0, packets::VICTIM_BLOCK, 1, false, true, false),
                            Outcome::Dodged => packets::swing_update(a, v, 0, packets::VICTIM_DODGE, 0, false, true, false),
                            Outcome::Parried => packets::swing_update(a, v, 0, packets::VICTIM_PARRY, 0, false, true, false),
                            Outcome::Miss => packets::swing_update(a, v, 0, packets::VICTIM_HIT, 0, true, true, false),
                        }
                    }
                };
                self.broadcast(&p);
                // Its weight, on the clients that play it: a hit-stop on both, a shake for those in
                // it.
                {
                    let kind = match style {
                        Blow::Light => "light",
                        Blow::Heavy => "heavy",
                        Blow::Arrow(_) => "arrow",
                        Blow::Dot => "dot",
                    };
                    let how = match outcome {
                        Outcome::Hit | Outcome::GuardBroken => "hit",
                        Outcome::Blocked => "block",
                        Outcome::Parried => "parry",
                        Outcome::Dodged => "dodge",
                        Outcome::Miss => "miss",
                    };
                    let (by, to) = (self.conn_of(attacker), victim.and_then(|v| self.conn_of(v)));
                    let conns: Vec<ConnId> = self.in_world().filter(|(_, _, w)| w.anim).map(|(c, _, _)| *c).collect();
                    for c in conns {
                        let (i, me) = (u8::from(by == Some(c)), u8::from(to == Some(c)));
                        self.ui_line(c, &format!("IM\t{a:#X}\t{v:#X}\t{kind}\t{how}\t{i}\t{me}"));
                    }
                }
                // The striker's own screen confirms what landed (the hit marker).
                if let Some(c) = self.conn_of(attacker).filter(|c| self.has_addon(*c)) {
                    let kind = match style {
                        Blow::Light => "light",
                        Blow::Heavy => "heavy",
                        Blow::Arrow(_) => "arrow",
                        Blow::Dot => "dot",
                    };
                    let how = match outcome {
                        Outcome::Hit | Outcome::GuardBroken => "hit",
                        Outcome::Blocked => "block",
                        Outcome::Parried => "parry",
                        Outcome::Dodged => "dodge",
                        Outcome::Miss => "miss",
                    };
                    self.ui_line(c, &format!("HT\t{damage}\t{kind}\t{how}"));
                }
                // The guard gives, or holds (the clip-playing clients have the reaction already).
                match outcome {
                    Outcome::GuardBroken => self.broadcast_stock(&packets::emote(v, EMOTE_STUN)),
                    Outcome::Blocked | Outcome::Parried => self.broadcast_stock(&packets::emote(v, EMOTE_SHIELD_BLOCK)),
                    _ => {}
                }
            }
            Event::Missile { caster, spell, target } => {
                // The mine's own (a hazard going off): shown from any hero here.
                let caster = if caster == 0 {
                    self.sim.players.keys().next().copied().unwrap_or(0)
                } else {
                    caster
                };
                let Some(c) = self.guid_of(caster) else { return };
                let aim = match target {
                    Target::Unit(t) if t == caster => Aim::Caster,
                    Target::Unit(t) => match self.guid_of(t) {
                        Some(g) => Aim::Unit(g),
                        None => return,
                    },
                    Target::Point(p) => Aim::Point(p),
                };
                let hits: Vec<u64> = match aim {
                    Aim::Unit(g) => vec![g],
                    Aim::Caster => vec![c],
                    Aim::Point(_) => Vec::new(),
                };
                let ranged = self.sim.content.ranged.iter().any(|r| r.spell == spell);
                let ammo = arrow_ammo(&self.sim.content, spell);
                // The shot ends the bow's drawn pose.
                if ranged {
                    self.casting.remove(&caster);
                }
                let go = packets::spell_go(c, spell, aim, &hits, ammo);
                // A stand-in made this tick (a flash where a Bank Shot struck) casts with its
                // missile, once the client has placed its body (`step_stand_ins`).
                if let Some(s) = self.stand_ins.get_mut(&caster).filter(|s| s.ticks < STAND_IN_CAST_TICKS) {
                    s.go.push(go);
                    return;
                }
                self.broadcast(&go);
            }
            Event::MissileFrom { anchor, caster, spell, from, target } => {
                // A missile from a spot (a boomerang turning back, an arrow called home). The
                // client flies a missile only from a unit (a game object's cast has no caster
                // unit, and it draws nothing), so an unseen creature stands there and flies it, a
                // tick after it's made (`step_stand_ins`), until the rules say it's gone.
                let (aim, hits) = match target {
                    // Coming home to whoever threw it: aimed at the spot they stand on, chest
                    // high, naming no one struck. The client plays a spell's impact on everyone
                    // it names struck, and the Axe Throw's impact knocked its own warrior down as
                    // he caught it (the visual check, Nov 2); but a unit aim that names no one
                    // flies nothing at all on the 1.12 client (its launcher flies a missile per
                    // unit named struck, or one at a point), which is why the axe's way back and
                    // the recalled arrows were never seen (Corey's review, run 040). A point aim
                    // flies exactly one, and its arrival is a ground impact, so nothing flinches.
                    Target::Unit(t) if t == caster => {
                        let at = self.sim.players.get(&t).map(|p| p.pos).or_else(|| self.sim.mobs.get(&t).map(|m| m.pos));
                        match at {
                            Some(p) => (Aim::Point([p[0], p[1], p[2] + HOME_HEIGHT]), Vec::new()),
                            None => return,
                        }
                    }
                    Target::Unit(t) => match self.guid_of(t) {
                        Some(g) => (Aim::Unit(g), vec![g]),
                        None => return,
                    },
                    Target::Point(p) => (Aim::Point(p), Vec::new()),
                };
                let (g, made) = self.stand_in_create(anchor, from);
                self.broadcast(&made);
                let go = packets::spell_go(g, spell, aim, &hits, arrow_ammo(&self.sim.content, spell));
                self.stand_ins.insert(anchor, StandIn { guid: g, go: vec![go], ticks: 0, gone: false });
            }
            Event::Cast { caster, spell, ms } => {
                let Some(c) = self.guid_of(caster) else { return };
                let ranged = self.sim.content.ranged.iter().any(|r| r.spell == spell);
                let p = packets::spell_start_ammo(c, spell, ms, Aim::Caster, ranged.then_some(ARROW_DISPLAY));
                // Everyone sees it, the caster too: their own body gathers, eats and casts as
                // others see it (their stock cast bar is hidden; the add-on's is theirs).
                self.broadcast(&p);
                self.casting.insert(caster, spell);
            }
            Event::CastEnd { caster } => {
                let Some(c) = self.guid_of(caster) else { return };
                let Some(spell) = self.casting.remove(&caster) else { return };
                self.broadcast(&packets::spell_failed_other(c, spell));
            }
            Event::Emote { id, emote } => {
                if let Some(g) = self.guid_of(id) {
                    // A reel comes with its reaction for the clip-playing clients.
                    if emote == EMOTE_STUN {
                        self.broadcast_stock(&packets::emote(g, emote));
                    } else {
                        self.broadcast(&packets::emote(g, emote));
                    }
                }
            }
            Event::Health { id, health, max } => {
                if let Some(g) = self.guid_of(id) {
                    let mut f = Fields::new();
                    f.set(idx::UNIT_HEALTH, health).set(idx::UNIT_MAXHEALTH, max);
                    self.broadcast(&packets::update_object(&[], &[(g, &f)]));
                }
            }
            Event::Stamina { player } => {
                let (Some(g), Some(p)) = (self.guid_of(player), self.sim.players.get(&player)) else { return };
                let mut f = Fields::new();
                f.set(idx::UNIT_POWER1 + 3, p.stamina.floor() as u32);
                self.to_player(player, packets::update_object(&[], &[(g, &f)]));
            }
            Event::Ward { player } => {
                let Some(warder) = self.sim.players.get(&player).map(|p| self.sim.guard_of(p).ward) else { return };
                if warder {
                    if let Some(c) = self.conn_of(player) {
                        let line = self.ward_line(player);
                        self.ui_line(c, &line);
                    }
                }
                // The warder's barrier for everyone who sees them (`WB guid own up share broken`):
                // drawn round the body on a client with SetUnitSprite. No ward staff now: none.
                let Some(g) = self.guid_of(player) else { return };
                let Some(p) = self.sim.players.get(&player) else { return };
                let share = (p.ward / self.sim.content.ward.bar.max(1.0)).clamp(0.0, 1.0);
                let (up, broken) = (u8::from(p.ward_up && warder), u8::from(p.ward_broken && warder));
                let mine = self.conn_of(player);
                let conns: Vec<ConnId> = self.in_world().filter(|(_, _, w)| w.addon).map(|(c, _, _)| *c).collect();
                for c in conns {
                    let own = u8::from(Some(c) == mine);
                    self.ui_line(c, &format!("WB\t{g:#X}\t{own}\t{up}\t{share:.2}\t{broken}"));
                }
            }
            Event::Perfect { player, ability } => {
                if let Some(c) = self.conn_of(player) {
                    let key = self.sim.content.abilities[ability].key.clone();
                    self.ui_line(c, &format!("PF\t{key}"));
                }
            }
            Event::CheckStart { player, ability, kind, a, b, c: crit, pattern } => {
                if let Some(c) = self.conn_of(player) {
                    let d = &self.sim.content.abilities[ability];
                    let (key, school) = (d.key.clone(), check_school(d));
                    self.ui_line(c, &format!("CK\t{key}\t{kind}\t{school}\t{a:.3}\t{b:.3}\t{crit:.3}\t{pattern}"));
                }
            }
            Event::OrbRoll { look, from, to, secs, model, scale } => {
                // Everyone with the add-on sees it roll (a client with PlayGroundOrb draws it).
                let line = format!("OB\t{look}\t{:.2}\t{:.2}\t{:.2}\t{:.2}\t{:.2}\t{:.2}\t{secs:.2}\t{}\t{scale:.2}", from[0], from[1], from[2], to[0], to[1], to[2], model.replace('\t', " "));
                let conns: Vec<ConnId> = self.in_world().filter(|(_, _, w)| w.addon).map(|(c, _, _)| *c).collect();
                for c in conns {
                    self.ui_line(c, &line);
                }
            }
            Event::SigilLaid { look, at, radius, beats } => {
                // Salvation for everyone with the add-on: a client with SetGroundRing draws its
                // circle, the rim closing on each beat and the centre's sigil (`SV`).
                let beats = beats.iter().map(|b| format!("{b:.2}")).collect::<Vec<_>>().join(",");
                let line = format!("SV	{look}	{:.2}	{:.2}	{:.2}	{radius:.2}	{beats}", at[0], at[1], at[2]);
                let conns: Vec<ConnId> = self.in_world().filter(|(_, _, w)| w.addon).map(|(c, _, _)| *c).collect();
                for c in conns {
                    self.ui_line(c, &line);
                }
            }
            Event::PriestLook { player, kind, n, on, a, b } => {
                // A Priest look in Corey's art (`PL`): on the hero's own screen, or (a perfect
                // Shield, Kindle, Fade's afterimage) for everyone who sees the unit it's on.
                let g = on.and_then(|u| self.guid_of(u)).unwrap_or(0);
                let mine = self.conn_of(player);
                let line = |own: u8| format!("PL\t{kind}\t{n}\t{g:#X}\t{a:.2}\t{b:.2}\t{own}");
                if matches!(kind, "shield" | "fade") {
                    let conns: Vec<ConnId> = self.in_world().filter(|(_, _, w)| w.addon).map(|(c, _, _)| *c).collect();
                    for c in conns {
                        self.ui_line(c, &line(u8::from(Some(c) == mine)));
                    }
                } else if let Some(c) = mine {
                    self.ui_line(c, &line(1));
                }
            }
            Event::PriestGround { kind, key, at, radius, secs } => {
                // A Priest's mark on the ground for everyone with the add-on (`PG`): a feather to
                // step on, a rift to keep out of.
                let line = format!("PG\t{kind}\t{key}\t{:.2}\t{:.2}\t{:.2}\t{radius:.2}\t{secs:.2}", at[0], at[1], at[2]);
                let conns: Vec<ConnId> = self.in_world().filter(|(_, _, w)| w.addon).map(|(c, _, _)| *c).collect();
                for c in conns {
                    self.ui_line(c, &line);
                }
            }
            Event::OrbGone { look } => {
                let conns: Vec<ConnId> = self.in_world().filter(|(_, _, w)| w.addon).map(|(c, _, _)| *c).collect();
                for c in conns {
                    self.ui_line(c, &format!("OX\t{look}"));
                }
            }
            Event::Stacks { player, kind, n, on } => {
                // A staff's heat, chill or charges, for the caster's reticle mark (the Priest's
                // Weaving and Searing Light on the foe).
                if let Some(c) = self.conn_of(player) {
                    let k = &self.sim.content;
                    let most = match kind {
                        "weave" => k.priest.weave_most,
                        "sear" => 1,
                        _ => k.staffs.most,
                    };
                    let g = on.and_then(|u| self.guid_of(u)).unwrap_or(0);
                    self.ui_line(c, &format!("SK	{kind}	{n}	{most}	{g:#X}"));
                }
            }
            Event::HoldPose { id, clip, secs, from } => {
                // A held pose (the ward's arm out), for every client that plays clips: `WP guid clip
                // secs own model` (secs 0: let go; the model whose take every race plays, if named).
                let Some(g) = self.guid_of(id) else { return };
                let mine = self.conn_of(id);
                let conns: Vec<ConnId> = self.in_world().filter(|(_, _, w)| w.anim).map(|(c, _, _)| *c).collect();
                for c in conns {
                    let own = u8::from(Some(c) == mine);
                    self.ui_line(c, &format!("WP\t{g:#X}\t{clip}\t{secs:.2}\t{own}\t{}", from.as_deref().unwrap_or("")));
                }
            }
            Event::StaffHeavy { player, lo, hi } => {
                if let Some(c) = self.conn_of(player) {
                    self.ui_line(c, &format!("SH	{lo:.2}	{hi:.2}"));
                }
            }
            Event::WardCheck { player, grade } => {
                if let Some(c) = self.conn_of(player) {
                    self.ui_line(c, &format!("CR	ward	{grade}"));
                }
            }
            Event::CheckResult { player, ability, grade } => {
                if let Some(c) = self.conn_of(player) {
                    let d = &self.sim.content.abilities[ability];
                    let key = d.key.clone();
                    // Its school, for an outcome with no ring up to show it on (it showed as the
                    // Mage's arcane burst after Penance: Corey); `world`, an outcome shown on the
                    // bodies it's about, nothing on the reticle (Penance's rings).
                    let school = if d.kind == "penance" { "world" } else { check_school(d) };
                    self.ui_line(c, &format!("CR\t{key}\t{grade}\t{school}"));
                }
            }
            Event::Speed { player, speed, back } => {
                let Some(g) = self.guid_of(player) else { return };
                let Some(c) = self.conn_of(player) else { return };
                // Everyone else sees them at their new pace (a sprint, a dash).
                self.broadcast_except(c, &packets::spline_speed(g, false, speed));
                self.broadcast_except(c, &packets::spline_speed(g, true, back));
                // A client running its own dodge needs no burst for it, but only the burst: the
                // speed after it (a run where a sprint was) must reach it, or it keeps sprinting
                // while the sim, sure it was told, stops charging stamina for it.
                let local = self.world_of(c).is_some_and(|w| w.local);
                let dash_secs = self.sim.content.dodge.dash_secs;
                let dashing = matches!(self.sim.players.get(&player).map(|p| &p.action), Some(htl_survival::Action::Dodge { t, dash: true, .. }) if *t < dash_secs);
                if local && dashing {
                    return;
                }
                for (back_too, v) in [(false, speed), (true, back)] {
                    let counter = self.world_mut(c).map_or(0, |w| {
                        w.counter += 1;
                        w.counter
                    });
                    let p = if back_too { packets::force_run_back_speed(g, counter, v) } else { packets::force_run_speed(g, counter, v) };
                    self.send(c, p);
                }
            }
            Event::MobWindup { id, clip, take, windup, recovery, blockable, from } => {
                let Some(g) = self.guid_of(id) else { return };
                // The swing's trail only to a client that starts a creature's late (`ac9`): an
                // older one would draw it from the start of the wind-up and be done before the blow.
                let trail = if blockable { FOE_TRAIL } else { FOE_TRAIL_UNBLOCKABLE };
                // The last field: another character model's take on the clip (`ac10`), or none.
                let from = from.as_deref().map(clean).unwrap_or_default();
                let with = format!("AN\t{g:#X}\t{clip}\t{windup:.3}\t{trail}\t{FOE_TRAIL_MS}\t0\t{take}\t{recovery:.3}\t0\tmob\t0\t{from}");
                let without = format!("AN\t{g:#X}\t{clip}\t{windup:.3}\t-\t0\t0\t{take}\t{recovery:.3}\t0\tmob\t0\t{from}");
                let conns: Vec<(ConnId, u8)> = self.in_world().filter(|(_, _, w)| w.anim).map(|(c, _, w)| (*c, w.level)).collect();
                for (c, level) in conns {
                    self.ui_line(c, if level >= FOE_TRAILS_LEVEL { &with } else { &without });
                }
            }
            Event::React { id, anim, from, lean, hold, fit } => {
                let Some(g) = self.guid_of(id) else { return };
                let at = self.sim.players.get(&id).map(|p| p.pos).or_else(|| self.sim.mobs.get(&id).map(|m| m.pos));
                let Some(at) = at else { return };
                // The way the blow drives them, flat, in the world's axes.
                let (dx, dy) = (at[0] - from[0], at[1] - from[1]);
                let len = (dx * dx + dy * dy).sqrt().max(1.0e-3);
                let line = format!("RE\t{g:#X}\t{anim}\t{:.3}\t{:.3}\t{lean:.2}\t{hold:.2}", dx / len, dy / len);
                let theirs = self.conn_of(id);
                let conns: Vec<ConnId> = self.in_world().filter(|(_, _, w)| w.anim).map(|(c, _, _)| *c).collect();
                for c in conns {
                    self.ui_line(c, &format!("{line}\t{}\t{fit:.2}", u8::from(theirs == Some(c))));
                }
            }
            Event::Visual { id, kit, by } => {
                if let Some(g) = self.guid_of(id) {
                    let p = packets::play_spell_visual(g, kit);
                    // The striker's own client (the fourth round) flashed its own blow already.
                    let own = [htl_survival::KIT_HEAVY, htl_survival::KIT_SLAM, htl_survival::KIT_BASH, htl_survival::KIT_CLEAVE].contains(&kit);
                    let striker = by.and_then(|b| self.conn_of(b)).filter(|c| own && self.world_of(*c).is_some_and(|w| w.level >= 4));
                    match striker {
                        Some(c) => self.broadcast_except(c, &p),
                        None => self.broadcast(&p),
                    }
                }
            }
            Event::Lock { player, target } => {
                let g = target.and_then(|t| self.guid_of(t)).unwrap_or(0);
                if let Some(c) = self.conn_of(player) {
                    if self.has_addon(c) {
                        self.ui_line(c, &format!("LO\t{g:#X}"));
                    }
                }
            }
            Event::Gear { player } => {
                self.show_gear(player);
                // A casting staff in hand: every add-on's client launches this hero's bolts from
                // its tip (a client with SetUnitMissileOrigin, patch 0044); anything else, the hands.
                if let Some(g) = self.guid_of(player) {
                    let staff = self.sim.players.get(&player).is_some_and(|p| self.sim.weapon_of(p).casts());
                    let line = format!("MO	{g:#X}	{}", if staff { "weapon" } else { "" });
                    let conns: Vec<ConnId> = self.in_world().filter(|(_, _, w)| w.addon).map(|(c, _, _)| *c).collect();
                    for c in conns {
                        self.ui_line(c, &line);
                    }
                    // The hero's own staff, for its mark on their screen.
                    if let Some(c) = self.conn_of(player) {
                        let line = self.staff_line(player);
                        self.ui_line(c, &line);
                    }
                }
            }
            // The add-on's effects line follows on the next flush.
            Event::Effects { .. } | Event::Goal | Event::Sites => {}
            Event::Guard { player, up } => {
                // The guard's pose, held while the hero stands with it up. Down, a pose the rules
                // hold of their own (Arrow Recall's arm out) comes back rather than nothing.
                let Some(g) = self.guid_of(player) else { return };
                let pose = if up { self.guard_pose(player) } else { self.poses.get(&player).copied().unwrap_or(0) };
                let mut f = Fields::new();
                f.set(idx::UNIT_NPC_EMOTESTATE, pose);
                self.broadcast(&packets::update_object(&[], &[(g, &f)]));
            }
            Event::Pose { id, pose } => {
                // A pose held until the rules let it go (0), the way the guard's is.
                if pose == 0 {
                    self.poses.remove(&id);
                } else {
                    self.poses.insert(id, pose);
                }
                let Some(g) = self.guid_of(id) else { return };
                let mut f = Fields::new();
                f.set(idx::UNIT_NPC_EMOTESTATE, pose);
                self.broadcast(&packets::update_object(&[], &[(g, &f)]));
            }
            Event::Winded { player } => {
                if let Some(c) = self.conn_of(player) {
                    if self.has_addon(c) {
                        self.ui_line(c, "WD");
                    } else {
                        self.say_to(c, "Too winded.");
                    }
                }
            }
            Event::Kill { killer, victim } => {
                let line = format!("KF\t{}\t{}", clean(killer.as_deref().unwrap_or("")), clean(&victim));
                let text = match &killer {
                    Some(k) => format!("{k} felled {victim}."),
                    None => format!("{victim} fell."),
                };
                let conns: Vec<ConnId> = self.in_world().map(|(c, _, _)| *c).collect();
                for c in conns {
                    if self.has_addon(c) {
                        self.ui_line(c, &line);
                    } else {
                        self.say_to(c, &text);
                    }
                }
            }
            Event::Danger { player, secs, from } => {
                // `WN\t<secs>\t<the creature's guid>`: the blow's foe warned; on a client that
                // can, the creature's weapon goes red for it (the guid empty: a hazard).
                let g = from.and_then(|m| self.guid_of(m)).map_or(String::new(), |g| format!("{g:#X}"));
                if let Some(c) = self.conn_of(player) {
                    if self.has_addon(c) {
                        self.ui_line(c, &format!("WN\t{secs:.2}\t{g}"));
                    }
                }
                // Everyone else near sees the weapon go red too (once per wind-up).
                if let Some(m) = from {
                    if self.warned_glow.get(&m).is_none_or(|t| *t < self.sim.clock) {
                        self.warned_glow.insert(m, self.sim.clock + secs);
                        let mine = self.conn_of(player);
                        let conns: Vec<ConnId> = self.in_world().filter(|(c, _, w)| w.addon && Some(**c) != mine).map(|(c, _, _)| *c).collect();
                        for c in conns {
                            self.ui_line(c, &format!("WG\t{g}\t{secs:.2}"));
                        }
                    }
                }
            }
            Event::Mount { id, display } => {
                // On a mount (Divine Steed's charger) or back on foot, for everyone.
                let Some(g) = self.guid_of(id) else { return };
                let mut f = Fields::new();
                f.set(idx::UNIT_MOUNTDISPLAYID, display);
                self.broadcast(&packets::update_object(&[], &[(g, &f)]));
            }
            Event::Grow { id, scale } => {
                // The body drawn larger (Rallying Cry), for everyone, its owner too.
                let Some(g) = self.guid_of(id) else { return };
                let mut f = Fields::new();
                f.set_f32(idx::SCALE_X, scale);
                self.broadcast(&packets::update_object(&[], &[(g, &f)]));
            }
            Event::Glow { id, secs } => {
                // `WG\t<guid>\t<secs>` to every add-on here, the striker's own included; a
                // warning for the same wind-up then glows nothing more.
                let Some(g) = self.guid_of(id) else { return };
                self.warned_glow.insert(id, self.sim.clock + secs);
                let conns: Vec<ConnId> = self.in_world().filter(|(_, _, w)| w.addon).map(|(c, _, _)| *c).collect();
                for c in conns {
                    self.ui_line(c, &format!("WG\t{g:#X}\t{secs:.2}"));
                }
            }
            Event::Flash { player, text } => {
                if let Some(c) = self.conn_of(player) {
                    if self.has_addon(c) {
                        self.ui_line(c, &format!("FL\t{}", clean(&text)));
                    }
                }
            }
            Event::Hear { player, sound } => {
                // The stock client plays it itself: no add-on needed.
                self.to_player(player, packets::play_sound(sound));
            }
            Event::Hop { player, dir, speed, lift, roll, style } => {
                let Some(g) = self.guid_of(player) else { return };
                let Some(c) = self.conn_of(player) else { return };
                // A dodge tumbles, on the clients that roll bodies themselves.
                if roll {
                    // A dodge's time; a sidestep riding a dash (Fade) sways only as long as the dash
                    // runs (Corey: the step kept going in place after the Fade was over).
                    let dodge = self.sim.content.dodge.secs;
                    let secs = match self.sim.players.get(&player).map(|p| &p.action) {
                        Some(htl_survival::Action::Dash { t, secs, .. }) => (secs - t + 0.05).clamp(0.12, dodge),
                        _ => dodge,
                    };
                    let conns: Vec<ConnId> = self.in_world().filter(|(_, _, w)| w.anim).map(|(c, _, _)| *c).collect();
                    for k in conns {
                        self.ui_line(k, &format!("RO\t{g:#X}\t{secs:.2}\t{}\t{style}", u8::from(k == c)));
                    }
                }
                // A dash along the ground (speed 0) is the run speed's; only a hop is thrown.
                if speed > 0.0 {
                    let counter = self.world_mut(c).map_or(0, |w| {
                        w.counter += 1;
                        w.counter
                    });
                    self.send(c, packets::knock_back(g, counter, dir, speed, lift));
                }
            }
            Event::Died { id } => {
                if let Some(g) = self.guid_of(id) {
                    let mut f = Fields::new();
                    f.set(idx::UNIT_HEALTH, 0).set_u64(idx::UNIT_TARGET, 0);
                    self.broadcast(&packets::update_object(&[], &[(g, &f)]));
                }
                if self.is_player(id) {
                    if let Some(c) = self.conn_of(id) {
                        let secs = self.sim.content.pvp.respawn_secs.unwrap_or(self.sim.content.player.respawn_secs);
                        // And what felled them: the blows (the last first) and a word on answering it.
                        let recap = self.sim.death_recap(id);
                        self.ui_line(c, &format!("DE\t{secs:.0}\t{}\t{}", recap.lines.join("^"), recap.tip));
                    }
                }
            }
            Event::Revived { player } => {
                if let (Some(g), Some(p)) = (self.guid_of(player), self.sim.players.get(&player)) {
                    let mut f = Fields::new();
                    f.set(idx::UNIT_HEALTH, p.health).set(idx::UNIT_MAXHEALTH, p.max_health).set(idx::UNIT_DYNAMIC_FLAGS, 0);
                    self.broadcast(&packets::update_object(&[], &[(g, &f)]));
                }
                if let Some(c) = self.conn_of(player) {
                    self.ui_line(c, "DE\t0");
                }
            }
            Event::Teleport { player, pos, facing } => {
                let Some(g) = self.guid_of(player) else { return };
                let Some(c) = self.conn_of(player) else { return };
                let counter = self.world_mut(c).map_or(0, |w| {
                    w.counter += 1;
                    w.counter
                });
                if let Some(w) = self.world_mut(c) {
                    w.ground_z = pos[2];
                    w.fall_top = None;
                }
                self.send(c, packets::teleport(g, counter, pos, facing));
                self.broadcast_except(c, &packets::heartbeat(g, pos, facing));
            }
            Event::ObjSpawned { id } => {
                // A node back, or new: (re)draw it.
                self.obj_destroy(id);
                self.obj_show(id);
            }
            // A stand-in goes in its own time (`step_stand_ins`).
            Event::ObjGone { id } => match self.stand_ins.get_mut(&id) {
                Some(s) => s.gone = true,
                None => self.obj_destroy(id),
            },
            Event::ObjChanged { id } => {
                // A door swings by its state, drawn where it is.
                if let (Some(v), Some(g)) = (
                    self.sim
                        .object(id)
                        .filter(|v| matches!(v.kind, htl_survival::ObjKind::Door { .. })),
                    self.guid_of(id),
                ) {
                    let mut f = Fields::new();
                    f.set(idx::GAMEOBJECT_STATE, self.obj_state(&v));
                    self.broadcast(&packets::update_object(&[], &[(g, &f)]));
                    return;
                }
                // A station being placed, moved or judged afresh: sent again under the same
                // guids to its placer alone, which a patched client (0017) moves in place, never
                // solid and never fading in and out as it follows the hero.
                if let (Some(owner), true) = (self.ghost_owner(id), self.guid_of(id).is_some()) {
                    // (An older client leaves a re-created object where it stood: drawn afresh.)
                    let patched = |c: ConnId| self.world_of(c).is_some_and(|w| w.level >= 12);
                    if let Some(c) = self.conn_of(owner).filter(|c| patched(*c)) {
                        for p in self.obj_create(id) {
                            self.send(c, p);
                        }
                        return;
                    }
                }
                // A new look or name (damaged): the client caches names by entry, so it's drawn
                // afresh.
                self.obj_destroy(id);
                if self.sim.objs.contains_key(&id) {
                    self.obj_show(id);
                }
            }
            Event::Mark { id, pos, radius, tele: _, ground } if ground != 0 => {
                // A real spell's look on the ground for it, for everyone, in place of a circle.
                let guid = super::draw::area_guid(id);
                self.broadcast(&super::draw::area_create(guid, 0, ground, pos, radius));
                self.area_looks.insert(id, guid);
            }
            Event::Mark { id, pos, radius, tele, .. } => {
                // The clients that draw a creature's blow get it (`TG`), those that lay ground
                // markers the circle (`MK`); the rest the danger model.
                let p = self.mark_create(id, pos, radius);
                let line = self.mark_line(id, pos, radius);
                let conns: Vec<(ConnId, Id, u8)> = self.in_world().map(|(c, _, w)| (*c, w.id, if w.addon { w.level } else { 0 })).collect();
                // Salvation's circle and rim: drawn by the add-on itself from `SV` on a client
                // with ground rings (ac24, patch 0041); the rest get the circle.
                let drawn = self.sim.drawn_by_addon(id);
                for (c, hero, level) in conns {
                    if drawn && level >= 24 {
                        continue;
                    }
                    // A leap's landing isn't drawn for the hero leaping (others still see it).
                    if tele.is_some_and(|t| t.own_hidden && t.mob == hero) {
                        continue;
                    }
                    match tele {
                        // A shape on the ground at a spot (a sky-fall, a strip), on the clients
                        // that lay one there; the rest the circle.
                        Some(t) if t.at.is_some() && level >= 8 => self.ui_line(c, &self.telegraph_at_line(id, &t, hero)),
                        Some(t) if t.at.is_some() && level >= 5 => self.ui_line(c, &line),
                        Some(t) if t.width > 0.0 && level < 8 => {
                            if level >= 5 {
                                self.ui_line(c, &line);
                            }
                        }
                        Some(t) if level >= 7 => {
                            if let Some(tg) = self.telegraph_line(id, &t, hero, level) {
                                self.ui_line(c, &tg);
                            }
                        }
                        _ if level >= 5 => self.ui_line(c, &line),
                        _ => self.send(c, p.clone()),
                    }
                }
            }
            Event::AreaLook { id, caster, spell, pos, radius } => {
                // The spell's own ground look for as long as it lingers: the client draws a
                // DynamicObject's area model and kit (Consecration's glow, Flamestrike's fire).
                let guid = super::draw::area_guid(id);
                let caster = self.guid_of(caster).unwrap_or(0);
                self.broadcast(&super::draw::area_create(guid, caster, spell, pos, radius));
                self.area_looks.insert(id, guid);
            }
            Event::Rope { id, a, b } => {
                // On the clients that stretch a rope from end to end (patch 0028), in place of the
                // strip on the ground laid for it just before (taken off theirs as a gone mark's
                // is: floor clutter, Corey's review); the rest keep the strip.
                let p = self.rope_create(id, a, b);
                let conns: Vec<(ConnId, bool)> = self.in_world().filter(|(_, _, w)| w.level >= super::draw::ROPE_LEVEL).map(|(c, _, w)| (*c, w.addon)).collect();
                for (c, addon) in conns {
                    self.send(c, p.clone());
                    if addon {
                        self.ui_line(c, &format!("MK\t{id}"));
                        self.ui_line(c, &format!("TG\t{id}"));
                    }
                }
                self.ropes.insert(id);
            }
            Event::Tether { from, to, secs } => {
                // A taut line from the hand (patch 0031): `RP <key> <from> <to's guid or -> <x> <y>
                // <z> <secs>`, keyed by who holds it, so a new line replaces theirs.
                let Some(g) = self.guid_of(from) else { return };
                let (to_guid, at) = match to {
                    Target::Unit(u) => match self.guid_of(u) {
                        Some(t) => (format!("0x{t:016X}"), [0.0; 3]),
                        None => return,
                    },
                    Target::Point(p) => ("-".to_string(), p),
                };
                let line = format!("RP\t{g:X}\t0x{g:016X}\t{to_guid}\t{:.2}\t{:.2}\t{:.2}\t{secs:.2}", at[0], at[1], at[2]);
                let conns: Vec<ConnId> = self.in_world().filter(|(_, _, w)| w.addon && w.level >= super::draw::UNIT_ROPE_LEVEL).map(|(c, _, _)| *c).collect();
                for c in conns {
                    self.ui_line(c, &line);
                }
            }
            Event::MissileTint { caster, spell, ability, bounce } => {
                // The next missile `caster` flies of the spell takes its bounce's colour, on the
                // clients that draw a missile's look (patch 0030): Bank Shot hotter each bounce.
                // A stand-in flies the arrow on from a ricochet, so its own guid names that one.
                let Some(g) = self.stand_ins.get(&caster).map(|s| s.guid).or_else(|| self.guid_of(caster)) else { return };
                let c = &self.sim.content;
                let Some(colour) = c.abilities.get(ability).and_then(|d| d.bounce_trail.get((bounce as usize).min(d.bounce_trail.len().saturating_sub(1)))) else { return };
                let base = c.missile_looks.iter().find(|l| l.spell == spell);
                let (width, secs, scale, spin, tumble, speed) = base.map_or((0.3, 0.4, 1.0, 0.0, false, 0.0), |l| (l.width.max(0.3), l.secs.max(0.4), l.scale, l.spin, l.tumble, l.speed));
                let line = super::draw::missile_look_line(spell, colour, width, secs, scale, spin, Some(g), tumble, speed);
                let conns: Vec<ConnId> = self.in_world().filter(|(_, _, w)| w.addon && w.level >= super::draw::MISSILE_LOOK_LEVEL).map(|(c, _, _)| *c).collect();
                for c in conns {
                    self.ui_line(c, &line);
                }
            }
            Event::MarkGone { id } => {
                if let Some(g) = self.area_looks.remove(&id) {
                    self.broadcast(&packets::destroy_object(g));
                }
                if self.ropes.remove(&id) {
                    self.broadcast(&packets::destroy_object(super::draw::rope_guid(id)));
                }
                self.mark_at.remove(&id);
                let conns: Vec<(ConnId, u8)> = self.in_world().filter(|(_, _, w)| w.addon && w.level >= 5).map(|(c, _, w)| (*c, w.level)).collect();
                for (c, level) in conns {
                    self.ui_line(c, &format!("MK\t{id}"));
                    if level >= 7 {
                        self.ui_line(c, &format!("TG\t{id}"));
                    }
                }
                if let Some(g) = self.unbind(id) {
                    self.broadcast(&packets::destroy_object(g));
                }
            }
            Event::Bags { player } => {
                if let Some(c) = self.conn_of(player) {
                    self.sync_items(c);
                }
            }
            Event::Storage => self.storage_dirty = true,
            Event::Tell { player, text } => {
                if let Some(c) = self.conn_of(player) {
                    self.tell(c, &text);
                }
            }
            Event::Announce { text } => self.broadcast(&packets::chat(packets::CHAT_SYSTEM, packets::LANG_UNIVERSAL, 0, &text)),
            Event::Banner { player, title, sub } => {
                let conns: Vec<ConnId> = match player {
                    Some(p) => self.conn_of(p).into_iter().collect(),
                    None => self.in_world().map(|(c, _, _)| *c).collect(),
                };
                for c in conns {
                    if self.has_addon(c) {
                        self.ui_line(c, &format!("BN\t{}\t{}", clean(&title), clean(&sub)));
                    } else {
                        self.send(c, packets::chat(packets::CHAT_RAID_WARNING, packets::LANG_UNIVERSAL, 0, &format!("{title}: {sub}")));
                    }
                }
            }
            Event::Letter { player, kind, title, place, text, clue, links } => {
                if let Some(c) = self.conn_of(player) {
                    self.letter(c, &kind, &title, &place, &text, &clue, &links);
                }
            }
            Event::StationOpen { player, station } => {
                if let Some(c) = self.conn_of(player) {
                    if let Some(w) = self.world_mut(c) {
                        w.station = Some(station);
                    }
                    self.ui_station(c, player, station);
                }
            }
            Event::Confirm { player, what, text, yes, no } => {
                if let Some(c) = self.conn_of(player) {
                    if self.has_addon(c) {
                        self.ui_line(c, &format!("CF\t{}\t{}\t{}\t{}", clean(&what), clean(&text), clean(&yes), clean(&no)));
                    } else {
                        self.say_to(c, &format!("{text} Type .{what} yes to confirm."));
                    }
                }
            }
            Event::Travel { player, to } => {
                if let Some(c) = self.conn_of(player) {
                    self.transfers.push((c, to));
                }
            }
            Event::ToCamp { counts, items } => self.to_camp.push((counts, items)),
            Event::WagerDone(run) => self.wagers.push(run),
            Event::Arena { player } => {
                if let Some(c) = self.conn_of(player) {
                    self.ui_arena(c, player);
                }
            }
            Event::Siege { player } => {
                self.siege_dirty.insert(player);
            }
            Event::Duel { a, b, news } => self.duel_news(a, b, news),
            Event::CastDone { caster, spell } => {
                // Its look ends too: the precast (a glow in the hands) stays on a caster standing
                // still until a GO of that same spell or a failure, and what goes is often
                // another spell. A bow's draw ends with its own shot, the same spell.
                if !self.casting.get(&caster).is_some_and(|s| spell == 0 || *s == spell) {
                    return;
                }
                let Some(was) = self.casting.remove(&caster) else { return };
                let ranged = self.sim.content.ranged.iter().any(|r| r.spell == was);
                if let (false, Some(c)) = (ranged, self.guid_of(caster)) {
                    self.broadcast(&packets::spell_failed_other(c, was));
                }
            }
            Event::Loose { caster, spell } => {
                // A bow ability's arrow: the go of the bow's own draw, which lets the string go
                // (AttackBow, as the base game's shot) with nothing flying of its own (no target,
                // no ammo): the ability's missiles fly theirs.
                self.casting.remove(&caster);
                if let (Some(c), true) = (self.guid_of(caster), spell != 0) {
                    self.broadcast(&packets::spell_go(c, spell, Aim::Caster, &[], None));
                }
            }
            Event::Beat { player, beat, on } => {
                // The hero's own screen alone (what everyone sees and hears went as its own
                // looks and sounds): nothing is forced on anyone else's view.
                let g = on.and_then(|u| self.guid_of(u)).unwrap_or(0);
                if let Some(c) = self.conn_of(player).filter(|c| self.has_addon(*c)) {
                    // Kill Shot's weight goes as `BT`; an outcome's beat (`cues::BEAT_*`) as `CU`, where the
                    // add-on plays its hit-stop, shake and kick.
                    let line = if beat.starts_with("kill_shot") { "BT" } else { "CU" };
                    self.ui_line(c, &format!("{line}\t{}\t{g:#X}", clean(&beat)));
                }
            }
            Event::Auras { id } => self.show_auras(id),
            Event::Rooted { player, on } => {
                let (Some(g), Some(c)) = (self.guid_of(player), self.conn_of(player)) else { return };
                let counter = self.world_mut(c).map_or(0, |w| {
                    w.counter += 1;
                    w.counter
                });
                self.send(c, packets::force_move_root(g, counter, on));
            }
            Event::FeatherFall { player, on } => {
                let (Some(g), Some(c)) = (self.guid_of(player), self.conn_of(player)) else { return };
                let counter = self.world_mut(c).map_or(0, |w| {
                    w.counter += 1;
                    w.counter
                });
                self.send(c, packets::move_feather_fall(g, counter, on));
            }
            Event::Dash { player, dir, speed, secs } => {
                let (Some(g), Some(c)) = (self.guid_of(player), self.conn_of(player)) else { return };
                let local = self.world_of(c).is_some_and(|w| w.local);
                if local && self.has_addon(c) {
                    // Its own client runs the dash, along the way it faces (the reticle's).
                    self.ui_line(c, &format!("DA\t{secs:.2}\t{speed:.1}\t{dir:.4}"));
                } else {
                    // A stock client is thrown along it.
                    let counter = self.world_mut(c).map_or(0, |w| {
                        w.counter += 1;
                        w.counter
                    });
                    self.send(c, packets::knock_back(g, counter, dir, speed * 0.8, 1.5));
                }
            }
            Event::Catch { player, ability, at } => {
                // Where an aimed grappling hook would catch, for the add-on's aim line.
                let Some(c) = self.conn_of(player).filter(|c| self.has_addon(*c)) else { return };
                let line = match (at, self.sim.players.get(&player).map(|p| p.pos)) {
                    (Some((at, caught)), Some(pos)) => {
                        let ground = self.sim.land().height_near(at[0], at[1], at[2] + 1.0);
                        let yards = htl_survival::dist2d(pos, at);
                        let key = self.sim.content.abilities.get(ability).map_or("", |d| d.key.as_str());
                        format!("GP\t{}\t{:.2}\t{:.2}\t{:.2}\t{yards:.2}\t{ground:.2}\t{key}", u8::from(caught), at[0], at[1], at[2])
                    }
                    _ => "GP".to_string(),
                };
                self.ui_line(c, &line);
            }
            Event::LeapLook { player, secs, clip, take, from, flip } => {
                // A leap's own look (a Disengage's flip), on the clients that play clips.
                let Some(g) = self.guid_of(player) else { return };
                let mine = self.conn_of(player);
                let conns: Vec<ConnId> = self.in_world().filter(|(_, _, w)| w.anim).map(|(c, _, _)| *c).collect();
                for c in conns {
                    let line = format!("FP\t{g:#X}\t{secs:.2}\t{clip}\t{take}\t{from}\t{}\t{}", u8::from(flip), u8::from(Some(c) == mine));
                    self.ui_line(c, &line);
                }
            }
            Event::Abilities { player } => {
                self.abilities_dirty.insert(player);
            }
            // The add-on's Valor line follows on the next flush.
            Event::Valor { .. } => {}
            Event::Clip { id, clip, take, from, secs, at, body } => {
                // A cast (a throw, a spell), for the clients that play clips: the caster's own
                // marked as theirs. The impact where the event says, else half way.
                let Some(g) = self.guid_of(id) else { return };
                let mine = self.conn_of(id);
                let impact = if at > 0.0 { at.min(secs) } else { secs * 0.5 };
                let conns: Vec<ConnId> = self.in_world().filter(|(_, _, w)| w.anim).map(|(c, _, _)| *c).collect();
                for c in conns {
                    let own = u8::from(Some(c) == mine);
                    let line = format!("AN\t{g:#X}\t{clip}\t{impact:.3}\t-\t0\t{own}\t{take}\t{:.3}\t0\tcast\t0\t{}\t{}", secs - impact, from.as_deref().unwrap_or(""), body.word());
                    self.ui_line(c, &line);
                }
            }
            Event::Flurry { id, secs, def } => {
                // Bladestorm's storm of cuts, for the clients that play clips (the caster's own
                // marked as theirs): the ability's `flurry`, field for field.
                let Some(g) = self.guid_of(id) else { return };
                let mine = self.conn_of(id);
                let conns: Vec<ConnId> = self.in_world().filter(|(_, _, w)| w.anim).map(|(c, _, _)| *c).collect();
                for c in conns {
                    let own = u8::from(Some(c) == mine);
                    let line = format!(
                        "FU\t{g:#X}\t{own}\t{secs:.2}\t{}\t{}\t{:.3}\t{:.3}\t{}/{}\t{:.3}\t{:.1}\t{}\t{}",
                        def.from.as_deref().unwrap_or(""),
                        def.cuts_text(),
                        def.cut,
                        def.windup,
                        def.finish[0],
                        def.finish[1],
                        def.finish_secs,
                        def.turn,
                        def.trail,
                        def.trail_ms
                    );
                    self.ui_line(c, &line);
                }
            }
            Event::GroundBurst { at, yaw, shape, reach, arc, width, secs, colour } => {
                // A shape flashed on the ground, for the clients that draw one (the add-on asks
                // its client; an older client ignores the line).
                let conns: Vec<ConnId> = self.in_world().filter(|(_, _, w)| w.anim).map(|(c, _, _)| *c).collect();
                let line = format!("GB\t{:.2}\t{:.2}\t{:.2}\t{yaw:.3}\t{shape}\t{reach:.2}\t{arc:.0}\t{secs:.2}\t{colour}\t{width:.2}", at[0], at[1], at[2]);
                for c in conns {
                    self.ui_line(c, &line);
                }
            }
            Event::ClipStop { id } => {
                // A feint: the swing let go on the clients that play clips (the caster's own
                // marked, which has let it go already).
                let Some(g) = self.guid_of(id) else { return };
                let mine = self.conn_of(id);
                let conns: Vec<ConnId> = self.in_world().filter(|(_, _, w)| w.anim).map(|(c, _, _)| *c).collect();
                for c in conns {
                    self.ui_line(c, &format!("ST\t{g:#X}\t{}", u8::from(Some(c) == mine)));
                }
            }
            Event::Channel { caster, target, spell, ms } => {
                let Some(g) = self.guid_of(caster) else { return };
                let t = target.and_then(|t| self.guid_of(t)).unwrap_or(0);
                // The beam to its target (the client draws a chain between them).
                let mut f = Fields::new();
                f.set_u64(idx::UNIT_CHANNEL_OBJECT, t).set(idx::UNIT_CHANNEL_SPELL, if target.is_some() { spell } else { 0 });
                self.broadcast(&packets::update_object(&[], &[(g, &f)]));
                if let Some(c) = self.conn_of(caster) {
                    if target.is_some() {
                        self.send(c, packets::channel_start(spell, ms));
                    } else {
                        self.send(c, packets::channel_update(0));
                    }
                }
            }
            Event::Sound { id, sound } => {
                if let Some(g) = self.guid_of(id) {
                    self.broadcast(&packets::play_object_sound(g, sound));
                }
            }
            Event::Healed { id, amount } => {
                *self.healed.entry(id).or_default() += amount;
            }
            Event::HealedOther { healer, target, amount } => {
                // The healer sees what they mended on a friend: from `ac28` a green number over
                // the friend's head, as WoW floats it (`SMSG_SPELLHEALLOG`, spell 0 as the mend
                // has no 1.12 spell; patch 0056); on an older client the add-on's `HO` line,
                // `<amount> <name>` rising mid-screen.
                let Some(c) = self.conn_of(healer) else { return };
                if self.world_of(c).is_some_and(|w| w.level >= HEAL_FLOAT_LEVEL) {
                    if let (Some(t), Some(h)) = (self.guid_of(target), self.guid_of(healer)) {
                        self.send(c, packets::spell_heal(t, h, 0, amount, false));
                    }
                } else if self.has_addon(c) {
                    let name = self.sim.players.get(&target).map(|p| p.name.clone()).unwrap_or_default();
                    self.ui_line(c, &format!("HO\t{name}\t{amount}"));
                }
            }
            Event::Log(text) => println!("[survival] {text}"),
            Event::Dirty => self.mark_dirty(),
        }
    }
}

impl SurvivalGame {
    /// A unit's lasting looks: each effect's and stance's 1.12 spell in its aura slots (the state
    /// visuals the clients play: the roots' vines, the stun's swirl, Ice Block's ice).
    pub(crate) fn show_auras(&mut self, id: Id) {
        let Some(g) = self.guid_of(id) else { return };
        let list = self.sim.auras_of(id);
        let mut bar = [(0u32, 0u8); 48];
        for (k, a) in list.iter().take(16).enumerate() {
            bar[k] = (*a, 1);
        }
        let before = self.auras_shown.get(&id).copied().unwrap_or(0);
        let n = list.len().min(16);
        let mut f = Fields::new();
        for slot in 0..n.max(before) {
            fields::aura_words(&mut f, &bar, slot);
        }
        if n.max(before) > 0 {
            self.broadcast(&packets::update_object(&[], &[(g, &f)]));
        }
        self.auras_shown.insert(id, n);
    }

    /// The pose a hero's raised guard shows.
    pub(crate) fn guard_pose(&self, player: Id) -> u32 {
        match self.sim.players.get(&player) {
            Some(p) if self.sim.guard_of(p).ward => POSE_WARD,
            Some(p) if self.sim.weapon_of(p).two_handed => POSE_TWO_HANDED,
            _ => POSE_SHIELD,
        }
    }
}

/// The school a skill check is drawn in: from its look (a fire, frost or arcane spell; a Priest's
/// holy or shadow one, never the Mage's arcane: Corey saw the arcane ring on Greater Smite), else
/// arcane (a mage's).
fn check_school(d: &htl_survival::content::AbilityDef) -> &'static str {
    let k = d.key.as_str();
    let priest = d.school == "priest" || ["smite", "shadow_burst", "shadow_bolt", "gust", "tether"].iter().any(|w| k.contains(w));
    if priest {
        if ["shadow", "mind", "void", "tether", "fade", "death"].iter().any(|w| k.contains(w)) {
            "shadow"
        } else {
            "holy"
        }
    } else if k.contains("fire") || k.contains("pyro") || k.contains("meteor") || k.contains("ember") || k.contains("flame") {
        "fire"
    } else if k.contains("frost") || k.contains("cold") || k.contains("ice") || k.contains("rime") || k.contains("orb") && !k.contains("arcane") {
        "frost"
    } else {
        "arcane"
    }
}

/// Text safe inside an add-on line (no tabs, line breaks or the add-on's own separators).
impl SurvivalGame {
    /// A page for one hero to read at leisure (`NT` with the add-on, a few chat lines without).
    pub(crate) fn letter(&self, c: ConnId, kind: &str, title: &str, place: &str, text: &str, clue: &str, links: &[(String, String)]) {
        if self.has_addon(c) {
            let page = |t: &str| clean(&t.replace('\n', "{n}"));
            let links: Vec<String> = links.iter().map(|(cmd, label)| format!("{}~{}", clean(cmd), clean(label))).collect();
            self.ui_line(c, &format!("NT\t{kind}\t{}\t{}\t{}\t{}\t{}", clean(title), clean(place), page(text), page(clue), links.join("^")));
        } else {
            // In chat, a line a paragraph: the title, the page, the clue.
            self.say_to(c, &format!("{title} ({place})"));
            for p in text.split('\n').filter(|p| !p.trim().is_empty()) {
                self.say_to(c, p);
            }
            if !clue.is_empty() {
                self.say_to(c, &format!("A clue: {clue}"));
            }
            if !links.is_empty() {
                let more: Vec<String> = links.iter().map(|(cmd, label)| format!("{cmd} ({label})")).collect();
                self.say_to(c, &format!("Read on: {}", more.join(", ")));
            }
        }
    }
}

pub(crate) fn clean(s: &str) -> String {
    s.replace(['\t', '\n', '\r', '|', '~', '^'], " ")
}

/// The arrow a missile flies with when its look is a nocked arrow (the bow's shot, the hunter's
/// shots in `ARROW_SPELLS`, and every Hunter arrow, those that plant), from the hero or from a
/// stand-in: the client draws a shot with no missile model of its own from the ammo it's sent,
/// and with none, nothing.
fn arrow_ammo(c: &htl_survival::content::Content, spell: u32) -> Option<u32> {
    let arrow = c.ranged.iter().any(|r| r.spell == spell)
        || ARROW_SPELLS.contains(&spell)
        || c.abilities.iter().any(|a| a.plants && (a.spell == spell || a.full_spell == spell));
    arrow.then_some(ARROW_DISPLAY)
}
