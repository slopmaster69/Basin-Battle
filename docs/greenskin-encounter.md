# Captain Greenskin: break the firing line

Implementation claim for the agreed Deadmines design. Two finite, reachable deck gunners
shoulder, freeze their aim and fire on separate beats. Their captain attacks while they reload.
When both gunners fall, his visible pistol draw and harpoon pull join the committed thrust and
clearing cut. Crew state drives the change; no replacement waves or health enrage.

Scope: a narrow firing-line encounter module, exact captain/gunner content, lifecycle hooks,
regression tests, read-only native asset/deck queries and a controlled visual-check scenario.
Existing bosses, client patches and add-on versions remain owned by their current branches.

Validation will cover executed opening cadence and frozen aim, one/two crew deaths, solo/duo
retargeting, wipe/reset/death cleanup, ordinary melee/ranged access and pack reward completion.
Required release build, release tests and repository checks run before the implementation push.
Native assets and reachable deck support are verified against the installed 1.12 data. Client
footage remains explicitly untested until the shared recording rig is available.
