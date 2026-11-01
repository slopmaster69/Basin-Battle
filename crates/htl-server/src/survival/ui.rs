//! The survival add-on's side of things: its lines (`ES\t...` add-on chat), the HUD it draws, and
//! the commands it (or a hero typing) sends as `.es ...` and plain `.` commands.
//!
//! Lines to the add-on:
//! - `V\t<version>`: the server answers the add-on's hello (the add-on wakes only then).
//! - `EF\t<name~seconds left~kind>^...`: what used items are still doing for the hero.
//! - `GL\t<place>\t<goals>\t<title>\t<what to do>\t<have>\t<need>`: the camp journal's goal.
//! - `H\t<health>\t<max>\t<stamina>\t<max>\t<kit>\t<weapon>\t<bags used>\t<bag slots>\t<cache yd>\t<cache bearing>\t<raid>\t<raid secs>\t<raiders left>\t<at camp>\t<foes in reach>\t<meals>\t<foe name>\t<foe health>\t<foe max>\t<ranged>\t<light clip>\t<light impact>\t<light secs>\t<heavy clip>\t<heavy impact>\t<heavy secs>\t<light stamina>\t<heavy stamina>\t<dodge stamina>\t<quick item>\t<its count>\t<armor %>\t<the raid's name>\t<sheathed>\t<jump stamina>`
//! - `A\t<action>\t<seconds in>\t<seconds long>`: what the hero is doing, for the action bar.
//! - `S\t<item~name~count~gear>^...`: the camp chest, while its window is open.
//! - `W\t<station>\t<name>\t<recipe~output~cost~can>^...`: a station's work.
//! - `BM\t<key~name~cost~can~built~text>^...`: the build menu.
//! - `AR\t<key~name~kind~held~text>^...`: the weapon rack.
//! - `FL\t<text>`: a word flashed big (a parry).
//! - `BP\t<name>\t<cost>\t<problem>\t<x~y~z~yards>`: the station being placed and where its ghost stands (`BP`
//!   alone: none).
//! - `CF\t<what>\t<text>`: confirm something (ringing the bell).
//! - `BN\t<title>\t<sub>`: a big banner. `T\t<text>`: a short message. `DE\t<secs>`: down, up in.
//! - `WD`: too winded for what was tried (the stamina bar flashes).
//! - `KF\t<killer>\t<victim>`: the kill feed (the killer empty when nobody struck them).
//! - `HT\t<damage>\t<light|heavy|arrow>\t<hit|block|parry|dodge|miss>`: a blow of yours landed.
//! - `WR\t<camp|below>`: which world the hero is in (below: no building, Draughts for food).
//! - `BK\t<open>\t<key~name~dungeon^...>\t<arena key~name~best wave>\t<siege>\t<dungeons>`: the
//!   Smuggler's Logbook at camp, its pages (lamps below, each with its dungeon's key), the Proving
//!   Ring's and Siege's (there from the start), then each dungeon
//!   (`key~name~icon~open~gate~lamps~pages~blurb~first page`, joined by `^`).
//! - `AA\t<wave>\t<foes standing>\t<secs to the next, or -1 in a wave>\t<the camp's best>`: the
//!   Proving Ring as a hero in it sees it (`AA` alone: not in it).
//! - `AW\t<wave>\t<in a wave>\t<secs to the next>\t<the camp's best>\t<heroes in it>`: the ring's
//!   banner, used.
//! - `LP\t<key>\t<name>\t<draughts>\t<most>\t<key~name^...>\t<key~name~price~can~icon^...>\t<plunder>`:
//!   a lamp below: the hero's Draughts, the other lit lamps, the quartermaster's wares.
//!
//! To the action-combat client that plays clips itself (`ac2`), in place of emotes:
//! - `AN\t<guid>\t<clip>\t<impact secs>\t<trail AARRGGBB>\t<trail ms>\t<mine>`: a hero swings.
//!   A creature's wind-up is the same line with kind `mob`; to `ac9` it carries a trail (amber: a
//!   guard stops it, red: it doesn't) that the client starts `trail ms` before the impact.
//! - `IM\t<attacker>\t<victim>\t<light|heavy|arrow>\t<outcome>\t<you struck>\t<you were struck>`:
//!   a blow lands (hit-stop, shake).
//! - `RO\t<guid>\t<secs>\t<mine>`: a hero rolls (a dodge).
//!
//! And to the third round (`ac3`), which runs its own dashes and lunges:
//! - `MV\t<two-handed>\t<chain after>\t<cancel after>\t<heavy cancel after>\t<combo grace>\t<lunge secs>\t<dodge secs>\t<dodge speed>\t<dash secs>\t<light blows ^>\t<heavy>\t<running>\t<bash>\t<kick>\t<friendly fire>`:
//!   the hero's blows, each `clip~take~impact~recovery~lunge~stamina~range~arc`.
//! - `AN` goes on `\t<take>\t<recovery>\t<lunge>\t<kind>\t<step>`.
//!   The kind: `light`, `heavy`, `running`, `bash`, `kick`, `cast`, a creature's `mob`, and the
//!   attacks out of a dodge (`roll`) and out of a Corey roll's landing (`leap`).
//! - `ST\t<guid>\t<mine>`: a swing called off (a feint), its clip let go.
//! - `TG\t<key>\t<creature guid>\t<wedge|ring>\t<reach>\t<arc>\t<secs>\t<block|unblockable>\t<parry window>\t<meant for you>`: a creature's blow winding up, drawn under it (`TG\t<key>`: gone).
//! - `RE\t<guid>\t<anim>\t<dx>\t<dy>\t<lean>\t<hold>\t<mine>\t<fit>`: a unit takes a blow (its reaction,
//!   the lean back along `(dx, dy)`, held `hold` seconds for a stagger, or the whole clip fitted to
//!   `fit` seconds: a knockdown).
//! - `RO` goes on `\t<style>`: `roll`, `left`, `right` or `back`.
//! - A creature's swing comes as an `AN` with its wind-up (`mob` its kind).
//! - `LO\t<guid>`: the hero's lock-on (`0x0`: none). `PI\t<n>`: a ping, answered `.es po <n>`.
//!
//! The abilities:
//! - `AB\t...`: the ability bar, one entry per key (see [`SurvivalGame::bar_line`]).
//! - `VA\t<valor>\t<most>\t<segment>`; `CC\t<kind~secs left~share>^...`: the effects on the hero.
//! - `TR\t<dev>\t<count>\t<card...>^...`: the board (N), four rows of four cards (see
//!   [`SurvivalGame::tree_line`]).
//! - `MK\t<id>\t<x>\t<y>\t<z>\t<radius>\t<secs>\t<friendly>`: ground marked (holy ground is a
//!   friend's), `MK\t<id>` gone.
//! - `DA\t<secs>\t<speed>\t<facing>`: run the dash of an ability (Charge) along the facing.
//! - `HL\t<amount>`: healed (a green number). `NP\t<verb>\t<name>`: what the use key would use.
//! - `A` goes on `\t<ability key>` (what's aimed, charged, cast or channelled).
//! - `RC\t<slot>\t<secs>\t<icon>\t<key>\t<name>\t<shape>\t<range>\t<radius>\t<arc>\t<aimed>\t<least charge>\t<full charge>`:
//!   the key in that slot (0 Q to 4 the ultimate) is armed for its recast for `secs` more: pressed
//!   again it fires that ability (its icon, its name, its aim and charge); `RC\t<slot>`: closed.
//! - `DP`, `DM`: the damage meters (see `meter_ui.rs`).
//! - `EM\t<slot>\t<secs>`: the key in that slot is empowered (a combo) for `secs` more: its next
//!   press is its EX version, for no Valor; `EM\t<slot>`: spent or gone.

use std::fmt::Write as _;

use htl_survival::{Action, Id, Input, ItemKind, Kit, ObjKind, RaidPhase};

/// A slot by its key (`q`, `e`, `1`, `2`, `3`) or its own name (`ranged`...).
fn slot_of(s: &str) -> &str {
    match s {
        "q" => "ranged",
        "e" => "close",
        "1" => "utility",
        "2" => "defense",
        "3" => "ultimate",
        other => other,
    }
}

/// What a card does in numbers, for its tooltip: damage, healing, reach, charge, wind-up,
/// stamina, cooldown, Valor (`·` between).
pub(crate) fn ability_facts(d: &htl_survival::content::AbilityDef, base: Option<&htl_survival::content::AbilityDef>, valor_max: f32) -> String {
    let n = |v: f32| -> String {
        if (v - v.round()).abs() < 0.05 {
            format!("{}", v.round() as i64)
        } else {
            format!("{v:.1}")
        }
    };
    let span = |r: [f32; 2], unit: &str| -> String {
        if (r[1] - r[0]).abs() < 0.01 {
            format!("{}{unit}", n(r[0]))
        } else {
            format!("{}–{}{unit}", n(r[0]), n(r[1]))
        }
    };
    let mut out: Vec<String> = Vec::new();
    if d.kind == "art" {
        out.push(format!("{}% weapon damage", n(d.weapon * 100.0)));
        if d.behind > 1.0 {
            out.push(format!("×{} from behind", n(d.behind)));
        }
        // A weapon art is a swing: it takes stamina as well (said, as it's easy to miss).
        if d.stamina > 0.0 {
            out.push(format!("{} stamina", n(d.stamina)));
        }
    } else if d.damage[1] > 0.0 {
        let count = if d.count > 1 { format!("{} × ", d.count) } else { String::new() };
        // A beam's is a second in it.
        let per = if d.kind == "beam" { " a second" } else { "" };
        out.push(format!("{count}{} damage{per}", span(d.damage, "")));
    }
    if d.linger > 0.0 && d.linger_damage > 0.0 {
        out.push(format!("{} a second for {} s", n(d.linger_damage), n(d.linger)));
    } else if d.linger > 0.0 {
        out.push(format!("{} s", n(d.linger)));
    }
    if d.ally_heal > 0.0 && d.kind == "beam" {
        out.push(format!("heals friends in it {}% a second", n(d.ally_heal * 100.0)));
    } else if d.ally_heal > 0.0 {
        out.push(format!("heals {}%", n(d.ally_heal * 100.0)));
    }
    if d.linger_heal > 0.0 {
        out.push(format!("heals {}% a second", n(d.linger_heal * 100.0)));
    }
    if d.area_heal > 0.0 {
        out.push(format!("heals {}%", n(d.area_heal * 100.0)));
    }
    match d.kind.as_str() {
        "nova" => out.push(format!("{} yd around you", n(d.radius[1]))),
        "ground" if d.at_self => out.push(format!("{} yd around you", n(d.radius[1]))),
        "ground" => out.push(format!("{} yd away, {} yd wide", n(d.range[1]), n(d.radius[1] * 2.0))),
        "cone" => out.push(format!("{} yd cone", n(d.range[1]))),
        "line" => out.push(format!("{} yd strip, {} yd wide", n(d.range[1]), n(d.radius[1] * 2.0))),
        "beam" => out.push(format!("{} yd beam, {} yd wide, held {} s", n(d.range[1]), n(d.radius[1] * 2.0), n(d.secs))),
        "ally" => out.push("on an ally, or yourself".to_string()),
        "leap" if d.back => out.push(format!("{} yd back", n(d.range[1]))),
        "leap" if d.radius[1] > 0.0 => out.push(format!("{} yd leap, lands {} yd wide", n(d.range[1]), n(d.radius[1] * 2.0))),
        "trap" => out.push(format!("{} yd away, springs within {} yd, lasts {} s", n(d.range[1]), n(d.radius[1]), n(d.secs))),
        "place" => out.push(format!("totem {} s, {} yd", n(d.secs), n(d.totem.as_ref().map_or(0.0, |t| t.radius)))),
        "blink" if d.anchor > 0.0 => out.push(format!("circle lasts {} s", n(d.anchor))),
        "art" if d.ambush > 0.0 => out.push(format!("{} yd step", n(d.ambush))),
        "art" | "buff" | "stance" => {}
        _ if d.range[1] > 0.0 => out.push(span(d.range, " yd")),
        _ => {}
    }
    if d.delay > 0.0 && matches!(d.kind.as_str(), "ground" | "line") {
        out.push(format!("lands after {} s", n(d.delay)));
    }
    if d.cast > 0.0 {
        out.push(format!("{} s wind-up", n(d.cast)));
    }
    if d.pierce > 0 {
        out.push(format!("through {} more", d.pierce));
    }
    if d.chain > 0 {
        out.push(format!("leaps on {}×", d.chain));
    }
    if d.splash > 0.0 && d.kind != "trap" {
        out.push(format!("{} yd burst", n(d.splash)));
    }
    if d.returns {
        out.push("comes back".to_string());
    }
    if d.lob {
        out.push("lobbed over foes".to_string());
    }
    if d.fuse > 0.0 {
        out.push(format!("sticks, goes off after {} s", n(d.fuse)));
    }
    for e in d.effects.iter().chain(&d.tick_effects) {
        match e.kind.as_str() {
            "knockback" => out.push(format!("knocks back {} yd", n(e.amount))),
            "pull" => out.push(if e.from_caster { "pulls to you".to_string() } else { format!("pulls {} yd in", n(e.amount)) }),
            _ => {}
        }
    }
    if d.kind == "stance" && d.secs > 0.0 {
        out.push(format!("{} s", n(d.secs)));
    }
    if d.kind == "buff" || d.kind == "ally" {
        if let Some(e) = d.effects.first() {
            out.push(format!("{} s", n(e.secs[1])));
        }
    }
    if d.charge[1] > 0.0 {
        if (d.charge[1] - d.charge[0]).abs() < 0.01 {
            out.push(format!("{} s charge", n(d.charge[1])));
        } else {
            out.push(format!("charge up to {} s", n(d.charge[1])));
        }
        if d.full_spell != 0 && d.full_speed > d.speed && d.speed > 0.0 {
            out.push(format!("full: {}× as fast", n(d.full_speed / d.speed)));
        }
    }
    if d.kind == "art" && d.windup > 0.0 {
        out.push(format!("{} s wind-up", n(d.windup)));
    }
    if d.stamina > 0.0 {
        out.push(format!("{} stamina", n(d.stamina)));
    }
    if d.through {
        out.push("runs through foes".to_string());
    }
    if !d.recast.is_empty() && d.recast_secs > 0.0 {
        out.push(format!("press again within {} s", n(d.recast_secs)));
    }
    if let Some(e) = &d.empower {
        out.push(format!("empowers for {} s", n(e.secs)));
    }
    // An EX version runs on its base's cooldown.
    let cd = base.map_or(d.cooldown, |b| b.cooldown);
    if base.is_none() && cd > 0.0 {
        out.push(format!("{} s cooldown", n(cd)));
    }
    if base.is_none() && d.charges > 1 {
        out.push(format!("{} charges", d.charges));
    }
    // (An EX version's Valor is shown beside its name.)
    if d.slot == "ultimate" {
        out.push(if d.valor as f32 >= valor_max { "the whole Valor bar".to_string() } else { format!("{} Valor", d.valor) });
    } else if d.valor > 0 && base.is_none() {
        out.push(format!("{} Valor", d.valor));
    }
    out.join(" · ")
}

use super::play::clean;
use super::*;

/// The controls in a sentence, for chat (the add-on's Keys card has them all).
pub(crate) const CONTROLS_TEXT: &str = "WASD moves. Left click attacks (hold it, or Shift+click, for a heavy attack); \
    right click raises your guard (just as a blow lands: a parry); mouse button 4 dodges (hold it to sprint). \
    Abilities: Q (ranged), E (close), 1 (utility), 2 (defense), 3 (ultimate: hold it); hold Q, E, 1 \
    or 2 a moment (or Ctrl with it) for its charged-up EX version; an aimed one raises the reticle, \
    left click charges or casts it, right click or a roll calls it off. F uses, Space jumps, R puts \
    your weapons away, T uses the quick item (G picks the next), X walks, B opens the bags, H \
    builds, N the board, V the health bars over heads. Tab locks on (Tab again lets go; Ctrl+Tab: the \
    next foe). K shows every key and changes them, and the mouse speed.";

pub(crate) const PREFIX: &str = "ES";
pub(crate) const VERSION: u32 = 2;
const HUD_EVERY: Duration = Duration::from_millis(150);
/// A champion's bar shows to the heroes this near it while it fights.
const BOSS_BAR_RANGE: f32 = 60.0;
const PING_EVERY: Duration = Duration::from_secs(2);
/// What was last sent of each key's recast window and empower (see [`SurvivalGame::combo_lines`]).
const RC_SENT: [&str; 5] = ["RC0", "RC1", "RC2", "RC3", "RC4"];
const EM_SENT: [&str; 5] = ["EM0", "EM1", "EM2", "EM3", "EM4"];
/// The corner's arrow points to a place not yet found this near (yards), when there are no
/// supplies to point to.
const UNFOUND_HINT: f32 = 600.0;

impl SurvivalGame {
    pub(crate) fn ui_line(&self, conn: ConnId, line: &str) {
        let Some(guid) = self.world_of(conn).map(|w| w.guid) else { return };
        let text = format!("{PREFIX}\t{line}");
        self.send(conn, packets::chat(packets::CHAT_PARTY, packets::LANG_ADDON, guid, &text));
    }

    /// A line to the add-on only when it differs from the last of its kind.
    pub(super) fn ui_line_changed(&mut self, conn: ConnId, verb: &'static str, line: String) {
        let same = self.world_of(conn).is_some_and(|w| w.sent.get(verb) == Some(&line));
        if same {
            return;
        }
        self.ui_line(conn, &line);
        if let Some(w) = self.world_mut(conn) {
            w.sent.insert(verb, line);
        }
    }

    pub(crate) fn has_addon(&self, conn: ConnId) -> bool {
        self.world_of(conn).is_some_and(|w| w.addon)
    }

    /// A message for one hero: on screen with the add-on (the chat stays for talking), in chat
    /// without it.
    pub(crate) fn tell(&self, conn: ConnId, text: &str) {
        if self.has_addon(conn) {
            self.ui_line(conn, &format!("T\t{}", clean(text)));
        } else {
            self.say_to(conn, text);
        }
    }

    // --- the HUD -------------------------------------------------------------------------------------

    /// What the hero is doing, how far in, and how long it lasts.
    fn action_line(&self, id: Id) -> Option<(String, f32)> {
        let p = self.sim.players.get(&id)?;
        let c = &self.sim.content;
        let now = self.sim.clock;
        let (elapsed, total) = match &p.action {
            Action::Swing { t, strike, .. } => (*t, strike.total()),
            Action::Dodge { t, .. } => (*t, c.dodge.secs),
            Action::Charge { since, .. } => (now - since, 0.0),
            Action::Cast { t, total, .. } => (*t, *total),
            Action::Channel { until, ability, .. } => {
                let secs = c.abilities[*ability].secs;
                (secs - (until - now), secs)
            }
            Action::Stance { until, ability } => {
                let secs = c.abilities[*ability].secs;
                (secs - (until - now), secs)
            }
            Action::Dash { t, secs, .. } => (*t, *secs),
            Action::Eat { t, item } => (*t, c.items[*item].use_secs),
            Action::Gather { t, node, .. } => {
                let secs = match self.sim.objs.get(node).map(|o| &o.kind) {
                    Some(ObjKind::Node { def, .. }) => c.nodes[*def].gather_secs,
                    _ => 1.0,
                };
                (*t, secs)
            }
            Action::Work { t, total, .. } => (*t, *total),
            Action::Stagger { until } => (0.0, (until - now).max(0.0)),
            Action::Dead { until } => (0.0, (until - now).max(0.0)),
            _ => (0.0, 0.0),
        };
        // Started at: what tells one swing from the next.
        let started = now - elapsed;
        // What's aimed or charged: its key, so the bar and the reticle show it.
        let what = match &p.action {
            Action::Aim { ability } | Action::Charge { ability, .. } | Action::Cast { ability, .. } | Action::Channel { ability, .. } | Action::Stance { ability, .. } | Action::Dash { ability, .. } => c.abilities[*ability].key.as_str(),
            // Work: what it makes, builds or mends, by name.
            Action::Work { job, .. } => match job {
                htl_survival::Job::Craft { recipe, .. } => c.recipes.iter().find(|r| &r.key == recipe).and_then(|r| c.item(&r.output)).map_or("", |i| c.items[i].name.as_str()),
                htl_survival::Job::Build => p.preview.as_ref().map_or("", |pv| c.stations[pv.def].name.as_str()),
                htl_survival::Job::Repair { station } => match self.sim.objs.get(station).map(|o| &o.kind) {
                    Some(ObjKind::Station { def, .. }) => c.stations[*def].name.as_str(),
                    _ => "",
                },
            },
            _ => "",
        };
        Some((format!("A\t{}\t{elapsed:.2}\t{total:.2}\t{what}", p.action.name()), started))
    }

    fn hud_line(&self, id: Id) -> Option<String> {
        let p = self.sim.players.get(&id)?;
        let c = &self.sim.content;
        let kit = if p.kit == Kit::Bow { "bow" } else { "melee" };
        let weapon = if p.kit == Kit::Bow { &c.items[p.bow].name } else { &c.items[p.weapon].name };
        let used = self.sim.bag_slots_used_by(p);
        // The nearest of what this hero left where they fell: how far, and which way.
        let cache = self
            .sim
            .caches_of(&p.account)
            .into_iter()
            .map(|(_, at)| (htl_survival::dist2d(p.pos, at), htl_survival::facing(p.pos, at)))
            .min_by(|a, b| a.0.total_cmp(&b.0));
        let (cd, cb) = match cache {
            Some((d, f)) => {
                let mut rel = (f - p.facing).to_degrees();
                while rel > 180.0 {
                    rel -= 360.0;
                }
                while rel < -180.0 {
                    rel += 360.0;
                }
                (d.round() as i32, rel.round() as i32)
            }
            None => (-1, 0),
        };
        // The nearest of Elwynn's places the camp hasn't found, not too far: its name, how far,
        // which way (for the corner's arrow when there are no supplies to point to).
        let rel = |f: f32| {
            let mut r = (f - p.facing).to_degrees();
            while r > 180.0 {
                r -= 360.0;
            }
            while r < -180.0 {
                r += 360.0;
            }
            r.round() as i32
        };
        let unfound = self
            .sim
            .sites_view()
            .into_iter()
            .filter(|(_, found)| !found)
            .map(|(s, _)| (s, [s.at[0], s.at[1], 0.0]))
            .map(|(s, at)| (s, htl_survival::dist2d(p.pos, at), htl_survival::facing(p.pos, at)))
            .filter(|(_, d, _)| *d <= UNFOUND_HINT)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        let (pn, pd, pb) = match unfound {
            Some((s, d, f)) => (clean(&s.name), (d - s.radius).max(0.0).round() as i32, rel(f)),
            None => (String::new(), -1, 0),
        };
        // Which way north is (facing 0 is north): the corner's arrow below, with nothing else to
        // point to.
        let north = rel(0.0);
        let raid = &self.sim.raid;
        let phase = match raid.phase {
            RaidPhase::Idle => "idle",
            RaidPhase::Warning => "warning",
            RaidPhase::Active => "active",
        };
        let left = raid.queue.len() + raid.raiders.iter().filter(|r| self.sim.mobs.get(r).is_some_and(|m| m.health > 0)).count();
        let heart = c.world.heart;
        let home = htl_survival::dist2d(p.pos, [heart[0], heart[1], 0.0]) <= c.world.rest_radius;
        let reach = if p.kit == Kit::Melee { self.sim.foes_in_reach(id) } else { 0 };
        let meals = self.sim.food_count(id);
        let (foe, foe_hp, foe_max) = self.sim.foe_view(id).map_or((String::new(), 0, 0), |(n, h, m)| (clean(&n), h, m));
        let ranged = if self.sim.ranged_of(p).draw_secs <= 0.0 { "crossbow" } else { "bow" };
        // The swings' clips and timings, for the client's own instant play.
        let w = self.sim.weapon_of(p);
        let (lw, la, lr) = w.times(None);
        let (hw, ha, hr) = w.times(Some(&c.heavy));
        let swings = format!(
            "{}\t{lw:.2}\t{:.2}\t{}\t{hw:.2}\t{:.2}\t{:.0}\t{:.0}\t{:.0}",
            w.swing_clip(false),
            lw + la + lr,
            w.swing_clip(true),
            hw + ha + hr,
            w.stamina,
            w.stamina * c.heavy.stamina,
            c.dodge.needs()
        );
        // The item R uses, how many, and how much the armor worn takes off a blow.
        let (quick, quick_n) = self.sim.quick_item(p).map_or((String::new(), 0), |i| (clean(&c.items[i].name), p.bag.get(&i).copied().unwrap_or(0)));
        let armor = (self.sim.armor_reduce(p) * 100.0).round() as u32;
        let raid_title = clean(&self.sim.current_raid().title);
        Some(format!(
            "H\t{}\t{}\t{}\t{}\t{kit}\t{}\t{used}\t{}\t{cd}\t{cb}\t{phase}\t{}\t{left}\t{}\t{reach}\t{meals}\t{foe}\t{foe_hp}\t{foe_max}\t{ranged}\t{swings}\t{quick}\t{quick_n}\t{armor}\t{raid_title}\t{}\t{:.0}\t{pn}\t{pd}\t{pb}\t{north}",
            p.health,
            p.max_health,
            p.stamina.floor() as u32,
            c.player.stamina as u32,
            clean(weapon),
            c.player.bag_slots,
            raid.timer.max(0.0).ceil() as u32,
            u8::from(home),
            u8::from(p.sheathed),
            c.player.jump_stamina
        ))
    }

    /// The journal's goal: `GL\t<place>\t<goals>\t<title>\t<what to do>\t<have>\t<need>`
    /// (the title empty once they're all done).
    fn goal_line(&self) -> String {
        let total = self.sim.content.goals.len();
        match self.sim.goal() {
            Some(g) => {
                let (mut have, mut need) = self.sim.goal_progress(g);
                // A camp to take: how many of its fighters are down, of all of them, as it goes.
                if g.kind == "clear" && have < need {
                    if let Some((left, all)) = self.sim.camp_fighters(&g.what) {
                        (have, need) = (all - left, all);
                    }
                }
                format!("GL\t{}\t{total}\t{}\t{}\t{have}\t{need}", self.sim.camp.goal + 1, clean(&g.title), clean(&g.text))
            }
            None => format!("GL\t{}\t{total}\t\t\t0\t0", total + 1),
        }
    }

    /// The whole journal: `GJ\t<title~state~what to do~reward>^...`, state 0 done, 1 the one
    /// it's on, 2 still ahead.
    fn journal_line(&self) -> String {
        let at = self.sim.camp.goal;
        let list: Vec<String> = self
            .sim
            .content
            .goals
            .iter()
            .enumerate()
            .map(|(k, g)| {
                let state = match k.cmp(&at) {
                    std::cmp::Ordering::Less => 0,
                    std::cmp::Ordering::Equal => 1,
                    std::cmp::Ordering::Greater => 2,
                };
                format!("{}~{state}~{}~{}", clean(&g.title), clean(&g.text), clean(&self.sim.cost_text(&g.reward)))
            })
            .collect();
        format!("GJ\t{}", list.join("^"))
    }

    /// Elwynn's places for the world map: `PM\t<found>~<places>~<chests opened>~<chests>~<secrets
    /// found>~<secrets>\t<key~x~y~kind~found~name~what's there~icon>^...`.
    fn pins_line(&self) -> String {
        let (open, all, sf, secrets) = self.sim.chests_tally();
        let sites = self.sim.sites_view();
        let found = sites.iter().filter(|(_, f)| *f).count();
        let list: Vec<String> = sites
            .iter()
            .map(|(s, f)| format!("{}~{:.0}~{:.0}~{}~{}~{}~{}~{}", s.key, s.at[0], s.at[1], s.kind, u8::from(*f), clean(&s.name), clean(&s.text), clean(&s.icon)))
            .collect();
        format!("PM\t{found}~{}~{open}~{all}~{sf}~{secrets}\t{}", sites.len(), list.join("^"))
    }

    /// A champion fighting near the hero, for the bar across the bottom of the screen:
    /// `BS\t<name>\t<health>\t<max>\t<enraged>` (just `BS` with none).
    fn boss_line(&self, id: Id) -> Option<String> {
        let p = self.sim.players.get(&id)?;
        let c = &self.sim.content;
        let near = self
            .sim
            .mobs
            .values()
            .filter(|m| c.mobs[m.def].boss && m.health > 0 && m.state.foe().is_some())
            .map(|m| (htl_survival::dist2d(m.pos, p.pos), m))
            .filter(|(d, _)| *d < BOSS_BAR_RANGE)
            .min_by(|a, b| a.0.total_cmp(&b.0));
        Some(match near {
            Some((_, m)) => format!("BS\t{}\t{}\t{}\t{}", clean(&c.mobs[m.def].name), m.health, m.max_health, u8::from(m.enraged)),
            None => "BS".to_string(),
        })
    }

    /// What used items are still doing for the hero: `EF\t<name~seconds left~kind>^...`.
    fn effects_line(&self, id: Id) -> Option<String> {
        let p = self.sim.players.get(&id)?;
        let now = self.sim.clock;
        let list: Vec<String> = p.effects.iter().filter(|e| e.until > now).map(|e| format!("{}~{}~{}", clean(&e.name), (e.until - now).ceil() as u32, e.kind)).collect();
        Some(format!("EF\t{}", list.join("^")))
    }

    /// The hero's blows, for the add-on's own instant play: the weapon's grip, the chain and
    /// cancel windows, the dodge, then each blow as `clip~take~impact~recovery~lunge~stamina`
    /// (the light chain `^`-separated), the heavy, running, bash and kick.
    fn moves_line(&self, id: Id) -> Option<String> {
        let p = self.sim.players.get(&id)?;
        let c = &self.sim.content;
        let w = self.sim.weapon_of(p);
        let set = c.moveset_of(w);
        let mv = |m: &htl_survival::Strike| {
            let impact = m.windup + m.active * 0.5;
            format!("{}~{}~{impact:.3}~{:.3}~{:.2}~{:.1}~{:.2}~{:.0}", m.clip, m.take, m.total() - impact, m.lunge, m.stamina, m.range, m.arc)
        };
        let light: Vec<String> = (0..set.light.len()).map(|k| mv(&self.sim.strike_at(id, htl_survival::StrikeKind::Light, k))).collect();
        let heavy = mv(&self.sim.strike_at(id, htl_survival::StrikeKind::Heavy, 0));
        let running = mv(&self.sim.strike_at(id, htl_survival::StrikeKind::Running, 0));
        let shove = |d: &htl_survival::content::ShoveDef| {
            let impact = d.windup + d.active * 0.5;
            format!("{}~0~{impact:.3}~{:.3}~{:.2}~{:.1}~{:.2}~{:.0}", d.clip, d.windup + d.active + d.recovery - impact, d.lunge, d.stamina, d.range, d.arc)
        };
        let k = &c.combat;
        Some(format!(
            "MV\t{}\t{:.2}\t{:.2}\t{:.2}\t{:.2}\t{:.2}\t{:.2}\t{:.1}\t{:.2}\t{}\t{heavy}\t{running}\t{}\t{}\t{}",
            u8::from(w.two_handed),
            k.chain_after,
            k.cancel_after,
            k.heavy_cancel_after,
            k.combo_grace,
            k.lunge_secs,
            c.dodge.secs,
            c.dodge.speed,
            c.dodge.dash_secs,
            light.join("^"),
            shove(&c.bash),
            shove(&c.kick),
            u8::from(c.pvp.friendly_fire),
        ))
    }

    fn placement_line(&self, id: Id) -> String {
        match self.sim.placement(id) {
            Some(pl) => {
                // Where it would go, for the ring the add-on draws round it (to a tenth of a yard,
                // so the line changes only as the spot does).
                let at = pl.at.map_or(String::new(), |a| format!("{:.1}~{:.1}~{:.1}~{:.1}", a[0], a[1], a[2], pl.size));
                format!("BP\t{}\t{}\t{}\t{at}", clean(&pl.name), clean(&pl.cost), clean(pl.problem.as_deref().unwrap_or("")))
            }
            None => "BP".to_string(),
        }
    }

    /// Each tick: what changed on every add-on's screen.
    pub(crate) fn ui_flush(&mut self) {
        let hud_due = self.hud_at.elapsed() >= HUD_EVERY;
        if hud_due {
            self.hud_at = Instant::now();
        }
        let meters_due = self.meters_due();
        let storage = std::mem::take(&mut self.storage_dirty);
        let chest = self.sim.storage_id();
        let conns: Vec<(ConnId, Id, Option<Id>)> = self.in_world().filter(|(_, _, w)| w.addon).map(|(c, _, w)| (*c, w.id, w.station)).collect();
        // A cooldown coming back changes the bar too.
        let now = self.sim.clock;
        for (_, id, _) in &conns {
            if let Some(p) = self.sim.players.get(id) {
                if p.cooldowns.iter().any(|(a, cd)| cd.charges < self.sim.content.abilities[*a].charges.max(1) && cd.next_at <= now) {
                    self.abilities_dirty.insert(*id);
                }
            }
        }
        for (conn, id, station) in conns {
            if let Some((line, started)) = self.action_line(id) {
                // A new action, or the same kind started again.
                let key = format!("{}@{:.1}", line.split('\t').nth(1).unwrap_or(""), started);
                let same = self.world_of(conn).is_some_and(|w| w.sent.get("A") == Some(&key));
                if !same {
                    self.ui_line(conn, &line);
                    if let Some(w) = self.world_mut(conn) {
                        w.sent.insert("A", key);
                    }
                }
            }
            if hud_due {
                if let Some(n) = self.healed.remove(&id).filter(|n| *n > 0) {
                    self.ui_line(conn, &format!("HL\t{n}"));
                }
                if let Some(h) = self.hud_line(id) {
                    self.ui_line_changed(conn, "H", h);
                }
                if let Some(mv) = self.moves_line(id) {
                    self.ui_line_changed(conn, "MV", mv);
                }
                if let Some(ef) = self.effects_line(id) {
                    self.ui_line_changed(conn, "EF", ef);
                }
                let gl = self.goal_line();
                self.ui_line_changed(conn, "GL", gl);
                let gj = self.journal_line();
                self.ui_line_changed(conn, "GJ", gj);
                if self.realm.below {
                    let wt = self.wager_line();
                    self.ui_line_changed(conn, "WT", wt);
                }
                if !self.realm.below {
                    if !self.realm.siege {
                        let pm = self.pins_line();
                        self.ui_line_changed(conn, "PM", pm);
                    }
                    // Placing a station: the building ground laid on the ground (a friend's
                    // green), for this hero alone; gone once they're done.
                    let placing = self.sim.players.get(&id).is_some_and(|p| p.preview.is_some());
                    let bg = match self.sim.build_ground(id).filter(|_| placing) {
                        Some((at, r)) => format!("MK\tbuildground\t{:.2}\t{:.2}\t{:.2}\t{r:.2}\t3600\t1", at[0], at[1], at[2]),
                        None => "MK\tbuildground".to_string(),
                    };
                    self.ui_line_changed(conn, "BG", bg);
                }
                if self.realm.siege {
                    let sg = self.siege_line(id);
                    self.ui_line_changed(conn, "SG", sg);
                }
                if let Some(bs) = self.boss_line(id) {
                    self.ui_line_changed(conn, "BS", bs);
                }
                if let Some(va) = self.valor_line(id) {
                    self.ui_line_changed(conn, "VA", va);
                }
                let cc = self.cc_line(id);
                self.ui_line_changed(conn, "CC", cc);
                // What the use key would use, for its prompt (empty: nothing in reach).
                let np = match self.sim.use_prompt(id) {
                    Some((_, name, verb)) => format!("NP\t{verb}\t{}", clean(&name)),
                    None => "NP".to_string(),
                };
                self.ui_line_changed(conn, "NP", np);
                // A ping every couple of seconds, for the round trip.
                let due = self.world_of(conn).is_some_and(|w| w.ping_at.is_none_or(|t| t.elapsed() >= PING_EVERY));
                if due {
                    let n = self.world_of(conn).map_or(0, |w| w.ping_n.wrapping_add(1));
                    if let Some(w) = self.world_mut(conn) {
                        w.ping_n = n;
                        w.ping = Some((n, Instant::now()));
                        w.ping_at = Some(Instant::now());
                    }
                    self.ui_line(conn, &format!("PI\t{n}"));
                }
            }
            // The ability bar when it changed (a cast, a cooldown back, a new key), or never sent.
            let never = self.world_of(conn).is_some_and(|w| !w.sent.contains_key("AB"));
            if self.abilities_dirty.contains(&id) || never {
                if let Some(ab) = self.bar_line(id) {
                    self.ui_line(conn, &ab);
                    if let Some(w) = self.world_mut(conn) {
                        w.sent.insert("AB", ab);
                    }
                }
            }
            // Keys armed for a recast, keys empowered: as they open and close.
            for (n, (rc, em)) in self.combo_lines(id).into_iter().enumerate() {
                self.ui_line_stamped(conn, RC_SENT[n], rc);
                self.ui_line_stamped(conn, EM_SENT[n], em);
            }
            let bp = self.placement_line(id);
            self.ui_line_changed(conn, "BP", bp);
            let wr = format!("WR\t{}", if self.realm.below { "below" } else if self.realm.siege { "siege" } else { "camp" });
            // Siege's window up, as it stands now.
            if self.siege_dirty.remove(&id) {
                if let Some(st) = station {
                    self.ui_siege_station(conn, id, st, false);
                }
                let sg = self.siege_line(id);
                self.ui_line_changed(conn, "SG", sg);
            }
            self.ui_line_changed(conn, "WR", wr);
            if meters_due {
                self.ui_meters(conn, id);
            }
            self.abilities_dirty.remove(&id);
            if storage && station == Some(chest) {
                self.ui_station(conn, id, chest);
            }
        }
    }

    // --- windows -------------------------------------------------------------------------------------

    /// Open (or refresh) a station's window: the chest's contents, or what a station makes.
    pub(crate) fn ui_station(&mut self, conn: ConnId, id: Id, station: Id) {
        let addon = self.has_addon(conn);
        if station == self.sim.storage_id() {
            let c = &self.sim.content;
            let mut line = String::from("S\t");
            let mut words = Vec::new();
            for (k, (key, name, n)) in self.sim.storage_view().into_iter().enumerate() {
                // 1: gear held in the hands; 2: gear worn (armor, a pack).
                let item = c.item(&key).map(|i| &c.items[i]);
                let gear = match item.map(|i| i.kind) {
                    Some(ItemKind::Weapon | ItemKind::Shield | ItemKind::Bow) => 1,
                    Some(ItemKind::Armor | ItemKind::Pack) => 2,
                    _ => 0,
                };
                let text = c.item(&key).map_or(String::new(), |i| {
                    let extra = self.sim.gear_words(i);
                    if extra.is_empty() { c.items[i].text.clone() } else { format!("{} {extra}", c.items[i].text) }
                });
                let text = text.as_str();
                let icon = item.map_or("", |i| i.icon.as_str());
                if k > 0 {
                    line.push('^');
                }
                let _ = write!(line, "{}~{}~{n}~{gear}~{}~{}", clean(&key), clean(&name), clean(text), clean(icon));
                words.push(format!("{n} {name} ({key})"));
            }
            if addon {
                self.ui_line(conn, &line);
            } else {
                let text = if words.is_empty() { "The camp's chest is empty.".to_string() } else { format!("The camp's chest: {}.", words.join(", ")) };
                self.say_to(conn, &text);
                self.say_to(conn, "Take things with .take <item> <count>; swap gear with .equip <item>.");
            }
            return;
        }
        if self.ui_siege_station(conn, id, station, true) {
            return;
        }
        if station == self.sim.book_id() && station != 0 {
            let pages = self.sim.book_pages();
            let arena = self.sim.arena_page();
            let siege = self.sim.siege_page();
            if addon {
                // Each page with the dungeon it's in, and the dungeons themselves after Siege's.
                let books = self.sim.book_dungeons();
                let list: Vec<String> = books
                    .iter()
                    .filter(|b| b.open)
                    .flat_map(|b| b.pages.iter().map(move |(k, n)| format!("{}~{}~{}", clean(k), clean(n), clean(&b.key))))
                    .collect();
                let dungeons: Vec<String> = books
                    .iter()
                    .map(|b| format!(
                            "{}~{}~{}~{}~{}~{}~{}~{}~{}~{}~{}~{}~{}~{}~{}",
                            clean(&b.key),
                            clean(&b.name),
                            clean(&b.icon),
                            u8::from(b.open),
                            b.gate,
                            b.lamps,
                            b.pages.len(),
                            clean(&b.blurb),
                            clean(&b.first_page),
                            b.secrets.0,
                            b.secrets.1,
                            b.notes.0,
                            b.notes.1,
                            b.wager.0,
                            b.wager.1.round()
                        ))
                    .collect();
                let ring = arena.map_or_else(String::new, |(k, n, best)| format!("{}~{}~{best}", clean(&k), clean(&n)));
                let siege = siege.map_or_else(String::new, |(k, n, t)| format!("{}~{}~{}", clean(&k), clean(&n), clean(&t)));
                self.ui_line(
                    conn,
                    &format!("BK\t{}\t{}\t{ring}\t{siege}\t{}", u8::from(self.sim.book_open()), list.join("^"), dungeons.join("^")),
                );
            } else {
                let mut list: Vec<String> = if self.sim.book_open() {
                    pages.iter().map(|(k, n)| format!(".travel {k} ({n})")).collect()
                } else {
                    Vec::new()
                };
                if let Some((k, n, _)) = arena {
                    list.push(format!(".travel {k} ({n})"));
                }
                if let Some((k, n, _)) = siege {
                    list.push(format!(".travel {k} ({n})"));
                }
                self.say_to(
                    conn,
                    &format!("The Smuggler's Logbook: {}", list.join("; ")),
                );
                self.say_to(conn, "Each dungeon's notes: .es lore <dungeon> (deadmines, stockade, ...); a note again: .es note <note>.");
            }
            return;
        }
        if let Some(ObjKind::Lamp { key }) = self.sim.objs.get(&station).map(|o| o.kind.clone()) {
            self.ui_lamp(conn, id, &key);
            return;
        }
        if matches!(self.sim.objs.get(&station).map(|o| &o.kind), Some(ObjKind::ArenaBanner)) {
            self.ui_arena_banner(conn, id);
            return;
        }
        if matches!(self.sim.objs.get(&station).map(|o| &o.kind), Some(ObjKind::Rack)) {
            let rack = self.sim.rack_view(id);
            if addon {
                let mut line = String::from("AR\t");
                let c = &self.sim.content;
                for (k, (key, name, kind, held, text)) in rack.iter().enumerate() {
                    if k > 0 {
                        line.push('^');
                    }
                    let icon = c.item(key).map_or("", |i| c.items[i].icon.as_str());
                    let _ = write!(line, "{}~{}~{kind}~{}~{}~{}", clean(key), clean(name), u8::from(*held), clean(text), clean(icon));
                }
                self.ui_line(conn, &line);
            } else {
                let list: Vec<String> = rack.iter().map(|(k, n, kind, _, _)| format!(".arm {k} ({n}, {kind})")).collect();
                self.say_to(conn, &format!("Weapon Rack: {}", list.join("; ")));
            }
            return;
        }
        let Some(v) = self.sim.object(station) else { return };
        let recipes = self.sim.recipes(id, station);
        if addon {
            let mut line = format!("W\t{station}\t{}\t", clean(&v.name));
            let c = &self.sim.content;
            for (k, (key, out, _, can)) in recipes.iter().enumerate() {
                if k > 0 {
                    line.push('^');
                }
                // What it makes and does, its icon, and each input with what the hero has of it.
                let recipe = c.recipes.iter().find(|r| &r.key == key);
                let made = recipe.and_then(|r| c.item(&r.output)).map(|i| &c.items[i]);
                let text = made.map_or("", |i| i.text.as_str());
                let icon = made.map_or("", |i| i.icon.as_str());
                let cost = recipe.map_or(String::new(), |r| self.cost_detail(id, &r.inputs));
                let _ = write!(line, "{}~{}~{}~{}~{}~{}", clean(key), clean(out), cost, u8::from(*can), clean(text), clean(icon));
            }
            let icon = match self.sim.objs.get(&station).map(|o| &o.kind) {
                Some(ObjKind::Station { def, .. }) => c.stations[*def].icon.clone(),
                _ => String::new(),
            };
            let _ = write!(line, "\t{}", clean(&icon));
            self.ui_line(conn, &line);
        } else {
            let list: Vec<String> = recipes.iter().map(|(k, out, cost, _)| format!(".craft {k} ({out}: {cost})")).collect();
            self.say_to(conn, &format!("{}: {}", v.name, list.join("; ")));
        }
    }

    /// The Proving Ring's banner: how the run stands, the next wave now, home.
    fn ui_arena_banner(&mut self, conn: ConnId, id: Id) {
        let a = &self.sim.arena;
        let (wave, fighting, secs, heroes) = (a.wave, a.fighting, a.timer.max(0.0), a.heroes.len());
        let best = self.sim.arena_best();
        if !self.sim.in_arena(id) {
            self.say_to(conn, "The Proving Ring's banner. Read the book at camp to come into the ring.");
            return;
        }
        if self.has_addon(conn) {
            self.ui_line(conn, &format!("AW\t{wave}\t{}\t{secs:.0}\t{best}\t{heroes}", u8::from(fighting)));
        } else {
            self.say_to(conn, "The Proving Ring's banner: .arena next to bring on the next wave now; .travel camp to go home.");
        }
    }

    /// How the ring stands, for a hero in it (or the line cleared, out of it).
    pub(super) fn ui_arena(&mut self, conn: ConnId, id: Id) {
        if !self.has_addon(conn) {
            return;
        }
        let line = match self.sim.arena_view(id) {
            Some((wave, standing, next, best)) => {
                format!("AA\t{wave}\t{standing}\t{}\t{best}", next.map_or(-1.0, |n| n.ceil()))
            }
            None => "AA".to_string(),
        };
        self.ui_line(conn, &line);
    }

    /// A lamp's window below: rest, the other lit lamps, the quartermaster, home.
    fn ui_lamp(&mut self, conn: ConnId, id: Id, key: &str) {
        let c = &self.sim.content;
        let Some(d) = c.dungeon.as_ref() else { return };
        let name = d
            .lamps
            .iter()
            .find(|l| l.key == key)
            .map_or(String::new(), |l| l.name.clone());
        let p = self.sim.players.get(&id);
        let draught = c.item(&d.draught);
        let have = draught
            .and_then(|i| p.and_then(|p| p.bag.get(&i)))
            .copied()
            .unwrap_or(0);
        let flasks = p
            .and_then(|p| {
                self.sim
                    .below
                    .as_ref()
                    .and_then(|b| b.heroes.get(&p.account))
            })
            .map_or(0, |h| h.flasks);
        let most = (d.draughts + flasks).min(d.draughts_max);
        let plunder = c
            .item(&d.plunder)
            .and_then(|i| p.and_then(|p| p.bag.get(&i)))
            .copied()
            .unwrap_or(0);
        let lamps: Vec<String> = self
            .sim
            .lit_lamps()
            .into_iter()
            .filter(|(k, _)| k != key)
            .map(|(k, n)| format!("{}~{}", clean(&k), clean(&n)))
            .collect();
        let trades: Vec<String> = self
            .sim
            .trades(id)
            .into_iter()
            .map(|(k, n, price, can)| {
                let icon = c.item(&k).map_or("", |i| c.items[i].icon.as_str());
                format!(
                    "{}~{}~{price}~{}~{}",
                    clean(&k),
                    clean(&n),
                    u8::from(can),
                    clean(icon)
                )
            })
            .collect();
        if self.has_addon(conn) {
            let line = format!(
                "LP\t{}\t{}\t{have}\t{most}\t{}\t{}\t{plunder}",
                clean(key),
                clean(&name),
                lamps.join("^"),
                trades.join("^")
            );
            self.ui_line(conn, &line);
        } else {
            self.say_to(conn, &format!("{name}: .rest to rest here (whole again, Draughts filled, the Brotherhood back at their posts); .travel <lamp> to go to another lit lamp; .travel camp to go home; .buy <ware> for plunder."));
        }
    }

    /// A cost for the add-on: each input as `name:need:have`, `;` between (what the hero has in
    /// their bags and the camp's chest together).
    pub(super) fn cost_detail(&self, id: Id, cost: &std::collections::BTreeMap<String, u32>) -> String {
        let c = &self.sim.content;
        let words: Vec<String> = cost
            .iter()
            .filter_map(|(k, n)| {
                let i = c.item(k)?;
                let name = clean(&c.items[i].name).replace([':', ';'], " ");
                Some(format!("{name}:{n}:{}", self.sim.available(id, i)))
            })
            .collect();
        words.join(";")
    }

    fn ui_build_menu(&mut self, conn: ConnId, id: Id) {
        let menu = self.sim.build_menu(id);
        if self.has_addon(conn) {
            let mut line = String::from("BM\t");
            let c = &self.sim.content;
            for (k, (key, name, _, can, built, text)) in menu.iter().enumerate() {
                if k > 0 {
                    line.push('^');
                }
                let st = c.station(key).map(|s| &c.stations[s]);
                let cost = st.map_or(String::new(), |s| self.cost_detail(id, &s.cost));
                let icon = st.map_or("", |s| s.icon.as_str());
                let _ = write!(line, "{}~{}~{}~{}~{built}~{}~{}", clean(key), clean(name), cost, u8::from(*can), clean(text), clean(icon));
            }
            self.ui_line(conn, &line);
        } else {
            let list: Vec<String> = menu.iter().map(|(k, n, cost, _, built, _)| format!(".build {k} ({n}, {cost}; {built} built)")).collect();
            self.say_to(conn, &format!("Build: {}. Then .place to set it down, .rotate to turn it, .cancel to stop.", list.join("; ")));
        }
    }

    // --- commands --------------------------------------------------------------------------------------

    /// `.<command>`, typed or from the add-on (`.es ...`).
    pub(crate) fn command(&mut self, conn: ConnId, cmd: &str) {
        let Some(me) = self.world_of(conn).map(|w| w.id) else { return };
        let mut words = cmd.split_whitespace();
        let verb = words.next().unwrap_or("").to_lowercase();
        let verb = if verb == "es" { words.next().unwrap_or("").to_lowercase() } else { verb };
        let arg = words.next().unwrap_or("").to_lowercase();
        let arg2 = words.next().unwrap_or("");
        let result: Result<(), String> = match verb.as_str() {
            "hello" => {
                if let Some(w) = self.world_mut(conn) {
                    w.addon = true;
                    w.sent.clear();
                }
                // `.es hello <version> ac`: the action-combat client, aiming over the shoulder;
                // `ac2` also plays clips, hit-stops and rolls itself; `ac3` runs its own dashes and
                // lunges too; `ac4` plays its own blows' weight the moment they connect; `ac9`
                // trails a creature's swing over its last stroke (`FOE_TRAILS_LEVEL`).
                let ac = arg2.to_ascii_lowercase();
                let level: u8 = match ac.strip_prefix("ac") {
                    Some("") => 1,
                    Some(n) => n.parse().unwrap_or(1),
                    None => 0,
                };
                if let Some(p) = self.sim.players.get_mut(&me) {
                    p.shoulder_cam = level >= 1;
                }
                let local = level >= 3;
                if let Some(w) = self.world_mut(conn) {
                    w.anim = level >= 2;
                    w.local = local;
                    w.level = level;
                }
                self.sim.set_local_moves(me, local);
                self.ui_line(conn, &format!("V\t{VERSION}"));
                Ok(())
            }
            "in" => {
                // An ability key: `ab <slot 0-5> <ex 0|1>`, or its role's own letter.
                let mut rest = arg2.to_string();
                let input = match arg.as_str() {
                    "a" => Some(Input::Attack),
                    "h" => Some(Input::Heavy),
                    "au" => Some(Input::AttackUp),
                    "g" => Some(Input::Guard),
                    "gu" => Some(Input::GuardUp),
                    "sp" => Some(Input::Sprint),
                    "spu" => Some(Input::SprintUp),
                    "d" => Some(Input::Dodge),
                    "u" => Some(Input::Use),
                    "b" => Some(Input::Bash),
                    "k" => Some(Input::Kick),
                    "lock" => Some(Input::Lock),
                    "unlock" => Some(Input::Unlock),
                    "cy" => Some(Input::Cycle),
                    "x" => Some(Input::Cancel),
                    "sh" => Some(Input::Sheathe),
                    "ab" => {
                        let slot = arg2.parse::<u8>().unwrap_or(0).min(5);
                        let ex = words.next().is_some_and(|w| w == "1");
                        rest = words.next().unwrap_or("").to_string();
                        Some(Input::Ability(slot, ex))
                    }
                    _ => None,
                };
                // An attack, its release and an ability key carry the reticle's line (the
                // client's own camera, as a facing and an elevation).
                if let (Some(Input::Attack | Input::Heavy | Input::AttackUp | Input::Ability(..)), Ok(f)) = (input, rest.parse::<f32>()) {
                    let e = words.next().and_then(|w| w.parse::<f32>().ok()).unwrap_or(0.0);
                    self.sim.set_aim_line(me, f, e);
                }
                if let Some(i) = input {
                    self.sim.input(me, i);
                }
                Ok(())
            }
            // The board (N): its window, and unlocking a card of it.
            "tree" => match arg.as_str() {
                "" | "open" => {
                    self.ui_tree(conn, me);
                    Ok(())
                }
                "take" => self.sim.take_node(me, &arg2.to_lowercase()).map(|name| {
                    self.tell(conn, &format!("{name} is yours."));
                    self.ui_tree(conn, me);
                }),
                "full" | "reset" if self.dev_ok() => {
                    self.sim.set_tree(me, arg == "full");
                    self.tell(conn, if arg == "full" { "Every card on the board is yours." } else { "The board is back to your class's first cards." });
                    self.ui_tree(conn, me);
                    Ok(())
                }
                _ => Err("Try .es tree (the board), .es tree take <ability> (dev: .es tree full, .es tree reset).".into()),
            },
            // Put an ability on a key: `.es slot q frostbolt` (or a slot's own name, or just the
            // ability, on its role's key).
            "slot" => {
                let (slot, key) = if arg2.is_empty() { (String::new(), arg.clone()) } else { (slot_of(&arg).to_string(), arg2.to_lowercase()) };
                self.sim.slot_ability(me, &slot, &key).map(|text| {
                    self.tell(conn, &text);
                    self.ui_tree(conn, me);
                })
            }
            // Every card on the board at once (debug): `.es spells` (`.es spells reset` takes them
            // back to the class's first cards).
            "spells" | "allspells" if self.dev_ok() => {
                let full = arg != "reset";
                self.sim.set_tree(me, full);
                self.tell(conn, if full { "Every spell is yours: open the board (N) to put them on your keys." } else { "The board is back to your class's first cards." });
                self.ui_tree(conn, me);
                Ok(())
            }
            "spells" | "allspells" => Err("Turn debug on first (.es debug), then .es spells.".into()),
            "valor" if self.dev_ok() => {
                let max = self.sim.content.valor.max;
                let v = if arg == "full" { max } else { arg.parse::<f32>().unwrap_or(0.0) };
                self.sim.set_valor(me, v);
                Ok(())
            }
            "cd" if self.dev_ok() => {
                let off = arg == "off";
                self.sim.set_cooldowns(me, off);
                self.tell(conn, if off { "Cooldowns off." } else { "Cooldowns on." });
                Ok(())
            }
            "heal" if self.dev_ok() => {
                self.sim.heal_full(me);
                Ok(())
            }
            "dummy" if self.dev_ok() => {
                let mode: Vec<&str> = cmd.split_whitespace().skip_while(|w| !w.eq_ignore_ascii_case("dummy")).skip(1).collect();
                self.sim.spawn_dummy(me, &mode.join(" ")).map(|t| self.tell(conn, &format!("{t} stands before you.")))
            }
            "cc" if arg == "log" => {
                let log = self.sim.cc_log_of(me);
                if log.is_empty() {
                    self.say_to(conn, "No crowd control yet.");
                }
                for l in log.iter().rev().take(12).rev() {
                    self.say_to(conn, l);
                }
                Ok(())
            }
            "pvp" if self.dev_ok() => {
                let on = arg != "off";
                self.sim.content.pvp.friendly_fire = on;
                self.announce(if on { "Friendly fire is on: heroes can strike each other." } else { "Friendly fire is off." });
                Ok(())
            }
            "valor" | "cd" | "heal" | "dummy" | "pvp" => Err("That's for dev builds ([dev] full_tree) or with .es debug on.".into()),
            // The add-on's answer to a ping: the round trip.
            "po" => {
                let n = arg.parse::<u32>().unwrap_or(0);
                let sent = self.world_of(conn).and_then(|w| w.ping).filter(|(k, _)| *k == n).map(|(_, at)| at);
                if let Some(at) = sent {
                    let rtt = at.elapsed().as_secs_f32();
                    if let Some(w) = self.world_mut(conn) {
                        // Smoothed, so one slow answer doesn't swing it.
                        w.rtt = if w.rtt <= 0.0 { rtt } else { w.rtt * 0.7 + rtt * 0.3 };
                        w.ping = None;
                        let r = w.rtt;
                        self.sim.set_rtt(me, r);
                    }
                }
                Ok(())
            }
            "use" | "eat" => {
                self.sim.input(me, Input::Use);
                Ok(())
            }
            "e" | "interact" => {
                self.sim.interact(me, None);
                Ok(())
            }
            "bm" | "build" if arg.is_empty() => {
                self.ui_build_menu(conn, me);
                Ok(())
            }
            "build" => self.sim.build_start(me, &arg),
            "rot" | "rotate" => {
                let steps = arg.parse::<i32>().unwrap_or(1);
                self.sim.build_rotate(me, steps);
                Ok(())
            }
            "place" | "ok" => self.sim.build_confirm(me).map(|_| ()),
            "cancel" => {
                self.sim.cancel_preview(me);
                Ok(())
            }
            "close" => {
                if let Some(w) = self.world_mut(conn) {
                    w.station = None;
                }
                Ok(())
            }
            // At Siege's Battle Trainer, "making" a card learns it.
            "craft" if self.world_of(conn).and_then(|w| w.station).is_some_and(|s| matches!(self.sim.objs.get(&s).map(|o| &o.kind), Some(ObjKind::SiegeTrainer { .. }))) => {
                let station = self.world_of(conn).and_then(|w| w.station).unwrap();
                self.sim.take_node(me, &arg).map(|_| self.ui_trainer(conn, me, station))
            }
            "craft" => {
                let station = self.world_of(conn).and_then(|w| w.station).or_else(|| self.station_near(me, &arg));
                match station {
                    Some(s) => self.sim.craft(me, s, &arg).map(|_| {
                        self.ui_station(conn, me, s);
                    }),
                    None => Err("Go to the station that makes it and use it first.".into()),
                }
            }
            "take" => {
                let n = arg2.parse::<u32>().unwrap_or(1);
                self.sim.withdraw(me, &arg, n)
            }
            "bank" | "deposit" => {
                self.sim.deposit_all(me);
                Ok(())
            }
            "equip" => self.sim.equip_from_chest(me, &arg),
            // The pages kept: a note read before, and a dungeon's secrets, rumours and notes.
            "note" | "reread" => self.sim.reread(me, &arg),
            "lore" | "secrets" => self.sim.lore_page(me, &arg),
            // The Bookmaker's Wager: its page, a level taken (from the book), its board.
            "wager" if arg2.is_empty() => self.ui_wager_page(conn, me, &arg),
            "wager" => match arg2.trim_start_matches('+').parse::<u32>() {
                Ok(level) => self.sim.travel_wager(me, &arg, level),
                Err(_) => Err("Try .es wager <dungeon> <level>, as .es wager deadmines 3.".into()),
            },
            "board" => self.ui_board_page(conn, &arg),
            "meter" => self.meter_command(conn, me, &arg),
            // The book at camp, and the lamps below.
            "travel" | "go" => {
                let to = if arg.is_empty() {
                    "camp".to_string()
                } else {
                    arg.clone()
                };
                self.sim.travel(me, &to)
            }
            "home" => self.sim.travel(me, "camp"),
            // Testing: straight to one of Elwynn's places.
            "goto" if self.dev_ok() => self.sim.goto_site(me, &arg).map(|name| self.tell(conn, &format!("You're at {name}."))),
            // The Proving Ring's banner: the next wave now.
            "arena" | "ring" => match arg.as_str() {
                "next" | "" => self.sim.arena_next_now(me).map(|_| {
                    self.tell(conn, "You raise the banner: here they come.");
                }),
                "leave" | "home" => self.sim.travel(me, "camp"),
                _ => Err("Try .arena next or .arena leave.".into()),
            },
            "unstuck" | "stuck" => self.sim.unstuck(me),
            "rest" => self.sim.rest(me).map(|_| self.refresh_lamp(conn, me)),
            "buy" => self.sim.buy(me, &arg).map(|name| {
                self.tell(conn, &format!("The quartermaster hands you a {name}."));
                self.refresh_lamp(conn, me);
            }),
            "arm" => self.sim.arm(me, &arg).map(|name| {
                self.tell(conn, &format!("You take up the {name}."));
                let rack = self.world_of(conn).and_then(|w| w.station).filter(|s| matches!(self.sim.objs.get(s).map(|o| &o.kind), Some(ObjKind::Rack))).unwrap_or_else(|| self.sim.rack_id());
                self.ui_station(conn, me, rack);
            }),
            "siege" => {
                let rest = arg2.to_string();
                self.siege_command(conn, me, &arg, &rest)
            }
            "raid" => match arg.as_str() {
                "yes" => self.sim.raid_start(me),
                "no" => {
                    self.sim.raid_decline(me);
                    Ok(())
                }
                "reset" if self.sim.debug => {
                    self.sim.raid_reset();
                    Ok(())
                }
                _ => Err("Ring the alarm bell at the camp to call the raid.".into()),
            },
            // Trying a look on yourself, for tuning: `.es fx emote 54`, `.es fx spell 845`,
            // `.es fx cast 75 1500` (a cast's start for that long), `.es fx stop 75`.
            "fx" => {
                let n = arg2.parse::<u32>().unwrap_or(0);
                let ms = words.next().and_then(|w| w.parse::<u32>().ok()).unwrap_or(1500);
                if let Some(g) = self.guid_of(me) {
                    let p = match arg.as_str() {
                        "emote" => Some(packets::emote(g, n)),
                        "spell" => Some(packets::spell_go(g, n, packets::Aim::Caster, &[g], None)),
                        "cast" => Some(packets::spell_start(g, n, ms, packets::Aim::Caster)),
                        "stop" => Some(packets::spell_failed_other(g, n)),
                        "pose" => {
                            let mut f = Fields::new();
                            f.set(idx::UNIT_NPC_EMOTESTATE, n);
                            Some(packets::update_object(&[], &[(g, &f)]))
                        }
                        _ => None,
                    };
                    match p {
                        Some(p) => self.broadcast(&p),
                        None => self.say_to(conn, "Try .es fx emote <id>, spell <id>, cast <id> [ms], stop <id> or pose <id> (0 ends it)."),
                    }
                }
                Ok(())
            }
            // Tuning the rules in play: `.es tune` (what's changed), `.es tune <path>` (its value),
            // `.es tune <path> <value>`, `.es tune reload` (the file again), `.es tune keep`
            // (write what's changed into the file).
            "tune" => {
                let path = cmd.split_whitespace().skip_while(|w| !w.eq_ignore_ascii_case("tune")).nth(1).unwrap_or("").to_string();
                let value: Vec<&str> = cmd.split_whitespace().skip_while(|w| !w.eq_ignore_ascii_case("tune")).skip(2).collect();
                self.tune(conn, me, &path, &value.join(" "))
            }
            // Everyone's round trip and the allowances made for it.
            "net" => {
                let k = &self.sim.content.combat;
                let mut rows: Vec<String> = self.in_world().map(|(_, _, w)| format!("{} {:.0} ms", w.name, w.rtt * 1000.0)).collect();
                rows.sort();
                let text = format!(
                    "Round trips: {}. A swing judges where others were up to {:.0} ms back; a blow on you waits up to {:.0} ms for your dodge or guard (.es tune combat.max_rewind / combat.max_grace).",
                    rows.join(", "),
                    k.max_rewind * 1000.0,
                    k.max_grace * 1000.0
                );
                self.say_to(conn, &text);
                Ok(())
            }
            // With debug on, a creature to fight at once: `.es spawn warlord`.
            // `.es spawn grunt 3`: that many.
            "spawn" if self.sim.debug => {
                let n = arg2.parse::<usize>().unwrap_or(1).clamp(1, 6);
                let mut said = Ok(String::new());
                for _ in 0..n {
                    said = self.sim.summon(me, &arg);
                    if said.is_err() {
                        break;
                    }
                }
                said.map(|name| {
                    let text = if n > 1 { format!("{n} {name}s come at you.") } else { format!("{name} comes at you.") };
                    self.say_to(conn, &text);
                })
            }
            "spawn" => Err("Turn debug on first (.es debug).".into()),
            // Testing the progression from a blank slate: this hero back to their first day, or
            // (debug) the camp's whole world, its save copied aside first.
            "reset" => match arg.as_str() {
                "me" if self.dev_ok() => self.sim.reset_hero(me).map(|_| self.ui_tree(conn, me)),
                // (The Deadmines start over with it, their save kept aside too: the realms'.)
                "camp" if self.sim.debug => match self.keep_save_aside() {
                    Err(e) => Err(e),
                    Ok(kept) => self.sim.reset_camp().map(|_| {
                        if let Some(to) = kept {
                            println!("[survival] the camp started over; the old save is at {}", to.display());
                            let name = to.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                            self.tell(conn, &format!("The old world is kept beside the save, as {name}."));
                        }
                        self.reset_below = true;
                        self.mark_dirty();
                        self.save();
                    }),
                },
                "me" | "camp" => Err("Turn debug on first (.es debug).".into()),
                _ => Err("Try .es reset me (this hero's first day) or .es reset camp (the whole world's).".into()),
            },
            "debug" => {
                self.sim.debug = !self.sim.debug;
                self.say_to(conn, if self.sim.debug { "Debug on: .es spells (every spell), .es raid reset, .es spawn <creature>, .es reset me and .es reset camp work." } else { "Debug off." });
                Ok(())
            }
            "where" => {
                if let Some(p) = self.sim.players.get(&me) {
                    let text = format!("You're at {:.1}, {:.1}, {:.1} facing {:.2}.", p.pos[0], p.pos[1], p.pos[2], p.facing);
                    self.say_to(conn, &text);
                }
                Ok(())
            }
            "status" => {
                let s = self.sim.storage_view();
                let chest: Vec<String> = s.iter().map(|(_, n, c)| format!("{c} {n}")).collect();
                let stations: Vec<String> = self.sim.objects().into_iter().filter(|v| matches!(v.kind, ObjKind::Station { .. })).map(|v| v.name).collect();
                let text = format!(
                    "Camp chest: {}. Stations: {}. Raid: {:?}.",
                    if chest.is_empty() { "empty".into() } else { chest.join(", ") },
                    if stations.is_empty() { "none yet".into() } else { stations.join(", ") },
                    self.sim.raid.phase
                );
                self.say_to(conn, &text);
                Ok(())
            }
            // Basin Battle's add-on says hello too: it's asleep here.
            "bb" => Ok(()),
            _ => {
                self.say_to(conn, &format!("Elwynn Survival: {CONTROLS_TEXT} Commands: .es unstuck (back to your lamp, or the camp), .bank, .take <item> <n>, .equip <item>, .arm <weapon> (at the rack), .build [station], .place, .rotate, .cancel, .craft <recipe>, .status, .where; .es tune to change the rules in play."));
                Ok(())
            }
        };
        if let Err(e) = result {
            self.tell(conn, &e);
        }
    }

    fn tune(&mut self, conn: ConnId, me: Id, path: &str, value: &str) -> Result<(), String> {
        if self.realm.siege {
            return Err("Tune the rules at camp: Siege's world plays them as the camp's are.".into());
        }
        let who = self.sim.players.get(&me).map_or_else(String::new, |p| p.name.clone());
        match (path, value) {
            ("", _) => {
                let text = if self.tuned.is_empty() {
                    "Nothing tuned yet. .es tune combat.chain_after 0.3 changes a value of survival.toml in play; .es tune keep writes the changes into it, .es tune reload reads it again.".to_string()
                } else {
                    let list: Vec<String> = self.tuned.iter().map(|(p, v)| format!("{p} = {v}")).collect();
                    format!("Tuned: {}.", list.join(", "))
                };
                self.say_to(conn, &text);
                Ok(())
            }
            ("reload", "") => {
                let path = self.sim.content.path.clone().ok_or("These rules weren't read from a file.")?;
                let fresh = htl_survival::Content::load(&path).map_err(|e| format!("{e:#}"))?;
                self.sim.content = fresh;
                self.tuned.clear();
                self.announce(&format!("{who} read the rules again from survival.toml."));
                Ok(())
            }
            ("keep", "") => {
                let path = self.sim.content.path.clone().ok_or("These rules weren't read from a file.")?;
                std::fs::write(&path, &self.sim.content.source).map_err(|e| format!("Couldn't write {}: {e}", path.display()))?;
                let n = self.tuned.len();
                self.tuned.clear();
                self.announce(&format!("{who} kept {n} tuned value(s) in survival.toml."));
                Ok(())
            }
            (path, "") => {
                let v = self.sim.content.tuning(path).map_err(|e| format!("{e:#}"))?;
                self.say_to(conn, &format!("{path} = {}", v.unwrap_or_else(|| "(its default)".into())));
                Ok(())
            }
            (path, value) => {
                let (fresh, old) = self.sim.content.tuned(path, value).map_err(|e| format!("{path}: {e:#}"))?;
                self.sim.content = fresh;
                let now = self.sim.content.tuning(path).ok().flatten().unwrap_or_else(|| value.to_string());
                self.tuned.retain(|(p, _)| p != path);
                self.tuned.push((path.to_string(), now.clone()));
                println!("[survival] {who} tuned {path}: {} -> {now}", old.as_deref().unwrap_or("default"));
                self.announce(&format!("{who} tuned {path}: {} to {now}.", old.as_deref().unwrap_or("default")));
                Ok(())
            }
        }
    }

    /// The dev-only commands: a dev build's rules, or debug on.
    fn dev_ok(&self) -> bool {
        self.sim.content.dev.full_tree || self.sim.debug
    }

    /// The board's window: every card, unlocked or not, and the keys they're on.
    pub(crate) fn ui_tree(&mut self, conn: ConnId, id: Id) {
        if !self.has_addon(conn) {
            let Some(p) = self.sim.players.get(&id) else { return };
            let owned: Vec<String> = p.owned.iter().cloned().collect();
            self.say_to(conn, &format!("Your cards: {}. Keys: {:?}. .es slot <q|e|r|f> <ability> puts one on a key.", owned.join(", "), p.loadout));
            return;
        }
        if let Some(line) = self.tree_line(id) {
            self.ui_line(conn, &line);
        }
    }

    /// The board (N): `TR\t<dev>\t<what the count says>\t<card>^...`, each card
    /// `key~name~row~column~icon~owned~can unlock~on key~kind~aimed~least charge~full charge~text~facts~EX key~EX name~EX icon~EX Valor~EX text~EX facts`.
    fn tree_line(&self, id: Id) -> Option<String> {
        let p = self.sim.players.get(&id)?;
        let c = &self.sim.content;
        let dev = c.dev.full_tree;
        let cards: Vec<String> = c
            .abilities
            .iter()
            .enumerate()
            .filter(|(_, a)| a.on_board())
            .map(|(k, a)| {
                let owned = p.owned.contains(&a.key);
                let can = !owned && self.sim.reachable(&p.owned, &a.key);
                let on = p.loadout.iter().find(|(_, v)| **v == a.key).map_or("", |(s, _)| s.as_str());
                let ex = c.ex_of(k).map(|x| &c.abilities[x]);
                let exs = ex.map_or("~~~~~".to_string(), |x| {
                    format!("{}~{}~{}~{}~{}~{}", x.key, clean(&x.name), clean(&x.icon), x.valor, clean(&x.text), clean(&ability_facts(x, Some(a), c.valor.max)))
                });
                let at = if owned { String::new() } else { clean(&self.sim.learned_where(&a.key)) };
                format!(
                    "{}~{}~{}~{}~{}~{}~{}~{}~{}~{}~{:.2}~{:.2}~{}~{}~{exs}~{at}",
                    a.key,
                    clean(&a.name),
                    a.slot,
                    a.school,
                    clean(&a.icon),
                    u8::from(owned),
                    u8::from(can),
                    on,
                    a.kind,
                    u8::from(a.aimed()),
                    a.charge[0],
                    a.charge[1],
                    clean(&a.text),
                    clean(&ability_facts(a, None, c.valor.max)),
                )
            })
            .collect();
        let owned = c.abilities.iter().filter(|a| a.on_board() && p.owned.contains(&a.key)).count();
        let points = if dev { "Dev build: every card".to_string() } else { format!("{owned} of {} cards", cards.len()) };
        Some(format!("TR\t{}\t{points}\t{}", u8::from(dev), cards.join("^")))
    }

    /// The ability bar: `AB\t<slot~ability~name~icon~kind~shape~least charge~full charge~cooldown left~cooldown~charges~most~range~radius~arc~Valor~EX key~EX name~EX icon~EX Valor~EX kind~EX shape~EX range~EX radius~EX arc~text~facts~EX text~EX facts~aimed~column>^...`
    /// (one per key, its fields empty when nothing's on it).
    pub(crate) fn bar_line(&self, id: Id) -> Option<String> {
        let c = &self.sim.content;
        let now = self.sim.clock;
        let p = self.sim.players.get(&id)?;
        let shape = |d: &htl_survival::content::AbilityDef| match d.kind.as_str() {
            // A lob is aimed at the ground, as ground is.
            "projectile" if d.lob => "circle",
            "ground" | "place" | "trap" | "leap" | "blink" => "circle",
            "cone" => "cone",
            "nova" | "stance" | "art" | "buff" => "self",
            _ => "line",
        };
        let shape = |d: &htl_survival::content::AbilityDef| if d.at_self { "self" } else { shape(d) };
        // The circle drawn where it's aimed: a lob's is its burst.
        let reach = |d: &htl_survival::content::AbilityDef| if d.lob { d.splash } else { htl_survival::content::AbilityDef::at(d.radius, 1.0) };
        let charge = |d: &htl_survival::content::AbilityDef| -> (f32, f32) {
            if d.kind == "shot" {
                let r = self.sim.ranged_of(p);
                if r.draw_secs <= 0.0 { (0.0, 0.0) } else { (r.min_draw.max(d.charge[0]), r.draw_secs) }
            } else {
                (d.charge[0], d.charge[1])
            }
        };
        let slots: Vec<String> = self
            .sim
            .bar(id)
            .into_iter()
            .map(|(slot, a, ex, cd)| {
                let Some(a) = a else { return format!("{slot}~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~") };
                let d = &c.abilities[a];
                let (lo, hi) = charge(d);
                let left = if p.cd_off || cd.charges > 0 { 0.0 } else { (cd.next_at - now).max(0.0) };
                let base_cd = d.cooldown;
                let range = htl_survival::content::AbilityDef::at(d.range, 1.0);
                // A blink's landing (no area) still shows where it goes.
                let radius = reach(d).max(if d.kind == "blink" { 1.0 } else { 0.0 });
                let exs = match ex.map(|x| &c.abilities[x]) {
                    Some(x) => format!(
                        "{}~{}~{}~{}~{}~{}~{:.1}~{:.1}~{:.0}",
                        x.key,
                        clean(&x.name),
                        clean(&x.icon),
                        x.valor,
                        x.kind,
                        shape(x),
                        htl_survival::content::AbilityDef::at(x.range, 1.0),
                        reach(x),
                        x.arc
                    ),
                    None => "~~~~~~~~".to_string(),
                };
                let (ext, exf) = ex.map(|x| &c.abilities[x]).map_or((String::new(), String::new()), |x| (clean(&x.text), clean(&ability_facts(x, Some(d), c.valor.max))));
                format!(
                    "{slot}~{}~{}~{}~{}~{}~{lo:.2}~{hi:.2}~{left:.1}~{base_cd:.1}~{}~{}~{range:.1}~{radius:.1}~{:.0}~{}~{exs}~{}~{}~{ext}~{exf}~{}~{}",
                    d.key,
                    clean(&d.name),
                    clean(&d.icon),
                    d.kind,
                    shape(d),
                    if p.cd_off { d.charges.max(1) } else { cd.charges },
                    d.charges.max(1),
                    d.arc,
                    d.valor,
                    clean(&d.text),
                    clean(&ability_facts(d, None, c.valor.max)),
                    u8::from(d.aimed()),
                    d.school,
                )
            })
            .collect();
        Some(format!("AB\t{}", slots.join("^")))
    }

    /// Each key's recast window (`RC`) and empower (`EM`) as it stands, with a stamp that changes
    /// only as one opens or closes (not as its seconds run down), and a closed one's stamp empty.
    fn combo_lines(&self, id: Id) -> Vec<((String, String), (String, String))> {
        let c = &self.sim.content;
        let now = self.sim.clock;
        let recasts = self.sim.recast_keys(id);
        let empowered = self.sim.empowered_keys(id);
        (0..RC_SENT.len())
            .map(|n| {
                let rc = match recasts.iter().find(|(k, ..)| *k == n) {
                    Some(&(_, a, until)) => {
                        let d = &c.abilities[a];
                        let shape = match d.kind.as_str() {
                            "ground" | "place" | "trap" | "leap" | "blink" => "circle",
                            "cone" => "cone",
                            "nova" | "stance" | "art" | "buff" => "self",
                            _ => "line",
                        };
                        let shape = if d.at_self || !d.aimed() { "self" } else { shape };
                        let radius = htl_survival::content::AbilityDef::at(d.radius, 1.0).max(if d.kind == "blink" { 1.0 } else { 0.0 });
                        (
                            format!("{}@{until:.2}", d.key),
                            format!(
                                "RC\t{n}\t{:.1}\t{}\t{}\t{}\t{shape}\t{:.1}\t{radius:.1}\t{:.0}\t{}\t{:.2}\t{:.2}",
                                (until - now).max(0.0),
                                clean(&d.icon),
                                d.key,
                                clean(&d.name),
                                htl_survival::content::AbilityDef::at(d.range, 1.0),
                                d.arc,
                                u8::from(d.aimed()),
                                d.charge[0],
                                d.charge[1]
                            ),
                        )
                    }
                    None => (String::new(), format!("RC\t{n}")),
                };
                let em = match empowered.iter().find(|(k, _)| *k == n) {
                    Some(&(_, until)) => (format!("{until:.2}"), format!("EM\t{n}\t{:.1}", (until - now).max(0.0))),
                    None => (String::new(), format!("EM\t{n}")),
                };
                (rc, em)
            })
            .collect()
    }

    /// Send a line kept under `verb` when its stamp changed (and once to a fresh world, closed or
    /// not, so nothing from the world before stays on the bar).
    fn ui_line_stamped(&mut self, conn: ConnId, verb: &'static str, (stamp, line): (String, String)) {
        let same = self.world_of(conn).is_some_and(|w| w.sent.get(verb) == Some(&stamp));
        if same {
            return;
        }
        self.ui_line(conn, &line);
        if let Some(w) = self.world_mut(conn) {
            w.sent.insert(verb, stamp);
        }
    }

    /// Valor: `VA\t<now>\t<most>\t<a segment>`.
    fn valor_line(&self, id: Id) -> Option<String> {
        let p = self.sim.players.get(&id)?;
        let v = &self.sim.content.valor;
        Some(format!("VA\t{:.0}\t{:.0}\t{:.0}", p.valor.floor(), v.max, v.segment))
    }

    /// The crowd control on the hero: `CC\t<kind~seconds left~share>^...` (the grace as `grace`).
    fn cc_line(&self, id: Id) -> String {
        let now = self.sim.clock;
        let mut list: Vec<String> = self.sim.cc_of(id).iter().filter(|c| c.until > now && c.kind != "mark").map(|c| format!("{}~{:.1}~{:.2}", c.kind, c.until - now, c.amount)).collect();
        if let Some(t) = self.sim.grace.get(&id).filter(|t| **t > now) {
            list.push(format!("grace~{:.1}~0", t - now));
        }
        if let Some(s) = self.sim.stances.get(&id).filter(|s| s.until > now) {
            list.push(format!("stance~{:.1}~0", s.until - now));
        }
        format!("CC\t{}", list.join("^"))
    }

    /// The lamp window again, if it's open (after resting or buying).
    fn refresh_lamp(&mut self, conn: ConnId, me: Id) {
        if let Some(s) = self.world_of(conn).and_then(|w| w.station) {
            if matches!(
                self.sim.objs.get(&s).map(|o| &o.kind),
                Some(ObjKind::Lamp { .. })
            ) {
                self.ui_station(conn, me, s);
            }
        }
    }

    /// A word to every hero in the world.
    fn announce(&self, text: &str) {
        let conns: Vec<ConnId> = self.in_world().map(|(c, _, _)| *c).collect();
        for c in conns {
            self.tell(c, text);
        }
    }

    /// The nearest station (in reach) that makes recipe `key`.
    fn station_near(&self, me: Id, key: &str) -> Option<Id> {
        let pos = self.sim.players.get(&me)?.pos;
        let c = &self.sim.content;
        let station = &c.recipes.iter().find(|r| r.key == key)?.station;
        self.sim
            .objs
            .iter()
            .filter(|(_, o)| matches!(o.kind, ObjKind::Station { def, .. } if &c.stations[def].key == station))
            .map(|(id, o)| (*id, htl_survival::dist2d(o.pos, pos)))
            .filter(|(_, d)| *d <= htl_survival::INTERACT_RANGE + 2.0)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(id, _)| id)
    }
}
