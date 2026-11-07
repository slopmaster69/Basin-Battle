# Mending the game: the 48-hour plan (Nov 7 to Nov 9, 2026)

Corey's brief, Nov 7: no new classes or abilities until the core loop is fun and stable; mend
everything that doesn't work; the enemies need the most work. Claude works on it alone for 48
hours: fixing the game, not only filming it for review, and keeping its own log of what it found
and changed (`docs/mend-log.md`).

## How the work lands

- **Every fix is its own small pull request**, tested, with its lab film where it changes what's
  seen. A reader can take or leave each one.
- **Fixes to play go to `claude/playtest` first**, never straight to `main`. That branch is `main`
  with every fix merged in, rebuilt as each lands, and installed on Corey's PC as its own folder
  (`Documents\Basin Battle Playtest`) beside his v0.4.2 game. Corey and David play it; the fixes
  they keep merge to `main` and go out as a release.
- **Tooling and docs** (the spell lab, the review, this plan) merge to `main` once green: they
  change nothing a player sees.
- The lab runs only on Corey's PC and only when no hero but Labwar is in the host log.

## The order, and why

The Nov 6 session's 24 falls came from three things: blows that can't be answered in time, blows
that can't be seen or heard coming, and heroes whose kits don't do enough to end a fight before
it wears them down. Fix them in that order: a fair blow first (nothing else can be judged until a
blow can be answered), then a readable one, then the fight's length.

### 1. A blow you can answer (the client and the rules)

1. **The strike is seen to swing** (client patch 0060): done, filming.
2. **The sparks on the hit's own frame**: they lag about 0.1 s. Find why in the client.
3. **A reaction budget test over every creature**: from each blow's wind-up, the hero's own
   commitments (a light swing's 0.69 s, a cast) and a 0.3 s human reaction, which blows can't be
   answered from which hero action. A test over all of `content/` so no string drifts back.
   Then fix what it finds, starting with the Deadmines' foot soldiers.
4. **Two never swing on one beat**: with two pressing one hero, the second starts its string only
   once the first's blow has landed, plus a beat.
5. **An approach that reads**: a foot soldier arriving at reach stops a beat before its first
   string (not a lunge, which has its own long tell).

### 2. A blow you can see and hear

6. **Sound on every creature wind-up**: a weapon swish by grip, a cast's own sound for a spell,
   played from the creature so it's heard from behind.
7. **A tell for bolts**: a small-splash missile (the Squallshaper's, the Crossbowman's) marks
   where it will land for the cast's last moment and its flight, and its target is warned when
   the caster is behind them.
8. **The pot's ring reads on any ground**: the drawn ring under a spell's ground look.
9. **Ability sound**: every ability gets a cast and an impact sound (none have any today).

### 3. A fight that ends

10. **The hero's real damage per second** by class and weapon against a Deadmines foot soldier,
    read in the lab (the trace's numbers), against the foe's effective health (`foe_health` 1.6).
    Set a target (a foot soldier in about 7 light blows, 3 to 5 seconds of a class's rotation)
    and tune foe health and the weakest kits' numbers to it.
11. **The meter**: a fight lasts while a foe is still on you, not 5 s after the last blow.
12. **Bladestorm on its key** (#233): done, filming.
13. **The Priest**: play its whole kit in the lab against a pack (the staff, the right-click heal,
    the Light and the Shadow) and find what "didn't work".
14. **The Mage and the Hunter**: their numbers against the target; the Hunter's draws and the
    Mage's charge times against a foot soldier's pace.

### 4. The Deadmines, encounter by encounter

15. **The bestiary**: every Deadmines creature's every string filmed the same way
    (`bestiary dm_*` in the lab queue), each read against the reaction budget.
16. **Encounter 1** (the beach, Cookie): the review's changes (lessons in order, the caster alone,
    Cookie as a lesson), tuned and filmed.
17. **Encounters 2 to 6**, reviewed the same way, as far as the time goes.

### Throughout

- `docs/mend-log.md`: what was measured, what was changed and why, what's still wrong, written as
  it happens.
- The playtest folder rebuilt as fixes land, with a short "what changed" note in it.
- David's Mac: the steps to play `claude/playtest`.
