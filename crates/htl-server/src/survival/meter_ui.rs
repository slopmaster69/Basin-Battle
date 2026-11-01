//! The damage meter on screen: the hero's own numbers by source since they reset it, and their
//! fight's, everyone in it.
//!
//! - `DP\t<secs>\t<dealt>\t<healed>\t<taken>\t<source~dealt~healed>^...`: your own meter since you
//!   reset it (`.es meter reset`): seconds in fights, the totals, then each source (a card, Melee,
//!   a burn), the most first.
//! - `DM\t<secs>\t<live>\t<name~class~dealt~healed~taken~you>^...`: the fight you're in (`live`
//!   1) or were last in: you and the heroes near you fighting with you, the most damage first.
//!   Not sent before your first fight.

use std::fmt::Write as _;
use std::time::{Duration, Instant};

use htl_survival::sim::Id;

use super::play::clean;
use super::SurvivalGame;
use crate::world::ConnId;

/// How often the meters go out while they change.
pub(super) const METER_EVERY: Duration = Duration::from_millis(1000);
/// The sources a line carries at most.
const MOST_ROWS: usize = 12;

impl SurvivalGame {
    /// The meters, about once a second while they change.
    pub(super) fn ui_meters(&mut self, conn: ConnId, id: Id) {
        if let Some(dp) = self.own_meter_line(id) {
            self.ui_line_changed(conn, "DP", dp);
        }
        if let Some(dm) = self.fight_meter_line(id) {
            self.ui_line_changed(conn, "DM", dm);
        }
    }

    /// Whether the meters are due to go out again (and if so, the clock starts over).
    pub(super) fn meters_due(&mut self) -> bool {
        if self.meter_at.elapsed() < METER_EVERY {
            return false;
        }
        self.meter_at = Instant::now();
        true
    }

    fn own_meter_line(&self, id: Id) -> Option<String> {
        let m = self.sim.own_meter(id)?;
        let mut line = format!("DP\t{:.0}\t{}\t{}\t{}\t", m.secs, m.dealt, m.healed, m.taken);
        let rows: Vec<String> = m.rows.iter().take(MOST_ROWS).map(|(l, d, h)| format!("{}~{d}~{h}", clean(l))).collect();
        let _ = write!(line, "{}", rows.join("^"));
        Some(line)
    }

    fn fight_meter_line(&self, id: Id) -> Option<String> {
        let f = self.sim.fight_meter(id)?;
        let rows: Vec<String> = f
            .rows
            .iter()
            .take(MOST_ROWS)
            .map(|r| format!("{}~{}~{}~{}~{}~{}", clean(&r.name), clean(&r.class), r.dealt, r.healed, r.taken, u8::from(r.id == id)))
            .collect();
        Some(format!("DM\t{:.0}\t{}\t{}", f.secs, u8::from(f.live), rows.join("^")))
    }

    /// `.es meter reset`: your own meter starts over.
    pub(super) fn meter_command(&mut self, conn: ConnId, me: Id, arg: &str) -> Result<(), String> {
        match arg {
            "reset" => {
                self.sim.reset_meter(me);
                if let Some(dp) = self.own_meter_line(me) {
                    self.ui_line_changed(conn, "DP", dp);
                }
                Ok(())
            }
            _ => Err("Try .es meter reset.".into()),
        }
    }
}
