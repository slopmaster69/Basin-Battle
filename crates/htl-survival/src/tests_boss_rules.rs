//! The bosses' and rares' documented rules, held to the sim (docs/*-encounter.md, the comments in
//! content/*.toml): "parry the first and the rest never come", "too big to parry", and the tells
//! that must read with the floor markers off.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::Foe;
use crate::sim::*;

fn world() -> (Survival, Id) {
    let mut c = Content::builtin();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    let p = s.join("ayla", "Ayla", None);
    s.move_player(p, [0.0, 0.0, 0.0], 0.0, Moving::default());
    let _ = s.step(0.0);
    if let Some(h) = s.players.get_mut(&p) {
        h.max_health = 1_000_000;
        h.health = 1_000_000;
    }
    (s, p)
}

/// What a hero's timed guard on a string's first blow does to the rest of it.
#[derive(Debug, Default)]
pub(crate) struct Answer {
    /// The first blow's outcome.
    pub first: Option<Outcome>,
    /// Whether the creature reels after it.
    pub reels: bool,
    /// Whether the string was still going the moment after the first blow.
    pub string_goes_on: bool,
    /// Blows that landed or were answered after the first, within `AFTER` seconds.
    pub later: Vec<Outcome>,
    /// Wind-ups begun after the first blow, within `AFTER` seconds.
    pub windups: usize,
}

/// How long after the first blow the next blows of a string would have come (its recovery and
/// the second wind-up are well inside this; a reel is 1.4 s).
const AFTER: f32 = 0.9;

/// Parry (guard raised 0.1 s before it lands) the first blow of `key`'s string `string`, then
/// watch the next `AFTER` seconds.
pub(crate) fn parry_first(key: &str, string: &str) -> Answer {
    let (mut s, p) = world();
    let def = s.content.mob(key).unwrap_or_else(|| panic!("no {key}"));
    let k = s.content.mobs[def].patterns.iter().position(|x| x.name == string).unwrap_or_else(|| panic!("{key} has no {string}"));
    let m = s.spawn_mob(def, [2.0, 0.0, 0.0], None, false);
    {
        let x = s.mobs.get_mut(&m).unwrap();
        x.state = crate::sim::mobs::MobState::Chase { foe: Foe::Hero(p) };
        x.ready_at = 0.0;
    }
    s.begin_pattern(m, k, false);
    s.windup(m, Foe::Hero(p), s.players[&p].pos);
    let mut a = Answer::default();
    let mut raise_at: Option<f32> = None;
    let mut raised = false;
    let mut landed_at = None;
    for _ in 0..600 {
        if let (Some(t), false) = (raise_at, raised) {
            if s.clock >= t {
                s.input(p, Input::Guard);
                raised = true;
            }
        }
        for e in s.step(0.02) {
            match e {
                Event::MobWindup { id, windup, .. } if id == m && raise_at.is_none() => raise_at = Some(s.clock + windup - 0.1),
                Event::Swing { attacker, victim: Some(v), outcome, anim: false, .. } if attacker == m && v == p => {
                    if a.first.is_none() {
                        a.first = Some(outcome);
                        landed_at = Some(s.clock);
                        a.string_goes_on = s.mobs[&m].pattern.is_some();
                    } else if landed_at.is_some_and(|t| s.clock - t <= AFTER) {
                        a.later.push(outcome);
                    }
                }
                Event::MobWindup { id, .. } if id == m && landed_at.is_some_and(|t| s.clock - t <= AFTER) => a.windups += 1,
                _ => {}
            }
        }
        if landed_at.is_some_and(|t| s.clock - t > AFTER) {
            break;
        }
    }
    a.reels = s.mobs[&m].stagger_until > 0.0;
    a
}

/// Every `# comment` that sits straight above a `[[mob.pattern]]`, with the creature's key and the
/// pattern's name, read from the content files themselves.
fn commented_strings() -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for text in std::iter::once(crate::content::DEFAULT).chain(std::iter::once(crate::content::DEFAULT_DUNGEON)).chain(crate::content::MORE_DUNGEONS.iter().map(|f| f.1)) {
        let (mut key, mut note) = (String::new(), String::new());
        let mut lines = text.lines().peekable();
        let mut in_mob = false;
        while let Some(l) = lines.next() {
            if l.starts_with("[[mob]]") {
                in_mob = true;
                note.clear();
            } else if l.starts_with("[[") && !l.starts_with("[[mob.") {
                in_mob = false;
            }
            if in_mob && l.starts_with("key = \"") {
                key = l["key = \"".len()..].trim_end_matches('"').to_string();
            }
            let t = l.trim();
            if t.starts_with('#') {
                if !note.is_empty() {
                    note.push(' ');
                }
                note.push_str(t.trim_start_matches('#').trim());
                continue;
            }
            if in_mob && t.starts_with("[[mob.pattern]]") {
                let mut name = String::new();
                for l in lines.clone().take(4) {
                    if let Some(n) = l.trim().strip_prefix("name = \"") {
                        name = n.trim_end_matches('"').to_string();
                        break;
                    }
                }
                if !note.is_empty() {
                    out.push((key.clone(), name, note.clone()));
                }
            }
            note.clear();
        }
    }
    out
}

/// What a comment promises of a string, checked against the string's own numbers. `None` for a
/// comment that promises nothing checkable.
fn idiom_fault(c: &Content, mob: &str, p: &crate::content::PatternDef, note: &str) -> Option<String> {
    let n = note.to_lowercase();
    let st = &p.steps;
    let d = &c.mobs[c.mob(mob).unwrap()];
    let base = |i: usize| st[i].over(&d.attack);
    let first = if n.contains("closes the gap in one blow") || n.contains("in from a few strides off") {
        (st.len() != 1 || st[0].lunge <= 0.0 || p.min_range < 3.0).then(|| "one lunging blow from range")
    } else if n.contains("held back to catch") || n.contains("delayed overhead") {
        (!st.iter().any(|s| s.poise == Some(true) || s.windup.is_some_and(|w| w >= 1.0))).then(|| "a held, delayed blow")
    } else if n.contains("three in a row") {
        (st.len() != 3 || base(2).windup < base(0).windup.max(base(1).windup) || base(2).damage <= base(0).damage).then(|| "two quick then a heavy third")
    } else if n.contains("a blow all the way round") || n.contains("a burst all round") {
        (!st.iter().any(|s| s.arc.is_some_and(|a| a >= 300.0))).then(|| "a blow all round")
    } else if n.contains("three bolts") {
        (st.len() != 3 || !st.iter().all(|s| base(0).ranged || s.ranged == Some(true))).then(|| "three bolts")
    } else if n.contains("marked as it winds up") {
        let late = n.contains("nearer its end");
        (!st.iter().any(|s| s.ranged == Some(true) && s.radius.unwrap_or(d.attack.radius) >= crate::sim::mobs::LOB_RADIUS) || (late && p.below >= 1.0)).then(|| "a marked throw (late in the fight, if it says so)")
    } else {
        None
    };
    first.map(|w| format!("{mob}/{}: \"{note}\" but it isn't {w}", p.name))
}

/// The creatures whose comment (above them or inside them) says "too big to parry".
fn too_big_comments() -> Vec<String> {
    let mut out = Vec::new();
    for text in std::iter::once(crate::content::DEFAULT).chain(std::iter::once(crate::content::DEFAULT_DUNGEON)).chain(crate::content::MORE_DUNGEONS.iter().map(|f| f.1)) {
        // (key, said so) of the block being read. Comments straight above a `[[mob]]` are its, not
        // the block before's.
        let mut block: Option<(String, bool)> = None;
        let mut pending = false;
        let done = |b: Option<(String, bool)>, out: &mut Vec<String>| {
            if let Some((k, true)) = b {
                out.push(k);
            }
        };
        for l in text.lines() {
            if l.trim().starts_with('#') {
                pending = pending || l.to_lowercase().contains("too big to parry");
                continue;
            }
            if l.trim().is_empty() {
                continue;
            }
            if l.starts_with("[[mob]]") {
                done(block.take(), &mut out);
                block = Some((String::new(), pending));
            } else if l.starts_with("[[") && !l.starts_with("[[mob.") {
                done(block.take(), &mut out);
            } else if let Some((k, s)) = block.as_mut() {
                *s = *s || pending;
                if l.starts_with("key = \"") {
                    *k = l["key = \"".len()..].trim_end_matches('"').to_string();
                }
            }
            pending = false;
        }
        done(block.take(), &mut out);
    }
    out
}

#[test]
fn every_commented_string_holds_what_its_comment_promises() {
    let c = Content::builtin();
    let all = commented_strings();
    assert!(all.len() > 300, "the scan reads the content files ({} strings)", all.len());
    let mut faults = Vec::new();
    for (k, n, note) in &all {
        let d = c.mob(k).unwrap_or_else(|| panic!("a comment above {k}'s {n}, and no such creature"));
        let p = c.mobs[d].patterns.iter().find(|p| &p.name == n).unwrap_or_else(|| panic!("{k} has no {n}"));
        faults.extend(idiom_fault(&c, k, p, note));
    }
    assert!(faults.is_empty(), "{}", faults.join("\n"));
}

#[test]
fn parry_the_first_and_the_rest_never_come_for_every_string_that_says_so() {
    let c = Content::builtin();
    let mut seen = 0;
    for (k, n, note) in commented_strings() {
        let lower = note.to_lowercase();
        if !lower.contains("parry the first") {
            continue;
        }
        seen += 1;
        let d = &c.mobs[c.mob(&k).unwrap()];
        let p = d.patterns.iter().find(|p| p.name == n).unwrap();
        assert!(p.steps.len() >= 2, "{k}/{n} is a string");
        let a = parry_first(&k, &n);
        assert_eq!(a.first, Some(Outcome::Parried), "{k}/{n}: the timed guard parries the first blow");
        assert!(!a.string_goes_on && a.later.is_empty(), "{k}/{n}: after the parry the rest of the string never comes: {a:?}");
    }
    assert!(seen >= 8, "the scan found the rule's strings ({seen})");
}

#[test]
fn a_boss_whose_string_a_parry_ends_still_does_not_reel_from_it() {
    // Corey's rule for bosses (Nov 2): a parry fills a boss's posture, it never stun-locks it. The
    // strings that say "parry the first" end, and that is all they do.
    let c = Content::builtin();
    for (k, n) in [("goldtooth", "flurry"), ("morgan", "flurry"), ("bh_paymaster", "stabs"), ("marzon", "stabs")] {
        let d = &c.mobs[c.mob(k).unwrap()];
        assert!(d.boss && d.posture > 0, "{k} is a boss with a posture");
        assert!(d.patterns.iter().find(|p| p.name == n).unwrap().parry_ends, "{k}/{n} is marked");
        let a = parry_first(k, n);
        assert!(!a.reels, "{k} doesn't reel: {a:?}");
    }
}

#[test]
fn too_big_to_parry_means_a_timed_guard_only_blocks_and_it_goes_on() {
    let c = Content::builtin();
    let claimed = too_big_comments();
    assert!(claimed.len() >= 6, "the scan found the giants ({})", claimed.len());
    for k in &claimed {
        assert!(c.mobs[c.mob(k).unwrap()].unparryable, "{k}'s comment says too big to parry, its data doesn't");
    }
    for d in c.mobs.iter().filter(|d| d.unparryable && !d.inert()) {
        // A melee string it can throw from where it stands, if it has one.
        let Some(p) = d.patterns.iter().find(|p| p.weight > 0.0 && p.min_range < 3.0 && p.steps.iter().all(|s| s.ranged != Some(true)) && !d.attack.ranged) else { continue };
        let a = parry_first(&d.key, &p.name);
        assert_eq!(a.first, Some(Outcome::Blocked), "{}/{}: a timed guard only blocks: {a:?}", d.key, p.name);
        assert!(!a.reels, "{}: and it doesn't reel", d.key);
    }
}

#[test]
fn every_boss_blow_has_a_tell_that_needs_no_floor_marker() {
    // An animation (or the cast), and a Danger flash on the screen of the hero a blow no guard
    // stops comes for: none of them a marker on the floor.
    let c = Content::builtin();
    let mut faults = Vec::new();
    let mut steps = 0;
    let (mut s, hero) = world();
    for d in c.mobs.iter().filter(|d| d.boss && !d.inert()) {
        for (k, p) in d.patterns.iter().enumerate() {
            for (si, step) in p.steps.iter().enumerate() {
                s.mobs.clear();
                let def = s.content.mob(&d.key).unwrap();
                let m = s.spawn_mob(def, [2.0, 0.0, 0.0], None, false);
                {
                    let x = s.mobs.get_mut(&m).unwrap();
                    x.state = crate::sim::mobs::MobState::Chase { foe: Foe::Hero(hero) };
                    x.ready_at = 0.0;
                }
                s.begin_pattern(m, k, false);
                s.mobs.get_mut(&m).unwrap().pattern = Some((k, si));
                s.windup(m, Foe::Hero(hero), s.players[&hero].pos);
                let a = step.over(&d.attack);
                let ev = s.step(0.02);
                steps += 1;
                // (An animation or cast, or the creature's own pose: a guard raised, a held clip, a glow:
                // VanCleef's Invitation and the Shredder's grind are posed by their modules.)
                let animated = ev.iter().any(|e| match e {
                    Event::MobWindup { id, .. } | Event::Cast { caster: id, .. } | Event::React { id, .. } | Event::Glow { id, .. } | Event::HoldPose { id, .. } => *id == m,
                    Event::MobGuard { id, up } => *id == m && *up,
                    _ => false,
                });
                let flashed = ev.iter().any(|e| matches!(e, Event::Danger { player, .. } if *player == hero));
                let at = format!("{}/{}[{si}]", d.key, p.name);
                if !animated {
                    faults.push(format!("{at}: nothing animates or casts as it winds up"));
                }
                if !a.blockable && !a.ranged && !flashed {
                    faults.push(format!("{at}: a blow no guard stops comes with no Danger flash"));
                }
            }
        }
    }
    assert!(steps > 150, "swept {steps} steps");
    assert!(faults.is_empty(), "{}", faults.join("\n"));
}

#[test]
fn the_elwynn_champions_turn_at_the_health_their_comments_name_and_call_who_they_say() {
    // (key, the share of health it turns at, who comes)
    let table: [(&str, f32, &[&str]); 11] = [
        ("captain", 0.4, &[]),
        ("shadowhunter", 0.5, &[]),
        ("warlord", 0.5, &[]),
        ("snivvle", 0.5, &["kobold_vermin", "kobold_vermin"]),
        ("grizzlefin", 0.4, &["murloc_forager", "murloc_forager"]),
        ("hogger", 0.5, &["gnoll_runt", "gnoll_runt"]),
        ("morgan", 0.5, &["defias_footpad", "defias_footpad"]),
        ("morvane", 0.4, &["risen_skeleton", "risen_skeleton"]),
        ("vexx", 0.5, &["voidwalker", "imp"]),
        ("blackfang", 0.5, &["prowler", "prowler"]),
        ("rakgash", 0.5, &[]),
    ];
    for (key, at, who) in table {
        let (mut s, hero) = world();
        let def = s.content.mob(key).unwrap_or_else(|| panic!("no {key}"));
        let m = s.spawn_mob(def, [2.0, 0.0, 0.0], None, false);
        let e = s.content.mobs[def].phases.iter().find(|p| p.at == at).unwrap_or_else(|| panic!("{key} has no phase at {at}"));
        assert!(e.speed < 1.0, "{key} turns quicker");
        assert_eq!(e.summon, who.iter().map(|w| w.to_string()).collect::<Vec<_>>(), "{key} calls {who:?}");
        let (text, max) = (e.text.clone(), s.mobs[&m].max_health);
        // Just above the line: nothing yet. Over it: the turn, said and answered.
        s.hurt_mob(m, hero, (max as f32 * (1.0 - at - 0.02)) as u32);
        let before = s.step(0.1);
        assert!(!before.iter().any(|e| matches!(e, Event::Yell { id, .. } if *id == m)) || text.is_empty(), "{key} hasn't turned above its line");
        let n = s.mobs.len();
        s.hurt_mob(m, hero, (max as f32 * 0.05) as u32);
        // (A blow that breaks its stance staggers it, and a stagger holds the turn back: wait.)
        let mut said = false;
        for k in 0..120 {
            said |= s.step(0.1).iter().any(|e| matches!(e, Event::Yell { id, .. } if *id == m));
            if k % 20 == 19 && !said && s.mobs.len() == n {
                s.hurt_mob(m, hero, 1);
            }
        }
        assert!(said || text.is_empty(), "{key} says its words as it turns");
        assert_eq!(s.mobs.len(), n + who.len(), "{key} calls {who:?} as it turns");
    }
}
