# Mend log

What Claude measured, changed and still sees wrong, as it happens (newest last). The plan is
`docs/mend-plan.md`; the encounter review is `docs/deadmines-review.md`.

## Nov 7, night

- **Floor markers had been off** all of Nov 6 for Corey (the lab wrote `noMarks = 1` into his saved
  settings on Nov 4). The lab now runs its own game folder on E: (`E:\Basin-Battle-Lab\game`) and
  never touches his.
- **Bladestorm "did no damage"**: the rules land it (5 to 16 a strike, 126 over 6 s); its key only
  raised the reticle and a left click was needed to spin. Fixed: it spins on the key (#233).
- **Fireball, Meteor, the Priest's spells** all land what their cards say in the lab. Their
  problem is size against foes at 1.6 times health, not bugs (Fireball: a tap 17, a full charge
  36, 43 on the flare; Meteor 70, 84 on the flare).
- **A creature's strike**: filmed at 20 frames a second against the trace. The telegraph is exact;
  the strike was a 42 to 80 ms snap after a 0.45 s frozen pose, and the return crawled at half
  speed through the recovery (a second swing with no blow). Fixed in the client (patch 0060).
- **The sparks lag the hit** by 2 to 3 frames (about 0.1 s). Not yet looked into.
- **The meter** starts its Fight tab over after 5 s with no blow dealt or taken. Not yet changed.
- **Session recorder** installed on Corey's PC: every game session filmed on the GPU, with every
  fall marked by its time in the video.
