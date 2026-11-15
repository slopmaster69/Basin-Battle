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
    // Ice Lance (Fireball's EX): its bolt's range and damage, the shatter and how far the ice bursts.
    let d = get("pyroblast");
    assert_eq!(d.name, "Ice Lance");
    for want in [
        format!("up to {} yd", said(d.range[1])),
        format!("deals {} damage", said(d.damage[1])),
        format!("for {}% more damage", said((crate::sim::abilities::SHATTER - 1.0) * 100.0)),
        format!("within {} yd shatters too", said(d.shatter_spread)),
    ] {
        assert!(d.text.contains(&want), "{want:?} in {}", d.text);
    }
    assert!(d.shatter && d.shatter_spread > 0.0 && d.damage[1] < c.abilities[c.ability("fireball").unwrap()].damage[1] / 2.0, "a shatter that spreads, for a low blow");
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

/// What a hero's text says after `from` (so a figure is read in the sentence it belongs to).
fn after<'a>(text: &'a str, from: &str) -> &'a str {
    text.find(from).map_or("", |i| &text[i..])
}

#[test]
fn shattering_text_says_the_shatter_the_code_does() {
    // Freezing Trap's text said "double damage" while `SHATTER` is 1.5 (Nov 15 2026): every text
    // that says what a shatter adds says the constant's, and none says "double" for it.
    let c = Content::dev();
    let more = said((crate::sim::abilities::SHATTER - 1.0) * 100.0);
    let mut said_it = 0;
    for d in &c.abilities {
        let rest = after(&d.text, "hatter");
        if rest.is_empty() {
            continue;
        }
        assert!(!rest.contains("double"), "{}: a shatter is not double: {}", d.name, d.text);
        if rest.contains("% more damage") {
            said_it += 1;
            assert!(rest.contains(&format!("for {more}% more damage")), "{}: {}", d.name, d.text);
        }
    }
    assert!(said_it >= 4, "the abilities that say the shatter's number: {said_it}");
}

#[test]
fn hunter_and_warrior_texts_say_the_numbers_the_code_uses() {
    use crate::sim::{hunter, warlord};
    let c = Content::dev();
    let get = |k: &str| &c.abilities[c.ability(k).unwrap()];
    let share = c.cc.player_share;
    // Arrow Recall: how many arrows Lacerate, and for how long.
    let d = get("arrow_recall");
    let want = format!(
        "struck by {} or more arrows is Lacerated: it bleeds {} damage per sec for {} sec, 1 sec longer for every arrow past the {}",
        hunter::LACERATE_AT,
        said(hunter::LACERATE_DPS),
        said(hunter::LACERATE_SECS),
        ["first", "second", "third", "fourth", "fifth"][hunter::LACERATE_AT as usize - 1],
    );
    assert!(d.text.contains(&want), "{want:?} in {}", d.text);
    // Tripwire: the wire's span, and how long its arrows stay planted.
    let d = get("tripwire");
    for want in [format!("within {} yd of the first", said(hunter::WIRE_SPAN)), format!("stay planted for {} sec", said(hunter::ANCHOR_SECS))] {
        assert!(d.text.contains(&want) || get("tripwire_second").text.contains(&want), "{want:?} in {}", d.text);
    }
    // Skyfall's hang, Call of the Wild's chain and empowered call.
    let want = format!("hangs in the air for {} sec", said(hunter::FLOAT_SECS));
    assert!(get("skyfall").text.contains(&want), "{want:?} in {}", get("skyfall").text);
    let d = get("call_of_the_wild");
    for want in [
        format!("{} sec without a call", said(hunter::WILD_SECS)),
        format!("empowers the next call for {} sec", said(hunter::WILD_PRIMED_SECS)),
        format!("deals {}% more damage", said((hunter::WILD_EMPOWERED - 1.0) * 100.0)),
    ] {
        assert!(d.text.contains(&want), "{want:?} in {}", d.text);
    }
    // Intimidating Roar's Cowed: its length (and a hero's), what it takes off, a boss's share.
    let d = get("intimidating_roar");
    let e = d.effects.iter().find(|e| e.kind == "cowed").expect("Cowed");
    for want in [
        format!("Cowed for {} sec ({} sec against heroes)", said(e.secs[1]), said(e.secs[1] * share)),
        format!("deal {}% less damage", said(e.amount * 100.0)),
        format!("wind up {}% slower", said(warlord::COWED_SLOW * 100.0)),
        format!("below {}% health flees", said(warlord::COWED_FLEE_BELOW * 100.0)),
        format!("deals only {}% less damage", said(warlord::COWED_BOSS_CUT * 100.0)),
    ] {
        assert!(d.text.contains(&want), "{want:?} in {}", d.text);
    }
}

#[test]
fn staff_texts_say_what_the_staffs_do() {
    // The staffs' stacks and marks are tuned in `[staffs]` and `[priest]`: their abilities' texts
    // say those numbers, so a retune that forgets one fails here.
    let c = Content::dev();
    let text = |k: &str| c.abilities[c.ability(k).unwrap()].text.clone();
    let (s, p) = (&c.staffs, &c.priest);
    let spend = said(s.spend_share * 100.0);
    let checks: Vec<(&str, Vec<String>)> = vec![
        ("ember_bolt", vec![format!("dealing {} damage to enemies within {} yd", said(s.ignite_damage), said(s.ignite_radius))]),
        ("ember_bolt_heavy", vec![format!("for {spend}% more damage per heat")]),
        ("rime_bolt", vec![format!("chills the target for {} sec, slowing it by {}% more", said(s.keep), said(s.chill_slow * 100.0)), format!("freezes it for {} sec", said(s.freeze_secs))]),
        ("rime_bolt_heavy", vec![format!("for {spend}% more damage per chill")]),
        ("arcane_bolt", vec![format!("for {} sec, up to {}", said(s.keep), s.most)]),
        (
            "arcane_bolt_heavy",
            vec![
                format!("{}% with 1 charge, {}% with 2 and {}% with 3", said(s.charge_scale[1] * 100.0), said(s.charge_scale[2] * 100.0), said(s.charge_scale[3] * 100.0)),
                format!("knocks the target back {} yd", said(s.charged_knockback)),
            ],
        ),
        (
            "acolyte_smite",
            vec![format!("Searing Light for {} sec", said(p.sear_secs)), format!("tear the mark out for {} more damage", said(p.sear_damage)), format!("recharges {} sec sooner", said(p.sear_refund))],
        ),
        (
            "shadowcaller_bolt",
            vec![
                format!("for {} sec, up to {}", said(p.weave_secs), p.weave_most),
                format!("{}% stronger on it", said(p.weave_step * 100.0)),
                format!("After {} sec without firing", said(p.weave_after)),
                format!("{} yd farther, {}% stronger, and worth {} Valor", said(p.empower_range), said((p.empower_damage - 1.0) * 100.0), said(p.empower_valor)),
            ],
        ),
        ("shadowcaller_bolt_heavy", vec![format!("{}% stronger for each stack", said(p.weave_step * 100.0))]),
    ];
    for (key, wants) in checks {
        let t = text(key);
        for want in wants {
            assert!(t.contains(&want), "{key}: {want:?} in {t}");
        }
    }
}
