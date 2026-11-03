# The spell lab: seeing a spell before calling it done

An ability isn't done when its tests pass. It's done when it has been **seen in the client** and
every beat of it reads on its own, with no text to explain it. The lab makes that a routine:
the same scene, the same presses, a burst of timed screenshots, and the server's own record of
what it sent, side by side on one sheet.

## Running it

1. Host with debug on (`.es debug`) or a dev build (`[dev] full_tree`). Stand somewhere open and
   flat. Put the camera in the same place every time: `/script ResetView(2) SetView(2)
   CameraZoomOut(3)` sets it behind the hero at a fixed distance and pitch. Nothing on screen you
   don't want in the frames.
2. Type `.es lab <ability>`. It sets out the scene in front of you, puts the ability on its key
   (cooldowns off, full Valor and health), starts a burst of screenshots and presses the key for
   you a quarter-second later. Hold still until it says "Lab done".
3. Copy the run's screenshots (`benilla-config\Screenshots`, the newest ones) and its trace
   (`logs/lab/<ability>-<scene>-<time>.txt`, where you host) into one place, and make the sheet:

   ```sh
   python3 tools/spell-lab/sheet.py <screenshots folder> <trace.txt> sheet.png
   ```

   Each frame shows its time from the press and what the server sent in that frame's interval:
   the presses (blue), spell visuals, clips, missiles, hops and reels.

### Options

`.es lab <ability> [scene] [creature] [do <steps>] [shots <n>@<ms>]`

- **Scenes:** `one` (one body 5 yd ahead), `pack` (three across the front), `close` (one at arm's
  length), `ring` (three at arm's length), `far` (one 14 yd off). The default is `close` for
  stances and weapon arts, `pack` for the rest.
- **Creature:** `dummy` (the training dummy, the default) or any creature's key, e.g. `brave`,
  which can be made to swing. Stances default to `brave`.
- **Steps:** comma-separated: `q e 1 2 3` (a key), `^2` (with Ctrl: the EX), `click` (a left-click
  tap), `click?` (only if aiming), `heavy`, `guard`, `guardup`, `dodge`, `bash`, `blow` (the
  nearest creature swings at you), `wait:<secs>`. The default is the key, then `click?`.
- **Shots:** 30 every 80 ms by default (2.4 sec). The client numbers shots within a second, so
  none are lost.
- `.es lab clear` takes the scene away.

Examples:

```
.es lab shockwave
.es lab retaliation close brave do 2,wait:0.2,blow shots 36@70
.es lab shield_wall ring brave do ^2,wait:0.5,guard,attack,wait:0.9,guardup shots 40@70
.es lab hurled_axe one dummy do q,click,wait:0.6,q
```

## What to check, for every spell

Go through the sheet frame by frame. Each of these must be **seen**, not inferred from the trace.

1. **The press shows at once.** Within a frame or two of the step, the hero does something: a
   windup clip, a brace, a cast glow. Nothing on screen for 0.2 sec after a press feels like a
   dropped input.
2. **The windup reads.** Anything with a windup (a slam, an overhead) shows the body loading
   the blow for the whole windup, and a foe could see it coming.
3. **The moment of effect is unmistakable.** The frame the blow lands has an impact: a flash, a
   burst, dust, a shake. A cone's or a ring's area is visible as it goes off.
4. **What it did to each target shows on that target.** Knocked back: it flies. Stunned or
   staggered: it reels. Rooted, frozen, Sundered, Marked: an effect stays on it as long as the
   state does, distinct from every other state's.
5. **Nothing happens that the spell doesn't do.** The caster doesn't jump, slide or flinch
   unless the spell moves them. No stray effect on the wrong unit, no effect left behind after
   the state ends.
6. **Missiles fly the whole way.** Out, back (a recalled axe), every bounce: in flight every
   frame until it ends, and where it ends is clear.
7. **The icon is right.** It's the spell's own (not a question mark or a hammer on a beast), the
   same on the bar and the board, and it doesn't share a look with another key of the kit.
8. **It reads in a crowd.** Run it on `pack` or `ring` too: with three bodies, can you still tell
   what happened to each?

Write down what the sheet showed, frame times included ("+0.40 the slam's dust, +0.45 two of three
thrown, the third stays: it was outside the cone"). A pull request that changes a spell's look
links or attaches its sheets, before and after.

## When it doesn't read

- **The trace has it, the frames don't:** the server sent it and the client didn't show it (a
  visual kit the client doesn't play on that model, a clip the race lacks, a missile id with no
  model). Try another id with `.es fx spell <id>`, `.es fx emote <id>` or `.es fx cast <id>` on
  yourself, in its own lab burst, before putting it in content.
- **Neither has it:** the rules never sent it. A test can catch that; add one.
- **The frames have something the trace doesn't:** the client is doing it on its own, from the
  spell's own data. The client plays a spell's **impact** (its visual and its clip, which can be
  a knockdown) on every unit the spell names as struck. A look sent at the caster
  (`Target::Unit(caster)`) plays the impact on the caster: that's how Shockwave knocked its own
  caster flat. Send an area's look at the spot (`Target::Point`) and keep `Unit` for a look that
  belongs on that body (a buff, an aura).
- **To try another look without a rebuild:** `.es tune ability.<key>.spell <id>`, then the lab
  again. `.es tune keep` writes it into the rules once it's right.

## Why it's done this way

Effects picked from a list of spell ids and never looked at are guesses, and players see the
misses first. The lab costs a minute a spell and turns every guess into something checked.
