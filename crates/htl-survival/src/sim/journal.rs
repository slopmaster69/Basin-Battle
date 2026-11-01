//! The camp's journal: goals in order (bank timber, build a fire, hold the camp...), each done
//! when the camp has done it, rewarded into the chest. It gives the loop its next step.

use super::*;

impl Survival {
    /// The goal the journal is on, if any are left.
    pub fn goal(&self) -> Option<&crate::content::GoalDef> {
        self.content.goals.get(self.camp.goal)
    }

    /// How far the camp is towards a goal: what it has, what it needs.
    pub fn goal_progress(&self, g: &crate::content::GoalDef) -> (u32, u32) {
        // (Armor worn is counted once someone is seen wearing it there.)
        let have = self.camp.counts.get(&format!("{}:{}", g.kind, g.what)).copied().unwrap_or(0);
        (have.min(g.count), g.count)
    }

    /// A Horde camp's fighters standing now, and how many it has in all (none: no such camp).
    pub fn camp_fighters(&self, camp: &str) -> Option<(u32, u32)> {
        let packs: Vec<usize> = self.content.packs.iter().enumerate().filter(|(_, p)| p.camp == camp).map(|(k, _)| k).collect();
        if packs.is_empty() {
            return None;
        }
        let of = |m: &super::Mob| m.pack.is_some_and(|p| packs.contains(&p));
        let standing = self.mobs.values().filter(|m| m.health > 0 && of(m)).count() as u32;
        let down = self.respawns.iter().filter(|r| r.2.is_some_and(|p| packs.contains(&p))).count() as u32;
        let all = standing + down;
        (all > 0).then_some((standing, all))
    }

    /// Below: the camp's journal as it stands (the goal it's on and what's been done), so a hero
    /// below sees the same goal and its count as at camp. The camp keeps it; this only shows it.
    pub fn mirror_journal(&mut self, goal: usize, counts: &BTreeMap<String, u32>) {
        if self.camp.goal != goal || &self.camp.counts != counts {
            self.camp.goal = goal;
            self.camp.counts = counts.clone();
            self.events.push(Event::Goal);
        }
    }

    /// Done goals: rewards into the chest, the journal on to the next (at camp: below, the
    /// journal is the camp's, mirrored).
    pub(crate) fn step_goals(&mut self) {
        if self.is_below() {
            return;
        }
        // Armor worn counts once seen.
        let worn: Vec<String> = self.players.values().flat_map(|p| p.armor.keys().cloned()).collect();
        for slot in worn {
            let k = format!("wear:{slot}");
            if !self.camp.counts.contains_key(&k) {
                self.camp.counts.insert(k, 1);
            }
        }
        for _ in 0..4 {
            let Some(g) = self.goal().cloned() else { return };
            let (have, need) = self.goal_progress(&g);
            if have < need {
                return;
            }
            for (k, n) in &g.reward {
                if let Some(i) = self.content.item(k) {
                    *self.storage.entry(i).or_default() += n;
                }
            }
            self.camp.goal += 1;
            let reward = if g.reward.is_empty() { String::new() } else { format!(" Into the chest: {}.", self.cost_text(&g.reward)) };
            let next = self.goal().map_or(" The journal is done: the camp stands. The raids come back, harder each time.".to_string(), |n| format!(" Next: {}.", n.title));
            // The add-on tells this one quietly by its "Done: " start (ES.Banner): keep the words.
            self.events.push(Event::Banner { player: None, title: format!("Done: {}", g.title), sub: if g.reward.is_empty() { String::new() } else { format!("Into the chest: {}", self.cost_text(&g.reward)) } });
            self.announce(format!("{} completed.{reward}{next}", g.title));
            self.events.push(Event::Storage);
            self.events.push(Event::Goal);
            self.events.push(Event::Dirty);
        }
    }
}
