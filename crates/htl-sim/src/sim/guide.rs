//! The objective tracker: what a player should be doing right now, in a few short lines. New
//! players kept asking "what do I do?", so the add-on shows this under the wave tracker.

use super::defenses::Build;
use super::*;

/// One line of the tracker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Objective {
    /// `main` (the phase's goal), `warn` (something urgent) or `tip`.
    pub kind: &'static str,
    pub title: String,
    pub detail: String,
}

fn line(kind: &'static str, title: impl Into<String>, detail: impl Into<String>) -> Objective {
    Objective { kind, title: title.into(), detail: detail.into() }
}

/// Hand casts after which the tracker stops reminding players of the keys.
const TEACH_CASTS: u32 = 8;

impl Sim {
    /// What player `id` should do now: the phase's goal first, then at most two tips.
    pub fn guidance(&self, id: EntityId) -> Vec<Objective> {
        let Some(e) = self.entities.get(&id) else { return Vec::new() };
        let Some(p) = e.player() else { return Vec::new() };
        let st = self.status();
        let hub = self.content.hub.name.clone();
        let mut out = Vec::new();

        // The goal of the moment.
        out.push(match st.phase {
            PhaseName::Waiting | PhaseName::Countdown => line(
                "main",
                "Get ready",
                if st.phase == PhaseName::Countdown {
                    format!("Wave 1 marches in {}s. Enemies come down the lanes to the {hub}: meet them there.", st.seconds_left)
                } else {
                    format!("Pick your first spell. Enemies will come down the lanes to the {hub}.")
                },
            ),
            PhaseName::Wave if !self.run.won && st.wave == self.content.game.victory_wave => line(
                "main",
                "Hold the final wave",
                format!("{} enemies left, the Archlich among them. Hold and the run is won!", st.enemies_left),
            ),
            PhaseName::Wave => line(
                "main",
                match self.run.event {
                    Some(e) => format!("Defend the {hub}: {}", e.name()),
                    None => format!("Defend the {hub}"),
                },
                if st.wave <= 2 {
                    format!("Wave {}: {} enemies left. If the {hub} falls, the run is over.", st.wave, st.enemies_left)
                } else {
                    format!("Wave {}: {} enemies left.", st.wave, st.enemies_left)
                },
            ),
            PhaseName::Intermission => line(
                "main",
                format!("Wave {} in {}s", st.wave + 1, st.seconds_left),
                if self.run.won && st.wave == self.content.game.victory_wave {
                    "Victory! The endless waves begin: every wave from here adds to your record.".to_string()
                } else {
                    let left = self.content.game.victory_wave.saturating_sub(st.wave);
                    let goal = if !self.run.won && left > 0 { format!(" {left} waves to victory.") } else { String::new() };
                    format!("A breather: spend your gold in the Shop, or push out and claim a base.{goal}")
                },
            ),
            PhaseName::Defeat => line("main", "Defeat", "A new run starts in a moment, with a new first spell."),
        });
        if st.phase == PhaseName::Defeat {
            return out;
        }

        let mut tips = Vec::new();
        if let Some(secs) = self.respawn_in(id) {
            let detail = if p.manual && p.gold > 0 {
                format!("Back on your feet at the {hub} in {}s. Spend your {} gold in the Shop meanwhile.", secs.ceil() as u32, p.gold)
            } else {
                format!("Back on your feet at the {hub} in {}s.", secs.ceil() as u32)
            };
            tips.push(line("warn", "You have fallen", detail));
        }
        let hub_pct = st.hub_health * 100 / st.hub_max_health.max(1);
        if st.phase == PhaseName::Wave && hub_pct <= 40 {
            let wrath = self.wrath_blocked().is_none() && p.gold >= self.forge_cost(super::bases::ForgeAction::Wrath);
            let detail = if wrath && p.manual {
                "Fall back and kill what's hitting it, or call its Wrath from the shop (B)!"
            } else {
                "Fall back and kill what's hitting it!"
            };
            tips.push(line("warn", format!("The {hub} is at {hub_pct}%"), detail));
        }
        // Picks need no reminder: the game pauses with the cards on screen until one is chosen.
        let spells = p.spells.iter().filter(|o| !o.talent).count();
        if p.manual && spells > 0 && p.hand_casts < TEACH_CASTS {
            const KEYS: [&str; 6] = ["1", "2", "3", "4", "Q", "E"];
            let shown = &KEYS[..spells.min(6)];
            let keys = match shown {
                [one] => one.to_string(),
                [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
                [] => String::new(),
            };
            tips.push(line(
                "tip",
                format!("Cast your spells: press {keys}"),
                "They hit your target, or the nearest enemy when you have none.",
            ));
        }
        // Caught in marked ground and never yet out of it: how strikes work.
        if p.struck >= 2 && p.dodges == 0 && st.wave <= 6 {
            tips.push(line(
                "tip",
                "Step out of the marked ground",
                "Before a big hit, the spot glows on the ground: move out before it lands.",
            ));
        }
        // The map, early on: where the waves are.
        if p.manual && st.phase == PhaseName::Wave && (2..=3).contains(&st.wave) {
            tips.push(line("tip", "Press M for the map", "It shows every enemy, your defences and your friends, live."));
        }
        // Gold to spend.
        let cheapest = [Build::Defender, Build::Protector, Build::Spring, Build::Upgrade]
            .into_iter()
            .filter_map(|b| self.build_cost(b).map(|c| (b, c)))
            .min_by_key(|(_, c)| *c);
        if let Some((what, cost)) = cheapest {
            if p.gold >= cost && st.phase != PhaseName::Wave || p.gold >= cost * 2 {
                let name = match what {
                    Build::Defender => "a Basin Defender",
                    Build::Protector => "a Treant Protector",
                    Build::Spring => "a Healing Spring",
                    Build::Rifleman => "a Basin Rifleman",
                    Build::Cleric => "a Basin Cleric",
                    Build::Upgrade => "fortified defences",
                };
                tips.push(line(
                    "tip",
                    format!("You have {} gold", p.gold),
                    format!("Go home to {} to buy {name} ({cost}g).", self.content.hub.vendor.name),
                ));
            }
        }
        // The nearest base still to take.
        let open = self
            .bases
            .iter()
            .enumerate()
            .filter(|(_, b)| matches!(b.state, BaseState::Guarded | BaseState::Cleared))
            .min_by(|a, b| {
                let da = dist2d(self.arena.bases[a.1.site].pos, e.pos);
                let db = dist2d(self.arena.bases[b.1.site].pos, e.pos);
                da.total_cmp(&db)
            });
        // Between waves only: in a wave the job is the wave.
        if let Some((_, b)) = open.filter(|_| st.phase != PhaseName::Wave) {
            let name = self.arena.bases[b.site].name.clone();
            let detail = match b.state {
                BaseState::Guarded => "Kill its guardians, then hold its banner. A claimed base stops sending enemies.",
                _ => "Its guardians are down: open the relic cache and hold the banner to claim it.",
            };
            tips.push(line("tip", format!("Claim {name}"), detail));
        }
        out.extend(tips.into_iter().take(2));
        out
    }
}
