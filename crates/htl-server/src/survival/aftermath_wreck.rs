//! The wreck a collapse leaves behind it (`[mob.phase.collapse] wreck`, Rhahk'Zor's): the
//! ruin's own pieces laid in each section once it is down, and the ship gone from the dry dock.
//!
//! The ruin is the cleared mine's (`compose`), fitted once from the installed data on a thread of
//! its own when the first wrecking collapse begins (it takes a second or two; the fight never
//! waits for it). A section's pieces are laid `SHOW_AFTER` seconds after it is down, only those
//! more than `BEHIND` yd behind its face: rubble to the roof stands between them and every hero,
//! so none of them can stand on the way or wall anyone in.
//!
//! The ship goes as the aftermath's does, by its marker (client patch 0087), which is all or
//! nothing: the client reloads the whole mine without the ship, and a client of patch 0091's
//! round hides the mine's own doodads and puts out their lanterns (it is sent the knocked copies
//! with it, as in the preview). A reload under a hero in a fight would hold them while the boss
//! swings, so the marker goes to each client only when its hero is out of the fight: at once for a
//! friend elsewhere in the mine (never in sight of the dock), the moment the fight ends for those
//! in it. No point of the collapse's route has sight of the dock (`rhahk_route.rs`), and the
//! dock lies behind the faces' far side, so nobody sees it go.
//!
//! A lost fight or a reset takes all of it back; a client that comes in (or comes back) is sent
//! what is down.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, TryRecvError};

use super::*;

/// A section's ruin is laid this long (game seconds) after it is down: its face is up by then.
const SHOW_AFTER: f32 = 3.0;
/// How far behind its section's face a piece of the ruin lies to be laid with it (yd).
pub(super) const BEHIND: f32 = 2.0;
/// A hero this far (yd) from the collapse's route, with no foe on them, is out of its fight.
const OUT_OF_FIGHT: f32 = 60.0;
/// The ship's berth, and round it as far as it is seen from (x and y, least and most): a hero in
/// here keeps the ship until they leave (it never vanishes before their eyes).
const DOCK: [[f32; 2]; 2] = [[-200.0, -960.0], [60.0, -700.0]];

/// The ruin as the cleared mine has it: its pieces, and the copies of the mine's own doodads.
pub(super) struct Ruin {
    pub(super) props: Vec<Prop>,
    pub(super) knocked: Vec<Prop>,
}

/// What of a wrecking collapse the clients have been sent.
#[derive(Default)]
pub(crate) struct StagedWreck {
    composing: Option<Receiver<Result<Ruin, String>>>,
    pub(super) ruin: Option<Ruin>,
    failed: bool,
    /// By section, the ruin's pieces it lays (indices into `ruin.props`), once the ruin is in.
    laid: Option<Vec<Vec<usize>>>,
    /// When each section was down (the game's clock).
    down_at: Vec<f32>,
    /// What each client has: how many sections' pieces, and whether the ship is gone for it.
    sent: BTreeMap<ConnId, (usize, bool)>,
}

/// A piece of the wreck's GUID: the ruin's entries, numbered apart from the preview's.
fn wreck_guid(index: usize, model: usize) -> u64 {
    (0xF110_u64 << 48) | (u64::from(FIRST_ENTRY + model as u32) << 24) | (0x40_0000 + index as u64)
}

/// The ruin fitted to the installed data (a thread's work).
fn compose_ruin(data: PathBuf) -> Result<Ruin, String> {
    let survey = htl_map::deadmines::load_aftermath_survey(&data).map_err(|e| e.to_string())?;
    let open = htl_map::deadmines::load_aftermath_open_survey(&data).map_err(|e| e.to_string())?;
    let displays: Vec<u32> = MODELS.iter().map(|m| m.display).collect();
    let meshes = htl_map::deadmines::aftermath::display_shapes_with(&data, &displays, doodads::EXTRA).map_err(|e| e.to_string())?;
    let mut ground = Ground::surveyed(survey, meshes.clone());
    ground.open = Some(Box::new(Ground::surveyed(open, meshes)));
    let (props, _) = compose(&ground);
    let knocked = piles::knock_over(ground.open.as_deref().unwrap(), &props).into_iter().map(|(_, p)| p).collect();
    Ok(Ruin { props, knocked })
}

/// How far a piece reaches from where it stands, however it is turned (yd).
pub(super) fn reach(p: &Prop) -> f32 {
    let m = &MODELS[p.model];
    m.min.iter().chain(&m.max).fold(0.0f32, |r, v| r.max(v.abs())) * p.scale
}

/// Which of the ruin's pieces each section lays: those more than `BEHIND` behind its face, past
/// all they reach.
pub(super) fn laid_by_section(sim: &htl_survival::Survival, props: &[Prop]) -> Vec<Vec<usize>> {
    let n = sim.collapse_wreck().map_or(0, |w| w.sections);
    let mut laid = vec![Vec::new(); n];
    for (i, p) in props.iter().enumerate() {
        if let Some(k) = sim.collapse_wreck_section(p.pos, BEHIND + reach(p)) {
            laid[k].push(i);
        }
    }
    laid
}

impl SurvivalGame {
    /// One tick of the wreck: what is down sent to whoever hasn't it, the ship to whoever is out
    /// of the fight, and all of it taken back once the collapse is (a lost fight, a reset).
    pub(crate) fn wreck_tick(&mut self) {
        let Some(view) = self.sim.collapse_wreck() else {
            self.wreck_take_back();
            return;
        };
        if self.wreck.ruin.is_none() && self.wreck.composing.is_none() && !self.wreck.failed {
            match super::lab::data_dir() {
                Some(data) => {
                    let (tx, rx) = channel();
                    std::thread::spawn(move || {
                        let _ = tx.send(compose_ruin(data));
                    });
                    self.wreck.composing = Some(rx);
                }
                None => {
                    self.wreck.failed = true;
                    println!("[wreck] no installed data: the collapse lays no ruin, and the ship stays for clients of round {}", doodads::LEVEL);
                }
            }
        }
        if let Some(rx) = &self.wreck.composing {
            match rx.try_recv() {
                Ok(Ok(ruin)) => {
                    println!("[wreck] the ruin fitted: {} pieces, {} copies of the mine's own doodads", ruin.props.len(), ruin.knocked.len());
                    self.wreck.ruin = Some(ruin);
                    self.wreck.composing = None;
                }
                Ok(Err(e)) => {
                    println!("[wreck] couldn't fit the ruin: {e}");
                    self.wreck.failed = true;
                    self.wreck.composing = None;
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    self.wreck.failed = true;
                    self.wreck.composing = None;
                }
            }
        }
        if self.wreck.laid.is_none() {
            if let Some(r) = &self.wreck.ruin {
                let laid = laid_by_section(&self.sim, &r.props);
                println!("[wreck] the ruin's pieces by section: {:?}", laid.iter().map(Vec::len).collect::<Vec<_>>());
                self.wreck.laid = Some(laid);
            }
        }
        while self.wreck.down_at.len() < view.fallen {
            self.wreck.down_at.push(self.sim.clock);
        }
        let ready = self.wreck.down_at.iter().filter(|t| self.sim.clock - **t >= SHOW_AFTER).count();
        let clients: Vec<(ConnId, Id, u8)> = self.in_world().map(|(c, _, w)| (*c, w.id, w.level)).collect();
        for (conn, id, level) in clients {
            let (had, ship) = self.wreck.sent.get(&conn).copied().unwrap_or_default();
            let mut now = (had, ship);
            if let Some(laid) = self.wreck.laid.as_ref() {
                if had < ready {
                    if had == 0 && !ship {
                        self.wreck_entries_to(conn);
                    }
                    let props = self.wreck.ruin.as_ref().map_or(&[][..], |r| &r.props[..]);
                    for k in had..ready.min(laid.len()) {
                        let at: Vec<usize> = laid[k].clone();
                        self.wreck_props_to(conn, props, &at, 0);
                    }
                    now.0 = ready;
                }
            }
            // (Without the copies, a client of 0091's round would lose the mine's doodads bare.)
            let copies = self.wreck.ruin.is_some() || level < doodads::LEVEL;
            if view.ship_gone && !ship && copies && self.wreck_out_of_fight(id, view.fight) {
                if now.0 == 0 {
                    self.wreck_entries_to(conn);
                }
                let f = go_fields(MARKER_GUID, MARKER_ENTRY, 0, [0.0; 3], 1.0, [0.0, 0.0, 0.0, 1.0], 0.0);
                self.send(conn, packets::update_object(&[Create { guid: MARKER_GUID, object_type: packets::TYPE_GAMEOBJECT,
                    is_self: false, placement: Placement::At { pos: [0.0; 3], facing: 0.0 }, fields: &f }], &[]));
                if level >= doodads::LEVEL {
                    if let Some(r) = self.wreck.ruin.as_ref() {
                        let all: Vec<usize> = (0..r.knocked.len()).collect();
                        self.wreck_props_to(conn, &r.knocked, &all, r.props.len());
                    }
                }
                now.1 = true;
            }
            if now != (had, ship) {
                self.wreck.sent.insert(conn, now);
            }
        }
    }

    /// Whether a hero can lose the ship now: no foe on them, out of the collapse's fight (or the
    /// fight over), and not where the dock is seen.
    fn wreck_out_of_fight(&self, id: Id, fight: bool) -> bool {
        let Some(p) = self.sim.players.get(&id).or_else(|| self.watchers.get(&id)) else { return false };
        let at = p.pos;
        if (0..2).all(|i| at[i] >= DOCK[0][i] && at[i] <= DOCK[1][i]) {
            return false;
        }
        if self.sim.foes_on(id) {
            return false;
        }
        !fight || self.sim.collapse_wreck_off_route(at).is_none_or(|off| off > OUT_OF_FIGHT)
    }

    /// The marker's and the ruin's entries, ahead of their objects.
    fn wreck_entries_to(&self, conn: ConnId) {
        self.send(conn, packets::gameobject_query_response(MARKER_ENTRY, GENERIC, 0, ""));
        for (i, m) in MODELS.iter().enumerate() {
            self.send(conn, packets::gameobject_query_response(FIRST_ENTRY + i as u32, GENERIC, m.display, m.name));
        }
    }

    /// The pieces `which` of `props` (numbered from `first`), in batches.
    fn wreck_props_to(&self, conn: ConnId, props: &[Prop], which: &[usize], first: usize) {
        for chunk in which.chunks(BATCH) {
            let fields: Vec<Fields> = chunk.iter().map(|i| {
                let p = &props[*i];
                go_fields(wreck_guid(first + i, p.model), FIRST_ENTRY + p.model as u32, MODELS[p.model].display, p.pos, p.scale, p.rotation, p.facing)
            }).collect();
            let creates: Vec<Create> = chunk.iter().enumerate().map(|(j, i)| Create {
                guid: wreck_guid(first + i, props[*i].model), object_type: packets::TYPE_GAMEOBJECT, is_self: false,
                placement: Placement::At { pos: props[*i].pos, facing: props[*i].facing }, fields: &fields[j],
            }).collect();
            self.send(conn, packets::update_object(&creates, &[]));
        }
    }

    /// The collapse is over without its wreck (a lost fight, a reset, the switch off): every
    /// client given any of it has it taken back, the ship with the rest.
    fn wreck_take_back(&mut self) {
        self.wreck.down_at.clear();
        let laid = self.wreck.laid.take();
        if self.wreck.sent.is_empty() {
            return;
        }
        let sent = std::mem::take(&mut self.wreck.sent);
        for (conn, (sections, ship)) in sent {
            if let (Some(r), Some(laid)) = (self.wreck.ruin.as_ref(), laid.as_ref()) {
                for k in 0..sections.min(laid.len()) {
                    for i in &laid[k] {
                        self.send(conn, packets::destroy_object(wreck_guid(*i, r.props[*i].model)));
                    }
                }
            }
            if ship {
                // (The preview's own marker is its own: never taken from under it.)
                if self.aftermath.is_none() {
                    self.send(conn, packets::destroy_object(MARKER_GUID));
                }
                if let Some(r) = self.wreck.ruin.as_ref() {
                    for (i, p) in r.knocked.iter().enumerate() {
                        self.send(conn, packets::destroy_object(wreck_guid(r.props.len() + i, p.model)));
                    }
                }
            }
        }
    }

    /// A client sent its world again (it came in, or back): it has none of the wreck now.
    pub(crate) fn wreck_forget(&mut self, conn: ConnId) {
        self.wreck.sent.remove(&conn);
    }
}

#[cfg(test)]
#[path = "aftermath_wreck_tests.rs"]
mod tests;
