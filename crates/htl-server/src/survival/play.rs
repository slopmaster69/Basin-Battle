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
const ARROW_DISPLAY: u32 = packets::ARROW_DISPLAY;
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

impl SurvivalGame {
    pub(crate) fn play(&mut self, events: Vec<Event>) {
        for e in events {
            self.event(e);
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
                let ammo = ranged.then_some(ARROW_DISPLAY);
                // The shot ends the bow's drawn pose.
                if ranged {
                    self.casting.remove(&caster);
                }
                self.broadcast(&packets::spell_go(c, spell, aim, &hits, ammo));
            }
            Event::MissileFrom { anchor, caster: _, spell, from, target } => {
                // A missile from a spot (a boomerang turning back): an unseen object stands there
                // and flies it, until the rules say it's gone (`ObjGone`).
                let (aim, hits) = match target {
                    Target::Unit(t) => match self.guid_of(t) {
                        Some(g) => (Aim::Unit(g), vec![g]),
                        None => return,
                    },
                    Target::Point(p) => (Aim::Point(p), Vec::new()),
                };
                let (g, made) = self.anchor_create(anchor, from);
                self.broadcast(&made);
                let ammo = self.sim.content.ranged.iter().any(|r| r.spell == spell).then_some(ARROW_DISPLAY);
                self.broadcast(&packets::spell_go(g, spell, aim, &hits, ammo));
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
            Event::Gear { player } => self.show_gear(player),
            // The add-on's effects line follows on the next flush.
            Event::Effects { .. } | Event::Goal | Event::Sites => {}
            Event::Guard { player, up } => {
                // The guard's pose, held while the hero stands with it up.
                let Some(g) = self.guid_of(player) else { return };
                let pose = if up { self.guard_pose(player) } else { 0 };
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
                    Some(k) => format!("{k} fells {victim}."),
                    None => format!("{victim} falls."),
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
                // The body drawn larger (Onslaught), for everyone, its owner too.
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
            Event::Hop { player, dir, speed, lift, roll, style } => {
                let Some(g) = self.guid_of(player) else { return };
                let Some(c) = self.conn_of(player) else { return };
                // A dodge tumbles, on the clients that roll bodies themselves.
                if roll {
                    let secs = self.sim.content.dodge.secs;
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
            Event::ObjGone { id } => self.obj_destroy(id),
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
                // A new look or name (damaged, a ghost moved): the client caches names by entry,
                // so it's drawn afresh.
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
                for (c, hero, level) in conns {
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
            Event::MarkGone { id } => {
                if let Some(g) = self.area_looks.remove(&id) {
                    self.broadcast(&packets::destroy_object(g));
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
            Event::Confirm { player, what, text } => {
                if let Some(c) = self.conn_of(player) {
                    if self.has_addon(c) {
                        self.ui_line(c, &format!("CF\t{}\t{}", clean(&what), clean(&text)));
                    } else {
                        self.say_to(c, &format!("{text} Type .{what} yes to go ahead."));
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
            Event::Auras { id } => self.show_auras(id),
            Event::Rooted { player, on } => {
                let (Some(g), Some(c)) = (self.guid_of(player), self.conn_of(player)) else { return };
                let counter = self.world_mut(c).map_or(0, |w| {
                    w.counter += 1;
                    w.counter
                });
                self.send(c, packets::force_move_root(g, counter, on));
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
            Event::Abilities { player } => {
                self.abilities_dirty.insert(player);
            }
            // The add-on's Valor line follows on the next flush.
            Event::Valor { .. } => {}
            Event::Clip { id, clip, secs } => {
                // A cast (a throw, a spell), for the clients that play clips: the caster's own
                // marked as theirs.
                let Some(g) = self.guid_of(id) else { return };
                let mine = self.conn_of(id);
                let impact = secs * 0.5;
                let conns: Vec<ConnId> = self.in_world().filter(|(_, _, w)| w.anim).map(|(c, _, _)| *c).collect();
                for c in conns {
                    let own = u8::from(Some(c) == mine);
                    let line = format!("AN\t{g:#X}\t{clip}\t{impact:.3}\t-\t0\t{own}\t0\t{:.3}\t0\tcast\t0", secs - impact);
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
                // `HO\t<name>\t<amount>`: the healer sees what they mended on a friend.
                let name = self.sim.players.get(&target).map(|p| p.name.clone()).unwrap_or_default();
                if let Some(c) = self.conn_of(healer).filter(|c| self.has_addon(*c)) {
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
            Some(p) if self.sim.weapon_of(p).two_handed => POSE_TWO_HANDED,
            _ => POSE_SHIELD,
        }
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
