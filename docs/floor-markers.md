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
| Other enemies: a lobbed splash too small for a ground look (under 6 yd), from the wind-up to the landing | Ring at the true radius (`GB`; noted "reads with the markers off" on Nov 8, before this rule) | No | The thrown thing flying its arc, at least 0.7 sec, to where it comes down | Marker fail, tell passes | Ring is `GroundWarn` |
| Other enemies: a step's `burst` shape flashed where its blow landed | A shape for 0.45 sec after the blow (`GB`) | No | Not a warning: it follows the blow, with its own sound and look | Pass (an effect) | None |
| Smite: the stomp and the hammer slam | `TG` lines (a wedge or ring), laid only through the setting | Yes | His own wind-up pose, the weapon glowing amber (a guard stops it) or red (nothing does), the flash a moment before the window to answer, the woosh: all the add-on's, drawn whatever the setting | Pass | None. The stomp is round his feet and the slam where his hammer comes down, so the striker is the place |
| VanCleef: the strings and the closed ring of guards | `TG`/`TA`/`MK` lines, through the setting | Yes | The guards' own bodies round the deck, his thrust pose and the same striker tells; `docs/vancleef-encounter.md` says nothing is drawn on the floor | Pass | None |
| Gilnid, Greenskin, Cookie, the Shredder | Each has its own rows above | | | | Each in its own pull request |

Smite and VanCleef were checked in the audit: their lines already go through the setting, and
the striker's weapon glow, the flash before the window and the woosh are drawn from the `TG` line
whatever the setting is, so both pass. Nothing of either has been seen in the client.
