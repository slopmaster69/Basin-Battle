# Gilnid rendering recovery

Claim: revert PR #299 before restoring its presentation changes on a separate branch.

The real client showed empty crew platforms after PR #299. Its immediate-prop predicate applies the preview-move flag to structural cargo and planks; client patch 0017 also removes their collision hulls. The engineers need those hulls for client floor projection.

This recovery keeps the existing Gilnid core and unrelated Shredder, Smite, VanCleef, and Cookie work. A follow-up will narrow immediate display to warning and debris props, preserve platform collision, and verify visible crew and valve movement in the canonical client scene.
