# Rhahk'Zor: the native route repair

Claim: `codex/rhahkzor-native-route`, separate from the rollback in #305. This repair
depends on that rollback and must integrate it before restoring the gauntlet.

The take298 capture confirmed that coarse route interpolation placed Rhahk'Zor over
missing upper floors and inside a roof. The walker's fall recovery could then restore
the same airborne start on every step. The active collapse already moves its home;
an original-home leash is not the cause.

Repair scope:

- Resolve authored collapse legs through the native land, preserving its corners and
  rejecting blocked, wet or wrong-floor endpoints.
- Follow those ordered corners without jumping to a coarse interpolated reset point.
- Recover from invalid footing only to verified footing, with bounded repeated recovery.
- Cover bent routes and stacked floors deterministically, and run an ignored real-map
  traversal against installed 1.12 data before restoring the encounter.

Keep the agreed 70% trigger, 4 yd/s front, 14 yd warning, 20% maximum health per second
crush, 26% health floor, 25% burning maul, 12% final pound and smugglers' exit. Measure
the resolved route's length and duration. A movement test is not an unassisted clear;
real client footage follows separately on the coordinated recording rig.

No other boss content, client patch, add-on or game-derived asset belongs in this repair.
