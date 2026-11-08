# Edwin VanCleef: the false opening

This encounter follows the agreed Deadmines design. VanCleef makes 2 committed cuts and
leaves an honest recovery. His Invitation is a planted, distinct stance: exactly 1 frontal
hero hit provokes a committed riposte with a full visible windup. Waiting ends the stance
without an attack. HumanMale clip 57 remains the special-blow animation.

A posture break keeps its punish window, then sends him a short distance along a checked
path under brief smoke cover. An arrival cue precedes his visible advancing combination.
He never teleports and remains vulnerable throughout. At 30% health his finite, noncombat
Blackguard observers walk away. A final combination keeps its minimum windups and leaves
an honest recovery. Unbowed uses longer combinations and longer recoveries, without adds.

## Implementation boundary

The encounter lives in a narrow simulation module, with small hooks into mob lifecycle,
pattern choice, posture breaks and direct hero hits. Its content, deterministic tests and
filming scenarios belong to this pull request. Rhahk'Zor, the shared boss module, Smite's
arsenal, Gilnid's workforce, client patches, and add-on versions are outside this change.

## Verification

Before the implementation is published: release server build, full release tests and PR
rules. The scenario will label forced demonstration beats. Client pose, smoke readability,
ship-floor paths and solo/duo timing remain under `needs playtest` until filmed.
