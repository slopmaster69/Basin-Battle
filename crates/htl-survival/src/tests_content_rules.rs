//! Rules the content must keep, checked over every creature in every dungeon.

use crate::content::Content;

#[test]
fn no_blow_all_round_flies_as_a_missile() {
    // A caster's ring at its feet when crowded (arc 360, a few yards) written without
    // `ranged = false` took the caster's own `ranged = true`: it flew as a small, unblockable bolt
    // at point blank, with no ring to read. Twenty-five strings did, Nov 7 2026.
    let c = Content::builtin();
    let mut wrong = Vec::new();
    for d in &c.mobs {
        for p in &d.patterns {
            for (k, s) in p.steps.iter().enumerate() {
                // A step that sets its own reach all round, says nothing of being a missile (no
                // speed, no splash) and so takes a caster's `ranged`.
                let ring = s.arc.is_some_and(|a| a >= 300.0) && s.speed.is_none() && s.radius.is_none();
                if ring && s.ranged.is_none() && d.attack.ranged {
                    wrong.push(format!("{} \"{}\" step {}", d.key, p.name, k + 1));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "rings that fly as missiles (give the step ranged = false): {wrong:?}");
}

/// A number as a tooltip writes it: whole numbers bare ("8"), else as few decimals as say it ("1.5").
fn said(v: f32) -> String {
    if (v - v.round()).abs() < 0.005 { format!("{}", v.round() as i64) } else { format!("{v}") }
}

#[test]
fn the_new_ex_tooltips_say_the_numbers_the_code_uses() {
    let c = Content::dev();
    let get = |k: &str| &c.abilities[c.ability(k).unwrap()];
    let share = c.cc.player_share;
    let effect = |d: &crate::content::AbilityDef, kind: &str| d.effects.iter().find(|e| e.kind == kind).cloned().unwrap_or_else(|| panic!("{} has a {kind}", d.name));
    // Polymorph (Fireball's EX): how long a sheep lasts, and a hero.
    let d = get("pyroblast");
    assert_eq!(d.name, "Polymorph");
    let e = effect(d, "disorient");
    assert!(d.text.contains(&format!("sheep for {} sec ({} sec against heroes)", said(e.secs[1]), said(e.hero_secs))), "{}", d.text);
    // Blast Wave (Flamestrike's EX).
    let d = get("flash_flamestrike");
    let (k, s) = (effect(d, "knockback"), effect(d, "snare"));
    assert_eq!(d.kind, "nova", "it goes off round the Mage");
    for want in [format!("Deals {} damage", said(d.damage[1])), format!("within {} yd", said(d.radius[1])), format!("back {} yd", said(k.amount)), format!("by {}% for {} sec", said(s.amount * 100.0), said(s.secs[1]))] {
        assert!(d.text.contains(&want), "{want:?} in {}", d.text);
    }
    // Capacitor Totem (Thunderstorm's EX).
    let d = get("thundercrack");
    let (st, radius) = (effect(d, "stun"), d.totem.as_ref().unwrap().radius);
    for want in [
        format!("up to {} yd away", said(d.range[1])),
        format!("After {} sec", said(d.detonate)),
        format!("within {} yd for {} sec ({} sec against heroes)", said(radius), said(st.secs[1]), said(st.secs[1] * share)),
    ] {
        assert!(d.text.contains(&want), "{want:?} in {}", d.text);
    }
    assert!(d.secs > d.detonate, "it stands past its fuse");
    // Prowl (Feral Leap's EX).
    let d = get("pounce");
    let v = effect(d, "veil");
    assert_eq!(effect(d, "prowl").secs[1], v.secs[1], "it holds as long as the hiding");
    let (bonus, stun) = (crate::sim::cc::PROWL_BONUS, crate::sim::cc::PROWL_STUN);
    for want in [format!("for {} sec,", said(v.secs[1])), format!("stuns its target for {} sec ({} sec against heroes)", said(stun), said(stun * share)), format!("deals {} more damage", said(bonus))] {
        assert!(d.text.contains(&want), "{want:?} in {}", d.text);
    }
}
