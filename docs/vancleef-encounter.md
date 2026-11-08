# Edwin VanCleef: the false opening

VanCleef opens with exactly 2 committed cuts and a 1.8 sec recovery. His next close
exchange is the Invitation: a held guard pose with both swords equipped for 1.8 sec.
Waiting ends it without an attack and leaves another 1.2 sec opening. A damaging direct
hit from within the frontal 140-degree sector spends the Invitation once. It still deals
its normal damage, then starts a 0.9 sec riposte toward that attacker. The facing is fixed
from its start, and the riposte leaves 2.4 sec of recovery. No instant counter damage.

Ordinary melee, weapon arts and direct projectiles use the same contact rule. A bursting
projectile counts only its directly contacted body. Splash, periodic damage, beams and pets
do not provoke it. Death, posture breaks and disabling effects take precedence. The encounter
uses no class, ability, hand or party-size requirement. Rear hits remain ordinary damage.
Control keeps the game's existing boss rules: effects explicitly allowed against bosses
can interrupt him; an ordinary stun or root is still resisted.

A posture break preserves the entire existing stagger reward. Afterward he walks a checked
4 yd route away from a living opponent, under a small native smoke emitter. The emitter goes
away before a 0.9 sec arrival cue and the advance's full 1.1 sec first windup. The advance
has 3 committed blows and a 2.5 sec recovery. Blocked, wet, wrong-floor or excessive-detour
routes are rejected; he gives the arrival cue where he stands. A rooted or stalled route
ends in place within 2.5 sec. He remains vulnerable throughout.

Two friendly, noncombat Blackguard observers stand near his home if both their standing
points and departure paths are reachable on that floor. They have no pack, aggro, attack,
loot or reward requirement. At 30% health they walk away along the deck and leave the
encounter. They never respawn during that fight. At 12%, his 4-blow last stand starts only
after the current string and recovery; it has 0.65 sec minimum windups and 3 sec recovery.
It happens once, can be interrupted, and never holds his health above zero.

Unbowed rises through the existing quarters-secret handoff. He has no observers or summons.
His ordinary and advancing strings have 4 blows, his last stand has 5, and recoveries grow
to 2.8, 3 and 3.4 sec respectively. Windups never accelerate at low health. Both forms bypass
random string cuts/extensions, held beats, extra chains and tactical retreats so the visible
exchange remains honest. The ordinary lunge still closes distance without spending the next
Invitation. Reset, return, wipe, death and handoff remove encounter poses, smoke and observers.

## Verification and filming

The installed-data regression in `crates/htl-server/tests/vancleef_native.rs` uses the actual
pack at (-84, -820), with the hero at (-76, -820, 40.2). It checks observer creation before
the health gate, every 50 ms of departure, both arrivals before cleanup, no attacks or
replacements, and the complete smoke/retreat/arrival/advance sequence. The hero is healed
and health gates are injected; this is movement proof, not an unassisted clear.

Deterministic tests cover actual solo/duo/4-hero AI, exact strings and recovery, waiting,
melee and projectile contact, the second simultaneous hitter, rear/splash/periodic immunity
to provocation, CC precedence, posture reward, continuous retreat, blocked/rooted fallback,
finite spectators, pack-clear independence, secret handoff, and encounter cleanup.

- `vancleef-invitation.txt` is a controlled lab demonstration of the honest recovery, safe
  waiting, a provoked riposte answered by a dodge, and Unbowed's longer string.
- `vancleef-story.txt` is a controlled encounter with labelled debug health changes, for the
  retreat, departing Brotherhood, final combination, actual secret handoff and cleanup.
- Neither scenario is an unassisted kill or a balance playtest. Film a complete solo and duo
  fight separately, with markers off, before making balance or full-fight showcase claims.

Read-only data queries verified display 2029 and 2357 use HumanMale, native HumanMale attack
17 and special 57 exist, and the held guard uses Emote43 / ShieldBlock24. Existing client
patch 0010 holds that guard pose; the HumanMale57 correction from PR285 remains intact.
Sound kits 5780/5781/5782 and smoke display45 resolve in the local 1.12 data. The smoke is a
particle emitter with no mesh; its opacity and timing need real-client inspection.
The original observer posts were valid, but their northbound departures around the cabin
measured 35.760 and 20.351 yd against a 19.4 yd short-route limit after the native navigation
repair. Every segment passed; total length rejected both observers before they spawned.
The posts stay at (-88, -818.5) and (-80, -818.5). Their exits now follow the cabin's southern
lip toward (-78, -816) and (-72, -820), on the same dry deck. Boss home, floor/height checks,
detour bounds, movement speed and the 8 sec cleanup remain unchanged. Custom terrain that
cannot support a standing point or departure still omits that observer.

The original short smoke retreat already works at home and after 12 sec of actual AI on
this floor, so its combat source is unchanged. From a posture break, allow about 2.2 sec
for the full stagger reward, then 1 sec to walk 4 yd. The cloud lasts 0.65 sec and is gone
before arrival; the advance begins about 4.2 sec after the break, with its full 1.1 sec
first windup. Frame the cabin frontage through that whole interval. The drop south at
(-84, -824) remains rejected rather than used as a retreat.

## Not tested in the client

The native held guard is the implemented Invitation silhouette. It cannot promise the exact
ideal of one blade across his chest and the other drawn back until filmed. Ordinary attack
recovery returns to the native ready pose; there is no custom shoulder-drop animation.

The smoke emitter, glint, actual stern-floor paths and departure staging, native animation
blend, controlled-scenario timing, and solo/duo encounter duration remain under `needs playtest`.
Observers walk away out of the duel and are removed after 8 sec; there is no cinematic or
entire-ship evacuation route. No client patch or camera takeover is added by this encounter.
