# Rhahk'Zor: restore the forge fight, then repair the gauntlet

The controlled client take `298-rz-gauntlet-fight` on source `299ac99` with client `skyB`
confirmed a route regression from #293. The server recorded 9 collapse route resets and 1,053
floor-recovery events, including a position on the cave roof. The demonstration used periodic
healing, positional corrections, health commands and a forced final kill; it is not a clear.

Native data probes on the server's default 1 yd map grid confirm the geometry:

- The interpolated reset point `(-205.1, -480.6, 52.6)` has only the roof floor at 302.2.
  The valid mine walk from `(-190, -480)` to `(-244, -482)` bends south near `(-200, -503)`.
- The repeated recovery positions near `(-211.7, -561.4, 52.4)` and
  `(-206.5, -561.8, 52.4)` have floor 21.0, with no upper walkway beneath them. The valid
  upper path between `(-216, -561)` and `(-192, -563)` bends through `(-204.4, -558.1, 51.2)`.

The immediate rollback restores #291's Rhahk'Zor encounter and the front-door exit only. The
generic collapse code, debug `clearfoes` command and later encounters remain available. A
separate repair will validate route geometry and recovery footing before restoring the gauntlet.

The intended gauntlet remains: collapse at 70%, a front moving at 4 yd/sec with 14 yd warning,
20% maximum health per second behind it, a 26% boss-health floor until the route ends, burning
maul at 25%, lodged pound at 12%, and escape through the smugglers' tunnel. Any change to that
design requires comparison, not an implicit consequence of a movement repair.

The rollback's release server build and full release test sweep passed: 1,102 tests passed,
19 ignored, no failures. Repository checks passed with no warnings. Parsed content comparison
confirms the Rhahk definition exactly matches #291 and all 29 other mob definitions match
main `7d2747d`. Every Cargo invocation used the Windows Rust setup script and one build job.

The follow-up [native route repair](rhahkzor-route-repair.md), PR #306, restores the gauntlet
after native walking, hold-release and wipe checks. No new client run of the restored forge
encounter or repaired gauntlet, or unassisted clear, is claimed by these server checks.
