//! Fronts: where each road meets the defence (a bridge post, or the Ancient itself), what the
//! next wave sends to each, what's on its way now, and who said they'd hold it. Pings let players
//! claim a front or call for help.

use super::*;

/// One front, for the tracker.
#[derive(Debug, Clone, PartialEq)]
pub struct FrontView {
    /// The site: 0 the Ancient, 1.. a post.
    pub site: usize,
    /// "the north bridge".
    pub name: String,
    /// What the next wave sends here (between waves).
    pub planned: u32,
    pub planned_elites: u32,
    /// Enemies with a role worth a warning: "2 Siegebreakers".
    pub notes: Vec<String>,
    /// The boss, if it comes this way.
    pub boss: Option<String>,
    /// Enemies on their way here right now.
    pub incoming: u32,
    pub incoming_elites: u32,
    pub boss_incoming: bool,
    /// Players who said they'd hold it.
    pub holders: Vec<String>,
}

/// What a player pings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ping {
    /// "I'll hold this one."
    Hold(usize),
    /// "Help me!" where they stand.
    Help,
}

impl Sim {
    /// The site a lane's road leads through: the first post it passes (0: none, straight to the
    /// Ancient).
    pub(super) fn route_front(&self, lane: usize, route: usize) -> usize {
        let Some(points) = self.arena.lanes.get(lane).and_then(|l| l.routes.get(route).or(Some(&l.points))) else { return 0 };
        for (k, post) in self.arena.posts.iter().enumerate() {
            if points.iter().any(|p| dist2d(*p, post.pos) <= 25.0) {
                return k + 1;
            }
        }
        0
    }

    /// Every front, with the next wave's plan (between waves) and what's coming now.
    pub fn fronts(&self) -> Vec<FrontView> {
        let sites = self.site_count();
        let mut out: Vec<FrontView> = (0..sites)
            .map(|s| FrontView {
                site: s,
                name: self.site_name(s),
                planned: 0,
                planned_elites: 0,
                notes: Vec::new(),
                boss: None,
                incoming: 0,
                incoming_elites: 0,
                boss_incoming: false,
                holders: Vec::new(),
            })
            .collect();
        // The next wave's plan.
        let between = matches!(self.phase, Phase::Countdown { .. } | Phase::Intermission { .. });
        if let Some(plan) = self.next_plan.as_ref().filter(|_| between) {
            let mut roles: Vec<BTreeMap<String, u32>> = vec![BTreeMap::new(); sites];
            for sp in &plan.spawns {
                let f = sp.route.map_or(0, |r| self.route_front(sp.lane, r)).min(sites - 1);
                let def = self.enemy_def(sp.kind);
                if sp.kind.boss {
                    out[f].boss = Some(def.name.clone());
                    continue;
                }
                out[f].planned += 1;
                if sp.elite {
                    out[f].planned_elites += 1;
                }
                if !def.role.is_empty() {
                    *roles[f].entry(def.name.clone()).or_default() += 1;
                }
            }
            for (f, r) in roles.into_iter().enumerate() {
                out[f].notes = r.into_iter().map(|(name, n)| if n == 1 { format!("1 {name}") } else { format!("{n} {}", plural(&name)) }).collect();
            }
        }
        // What's on its way now: still short of its post, or (for the Ancient) everything that
        // came straight in or is past its post.
        let hub = self.arena.hub;
        for e in self.entities.values() {
            let Some(en) = e.enemy() else { continue };
            if !e.alive() || en.guard.is_some() {
                continue;
            }
            let mut f = en.front.min(sites - 1);
            if f > 0 {
                let post = &self.arena.posts[f - 1];
                if dist2d(e.pos, hub) < dist2d(post.pos, hub) - 4.0 {
                    f = 0;
                }
            }
            out[f].incoming += 1;
            if en.elite {
                out[f].incoming_elites += 1;
            }
            if en.kind.boss {
                out[f].boss_incoming = true;
            }
        }
        for id in &self.order {
            let Some(p) = self.entities.get(id).and_then(|e| e.player()) else { continue };
            if let Some(s) = p.holds.filter(|s| *s < sites) {
                out[s].holders.push(p.name.clone());
            }
        }
        out
    }

    /// A player pings: claims a front, or calls for help where they stand. The line everyone is
    /// told.
    pub fn ping(&mut self, id: EntityId, ping: Ping) -> Option<String> {
        let (name, pos) = {
            let e = self.entities.get(&id)?;
            (e.player()?.name.clone(), e.pos)
        };
        let text = match ping {
            Ping::Hold(site) => {
                if site >= self.site_count() {
                    return None;
                }
                let place = self.site_name(site);
                let p = self.entities.get_mut(&id)?.player_mut()?;
                if p.holds == Some(site) {
                    p.holds = None;
                    format!("{name} leaves {place}.")
                } else {
                    p.holds = Some(site);
                    format!("{name} will hold {place}.")
                }
            }
            Ping::Help => format!("{name} needs help {}!", self.place_name(pos)),
        };
        self.events.push(Event::Ping { player: id, text: text.clone(), help: ping == Ping::Help });
        Some(text)
    }

    /// "at the north bridge", "near the Farm": where a spot is, in words.
    pub fn place_name(&self, pos: Vec3) -> String {
        let mut best: (f32, String) = (dist2d(pos, self.arena.hub), format!("at the {}", self.content.hub.name));
        for p in &self.arena.posts {
            let d = dist2d(pos, p.pos);
            if d < best.0 {
                best = (d, format!("at {}", p.name));
            }
        }
        for b in &self.arena.bases {
            let d = dist2d(pos, b.pos) * 1.3;
            if d < best.0 {
                best = (d, format!("near {}", b.name));
            }
        }
        best.1
    }

    /// The next wave in a sentence: "Wave 6: 9 at the north bridge, 7 at the south bridge
    /// (2 Siegebreakers). Bonelord Kharos leads them over the south bridge."
    /// The special wave in play, or coming next between waves: "The Swarm".
    pub fn wave_event_name(&self) -> Option<&'static str> {
        match self.phase {
            Phase::Wave { .. } => self.run.event.map(|e| e.name()),
            _ => self.next_plan.as_ref().and_then(|p| p.event).map(|e| e.name()),
        }
    }

    pub(super) fn preview_text(&self) -> Option<String> {
        let plan = self.next_plan.as_ref()?;
        let fronts = self.fronts();
        let parts: Vec<String> = fronts
            .iter()
            .filter(|f| f.planned > 0)
            .map(|f| {
                let mut s = format!("{} {}", f.planned, if f.site == 0 { format!("straight at the {}", self.content.hub.name) } else { format!("at {}", f.name) });
                if !f.notes.is_empty() {
                    s.push_str(&format!(" ({})", f.notes.join(", ")));
                }
                s
            })
            .collect();
        let last = !self.run.won && plan.wave == self.content.game.victory_wave;
        let mut text = if last {
            format!("The final wave, {}: {}. Hold it and the run is won!", plan.wave, parts.join(", "))
        } else if let Some(e) = plan.event {
            format!("Next, wave {}, {} ({}): {}.", plan.wave, e.name(), e.text(), parts.join(", "))
        } else {
            format!("Next, wave {}: {}.", plan.wave, parts.join(", "))
        };
        if let Some(f) = fronts.iter().find(|f| f.boss.is_some()) {
            let way = if f.site == 0 { "straight at the Ancient".to_string() } else { format!("over {}", f.name) };
            text.push_str(&format!(" {} leads them {way}.", f.boss.clone().unwrap_or_default()));
        }
        Some(text)
    }
}

fn plural(name: &str) -> String {
    if name.ends_with('s') {
        name.to_string()
    } else if let Some(stem) = name.strip_suffix("man") {
        format!("{stem}men")
    } else {
        format!("{name}s")
    }
}
