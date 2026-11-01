//! What a card does, in plain sentences with real numbers: the card picker shows these, and chat
//! repeats them when a card is taken.
//!
//! Damage is worked out from the player's current "hit" (see `content/cards.toml`), so a card
//! offered at level 20 shows bigger numbers than the same card at level 2, exactly as it would hit.

use crate::content::{CardDef, CardKind};

/// A card described for one player at one rank.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CardText {
    /// One to three sentences. With colour on, numbers are white and damage words take their
    /// school's colour, for a light-grey base text.
    pub text: String,
    /// What ranking up changes, such as "54 → 81 damage"; empty for a new card.
    pub upgrade: String,
    /// "Spell", "Passive", "Proc" or "Aura".
    pub kind: &'static str,
    /// The school's name, or "" for cards without one.
    pub school: &'static str,
    /// Seconds between casts, for spells that fire on their own; 0 otherwise.
    pub cooldown: f32,
}

pub fn school_name(school: u8) -> &'static str {
    match school {
        0 => "Physical",
        1 => "Holy",
        2 => "Fire",
        3 => "Nature",
        4 => "Frost",
        5 => "Shadow",
        6 => "Arcane",
        _ => "",
    }
}

/// The colour code a school's damage words take.
pub fn school_color(school: u8) -> &'static str {
    match school {
        0 => "ffe8c89a",
        1 => "fffff08a",
        2 => "ffff8a4a",
        3 => "ff8cff6a",
        4 => "ff8ad8ff",
        5 => "ffc08cff",
        6 => "ffff9cf6",
        _ => "ffffffff",
    }
}

/// A number as players read it: whole above 10, one decimal below.
pub fn num(v: f32) -> String {
    let v = v.max(0.0);
    if v >= 9.95 {
        format!("{}", v.round() as i64)
    } else {
        let s = format!("{v:.1}");
        s.trim_end_matches(".0").to_string()
    }
}

fn pct_str(v: f32) -> String {
    let p = v * 100.0;
    if (p - p.round()).abs() < 0.05 || p >= 10.0 {
        format!("{}%", p.round() as i64)
    } else {
        format!("{}%", num(p))
    }
}

struct Writer {
    color: bool,
}

impl Writer {
    /// A number, highlighted.
    fn n(&self, s: String) -> String {
        if self.color {
            format!("|cffffffff{s}|r")
        } else {
            s
        }
    }
    fn amount(&self, v: f32) -> String {
        self.n(num(v.max(1.0)))
    }
    fn pct(&self, v: f32) -> String {
        self.n(pct_str(v))
    }
    fn secs(&self, v: f32) -> String {
        self.n(num(v))
    }
    /// "54 Frost damage", the school's words coloured.
    fn damage(&self, v: f32, school: u8) -> String {
        let words = match school_name(school) {
            "" => "damage".to_string(),
            s => format!("{s} damage"),
        };
        let words = if self.color { format!("|c{}{}|r", school_color(school), words) } else { words };
        format!("{} {words}", self.amount(v))
    }
}

/// The card at `rank` (1 for a new card), for a player whose hit is `hit` and whose areas are
/// `radius` times their base size.
pub fn card_text(def: &CardDef, rank: u8, hit: f32, radius: f32, color: bool) -> CardText {
    let w = Writer { color };
    let m = def.rank_mult(rank);
    let hit = hit.max(1.0);
    let text = sentences(def, m, hit, radius.max(0.1), &w);
    let upgrade = if rank > 1 { upgrade(def, rank, hit, &w) } else { String::new() };
    let kind = match def.kind {
        CardKind::Bolt | CardKind::Nova | CardKind::Zone | CardKind::Heal => "Spell",
        CardKind::Modifier | CardKind::Stat => "Passive",
        CardKind::Trigger => "Proc",
        CardKind::Team => "Aura",
    };
    let has_school = matches!(def.kind, CardKind::Bolt | CardKind::Nova | CardKind::Zone)
        || (def.kind == CardKind::Modifier && def.modifier == "dot");
    CardText {
        text,
        upgrade,
        kind,
        school: if has_school { school_name(def.school) } else { "" },
        cooldown: if def.is_ability() { def.cooldown } else { 0.0 },
    }
}

fn sentences(def: &CardDef, m: f32, hit: f32, radius: f32, w: &Writer) -> String {
    let dmg = hit * def.damage * m;
    let range = if def.range > 0.0 { def.range } else { 30.0 };
    let mut out: Vec<String> = Vec::new();
    match def.kind {
        CardKind::Bolt => {
            let several = def.targets > 1 || def.chain > 0;
            if def.damage <= 0.0 && def.dot > 0.0 {
                out.push(format!(
                    "Afflicts your target within {} yd with {} over {} sec.",
                    w.n(num(range)),
                    w.damage(hit * def.dot * m, def.school),
                    w.secs(def.dot_secs)
                ));
            } else if def.chain > 0 {
                let weaker = 1.0 - if def.falloff > 0.0 { def.falloff } else { 0.8 };
                out.push(format!(
                    "Strikes your target within {} yd for {}, then leaps to {} more nearby, {} weaker each jump.",
                    w.n(num(range)),
                    w.damage(dmg, def.school),
                    w.n(def.chain.to_string()),
                    w.pct(weaker)
                ));
            } else if def.projectiles > 1 {
                out.push(format!(
                    "Launches {} missiles at your target within {} yd, each for {}.",
                    w.n(def.projectiles.to_string()),
                    w.n(num(range)),
                    w.damage(dmg, def.school)
                ));
            } else if def.targets > 1 {
                out.push(format!(
                    "Strikes your target and {} more enemies within {} yd for {} each.",
                    w.n((def.targets - 1).to_string()),
                    w.n(num(range)),
                    w.damage(dmg, def.school)
                ));
            } else {
                out.push(format!(
                    "Strikes your target within {} yd for {}.",
                    w.n(num(range)),
                    w.damage(dmg, def.school)
                ));
            }
            if def.dot > 0.0 && def.damage > 0.0 {
                out.push(format!(
                    "Then {} over {} sec.",
                    w.damage(hit * def.dot * m, def.school),
                    w.secs(def.dot_secs)
                ));
            }
            hit_effects(def, several, &mut out, w);
        }
        CardKind::Nova => {
            let r = def.radius * radius;
            if def.damage > 0.0 {
                out.push(format!(
                    "Blasts every enemy within {} yd of you for {}.",
                    w.n(num(r)),
                    w.damage(dmg, def.school)
                ));
                if def.dot > 0.0 {
                    out.push(format!(
                        "Then {} over {} sec.",
                        w.damage(hit * def.dot * m, def.school),
                        w.secs(def.dot_secs)
                    ));
                }
                hit_effects(def, true, &mut out, w);
            }
            if def.heal > 0.0 {
                out.push(format!(
                    "Heals you and allies within {} yd for {} of their max health.",
                    w.n(num(r)),
                    w.pct(def.heal * m)
                ));
            }
        }
        CardKind::Zone => {
            let r = def.radius * radius;
            let total = dmg * def.duration;
            let place = if def.at == "self" {
                "Covers the ground around you".to_string()
            } else {
                format!("Lands on your target (or the thickest pack) within {} yd", w.n(num(range)))
            };
            out.push(format!(
                "{place} for {} sec: every enemy within {} yd takes {} each second, {} in all.",
                w.secs(def.duration),
                w.n(num(r)),
                w.damage(dmg, def.school),
                w.amount(total)
            ));
            if def.slow > 0.0 {
                out.push(format!("Slows them by {} while they stand in it.", w.pct(def.slow)));
            }
        }
        CardKind::Heal => {
            let mut s = format!(
                "Heals the most injured ally within {} yd for {} of their max health",
                w.n(num(if def.range > 0.0 { def.range } else { 40.0 })),
                w.pct(def.heal * m)
            );
            if def.targets > 1 {
                s.push_str(&format!(", then the next {} for a little less", w.n((def.targets - 1).to_string())));
            }
            s.push('.');
            out.push(s);
            out.push("Waits until someone is hurt.".to_string());
        }
        CardKind::Modifier => {
            let a = def.amount * m;
            out.push(match def.modifier.as_str() {
                "dot" => format!(
                    "Your auto attacks also deal {} over {} sec.",
                    w.damage(hit * a, def.school),
                    w.secs(def.secs)
                ),
                "burn" => format!(
                    "Your Fire missiles set enemies ablaze for {} over {} sec.",
                    w.damage(hit * a, 2),
                    w.secs(def.secs)
                ),
                "cleave" => format!("Your auto attacks also hit another enemy nearby for {} of the damage.", w.pct(a)),
                "radius" => format!("Your area spells cover {} more ground.", w.pct(a)),
                "cdr" => format!("All your spells recharge {} faster.", w.pct(1.0 - 1.0 / (1.0 + a))),
                "echo" => format!("Your spells have a {} chance to cast twice.", w.pct(a.min(1.0))),
                "split" => format!(
                    "Your missiles burst on impact, hitting {} more enemies nearby for {} of the damage.",
                    w.n("2".into()),
                    w.pct(a)
                ),
                "windfury" => format!("Your auto attacks have a {} chance to strike again at once.", w.pct(a)),
                other => other.to_string(),
            });
        }
        CardKind::Stat => {
            let a = def.amount * m;
            out.push(match def.stat.as_str() {
                "damage" => format!("You deal {} more damage with everything.", w.pct(a)),
                "haste" => format!("You attack and cast {} faster.", w.pct(a)),
                "crit" => format!(
                    "{} critical strike chance. Critical strikes deal {} damage.",
                    w.n(format!("+{}", pct_str(a))),
                    w.n("150%".into())
                ),
                "health" => format!("{} max health.", w.n(format!("+{}", pct_str(a)))),
                "regen" => format!("Regenerate {} of your max health every second.", w.pct(a)),
                "speed" => format!("You move {} faster.", w.pct(a)),
                "per_wave" => format!("You deal {} more damage for every wave reached.", w.pct(a)),
                "avatar" => format!("You deal {} more damage and have {} more max health.", w.pct(a), w.pct(a)),
                "leech" => format!("All your damage heals you for {} of it.", w.pct(a)),
                "execute" => format!("You deal {} more damage to enemies below 25% health.", w.pct(a)),
                other => other.to_string(),
            });
        }
        CardKind::Trigger => {
            let a = def.amount * m;
            out.push(match def.trigger.as_str() {
                "lightning_shield" => format!(
                    "The first {} enemies to hit you each wave take {}.",
                    w.n("3".into()),
                    w.damage(hit * a, 3)
                ),
                "bloodthirst" => format!("Your critical strikes heal you for {} of your max health.", w.pct(a)),
                "divine_shield" => format!(
                    "When you drop below 35% health, you become immune to damage for {} sec. Once every {} sec.",
                    w.secs(a),
                    w.secs(def.cooldown)
                ),
                "raise_dead" => format!(
                    "Enemies you kill have a {} chance to rise as a skeleton that fights for you.",
                    w.pct(a)
                ),
                "soul_harvest" => format!("Every 10 kills this run (up to 2,000), you deal {} more damage.", w.pct(a)),
                "undying" => format!(
                    "Once a wave, a killing blow leaves you at {} health instead.",
                    w.n("30%".into())
                ),
                "shield" => format!(
                    "A shield absorbs damage up to {} of your max health. It renews {} sec after breaking.",
                    w.pct(a),
                    w.secs(def.cooldown.max(3.0))
                ),
                "thorns" => format!("Enemies that hit you take {} each time.", w.damage(hit * a, 3)),
                other => other.to_string(),
            });
        }
        CardKind::Team => {
            let a = def.amount * m;
            let r = w.n(num(def.radius));
            out.push(match def.team.as_str() {
                "damage_taken" => format!("You and allies within {r} yd take {} less damage.", w.pct(a)),
                "damage" => format!("You and allies within {r} yd deal {} more damage.", w.pct(a)),
                "haste" => format!("You and allies within {r} yd attack and cast {} faster.", w.pct(a)),
                "regen" => format!(
                    "You and allies within {r} yd regenerate {} of max health every second.",
                    w.pct(a)
                ),
                other => other.to_string(),
            });
        }
    }
    out.join(" ")
}

/// Slows, roots, leech and execute, as sentences.
fn hit_effects(def: &CardDef, several: bool, out: &mut Vec<String>, w: &Writer) {
    let them = if several { "them" } else { "it" };
    if def.slow > 0.0 {
        let secs = if def.slow_secs > 0.0 { def.slow_secs } else { 3.0 };
        out.push(format!("Slows {them} by {} for {} sec.", w.pct(def.slow), w.secs(secs)));
    }
    if def.root_secs > 0.0 {
        out.push(format!("Roots {them} in place for {} sec.", w.secs(def.root_secs)));
    }
    if def.leech > 0.0 {
        out.push(format!("Heals you for {} of the damage.", w.pct(def.leech)));
    }
    if def.execute > 0.0 {
        out.push(format!("Deals {} more to enemies below 25% health.", w.pct(def.execute)));
    }
}

/// What going from `rank - 1` to `rank` changes, as "before → after what".
fn upgrade(def: &CardDef, rank: u8, hit: f32, w: &Writer) -> String {
    let (m0, m1) = (def.rank_mult(rank - 1), def.rank_mult(rank));
    let arrow = |a: String, b: String, what: &str| format!("{} → {} {what}", a, w.n(b));
    let pct = |v: f32| pct_str(v);
    match def.kind {
        CardKind::Bolt | CardKind::Nova | CardKind::Zone if def.damage > 0.0 => {
            arrow(num(hit * def.damage * m0), num(hit * def.damage * m1), "damage")
        }
        CardKind::Bolt | CardKind::Nova if def.dot > 0.0 => {
            arrow(num(hit * def.dot * m0), num(hit * def.dot * m1), "damage")
        }
        CardKind::Heal | CardKind::Nova if def.heal > 0.0 => arrow(pct(def.heal * m0), pct(def.heal * m1), "healing"),
        CardKind::Modifier if matches!(def.modifier.as_str(), "dot" | "burn") => {
            arrow(num(hit * def.amount * m0), num(hit * def.amount * m1), "damage")
        }
        CardKind::Modifier if def.modifier == "cdr" => arrow(
            pct(1.0 - 1.0 / (1.0 + def.amount * m0)),
            pct(1.0 - 1.0 / (1.0 + def.amount * m1)),
            "faster",
        ),
        CardKind::Trigger if matches!(def.trigger.as_str(), "lightning_shield" | "thorns") => {
            arrow(num(hit * def.amount * m0), num(hit * def.amount * m1), "damage")
        }
        CardKind::Trigger if def.trigger == "divine_shield" => {
            arrow(num(def.amount * m0), num(def.amount * m1), "sec")
        }
        _ => arrow(pct(def.amount * m0), pct(def.amount * m1), ""),
    }
    .trim_end()
    .to_string()
}

// --- class spells ---------------------------------------------------------------------------------

use crate::kit::{Nums, PerkDef, SpellDef, SpellKind};

/// A class spell as it stands (upgrades and talents in `n`), for a caster whose hit is `hit`.
pub fn spell_text(def: &SpellDef, n: &Nums, hit: f32, color: bool) -> CardText {
    let w = Writer { color };
    let hit = hit.max(1.0) * (1.0 + n.power);
    let mut out: Vec<String> = Vec::new();
    let range = if n.range > 0.0 { n.range } else { 30.0 };
    let dmg = hit * n.damage;
    let school = def.school;
    let target_phrase = |several: bool| if several { "them" } else { "it" };
    let mut several = false;
    match def.kind {
        SpellKind::Bolt => {
            if n.mark > 0.0 {
                out.push(format!("Marks your target within {} yd: it takes {} more damage from you and your pet.", w.n(num(range)), w.pct(n.mark)));
                if n.crit > 0.0 {
                    out.push(format!("You have {} more critical strike chance against it.", w.pct(n.crit)));
                }
                if n.buff_value > 0.0 {
                    out.push(format!("Your pet goes for it and deals {} more to it.", w.pct(n.buff_value)));
                }
            } else {
                let melee = range <= 8.0;
                let at = if melee { "your target in melee".to_string() } else { format!("your target within {} yd", w.n(num(range))) };
                let extra = n.targets.round() as i64;
                several = extra > 0 || n.chain > 0.0;
                if n.damage <= 0.0 && n.dot > 0.0 {
                    let who = if extra > 0 { format!("{at} and {} more enemies near it", w.n(extra.to_string())) } else { at };
                    out.push(format!("Afflicts {who} with {} over {} sec.", w.damage(hit * n.dot, school), w.secs(n.dot_secs)));
                } else if n.damage > 0.0 {
                    let missiles = n.projectiles.round() as i64;
                    let mut s = if missiles > 1 {
                        format!("Launches {} {} at {at}, {} each", w.n(missiles.to_string()), if def.key == "mind_flay" { "waves" } else { "missiles" }, w.damage(dmg, school))
                    } else if extra > 0 {
                        format!("Strikes {at} and {} more enemies near it for {} each", w.n(extra.to_string()), w.damage(dmg, school))
                    } else {
                        format!("Strikes {at} for {}", w.damage(dmg, school))
                    };
                    if n.chain > 0.0 {
                        let weaker = 1.0 - if n.falloff > 0.0 { n.falloff } else { 0.8 };
                        s.push_str(&format!(", then leaps to {} more, {} weaker each jump", w.n(num(n.chain)), w.pct(weaker)));
                    }
                    s.push('.');
                    out.push(s);
                    if n.dot > 0.0 {
                        out.push(format!("Then {} over {} sec.", w.damage(hit * n.dot, school), w.secs(n.dot_secs)));
                    }
                }
                if n.only_below > 0.0 {
                    out.push(format!("Only against enemies below {} health.", w.pct(n.only_below)));
                }
            }
        }
        SpellKind::Nova | SpellKind::Cone => {
            several = true;
            let r = w.n(num(n.radius));
            let place = if def.kind == SpellKind::Cone { format!("in front of you (up to {r} yd)") } else { format!("within {r} yd of you") };
            if n.damage > 0.0 {
                out.push(format!("Blasts every enemy {place} for {}.", w.damage(dmg, school)));
            }
            if n.taunt_secs > 0.0 {
                out.push(format!("Every enemy {place} turns on you for {} sec.", w.secs(n.taunt_secs)));
            }
            if n.damage <= 0.0 && n.fear_secs > 0.0 {
                out.push(format!("Every enemy {place} flees in terror for {} sec.", w.secs(n.fear_secs)));
            }
            if n.heal > 0.0 {
                out.push(format!("Heals you and allies within {r} yd for {} of their max health.", w.pct(n.heal * (1.0 + n.power))));
            }
            if n.dot > 0.0 {
                out.push(format!("Then {} over {} sec.", w.damage(hit * n.dot, school), w.secs(n.dot_secs)));
            }
        }
        SpellKind::Zone => {
            several = true;
            let r = w.n(num(n.radius));
            let dur = if n.duration > 0.0 { n.duration } else { 6.0 };
            if n.heal > 0.0 && n.tick <= 0.0 {
                out.push(format!("For {} sec, you and allies within {r} yd heal {} of max health every second.", w.secs(dur), w.pct(n.heal)));
            } else {
                let place = if def.target == "self" { "Covers the ground around you".to_string() } else { format!("Lands on your target (or the thickest pack) within {} yd", w.n(num(range))) };
                if n.damage > 0.0 {
                    out.push(format!("{place}: a blast for {} to every enemy within {r} yd.", w.damage(dmg, school)));
                    out.push(format!("Then for {} sec they take {} each second.", w.secs(dur), w.damage(hit * n.tick, school)));
                } else {
                    out.push(format!(
                        "{place} for {} sec: every enemy within {r} yd takes {} each second, {} in all.",
                        w.secs(dur),
                        w.damage(hit * n.tick, school),
                        w.amount(hit * n.tick * dur)
                    ));
                }
            }
        }
        SpellKind::Heal => {
            let who = if def.target == "self" {
                "Heals you".to_string()
            } else if n.targets > 1.0 {
                format!("Heals you and the {} most hurt allies within {} yd", w.n(num(n.targets)), w.n(num(range)))
            } else {
                format!("Heals you or the most hurt ally within {} yd", w.n(num(range)))
            };
            out.push(format!("{who} for {} of max health.", w.pct(n.heal * (1.0 + n.power))));
        }
        SpellKind::Hot => {
            let who = if n.targets > 1.0 { format!("you or the {} most hurt allies", w.n(num(n.targets))) } else { "you or the most hurt ally".to_string() };
            out.push(format!(
                "Heals {who} within {} yd for {} of max health over {} sec.",
                w.n(num(range)),
                w.pct(n.hot * (1.0 + n.power)),
                w.secs(if n.hot_secs > 0.0 { n.hot_secs } else { 12.0 })
            ));
        }
        SpellKind::Shield => {
            let who = if def.target == "self" { "Shields you".to_string() } else { format!("Shields you or the most hurt ally within {} yd", w.n(num(range))) };
            out.push(format!(
                "{who}, absorbing {} of max health for {} sec.",
                w.pct(n.absorb * (1.0 + n.power)),
                w.secs(if n.duration > 0.0 { n.duration } else { 15.0 })
            ));
            if n.reflect > 0.0 {
                out.push(format!("Strikes back for {} of what it absorbs.", w.pct(n.reflect)));
            }
            if n.burst > 0.0 {
                out.push(format!("When it breaks it bursts for {} to enemies within 8 yd.", w.damage(hit * n.burst, 1)));
            }
        }
        SpellKind::Trap => {
            several = true;
            let r = w.n(num(n.radius));
            if n.damage > 0.0 {
                out.push(format!("Set at your feet, it explodes when an enemy comes close: {} to every enemy within {r} yd.", w.damage(dmg, school)));
            } else {
                out.push(format!("Set at your feet, it springs when an enemy comes close, on every enemy within {r} yd."));
            }
            if n.tick > 0.0 && n.duration > 0.0 {
                out.push(format!("The ground burns for {} sec: {} each second.", w.secs(n.duration), w.damage(hit * n.tick, school)));
            }
        }
        SpellKind::Pet => {
            let mut s = format!("Your pet strikes your target for {}", w.damage(dmg * 0.0 + hit * n.damage, school));
            if n.targets > 0.0 {
                s.push_str(&format!(" and {} more next to it", w.n(num(n.targets))));
                several = true;
            }
            s.push('.');
            out.push(s);
            if n.dot > 0.0 {
                out.push(format!("Then {} over {} sec.", w.damage(hit * n.dot, school), w.secs(n.dot_secs)));
            }
        }
        SpellKind::Blink => {
            out.push(format!("Teleports you {} yd the way you face.", w.n(num(n.range))));
            if n.damage > 0.0 {
                out.push(format!("Where you stood, an explosion hits enemies within {} yd for {}.", w.n(num(n.radius)), w.damage(dmg, school)));
            }
        }
        SpellKind::Charge => {
            out.push(format!(
                "Rush to your target ({} to {} yd), strike it for {} and gain {} rage.",
                w.n(num(n.min_range)),
                w.n(num(range)),
                w.damage(dmg, school),
                w.n(num(n.rage))
            ));
            if n.radius > 0.0 {
                out.push(format!("The landing hits every enemy within {} yd.", w.n(num(n.radius))));
                several = true;
            }
        }
        SpellKind::Buff | SpellKind::Toggle | SpellKind::Special => {}
    }
    hit_effects_n(n, several, target_phrase(several), &mut out, &w, hit);
    if !def.note.is_empty() {
        out.push(highlight(&def.note, &w));
    }
    if n.cost > 0.0 {
        out.push(format!("Costs {} rage.", w.n(num(n.cost))));
    }
    if n.channel > 0.05 {
        out.push(format!("Channelled over {} sec: stand still, or the rest is lost.", w.secs(n.channel)));
    }
    if n.cast > 0.05 {
        out.push(format!("{} sec cast: stand still.", w.secs(n.cast)));
    }
    if n.echo > 0.0 && !matches!(def.kind, SpellKind::Buff | SpellKind::Toggle) {
        out.push(format!("{} chance to cast again for free.", w.pct(n.echo.min(1.0))));
    }
    CardText {
        text: out.join(" "),
        upgrade: String::new(),
        kind: if def.talent { "Talent spell" } else { "Spell" },
        school: school_name(school),
        cooldown: if def.kind == SpellKind::Toggle { 0.0 } else { n.cooldown },
    }
}

/// What a hit leaves behind, as sentences.
fn hit_effects_n(n: &Nums, _several: bool, them: &str, out: &mut Vec<String>, w: &Writer, hit: f32) {
    let them_cap = if them == "them" { "They" } else { "It" };
    if n.slow > 0.0 {
        let secs = if n.slow_secs > 0.0 { n.slow_secs } else { 3.0 };
        out.push(format!("Chills {them}: {} slower for {} sec.", w.pct(n.slow), w.secs(secs)));
    }
    if n.freeze_chance > 0.0 {
        let secs = if n.freeze_secs > 0.0 { n.freeze_secs } else { 3.0 };
        if n.freeze_chance >= 0.999 {
            out.push(format!("Freezes {them} for {} sec.", w.secs(secs)));
        } else {
            out.push(format!("{} chance to Freeze {them} for {} sec.", w.pct(n.freeze_chance), w.secs(secs)));
        }
    }
    if n.root_secs > 0.0 {
        if n.root_chance > 0.0 && n.root_chance < 0.999 {
            out.push(format!("{} chance to root {them} for {} sec.", w.pct(n.root_chance), w.secs(n.root_secs)));
        } else {
            out.push(format!("Roots {them} for {} sec.", w.secs(n.root_secs)));
        }
    }
    if n.stun_chance > 0.0 {
        let secs = if n.stun_secs > 0.0 { n.stun_secs } else { 2.0 };
        if n.stun_chance >= 0.999 {
            out.push(format!("Stuns {them} for {} sec.", w.secs(secs)));
        } else {
            out.push(format!("{} chance to stun {them} for {} sec.", w.pct(n.stun_chance), w.secs(secs)));
        }
    }
    if n.fear_secs > 0.0 && n.damage > 0.0 {
        out.push(format!("{them_cap} flee in terror for {} sec.", w.secs(n.fear_secs)));
    }
    for (amount, name, what) in [
        (n.vuln_fire, "Fire Vulnerability", "+4% fire damage taken per stack, up to 5"),
        (n.vuln_frost, "Winter's Chill", "+2% frost critical strike chance against it per stack, up to 5"),
        (n.vuln_shadow, "Shadow Vulnerability", "+4% shadow damage taken per stack, up to 5"),
        (n.vuln_sunder, "Sunder Armor", "+5% physical damage taken per stack, up to 5"),
    ] {
        if amount <= 0.0 {
            continue;
        }
        let whole = amount.floor();
        let frac = amount - whole;
        let s = if frac < 0.01 {
            format!("Leaves {} {name} ({what}).", w.n(format!("{} {}", num(whole), if whole > 1.0 { "stacks of" } else { "stack of" })))
        } else if whole < 1.0 {
            format!("{} chance to leave a stack of {name} ({what}).", w.pct(frac))
        } else {
            format!("Leaves {} {name}, {} chance of one more ({what}).", w.n(format!("{} stacks of", num(whole))), w.pct(frac))
        };
        out.push(s);
    }
    if n.demoralize > 0.0 {
        out.push(format!("{them_cap} deal {} less damage for {} sec.", w.pct(n.demoralize), w.secs(n.demoralize_secs.max(1.0))));
    }
    if n.splash > 0.01 {
        out.push(format!("{} of the damage splashes onto enemies within {} yd.", w.pct(n.splash), w.n(num(if n.splash_radius > 0.0 { n.splash_radius } else { 8.0 }))));
    }
    if n.leech > 0.0 {
        out.push(format!("Heals you for {} of the damage.", w.pct(n.leech)));
    }
    if n.mend > 0.0 {
        out.push(format!("Heals the most hurt ally near you for {} of the damage.", w.pct(n.mend)));
    }
    if n.heal_self > 0.0 {
        out.push(format!("Heals you {} of your max health.", w.pct(n.heal_self)));
    }
    if n.heal_per_hit > 0.0 {
        out.push(format!("Heals you {} of your max health for each enemy hit.", w.pct(n.heal_per_hit)));
    }
    if n.ward > 0.0 {
        out.push(format!("Each enemy frozen wraps you in ice that absorbs {} of your max health.", w.pct(n.ward)));
    }
    for (v, what) in [
        (n.vs_frozen, "Frozen enemies"),
        (n.vs_chilled, "enemies already Chilled or slowed"),
        (n.vs_burning, "burning enemies"),
        (n.vs_bleeding, "bleeding enemies"),
        (n.vs_afflicted, "enemies in Shadow Word: Pain"),
        (n.vs_stung, "poisoned enemies"),
        (n.vs_stunned, "stunned enemies"),
    ] {
        if v > 0.0 {
            let what = if what.starts_with("Frozen") { what.to_string() } else { what.to_string() };
            out.push(format!("Deals {} more to {what}.", w.pct(v)));
        }
    }
    if n.vs_sunder > 0.0 {
        out.push(format!("Deals {} more per Sunder Armor stack on the target.", w.pct(n.vs_sunder)));
    }
    if n.crit_vs_frozen > 0.0 {
        out.push(format!("{} more critical strike chance against Frozen enemies.", w.pct(n.crit_vs_frozen)));
    }
    if n.execute > 0.0 {
        out.push(format!("Deals {} more to enemies below {} health.", w.pct(n.execute), w.pct(n.execute_at.max(0.2))));
    }
    if n.crit > 0.0 && n.mark <= 0.0 {
        out.push(format!("{} more critical strike chance.", w.pct(n.crit)));
    }
    if n.crit_mult > 0.0 {
        out.push(format!("Its critical strikes deal {} more.", w.pct(n.crit_mult / 1.5)));
    }
    if n.spread > 0.0 {
        out.push(format!("Spreads the target's burns to {} enemies nearby.", w.n(num(n.spread))));
    }
    if n.spread_on_death > 0.0 {
        out.push(format!("When an afflicted enemy dies, it jumps to {} more nearby.", w.n(num(n.spread_on_death))));
    }
    if n.detonate > 0.0 {
        out.push(format!("Sets off the target's poison: {} of what's left lands at once.", w.pct(n.detonate)));
    }
    if n.cdr_per_hit > 0.0 {
        out.push(format!("Each enemy hit takes {} sec off your other cooldowns.", w.secs(n.cdr_per_hit)));
    }
    if n.refund > 0.0 {
        out.push(format!("Its critical strikes refund {} rage.", w.n(num(n.refund))));
    }
    if n.rage_on_kill > 0.0 {
        out.push(format!("A kill refunds {} rage.", w.n(num(n.rage_on_kill))));
    }
    let _ = hit;
}

/// Numbers in hand-written text, highlighted.
fn highlight(text: &str, w: &Writer) -> String {
    if !w.color {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let mut j = i;
            while j < chars.len() && (chars[j].is_ascii_digit() || (chars[j] == '.' && j + 1 < chars.len() && chars[j + 1].is_ascii_digit())) {
                j += 1;
            }
            if j < chars.len() && chars[j] == '%' {
                j += 1;
            }
            let n: String = chars[i..j].iter().collect();
            out.push_str(&w.n(n));
            i = j;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// An upgrade card: what it adds to the spell, at the rank it brings the spell to.
pub fn perk_text(def: &SpellDef, perk: &PerkDef, rank: u8, color: bool) -> CardText {
    perk_text_with(def, perk, rank, 0.1, 0.0, color)
}

/// An upgrade card's text, with the power (and rage discount) every upgrade of the kit brings.
pub fn perk_text_with(def: &SpellDef, perk: &PerkDef, rank: u8, power: f32, cost_cut: f32, color: bool) -> CardText {
    let w = Writer { color };
    let mut upgrade = format!("{} rank {} → {}", def.name, rank - 1, rank);
    let hits = !matches!(def.kind, SpellKind::Buff | SpellKind::Toggle | SpellKind::Special);
    if hits && power > 0.0 {
        upgrade.push_str(&format!(", {} more power", pct_str(power)));
    }
    if cost_cut > 0.0 && def.nums.cost > 0.0 {
        upgrade.push_str(&format!(", {} less rage", pct_str(cost_cut)));
    }
    CardText {
        text: format!("{}: {}", def.name, highlight(&perk.text, &w)),
        upgrade,
        kind: "Upgrade",
        school: school_name(def.school),
        cooldown: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Content;

    #[test]
    fn every_spell_and_perk_reads_as_sentences_with_numbers() {
        let content = Content::builtin();
        for kit in &content.kits {
            for def in &kit.spells {
                let t = spell_text(def, &def.nums, 40.0, false);
                assert!(t.text.ends_with('.'), "{}: {:?}", def.key, t.text);
                let numbers = !matches!(def.kind, SpellKind::Buff | SpellKind::Toggle | SpellKind::Special);
                assert!(!numbers || t.text.chars().any(|c| c.is_ascii_digit()), "{}: {:?}", def.key, t.text);
                assert!(!t.text.contains("  "), "{}: {:?}", def.key, t.text);
                assert!(!t.text.contains('~') && !t.text.contains('^') && !t.text.contains(';'), "{}", def.key);
                let colored = spell_text(def, &def.nums, 40.0, true);
                assert!(!numbers || colored.text.contains("|cffffffff"), "{}", def.key);
                for perk in &def.perks {
                    let p = perk_text(def, perk, 2, true);
                    assert!(p.text.ends_with('.'), "{}: {:?}", perk.key, p.text);
                }
            }
        }
        for def in &content.cards {
            for rank in 1..=def.max_rank() {
                let t = card_text(def, rank, 40.0, 1.0, false);
                assert!(t.text.ends_with('.'), "{}: {:?}", def.key, t.text);
            }
        }
        let mage = &content.kits[content.class_index("mage").unwrap()];
        let frostbolt = &mage.spells[mage.spell_index("frostbolt").unwrap()];
        let t = spell_text(frostbolt, &frostbolt.nums, 60.0, false);
        assert!(t.text.starts_with("Strikes your target within 30 yd for 90 Frost damage. Chills it: 30% slower for 4 sec."), "{}", t.text);
    }
}

#[cfg(test)]
mod dump {
    use super::*;
    use crate::content::Content;

    #[test]
    #[ignore]
    fn dump_spell_text() {
        let content = Content::builtin();
        for kit in &content.kits {
            for def in &kit.spells {
                println!("{}: {}", def.key, spell_text(def, &def.nums, 40.0, false).text);
                for perk in &def.perks {
                    println!("    {} -- {}", perk.key, perk_text(def, perk, 2, false).text);
                }
            }
            for t in &kit.talents {
                println!("  T {}: {}", t.key, t.text_at(t.ranks));
            }
        }
    }
}
