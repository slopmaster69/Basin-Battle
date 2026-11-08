# Sneed's Shredder: the machine and the coward

The machine uses one complete saw sweep, then a stamp and rake, then a straight grind. Only
executed strings build pressure. Smoke puffs come twice as often at the second pressure step.
The grind locks its direction at the first pose, winds up for 1.4 sec, travels 6 yd in 0.9 sec,
and stalls for 3.5 sec. Its 3 yd lane checks the actual swept path and can hit each hero once.
Walls stop the grind. A sidestep works without a dodge, parry, interrupt or ranged loadout.
There are no health-triggered phases or damage multipliers in either form.

The machine still has 950 base health. Its driver has 340 base health, down from 700. Sneed
aims the native wrench missile during its tell, then scrambles back 4 yd where the ground allows, and flurries up
close. After at least 5 sec on foot, at a safe attack boundary, he returns to the wreck. He has
6 sec to get there; no path means he abandons the attempt. At the wreck he visibly works for
4 sec, using the existing Mining cast and repeated sparks. Any positive damage or a stagger
stops that attempt permanently. No particular class ability is required.

A successful repair consumes the corpse and removes the driver, replacing them with one machine
at one third of its original maximum health. The driver's remaining health is saved; when the
machine breaks again, he emerges with that health and no repair left. Only the final driver's
death awards plunder and gear and clears the pack. A wipe restores one whole machine at its
original post. A lamp rest and a driver death remove any retained wreck.

The implementation is in `sim/machinery.rs`, with small attack and lifecycle hooks. Other boss
content, shared boss-run behavior, the client patches and the native creature models stay with
their existing owners. The native Throw Wrench spell 13398 was verified against the rig's client
data: `Spells\Missile_Wrench.mdx`, speed 20. Sneed uses GoblinMale's native AttackThrown 107,
whose release event is at 0.20 sec in its 0.67 sec take; that model has no clip 61. Smoke and work use existing spell visuals 1856 and
2575. The native model audit confirmed attack 16 takes 0/1, special 130 and StandWound 8; it
has no separate stun clip. Sweep uses take 0, stamp uses 130 and grind uses take 1, with native
attack/pre-aggro/aggro sounds 1977/1981/1982. Stall uses StandWound and wound sound 1979. The
special and take 1 share impact timings, so distinct silhouettes must still be checked on film.
Exact smoke appearance, sound separation and the work animation also need that visual pass.

## Capture checklist

`tools/visual-check/shredder-pressure-repair.txt` is a **dev-command demonstration**, with forced
health transitions and repeated healing. It does not prove an unassisted full fight. Start with
an unbeaten Sneed pack. Record the source commit and client build in the capture notes.

- Approach from the Mast Room entry and frame both safe sides of the machine.
- Show each normal move's preparation, danger and recovery. Sidestep the locked grind and strike
  during the stall. Keep the full first pressure cycle in one shot.
- Show driver emergence, a wrench covering his retreat, and a close flurry.
- Let one repair complete. Show its full 4 sec and the one-third health bar. Break the machine
  again and verify that the driver retains his damage and never repairs again.
- On a fresh pull, interrupt the work with a normal light swing, with an empty ability loadout.
  Show the wreck disappearing and Sneed remaining on foot.
- On separate fresh pulls, wipe during work and during the grind. Return to the post, then rest
  at a lamp: one full-health machine, no duplicate Sneed, lingering attack or ghost wreck.
- Record solo and duo fights, with melee and ranged loadouts, including earned finishes. Record
  at least one full fight with floor markers turned off in Controls. Record that separately
  from the scripted demonstration, and report boss length and any unclear native animation.

The script can be parsed without launching a client using
`scripts/visual-check-windows.ps1 -Check shredder-pressure-repair`.

## Actor diagnostics and their limits

The existing ignored `tests_fair_hero::fair_hero_one` report was run with `FAIR_HERO_FOES`
set to each form and `FAIR_HERO_SEED` set to 1, 7 and 19. Its careful sword actor moves,
attacks and answers tells, but has 1,000,000 health and fights on flat open-world ground.
The machine's first life lasted 78.62, 74.26 and 76.58 sec; the actor took 56, 28 and 0
damage. Sneed spawned alone lasted 25.78, 31.98 and 41.92 sec, with no damage taken.
Neither form produced a hit the report classed as too quick to answer.

These are isolated-form diagnostics. The harness tracks only the original creature IDs, so
its machine report stops at driver emergence; its standalone Sneed has no wreck or repair.
It has no duo mode. These timings must not be added together or called full encounter balance
evidence. Whole-encounter solo and duo pacing, normal-health survival and earned finishes
remain to be measured in play or a separately extended actor report.
