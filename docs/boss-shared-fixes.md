# The bosses' shared fixes

The Nov 8 review of the Deadmines footage found eleven faults that every boss shared. These are
the ones now fixed in the engine for every creature. The VanCleef, Shredder and Gilnid fights also
carry their own local versions (`docs/vancleef-encounter.md`, `docs/shredder-encounter.md`,
`docs/gilnid-encounter.md`); those stay, and agree with these.

| Fault | Engine-wide now |
| --- | --- |
| Debug damage faked the fight | Damage labelled `debug` (`.es hurt`) moves health only. It never builds posture, delays Smite's weapon change or counts as an exchange (`DEBUG_LABEL`, `hurt_mob_as`). |
| Effects played at the wrong moment | A step's `kit` is still the wind-up's tell. Its new `impact_kit` plays on the creature as the blow lands, hit or miss. A heavy step of a string now shakes the ground (`KIT_DUST`) as a heavy special always did. |
| Small hazards were invisible with markers off | A lobbed splash too small for a ground look (under `GROUND_LOOK_MIN`, 6 yd) is drawn to its true size as a ground burst, an effect the markers setting doesn't hide. A step can also ask for its own shape to flash where it lands (`burst = "RRGGBB"`). |
| Second forms were instant swaps | A form that rises from a fallen one waits `SECOND_FORM_WAIT` (2 s) before its first blow. Unbowed keeps his own kneel and wait. |
| Deaths lagged | A creature that falls mid-blow, or any champion, lets go of its swing or held pose (`HoldPose` clip 0) as it dies, so the death plays at once. |
| Things outlived their boss | When a creature dies, what it called up goes with it, and its throws still in the air are spent: nothing it threw lands after its death. |
| Yells collided | Lines within 40 yd of each other come at least 2 s apart (`LINE_GAP`). One due sooner waits its turn, and is dropped if its speaker is gone or it waited 8 s. |

Still for each boss, because they depend on its own models, clips and props:

- slower projectiles with a visible flight (Greenskin's shots, Cookie's mug);
- held wind-ups that look held (a raised hoof, an aimed gun), with the swing on the real release;
- the "open" look for punish windows (VanCleef's hunch, `HoldPose` StandWound 8 or kneel 75);
- stopping distances that keep the hero outside a big model.

The tests are `crates/htl-survival/src/sim/shared_fixes_tests.rs`.
