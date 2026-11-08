# Gilnid rendering recovery

Claim: revert PR #299 before restoring its presentation changes on a separate branch.

The real client showed empty crew platforms after PR #299. Its immediate-prop predicate applies the preview-move flag to structural cargo and planks; client patch 0017 also removes their collision hulls. The engineers need those hulls for client floor projection.

This recovery keeps the existing Gilnid core and unrelated Shredder, Smite, VanCleef, and Cookie work. A follow-up will narrow immediate display to warning and debris props, preserve platform collision, and verify visible crew and valve movement in the canonical client scene.

Recovery evidence (2026-11-08): exact revert commit `b97f8d9`, client `skyB`, canonical camera `0 -65 24`, markers hidden. Both workers are again visible on cargo/plank supports. Watcher request `gilnid-revert-b97f8d9-platforms` completed with 39/40 stills and an MP4; the missing still is the simultaneous video cue. This is a controlled comparison with debug healing and cleanup, not an unassisted clear or balance test.
