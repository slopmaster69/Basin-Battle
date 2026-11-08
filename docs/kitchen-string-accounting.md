# Kitchen string accounting follow-up

The counter advances only when the final authored step releases. A first cut or slap that
breaks a guard or leaves a hero reeling still triggers the existing mercy cutoff, and does not
complete the pair. Interrupting either windup or the recovery between blows retries the pair.
A final blow that misses or staggers the hero does count: both blows have then executed.
Interrupting Gorged's belch retries the belch; releasing it starts the full 4 sec cough.

The old callback inferred completion from a missing pattern after release. At zero network
round trip, the first slap could resolve inside release, mercy-cancel the remaining step,
and falsely advance Gorged to belch. With positive round trip, the hit is queued; mercy cancels
the next step when that hit later settles. That path already retried slaps. The fix passes the
released step to kitchen accounting and checks it against the authored final step. The mercy
rule, timing, other encounter callbacks and add-on version 60 are unchanged.

The production-content regression failed on the original code at zero round trip (1 completed
exchange after only the first slap). The repaired ordinary-AI tests cover 0, 100 and 350 ms
round trips (350 ms uses the existing 200 ms grace cap); real guard breaks;
first-windup, between-step and second-windup interruption;
full pairs with final hits/misses; and interrupted/released belch with its complete cough.
They use a normal starting hero for the new focused cases. Movement is supplied through the
simulation's move input; crowd-control interruption is explicit test assistance, not an
unassisted playthrough. The older kitchen tests retain their stated inflated-health fixtures.

Native footage on feaf3d5 obtained the real pantry ingredient, killed ordinary Cookie with
explicit debug damage and observed the genuine delayed orange 820 HP form. A passive camera
hero repeatedly took the first heavy slap and visibly staggered. Direct summon and closer
placement repeated that result. These takes show the authored first slap (0.9 sec windup,
0.4 sec recovery), not the default melee attack. Their repeated first slaps are consistent
with deferred-hit mercy; footage alone does not expose private counters or prove an initializer
failure. Apparent model growth has not been verified.

For a full-cycle take, avoid the first slap with every hero in its arc. The 0.9 sec windup locks
its facing after 0.5 sec; move beside it after the lock, or time a dodge over the release. A
healthy guard that does not break also works. Then show the 0.4 sec recovery, 0.8 sec second
windup and 1.6 sec final recovery. The second slap may hit without invalidating a fully released
pair. The belch warns for 2.2 sec with direction fixed at its first pose, then leaves 4 sec of
cough recovery. A healed, motionless camera hero cannot demonstrate the pair: mercy repeatedly
cuts it off. Healing/debug controls must remain disclosed.

Release build, complete release sweep and production death-to-secret lifecycle validation
are recorded in the pull request after they finish. Repaired-source native footage remains
pending under needs playtest; the coordinated capture worker owns the rig.
