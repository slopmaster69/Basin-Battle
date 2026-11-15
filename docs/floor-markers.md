# Optional floor markers

The agreed encounter designs use animation, sound and
environmental effects as the default warning language. Floor markers remain available in
Controls for players who want them, but start hidden until explicitly enabled.

Scope: the add-on setting and its toggle, the add-on/server version notice, and the existing
add-on checks. Spell effects and actual environmental warnings remain visible. No server
damage shapes, combat timing, encounter code or client patches are part of this change.

An unset preference or saved `noMarks = 1` hides overlays. Explicitly enabling them in
Controls saves `noMarks = false`; toggling them off saves `1` and clears visible overlays.
Add-on version 60 matches the server's update notice.

Automated validation passed: `scripts/check-addon.sh` reported zero Lua errors for both add-ons,
`cargo build --release -p htl-server` succeeded, and `cargo test --release --no-fail-fast`
passed 1,114 tests (19 ignored). Repository rules passed with no warnings. On Windows GNU,
the add-on harness needed a local import library for the native UCRT timezone functions;
no repository source or shared toolchain was changed for that workaround.

The committed add-on scenario asserts the default, Controls toggle, and immediate clearing.
A focused harness also verified absent settings, an unset preference, saved `1`, explicit `false`,
the Controls labels and toggle, suppression of TG/TA/MK overlays, and immediate clearing.
Real-client rendering and persistence through an actual save and reload remain untested.
Use a fresh temporary client home for that check; the player's settings must stay untouched.

## The sweep: every tell must read with the markers off

The owner's rule, word for word: "Remember the game is supposed to be designed without having to
need the visual floor indicators. Always plan around NOT having them, but turning them on is for
noobs."

A floor marker is anything drawn flat on the ground only to warn: a ring, wedge, strip, lane,
circle or decal. World effects are not markers and stay: real fire, dust, sparks, falling rock,
torches, objects, a creature's own pose, sound, yells.

How a marker reaches the screen. Three server lines are gated by the add-on's "Floor markers"
setting (`ES.MarksOn()`): `TG` (a creature's blow), `TA` (a shape at a spot) and `MK` (a circle).
A fourth, `GB` (a shape flashed on the ground, patch 0026), was drawn whatever the setting was.
Warning-only shapes sent as `GB` broke the rule. They now go out as `Event::GroundWarn`, which
sends `GB` with a last field of 1; the add-on lays those only with the markers on. A `GB` that is
an effect (dust thrown up where a rock struck, the cone a landed blow covered) stays as it was.
A client with no add-on gets the danger model from `Event::Mark` as before (it cannot choose).

### Audit table

Status: pass (tells with markers off already), fixed (this sweep), open (found, fix queued).
"Setting" is whether the marker is drawn only through the setting.

| Event | What the marker shows | Setting | What tells it with markers off | Result | Fix |
| --- | --- | --- | --- | --- | --- |
| Rhahk'Zor: a rock about to fall (slam, 45% ring, ahead of the front) | Ring under the rock, 1.3 sec (`GB`) | No | Dust cloud on the spot, the Danger flash on a hero's screen, the rock itself in the last 0.7 sec; nothing told the spot for the first 0.6 sec | Fail | Ring is `GroundWarn` (the setting's). Dust trickles from the roof over the spot, 2 clouds every 0.3 sec, from the first moment to the landing |
| Rhahk'Zor: the rock marked under each hero (slam) | Same ring, plus the `MK` circle | Circle yes, ring no | As above | Fail | As above (the same fall) |
| Rhahk'Zor: a surge's line | Bright band across the section's front edge (`GB`) | No | Torches 3 yd past the front edge, lit from the warning on; dust from the roof over the whole section; rumble; shake and Danger on a hero inside it | Marker fail, tell passes | Band is `GroundWarn` |
| Rhahk'Zor: the wave's rows | None (no ring, by design) | n/a | Rock dropping, dust rolling at the front | Pass | None |
| Rhahk'Zor: burning ground (burning maul, lodged pound) | Fire-coloured ring at the true edge, redrawn every 0.5 sec (`GB`) | No | The flames themselves: one at the heart and 3 round it at 0.55 of the radius | Marker fail, tell passes | Ring is `GroundWarn` |
| Rhahk'Zor: dust ring where a rock struck, at the slam's wall, the end dust line | A brief dust-coloured shape after the strike (`GB`, 0.5 to 1.2 sec) | No | Not a warning: it follows the blow, with real dust beside it | Pass (an effect) | None |
| The Shredder: the log's drop strip (9.5 yd by 3.2), quiet from the pull and bright from the cut | Strip on the floor (`GB`; the doc called it the client's own, shown with markers off) | No | The log hanging 6 yd over it, then the rope whipping away and the creak (3360) for 0.7 sec; nothing on the floor | Fail | Strip is `GroundWarn`. Dust shakes off the log along the drop every 0.3 sec through the creak |
| The Shredder: the tie-off ring (lit while Sneed mends) | Ring at the stake (`GB`) | No | The stake itself (waist high, 3 times its size); nothing said when to cut it | Fail | Ring is `GroundWarn`. A torch burns beside the stake while Sneed mends and goes out with the rope |
| Gilnid, Greenskin, Cookie, Smite, VanCleef, the Shredder, other enemies | Warning shapes sent as `GB` regardless: see below | No | Per boss, in its own pull request | Open | Each in its own pull request |

Sites still to move to `GroundWarn` (found in the audit, each warning-only and sent as `GB`):
Cookie's scoop wedge, pot ring and patch ring (`sim/kitchen.rs`), Gilnid's valve lane, stripes and
stick ring (`sim/perches.rs`), Greenskin's firing line (`sim/firing_line.rs`), small splashes of other enemies
(`sim/mobs.rs`, noted "so it reads with the floor markers off" on Nov 8, before this rule).
Smite's stomp and slam and VanCleef's strings use `TG`/`TA`/`MK` lines, which the setting already
gates; what tells them with the markers off is not yet checked (the striker's weapon glow and the
flash before the window are the add-on's, and read on the creature itself).
