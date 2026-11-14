//! A hero who plays the Deadmines by its tells alone: a player with a human's reaction and no floor
//! markers, who knows the game. They parry a blade they can block, roll a heavy blow (its weapon
//! glows red) and a bolt (timed from its cast), bash a cast within reach aside, keep a roll's
//! stamina back, wait out a boss's string (a parry doesn't stop a boss), and otherwise swing at
//! the nearest foe with the starting sword. Every pack is fought this way, five times over, and
//! the report says what each costs a hero who plays well, and why what landed landed: too quick
//! to answer (the game's fault), or caught committed (the player's choice).
//! (Corey, Nov 7: "playable without the floor markers".)
//!
//! `cargo test --release -p htl-survival fair_hero_report -- --ignored --nocapture` prints it;
//! `FAIR_HERO_FOES=dm_smite FAIR_HERO_TRACE=1 cargo test --release -p htl-survival fair_hero_one
//! -- --ignored --nocapture` tells one fight moment by moment.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::{Foe, MobState};
use crate::sim::*;
use crate::tests_fair::{FOLLOW_REACTION, REACTION};

const DT: f32 = 0.02;
/// The longest a fight is played before it's called (a fight that long is a finding of its own).
const CAP: f32 = 90.0;

/// A blow the hero has seen coming: when it lands, when they saw it, and whether they've answered.
struct Threat {
    land: f32,
    seen: f32,
    from: Id,
    /// A blockable blade: parried (the guard raised just before it lands), not dodged.
    parry: bool,
    pressed: bool,
}

/// How long before a blow lands the hero raises the guard to parry it (inside `[parry] window`).
const PARRY_LEAD: f32 = 0.12;
/// How close a caster must be for the shield bash to reach it (the bash's 2.5 yd, less a little).
const BASH_REACH: f32 = 2.2;

/// How well the hero plays.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Profile {
    /// From a wind-up starting (or a bolt flying) to the hero acting on it.
    pub reaction: f32,
    /// ...for a string's follow-up, its first already seen.
    pub follow: f32,
    /// The share of blows misread: not answered at all.
    pub miss: f32,
    /// Waits out a boss's string and keeps a roll's stamina back.
    pub patient: bool,
}

/// One who knows the game and plays it cleanly.
pub(crate) const CAREFUL: Profile = Profile { reaction: REACTION, follow: FOLLOW_REACTION, miss: 0.0, patient: true };
/// One still learning it: slower, misreads a quarter of the blows, swings when they shouldn't.
pub(crate) const LEARNING: Profile = Profile { reaction: 0.45, follow: 0.3, miss: 0.25, patient: false };

/// What one fight cost.
#[derive(Default, Debug)]
pub(crate) struct Outcome {
    pub secs: f32,
    pub cleared: bool,
    pub damage: u32,
    /// Blows (and bolts) that landed on the hero.
    pub landed: u32,
    /// ...of which the hero saw coming too late to dodge (the wind-up, less the reaction, shorter
    /// than a dodge takes to evade).
    pub too_quick: u32,
    /// ...of which the hero was inside their own swing, or out of stamina, when they meant to
    /// dodge.
    pub committed: u32,
    /// Blows the hero answered: dodged clean, or parried.
    pub dodged: u32,
    pub parried: u32,
    /// Casts knocked aside with a shield bash (or a kick) before they went.
    pub bashed: u32,
}

fn world(seed: u64) -> (Survival, Id) {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), seed, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    let p = s.join("ayla", "Ayla", None);
    s.move_player(p, [0.0, 0.0, 0.0], 0.0, Moving::default());
    let _ = s.step(0.0);
    (s, p)
}

/// One fight: these creatures (by key) set 10 yd ahead of a fresh hero, played to the end.
pub(crate) fn fight(foes: &[&str], seed: u64, who: Profile) -> Outcome {
    let (reaction, follow) = (who.reaction, who.follow);
    // The hero's own dice (a misread), apart from the world's.
    let mut dice = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let mut misread = move || {
        dice ^= dice << 13;
        dice ^= dice >> 7;
        dice ^= dice << 17;
        ((dice >> 11) as f64 / (1u64 << 53) as f64) < who.miss as f64
    };
    let (mut s, p) = world(seed);
    if let Some(h) = s.players.get_mut(&p) {
        // Played to the end however it goes: what it cost is the measure, against `max`.
        h.max_health = 1_000_000;
        h.health = 1_000_000;
    }
    // `FAIR_HERO_WEAPON=maul`: the hero fights with that weapon instead.
    if let Ok(w) = std::env::var("FAIR_HERO_WEAPON") {
        let i = s.content.weapons.iter().position(|x| x.key == w).unwrap_or_else(|| panic!("no weapon {w}"));
        s.players.get_mut(&p).unwrap().weapon = i;
    }
    let mut ids = Vec::new();
    for (i, key) in foes.iter().enumerate() {
        let def = s.content.mob(key).unwrap_or_else(|| panic!("no creature {key}"));
        let side = (i as f32 - (foes.len() as f32 - 1.0) / 2.0) * 1.8;
        let id = s.spawn_mob(def, [10.0, side, 0.0], None, false);
        s.mobs.get_mut(&id).unwrap().state = MobState::Chase { foe: Foe::Hero(p) };
        ids.push(id);
    }
    let (evade_from, evade_to) = (s.content.dodge.evade_from, s.content.dodge.evade_to);
    let mid = 0.5 * (evade_from + evade_to);
    let weapon = s.weapon_of(&s.players[&p]).clone();
    let lock = weapon.windup + weapon.active + weapon.recovery * s.content.combat.cancel_after;
    let mut threats: Vec<Threat> = Vec::new();
    let mut seen_bolts: Vec<Id> = Vec::new();
    // Casts at the hero being wound up: the caster, when it's seen, when it goes.
    let mut casts: Vec<(Id, f32, f32)> = Vec::new();
    let mut bash_try: Option<Id>;
    // When each foe's last blow at the hero landed (or was answered).
    let mut landed_at: Vec<(Id, f32)> = Vec::new();
    let mut out = Outcome::default();
    let start = s.clock;
    // `FAIR_HERO_TRACE=1`: the fight told moment by moment.
    let trace = std::env::var("FAIR_HERO_TRACE").is_ok();
    while s.clock - start < CAP {
        let now = s.clock;
        let me = s.players[&p].pos;
        // Bolts and pots in the air at the hero: seen as they fly.
        for ms in &s.missiles {
            if !seen_bolts.contains(&ms.mark) && dist2d(ms.at, me) <= ms.radius + 1.0 {
                seen_bolts.push(ms.mark);
                // The cast it ends was watched: the bolt's real landing, seen as early as the cast.
                if let Some(t) = threats.iter_mut().find(|t| t.from == ms.owner && !t.parry && !t.pressed && t.seen <= now) {
                    t.land = ms.land_at;
                    continue;
                }
                if trace {
                    println!("{:6.2} bolt flies, lands at {:.2} within {:.1} yd, {:.1} yd from the hero", now - start, ms.land_at - start, ms.radius, dist2d(ms.at, me));
                }
                threats.push(Threat { land: ms.land_at, seen: now + reaction, from: ms.owner, parry: false, pressed: misread() });
            }
        }
        threats.retain(|t| t.land + 0.3 > now);
        bash_try = None;
        casts.retain(|c| c.2 > now && s.mobs.get(&c.0).is_some_and(|m| matches!(m.state, MobState::Windup { .. })));
        // A cast in reach of the shield: bashed aside (its bolt with it).
        let bash = casts.iter().find(|c| now >= c.1 && s.mobs.get(&c.0).is_some_and(|m| dist2d(m.pos, me) <= BASH_REACH)).map(|c| c.0);
        if let Some(caster) = bash {
            if matches!(s.players[&p].action, Action::Ready | Action::Guard) {
                let at = s.mobs[&caster].pos;
                s.move_player(p, me, facing(me, at), Moving::default());
                s.input(p, Input::Guard);
                s.input(p, Input::Bash);
                s.input(p, Input::GuardUp);
                bash_try = Some(caster);
            }
        }
        // A blade it can block: parried, the guard raised just before it lands and let down after.
        // Anything else: a dodge timed so its evade covers the landing, as soon as it's seen.
        // (Pressed again each moment until it takes: a swing can't be called off at once.)
        let ready = s.players[&p].stamina >= s.content.dodge.needs();
        let rolling = matches!(s.players[&p].action, Action::Dodge { .. });
        let guarding = matches!(s.players[&p].action, Action::Guard);
        let mut dodge = false;
        let mut parry: Option<Id> = None;
        for t in threats.iter_mut().filter(|t| !t.pressed && now >= t.seen) {
            if t.parry && now >= t.land - PARRY_LEAD {
                if guarding {
                    t.pressed = true;
                } else {
                    parry = Some(t.from);
                }
            } else if !t.parry && now >= t.land - mid {
                if rolling {
                    t.pressed = true;
                } else {
                    dodge = true;
                }
            }
        }
        let hold = threats.iter().any(|t| t.parry && t.pressed && now < t.land + 0.1);
        if let Some(from) = parry {
            if let Some(at) = s.mobs.get(&from).map(|m| m.pos) {
                s.move_player(p, me, facing(me, at), Moving::default());
            }
            s.input(p, Input::Guard);
        } else if guarding && !hold {
            s.input(p, Input::GuardUp);
        }
        let dodge = dodge && parry.is_none();
        if trace && dodge {
            println!("{:6.2} dodge pressed: {:?}, stamina {:.0}", now - start, s.players[&p].action, s.players[&p].stamina);
        }
        if dodge && ready {
            // Sideways, out of a bolt's ring as well as under a blade.
            let keys = Moving { left: true, ..Moving::default() };
            let f = s.players[&p].facing;
            s.move_player(p, me, f, keys);
            s.input(p, Input::Dodge);
        }
        // Otherwise swing at the nearest foe, unless a seen blow lands inside the swing.
        let near = ids.iter().filter(|m| s.mobs.get(m).is_some_and(|x| x.health > 0)).min_by(|a, b| {
            dist2d(s.mobs[a].pos, me).total_cmp(&dist2d(s.mobs[b].pos, me))
        });
        let Some(&target) = near else {
            out.cleared = true;
            break;
        };
        let tpos = s.mobs[&target].pos;
        let d = dist2d(tpos, me);
        // Nor while a foe in reach is mid-string: winding up, or a breath between its blows (a
        // boss isn't stopped by a parry, so its string goes on).
        // (A caster's cast is no string: it's struck through.)
        let mid_string = ids.iter().any(|m| {
            let strung = landed_at.iter().any(|(x, t)| x == m && now - *t < 0.6);
            !s.attack_now(*m).ranged
                && s.mobs.get(m).is_some_and(|x| {
                    x.health > 0
                        && dist2d(x.pos, me) <= 6.0
                        && match x.state {
                            MobState::Windup { .. } => strung,
                            MobState::Recover { until, .. } => until - now < 0.35,
                            _ => false,
                        }
                })
        });
        // And keeps a roll's worth of stamina back (a swing costs about 17 at its dearest).
        let spare = s.players[&p].stamina >= s.content.dodge.needs() + 17.0;
        let busy = (who.patient && (!spare || mid_string)) || threats.iter().any(|t| t.land > now && t.land - now < lock + mid);
        if matches!(s.players[&p].action, Action::Ready) && !dodge && parry.is_none() {
            let f = facing(me, tpos);
            if d <= weapon.range + 0.5 {
                s.move_player(p, me, f, Moving::default());
                if !busy {
                    s.input(p, Input::Attack);
                }
            } else {
                // Walk to it (a caster keeps its distance).
                let speed = 7.0 * DT;
                let to = [me[0] + f.cos() * speed, me[1] + f.sin() * speed, me[2]];
                s.move_player(p, to, f, Moving { forward: true, ..Moving::default() });
            }
        }
        for e in s.step(DT) {
            // When each foe's blow at the hero last landed (or was answered): a string's next
            // blow is read faster.
            if let Event::Swing { attacker, victim: Some(v), .. } = &e {
                if *v == p && ids.contains(attacker) {
                    landed_at.retain(|(m, _)| m != attacker);
                    landed_at.push((*attacker, s.clock));
                }
            }
            match e {
                // A creature's cast (its bolt) is told as a cast, not a swing's wind-up.
                Event::Cast { caster, ms, .. } if ids.contains(&caster) => {
                    let at_me = s.mobs.get(&caster).is_some_and(|m| matches!(m.state, MobState::Windup { foe: Foe::Hero(h), .. } if h == p));
                    if at_me && !casts.iter().any(|c| c.0 == caster) {
                        let windup = ms as f32 / 1000.0;
                        if trace {
                            println!("{:6.2} {} casts {:.2} s", now - start, s.content.mobs[s.mobs[&caster].def].key, windup);
                        }
                        let a = s.attack_now(caster);
                        let far = dist2d(s.mobs[&caster].pos, s.players[&p].pos);
                        let flight = if a.speed > 0.0 { far / a.speed } else { 0.0 };
                        casts.push((caster, now + reaction, now + windup));
                        threats.push(Threat { land: now + windup + flight, seen: now + reaction, from: caster, parry: false, pressed: misread() });
                    }
                }
                Event::MobWindup { id, windup, blockable, .. } => {
                    let at_me = s.mobs.get(&id).is_some_and(|m| matches!(m.state, MobState::Windup { foe: Foe::Hero(h), .. } if h == p));
                    // A caster's bolt is seen when it flies (above); a blade from its wind-up.
                    if trace && at_me {
                        println!("{:6.2} {} winds up {:.2} s", now - start, s.content.mobs[s.mobs[&id].def].key, windup);
                    }
                    let a = s.attack_now(id);
                    // A string's follow-up, its first already seen: read faster.
                    let react = if landed_at.iter().any(|(m, t)| *m == id && now - *t < 0.6) { follow } else { reaction };
                    if at_me && !a.ranged {
                        // A heavy blow (its weapon glows red) tears through a guard: rolled, not parried.
                        threats.push(Threat { land: now + windup, seen: now + react, from: id, parry: blockable && !a.heavy, pressed: misread() });
                    } else if at_me && !casts.iter().any(|c| c.0 == id) {
                        // A cast at the hero: its bolt lands its flight after the cast. Up close the
                        // cast is bashed aside; further off, rolled out of at the bolt's landing.
                        let far = s.mobs.get(&id).map_or(0.0, |m| dist2d(m.pos, s.players[&p].pos));
                        let flight = if a.speed > 0.0 { far / a.speed } else { 0.0 };
                        casts.push((id, now + reaction, now + windup));
                        threats.push(Threat { land: now + windup + flight, seen: now + reaction, from: id, parry: false, pressed: misread() });
                    }
                }
                Event::Swing { attacker, victim: Some(v), damage, style, .. } if v == p && attacker != p && damage > 0 => {
                    out.damage += damage;
                    if trace {
                        println!("{:6.2} hit for {damage} ({style:?}), hero {:?}", s.clock - start, s.players[&p].action);
                    }
                    // A burn's tick isn't a blow.
                    if ids.contains(&attacker) && !matches!(style, Blow::Dot) {
                        out.landed += 1;
                        let t = threats.iter().filter(|t| t.from == attacker).min_by(|a, b| (a.land - s.clock).abs().total_cmp(&(b.land - s.clock).abs()));
                        match t {
                            Some(t) if t.land - t.seen < if t.parry { 0.05 } else { evade_from } => {
                                out.too_quick += 1;
                                if trace {
                                    println!("       too quick: landed {:.2} s after it was seen", t.land - t.seen);
                                }
                            }
                            _ => out.committed += 1,
                        }
                    }
                }
                Event::Swing { attacker, victim: Some(v), damage: 0, outcome, .. } if v == p && ids.contains(&attacker) => {
                    if outcome == crate::sim::Outcome::Parried {
                        out.parried += 1;
                    } else {
                        out.dodged += 1;
                    }
                }
                _ => {}
            }
        }
        // Knocked aside: out of its wind-up before the cast was due.
        if let Some(c) = bash_try {
            let due = casts.iter().find(|x| x.0 == c).map_or(0.0, |x| x.2);
            if s.clock < due && !s.mobs.get(&c).is_some_and(|m| matches!(m.state, MobState::Windup { .. })) {
                out.bashed += 1;
                casts.retain(|x| x.0 != c);
                threats.retain(|t| t.from != c || t.parry);
                if trace {
                    println!("{:6.2} bashed the cast", s.clock - start);
                }
            }
        }
    }
    out.secs = s.clock - start;
    out
}

/// The fights: each pack as it stands, and the pulls that bring another with them (the pull
/// report, `crates/htl-server/tests/pulls.rs`).
fn fights(c: &Content) -> Vec<(String, Vec<String>)> {
    let d = c.dungeon.as_ref().expect("the Deadmines");
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    for (i, pk) in d.packs.iter().enumerate() {
        let name = c.mob(&pk.mob).map_or(pk.mob.clone(), |m| c.mobs[m].name.clone());
        let what = if pk.key.is_empty() { format!("{}: {}", i + 1, name) } else { format!("{}: {} ({})", i + 1, name, pk.key) };
        out.push((what, vec![pk.mob.clone(); pk.count.max(1)]));
    }
    // The beach's first open fight: two pirates on patrol and the Squallshaper on their path.
    out.push(("beach: 2 pirates and a Squallshaper".into(), vec!["dm_pirate".into(), "dm_pirate".into(), "dm_squallshaper".into()]));
    out
}

/// Each pack's place in `deadmines.toml`: the `# --- ... ---` heading it stands under.
fn pack_areas() -> Vec<String> {
    let mut area = String::new();
    let mut out = Vec::new();
    for line in crate::content::DEFAULT_DUNGEON.lines() {
        if let Some(h) = line.strip_prefix("# --- ") {
            area = h.trim_end_matches(['-', ' ']).to_string();
        } else if line.trim() == "[[dungeon.pack]]" {
            out.push(area.clone());
        }
    }
    out
}

#[test]
#[ignore]
fn fair_hero_report() {
    let c = Content::dev();
    let (s, p) = world(1);
    let max = s.players[&p].max_health;
    println!("A hero with {max} health and no markers. Careful: {:.2} s reaction, parries and rolls what it sees,", CAREFUL.reaction);
    println!("waits out strings. Learning: {:.2} s, misreads {:.0}% of blows, swings when it shouldn't. Mean of 5 runs.", LEARNING.reaction, LEARNING.miss * 100.0);
    println!(
        "{:<40} | {:>5} {:>5} {:>5} {:>5} | {:>5} {:>5} {:>5}",
        "fight", "secs", "dmg", "quick", "busy", "secs", "dmg", "worst"
    );
    const SEEDS: u64 = 5;
    let areas = pack_areas();
    let mut by_area: Vec<(String, f32, f32)> = Vec::new();
    for (i, (what, foes)) in fights(&c).into_iter().enumerate() {
        let keys: Vec<&str> = foes.iter().map(String::as_str).collect();
        let careful: Vec<Outcome> = (1..=SEEDS).map(|seed| fight(&keys, seed, CAREFUL)).collect();
        let learning: Vec<Outcome> = (1..=SEEDS).map(|seed| fight(&keys, seed, LEARNING)).collect();
        let n = SEEDS as f32;
        let mean = |runs: &[Outcome], f: &dyn Fn(&Outcome) -> f32| runs.iter().map(f).sum::<f32>() / n;
        let pct = |d: f32| 100.0 * d / max as f32;
        let (cd, ld) = (mean(&careful, &|o| o.damage as f32), mean(&learning, &|o| o.damage as f32));
        let worst = learning.iter().map(|o| o.damage).max().unwrap_or(0) as f32;
        let uncleared = careful.iter().chain(&learning).filter(|o| !o.cleared).count();
        println!(
            "{:<40} | {:>5.0} {:>4.0}% {:>5.1} {:>5.1} | {:>5.0} {:>4.0}% {:>4.0}%{}",
            what,
            mean(&careful, &|o| o.secs),
            pct(cd),
            mean(&careful, &|o| o.too_quick as f32),
            mean(&careful, &|o| o.committed as f32),
            mean(&learning, &|o| o.secs),
            pct(ld),
            pct(worst),
            if uncleared > 0 { format!("  ({uncleared} of {} runs uncleared at {CAP:.0} s)", 2 * SEEDS) } else { String::new() }
        );
        if let Some(a) = areas.get(i) {
            match by_area.iter_mut().find(|x| x.0 == *a) {
                Some(x) => {
                    x.1 += cd;
                    x.2 += ld;
                }
                None => by_area.push((a.clone(), cd, ld)),
            }
        }
    }
    println!();
    println!("Every pack of each heading in deadmines.toml, fought once (a lamp gives {max} health and 3 Draughts of 40%):");
    for (a, cd, ld) in by_area {
        println!("{:<44} careful {:>5.0} ({:>3.0}%)   learning {:>5.0} ({:>3.0}%)", a, cd, 100.0 * cd / max as f32, ld, 100.0 * ld / max as f32);
    }
}

#[test]
fn a_hero_who_answers_what_they_see_beats_one_pirate_without_a_blow_too_quick_to_answer() {
    // (The hero's reaction is `tests_fair`'s REACTION and FOLLOW_REACTION: retuned there, this
    // can fail with nothing in the game changed.)
    for seed in 1..=3 {
        let o = fight(&["dm_pirate"], seed, CAREFUL);
        assert!(o.cleared, "one pirate falls: {o:?}");
        assert_eq!(o.too_quick, 0, "no pirate blow comes quicker than a hero can see and answer: {o:?}");
        assert!(o.parried > 0, "its blades are parried: {o:?}");
    }
}

#[test]
#[ignore]
fn fair_hero_one() {
    let foes = std::env::var("FAIR_HERO_FOES").unwrap_or_else(|_| "dm_squallshaper".into());
    let keys: Vec<&str> = foes.split(',').collect();
    let seed = std::env::var("FAIR_HERO_SEED").ok().and_then(|x| x.parse().ok()).unwrap_or(7);
    let who = if std::env::var("FAIR_HERO_LEARNING").is_ok() { LEARNING } else { CAREFUL };
    println!("{:?}", fight(&keys, seed, who));
}

