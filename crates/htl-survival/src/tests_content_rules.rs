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
