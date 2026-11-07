//! Duels: one hero challenges another (WoW's own Duel, from the portrait's menu or `/duel`), the
//! one challenged takes it, and after a countdown the two fight until one would fall. That blow
//! leaves them at 1 health and the other wins; leaving the flag's bounds too long, `/forfeit` or
//! logging out gives the duel away.
//!
//! A duel is the two duellists' alone: once it's on they're each other's foes (struck, never
//! helped, whatever friendly fire says), nobody else can strike either of them or help them, and
//! neither can strike anyone else. Outside Siege only.

use super::{dist2d, Event, Id, Obj, ObjKind, Survival, Vec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DuelPhase {
    /// Challenged, waiting for an answer until then.
    Asked { until: f32 },
    /// Taken: the fight starts then.
    Counting { until: f32 },
    Fighting,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Duel {
    /// Who challenged, and who was challenged.
    pub a: Id,
    pub b: Id,
    /// The flag between them, and where it stands.
    pub flag: Id,
    pub at: Vec3,
    pub phase: DuelPhase,
    /// Since when each (a, b) has been out of bounds.
    pub out: [Option<f32>; 2],
    /// Their names (for the winner's line, though one has gone).
    pub names: [String; 2],
}

impl Duel {
    pub fn has(&self, id: Id) -> bool {
        self.a == id || self.b == id
    }

    pub fn other(&self, id: Id) -> Id {
        if self.a == id {
            self.b
        } else {
            self.a
        }
    }

    /// Under way (counting or fighting), not only asked.
    pub fn taken(&self) -> bool {
        !matches!(self.phase, DuelPhase::Asked { .. })
    }
}

/// What a duel's clients are told.
#[derive(Debug, Clone, PartialEq)]
pub enum DuelNews {
    /// `a` challenged `b`; the flag stands between them.
    Asked,
    /// Taken: the fight starts in `secs`.
    Counting { secs: f32 },
    /// On: each is the other's foe.
    Fighting,
    /// `who` went out of bounds (back within `secs` or forfeit), or came back (`secs` 0).
    Bounds { who: Id, out: bool, secs: f32 },
    /// Over. `started`: it had begun (false: declined, cancelled or never answered). `winner` and
    /// `fled`: who won, and whether the loser gave it away (forfeit, out of bounds, gone) rather
    /// than being beaten. `names`: a's and b's.
    Ended { started: bool, winner: Option<Id>, fled: bool, names: [String; 2] },
}

impl Survival {
    /// The duel a hero is in, asked or under way.
    pub fn duel_of(&self, id: Id) -> Option<&Duel> {
        self.duels.iter().find(|d| d.has(id))
    }

    /// The duel a hero is in once it's taken (counting or fighting).
    fn duel_taken(&self, id: Id) -> Option<&Duel> {
        self.duels.iter().find(|d| d.has(id) && d.taken())
    }

    /// Whether `a` and `b` are fighting each other in a duel.
    pub fn duel_foes(&self, a: Id, b: Id) -> bool {
        a != b && self.duels.iter().any(|d| d.has(a) && d.other(a) == b && d.phase == DuelPhase::Fighting)
    }

    /// Whether any hero can strike another just now: friendly fire on, or a duel under way.
    pub(crate) fn heroes_clash(&self) -> bool {
        self.content.pvp.friendly_fire || self.duels.iter().any(|d| d.phase == DuelPhase::Fighting)
    }

    /// Whether a hero's blows can land on other heroes just now: in a duel, once it's on;
    /// otherwise with friendly fire.
    pub fn strikes_heroes(&self, id: Id) -> bool {
        if self.in_duel(id) {
            self.duels.iter().any(|d| d.has(id) && d.phase == DuelPhase::Fighting)
        } else {
            self.content.pvp.friendly_fire
        }
    }

    /// Whether a hero is held in a duel, apart from everyone else (counting or fighting).
    pub fn in_duel(&self, id: Id) -> bool {
        self.duel_taken(id).is_some()
    }

    /// For the clients: a hero's duel's flag, and their side in it (1 the challenger, 2 the one
    /// challenged; 0 until it's on).
    pub fn duel_side(&self, id: Id) -> Option<(Id, u32)> {
        let d = self.duel_of(id)?;
        let side = match d.phase {
            DuelPhase::Fighting if d.a == id => 1,
            DuelPhase::Fighting => 2,
            _ => 0,
        };
        Some((d.flag, side))
    }

    /// `a` challenges `b` to a duel.
    pub fn duel_challenge(&mut self, a: Id, b: Id) -> Result<(), String> {
        let d = self.content.duel.clone();
        if self.is_siege() {
            return Err("No duels in Siege.".into());
        }
        let (Some(pa), Some(pb)) = (self.players.get(&a), self.players.get(&b)) else {
            return Err("Only another hero can be challenged to a duel.".into());
        };
        if a == b {
            return Err("You can't duel yourself.".into());
        }
        if pa.health == 0 || pb.health == 0 {
            return Err("Not while either of you is down.".into());
        }
        let name = pb.name.clone();
        let names = [pa.name.clone(), name.clone()];
        if dist2d(pa.pos, pb.pos) > d.reach {
            return Err(format!("{name} is too far away to duel."));
        }
        if self.duel_of(a).is_some() {
            return Err("You are already in a duel.".into());
        }
        if self.duel_of(b).is_some() {
            return Err(format!("{name} is already in a duel."));
        }
        if self.in_arena(a) || self.in_arena(b) {
            return Err("No duels in the Proving Ring.".into());
        }
        // The flag halfway between them, on the ground.
        let (pa, pb) = (pa.pos, pb.pos);
        let at = self.ground_near((pa[0] + pb[0]) / 2.0, (pa[1] + pb[1]) / 2.0, (pa[2] + pb[2]) / 2.0 + 2.0);
        let flag = self.alloc();
        let facing = super::facing(pa, pb);
        self.objs.insert(flag, Obj { kind: ObjKind::Prop { display: d.flag_display, scale: d.flag_scale }, pos: at, facing });
        self.events.push(Event::ObjSpawned { id: flag });
        let until = self.clock + d.ask_secs;
        self.duels.push(Duel { a, b, flag, at, phase: DuelPhase::Asked { until }, out: [None; 2], names });
        self.events.push(Event::Duel { a, b, news: DuelNews::Asked });
        Ok(())
    }

    /// The one challenged takes it: the countdown starts.
    pub fn duel_accept(&mut self, id: Id) {
        let secs = self.content.duel.countdown_secs;
        let now = self.clock;
        let Some(d) = self.duels.iter_mut().find(|d| d.b == id && matches!(d.phase, DuelPhase::Asked { .. })) else { return };
        d.phase = DuelPhase::Counting { until: now + secs };
        let (a, b) = (d.a, d.b);
        self.events.push(Event::Duel { a, b, news: DuelNews::Counting { secs } });
    }

    /// Declined or called off before it starts, or given away once it has (the other wins).
    pub fn duel_cancel(&mut self, id: Id) {
        let Some(k) = self.duels.iter().position(|d| d.has(id)) else { return };
        if self.duels[k].phase == DuelPhase::Fighting {
            let winner = self.duels[k].other(id);
            self.end_duel(k, Some(winner), true);
        } else {
            self.end_duel(k, None, false);
        }
    }

    /// A hero leaving the world: a duel of theirs ends (given away, if it was on).
    pub(crate) fn duel_gone(&mut self, id: Id) {
        self.duel_cancel(id);
    }

    /// A blow that would fell a duellist, struck by their foe in it (or the foe's totem): they're
    /// left at 1 health and the foe wins. What the blow takes.
    pub(crate) fn duel_knockout(&mut self, id: Id, damage: u32) -> u32 {
        let Some(h) = self.players.get(&id).map(|p| p.health) else { return damage };
        if h == 0 || damage < h {
            return damage;
        }
        let by = self.players.get(&id).and_then(|p| p.cause.as_ref()).map_or(0, |c| c.by);
        let by = self.totems.iter().find(|t| t.mob == by).map_or(by, |t| t.owner);
        let Some(k) = self.duels.iter().position(|d| d.has(id) && d.phase == DuelPhase::Fighting) else { return damage };
        if self.duels[k].other(id) != by {
            return damage;
        }
        self.end_duel(k, Some(by), false);
        h - 1
    }

    pub(crate) fn step_duels(&mut self) {
        let now = self.clock;
        let def = self.content.duel.clone();
        let mut k = 0;
        while k < self.duels.len() {
            let d = self.duels[k].clone();
            let (pa, pb) = (self.players.get(&d.a), self.players.get(&d.b));
            // Either one gone or down: a duel asked or counting is off; one under way ends with
            // no winner when something else felled them (a foe's blow ends it in `duel_knockout`).
            let (Some(pa), Some(pb)) = (pa, pb) else {
                let left = if self.players.contains_key(&d.a) { d.b } else { d.a };
                let winner = (d.phase == DuelPhase::Fighting).then(|| d.other(left)).filter(|w| self.players.contains_key(w));
                self.end_duel(k, winner, true);
                continue;
            };
            if pa.health == 0 || pb.health == 0 {
                self.end_duel(k, None, false);
                continue;
            }
            match d.phase {
                DuelPhase::Asked { until } if now >= until => {
                    self.end_duel(k, None, false);
                    continue;
                }
                DuelPhase::Counting { until } if now >= until => {
                    self.duels[k].phase = DuelPhase::Fighting;
                    self.events.push(Event::Duel { a: d.a, b: d.b, news: DuelNews::Fighting });
                }
                DuelPhase::Fighting => {
                    // The flag's bounds: out too long and the duel is given away.
                    let mut lost = None;
                    for (side, (who, pos)) in [(d.a, pa.pos), (d.b, pb.pos)].into_iter().enumerate() {
                        let far = dist2d(pos, d.at);
                        match d.out[side] {
                            None if far > def.bounds => {
                                self.duels[k].out[side] = Some(now);
                                self.events.push(Event::Duel { a: d.a, b: d.b, news: DuelNews::Bounds { who, out: true, secs: def.return_secs } });
                            }
                            Some(_) if far <= def.back_in => {
                                self.duels[k].out[side] = None;
                                self.events.push(Event::Duel { a: d.a, b: d.b, news: DuelNews::Bounds { who, out: false, secs: 0.0 } });
                            }
                            Some(since) if now - since >= def.return_secs => lost = Some(who),
                            _ => {}
                        }
                    }
                    if let Some(who) = lost {
                        self.end_duel(k, Some(d.other(who)), true);
                        continue;
                    }
                }
                _ => {}
            }
            k += 1;
        }
    }

    /// A duel over: its flag down, and what each did to the other wears off (a slow, a burn, a
    /// hold), their locks on each other let go.
    fn end_duel(&mut self, k: usize, winner: Option<Id>, fled: bool) {
        let d = self.duels.remove(k);
        let started = d.phase == DuelPhase::Fighting;
        if self.objs.remove(&d.flag).is_some() {
            self.events.push(Event::ObjGone { id: d.flag });
        }
        for (me, them) in [(d.a, d.b), (d.b, d.a)] {
            if started {
                self.cleanse_from(me, them);
            }
            if let Some(p) = self.players.get_mut(&me) {
                if p.lock == Some(them) {
                    p.lock = None;
                }
            }
        }
        self.events.push(Event::Duel { a: d.a, b: d.b, news: DuelNews::Ended { started, winner: winner.filter(|_| started), fled, names: d.names } });
    }
}
