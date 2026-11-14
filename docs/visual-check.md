# The visual check

An ability isn't done until it has been seen working in the game client: its effects, animations,
icons and visuals, on screen (#110). The visual check makes that a script: it starts the real
client on the PC that hosts the test server, logs the test hero in, casts the abilities a scenario
names through the server's dev commands, and keeps what the screen showed as a burst of stills and
an MP4. Attach the stills (or the video) to the pull request, and say what you looked for.

It runs on Windows, on the PC that hosts the test server (Corey's), by hand or by itself (the
watcher, below).

## What it needs, once

1. **The test server**, running on that PC: `Host Basin Battle.bat`, built from the branch under
   test. The scenarios use `.es debug quiet`, `.es dummy clear`, `.es target` and `.es give`, which
   servers from before this check don't have.
2. **benilla's dev build.** The player build has none of the scripted probes this drives. On
   GitHub: Actions, Release, Run workflow, pick the branch, tick **dev**. (Or
   `gh workflow run release.yml --ref <branch> -f dev=true`.) The run's artifact
   "Basin Battle for Windows (visual check)" holds `benilla.exe`; unzip it into
   `visual-check-client\` in the repository folder (gitignored), or anywhere and say where in the
   settings. A dev client built by `scripts\play-windows.ps1` (`benilla\target\release\benilla.exe`)
   works too. The client only needs rebuilding when `patches/benilla/` changes: the add-ons and the
   scenarios come from the repository folder on every run.
3. **ffmpeg**, for the video: a Windows static build's `ffmpeg.exe` in
   `Documents\visual-check\ffmpeg\`. Without it the check takes the stills only and says so.
4. **The settings**: the first run of `Visual Check.bat` writes `visual-check-settings.txt` in the
   repository folder (gitignored; never commit it) and asks for what it can't find. What's in it:

   | Setting | Default | |
   | --- | --- | --- |
   | `client` | `visual-check-client\benilla.exe` | the dev build's exe (or its folder) |
   | `data` | the one in `settings-windows.txt` | the WoW 1.12.1 `Data` folder |
   | `host`, `port`, `realm` | `127.0.0.1`, `3724`, `Elwynn Survival` | the local test server |
   | `account`, `password` | `spelltest`, `hold` | the test account (test credentials only, local) |
   | `hunter`, `warrior` | `Vchunter`, `Vcwarrior` | the test heroes the scenarios play |
   | `cameraman` | `Vccamera` | the hero whose client films a scenario with a `camera` line ("Filming a boss") |
   | `resolution` | `1280x720` | the client's window while it runs (the cameraman's, with a `camera` line) |
   | `hero_resolution` | `560x315` | with a `camera` line, the hero's own window, beside the cameraman's on the same screen |
   | `monitor` | empty | which screen the game window goes on and the video films: empty or `0` is wherever Windows opens it; `1` is the main screen, `2` the next, and so on (the others in the order Windows lists them); or a name like `DISPLAY2` (Settings > System > Display > Identify shows which is which) |
   | `mute` | `1` | `1` mutes the game's sound in Windows' volume mixer from the moment it has any (the client has no sound setting of its own); `0` leaves it |
   | `format`, `width` | `jpg`, `1280` | the stills: JPG (or `png`), scaled down past this width |
   | `video` | `screen` | `screen` records the game window's picture (inside its title bar and borders) by Desktop Duplication (it sees every frame); `title` and `desktop` are GDI's grabs, which see only a few stale frames of benilla's Vulkan window; `off` none |
   | `ffmpeg` | `Documents\visual-check\ffmpeg\ffmpeg.exe` | |
   | `settle` | `6` | seconds the world gets to load in before a scenario starts |
   | `login_timeout` | `120` | seconds the client gets to log in |
   | `tail` | `5` | with a `camera` line, seconds the video goes on once the fight is over (the hero or the boss fell); `off` films to the scenario's end |

5. **The test heroes**: run the `setup` scenario once (`Visual Check.bat setup`). It makes a dwarf
   hunter, a human warrior and a human cameraman with the names in the settings on the test
   account, through the character screen (benilla's `WOW_PROBE_CHARCREATE`); a hero already there is left alone. Any
   existing heroes on the account do as well: put their names in `hunter` and `warrior`. The class
   only decides the hero's look and gear: each scenario slots its own abilities (`.es spells` owns
   them all).

## Running it

Double-click `Visual Check.bat`: it lists the scenarios and asks which. Or give names:

```
"Visual Check.bat" hunter-aimed-shot
"Visual Check.bat" hunter-raptor-strike warrior-hurled-axe
"Visual Check.bat" hunter-*
"Visual Check.bat" all
```

Each scenario takes about a minute: the client opens (on top of everything, at the window size in
the settings: leave the PC alone meanwhile), logs in, plays the scenario and closes itself. It
never runs past its time limit; a client or a recording left from a run cut short is closed before
the next. One check runs at a time on the PC.

What it leaves in `visual-check-out\<scenario>\`:

- `<burst>-NN.jpg`: the stills, named by the scenario's bursts (`draw-00.jpg`, `flight-00.jpg`...).
- `<scenario>.mp4`: the game window from just before the first burst to the end.
- `run.txt`: every line sent, with when, and every answer from the server (an ability not owned, a
  key with nothing on it, not enough Valor...), and whether the video was made.
- `client.log`: the client's own log, for when it never got into the world.

## The watcher

For checks nobody has to click: `Install Visual Check Watcher.bat` starts the watcher now
(minimized) and at every sign-in (a small .bat in the Startup folder);
`Uninstall Visual Check Watcher.bat` stops it and takes it out. It works in `Documents\visual-check\`:

- **A request** is a text file dropped in `requests\`, `<id>.txt`, naming scenarios (a space or a
  line between; `all`; `setup`; `hunter-*`). It's taken once nothing has written to it for 2 s,
  oldest first, one at a time.
- Its scenarios run into `out\<id>\<scenario>\` (as above); `out\<id>\status.txt` says it's running.
- Then `out\<id>.done`, or `out\<id>.failed` with each failure and the client's last lines, and
  the request moves to `requests\done\`.
- If the test server isn't answering, the request fails at once and `.failed` says so.
- `watcher.log` there has a line for every request and every scenario run.
- `scenarios\` there may hold scenario files too; one there wins over the repository's of the same
  name, so a scenario can be adjusted without a commit.

- **A new build**, with nobody at the PC: a branch's source zip (`git archive --format=zip HEAD`)
  dropped in `deploy\` as `<name>.zip`. Between requests the watcher hands over to a small script
  that stops the server and the watcher, replaces everything in the game folder but `target\`,
  `saves\`, `logs\`, the settings files and the visual-check files, starts the server on it (it
  builds first) and, once it answers, a new watcher from the new source. Each step goes to
  `watcher.log`. Only a test copy made from such a zip is ever replaced: in a git checkout the zip
  is refused (renamed `.refused`).

- **Data questions**: a text file in `queries\` whose lines are `<tool> <args...>`, each run as one
  of the repository's read-only data tools against the WoW data (`cargo run -p htl-map --example
  <tool> -- <Data> <args...>`: `spellfx`, `creaturedisp`, `displays`, `kitanims` and the rest in
  `crates/htl-map/examples/`), to find a look to borrow. The answers go to `out\<id>.queries.txt`.
  Nothing in the game, the data or the repository changes. `godisp path <model path>` measures a
  doodad no display names (`+` for a space in its path).

- **Another client**, for a branch's client patches: the dev build's zip (the Release workflow's
  `dev` artifact) cut into `deploy\clients\<name>\part-0`, `part-1`, ... with
  `<name>.sha256` (the whole zip's SHA-256) written last. The watcher joins the parts, checks the
  hash and unpacks the client into `clients\<name>\`. A request with a line `client <name>` runs
  on it; every other request keeps the default client from the settings.

The watcher needs nothing typed: a missing setting fails the request (in `.failed`) rather than
asking. It uses the repository folder it was installed from, at whatever branch is checked out there.

## Writing a scenario

A scenario is `tools/visual-check/<name>.txt`. Lines (`#` starts a comment):

```
hero hunter                   # which test hero plays it: hunter (the default) or warrior
cam 0 -10 9                   # the camera: yaw, pitch, distance (the default is this)
include _setup                # another file's lines, at their own times
5.0 .es slot q aimed_arrow    # <seconds> <a chat line>, said at that time
6.0 .es in ab 0 0             #   ...the Q key
6.5 .es in a {facing} -0.4    #   ...the left button, aimed down at the ground ahead
6.6 .es in au {facing} -0.4   #   ...and let go
5.5 lua MoveForwardStart()    # <seconds> lua <code>, run in the game's interface
6.0 shots draw 20 0.1         # <seconds> shots <label> <count> <every>: a burst of stills
12 end                        # quit then (default: 3 s after the last line or burst)
4.0 video                     # start the video then (default: 1.5 s before the first burst)
```

- **Times** count from when the hero is in the world and `settle` seconds have passed, whatever the
  login took. `_setup` takes the first 4 s: debug's commands on without its drawing (`.es debug quiet`), every spell owned, cooldowns off, the hero
  healed with full Valor, at Goldshire facing north, one fresh training dummy 6 yd ahead.
- **Casting**: `.es in ab <slot> <ex>` presses an ability key (slot 0 to 4: Q, E, 1, 2, 3; ex 1 is
  Ctrl with it, the EX version). An aimed ability comes up and `.es in a` (the left button) casts
  it, or starts its charge, which `.es in au` lets go. `.es slot <q|e|1|2|3> <ability>` puts an
  ability on a key, by its key in `content/survival.toml`.
- **Aim**: with no aim on the line the server aims along the hero's facing, level. `{facing}`
  (or `{facing+0.3}`, radians) is filled in from where the camera looks when the line is sent, and
  the number after it is the elevation (up positive): about `-0.33` lands at the dummy's feet,
  `-0.56` 3 yd ahead. An aim is only good for 0.6 s, so a charged release carries it again.
  `{aim}` is the camera's facing and elevation both, as the add-on sends them.
- **The camera** is held where `cam` says from the moment the hero is in the world: yaw is a map
  facing in degrees (0 is behind the hero `.es goto` leaves facing north), pitch below 0 looks down.
  `cam off` holds no camera at all, for a scenario that turns it itself (`lua` steps).
- **Bursts**: 0.05 to 0.1 s apart is dense enough to step through an animation; the client takes
  at most one still a frame. Keep the count to what the moment needs: each still is a file.
- **Moving**: a script can't press the movement or jump keys (they're protected), so a strafe is
  `lua ActionDash(<secs>, <yards a second>, "l")` (or `"r"`: a sidestep at that speed) and a jump
  is `.es hop [yards]` (the server throws the hero up where they stand). Press the ability partway
  through either. A dash stops as soon as the hero swims; for a longer walk or a swim (client
  patch 0085 and later) `lua ConsoleExec("probemove <secs> <f|b|l|r|fl|fr|bl|br> [facing]")`
  holds those keys for that long at the swim speed, wading or swimming, facing that map facing
  first (degrees, 0 north, 90 west) when given; `probemove stop` lets go.
- **Tripwire** takes two presses, each a placed arrow, and **Stoop** goes on the 4th E by itself:
  their scenarios play them as #110's rework (#120) has them.

`scripts\visual-check-windows.ps1 -Check` reads every scenario and builds what it would hand the
client, without running anything (the dev Release run does this too).

## Filming a boss

A boss's clip is filmed by a cameraman: a second client, on the same test account, whose hero takes
no part in the fight and whose camera keeps the boss and the hero in view. The hero is played by
the first client, as in any scenario. One line in the scenario asks for it:

```
hero warrior
camera duel Rhahk'Zor          # camera <shot> <the subject's name, as the game shows it>
camera high Cookie|Gorged Cookie  # a boss and the form it comes back as, in turn
camera duel Edwin VanCleef|Edwin VanCleef  # a form that comes back under the same name: named twice
camera wide Gilnid + Furnace Engineer    # a boss, and his crew kept in the picture
```

- **The shots**: `duel` is a three-quarter side view from the hero's side, 8 to 12 yd out (a
  duel; 10 to 14 before patch 0092); `low` is 20 yd out and low (the Shredder); `high` is 45 to 60 degrees up and 15 to 22 yd
  out, so a ground hazard's edge shows; `wide` is 20 to 25 yd out and 30 degrees up, for the
  arena (a crew on their planks, a stripe of fire across the floor). The camera stands further
  back than that only when the two wouldn't fit. Under a low roof (the Foundry's cave) a shot's
  positions are tried 12 and then 24 degrees lower before the camera comes in along its line.
  With no name (`camera duel`) it films the hero alone, looking past them the way they face.
  With a boss's forms named in turn (`Cookie|Gorged Cookie`, `Sneed's Shredder|Sneed`), the living one
  nearest the hero is filmed.
- **The crew**: after a `+`, the names of creatures that are part of the fight (a boss's crew,
  his adds), apart by `|`: `camera wide Gilnid + Furnace Engineer`,
  `camera duel Edwin VanCleef + Defias Blackguard`. Each living one within 15 yd of the boss or
  the hero (along the ground: overhead counts) is kept in the picture with the two, whatever the
  shot: the camera looks at the middle of them all and stands back (to 50 yd at most) until every
  one fits, and prefers positions that see them past the world.
- **Never inside a body, never outside the world**: the camera keeps out of every body's bounds
  (the boss's, the hero's and anyone's near them) by more than a yard, and out of where a body's
  walk takes it in the next second, so it cuts before a charging boss reaches its spot. Each
  position has solid ground below it and a clear line from the fight, and the camera never sweeps
  through a wall between two positions. The point it looks at is one the hero sees, and every
  position sees the hero or the boss from inside the world (patch 0092): round a bend in a tunnel
  their middle is in the void outside the map, and a camera measured out from there stood in the
  void (take 418).
- **Out of view**: one of the two is hidden when most of five points on them (low, chest, head,
  either side) can't be seen, so a thin post or a crate in front doesn't count. A position that
  has lost the hero (hidden for more than 0.3 s) is left once it has been held a second; the
  boss alone has to be hidden for more than a second, and the position held 5 s, to move it
  (patch 0093; before it, every position was held 3 s, and a boss under a plank was a cut every
  3 s while the camera looked at fallen rock for the last 3 s of Rhahk'Zor's collapse). It cuts
  to its clearest position, if that is clearly better: nothing hiding either, little in the
  foreground, then the other side, square to the side, behind, then the same higher up. Both
  hidden for more than 0.3 s cuts at once. Each cut is in `run.txt` with its time and reason
  (`camera: spectate: t=12.3 ...`).
- **A wall filling the picture** (patch 0092): of twenty lines across the picture, out to its
  edges and 85 hundredths of the way to the two, the world on six (three tenths: a cabin's
  corner, a beam, the rock of a bend; the floor under the two doesn't count) for a second is a
  cut too, once the position has been held 5 s (a second, with the world on half the picture,
  nine lines: it has lost the hero), to one clearly better (`the world on 9 of 20 lines of the
  foreground`). So is the hero or the boss cut off by the frame's edge (a wall
  pulled the eye in close behind the hero, take 425: `1 of the two cut off by the frame`). Before it, a corner of
  VanCleef's cabin filled a third to half of his takes for a minute. The fight's axis, which the
  shot's positions turn with, follows the two over 1.5 s, so the camera doesn't swing round into
  a wall as they circle.
- **The end of the fight**: when the hero falls (down for 0.3 s), or the subject does (the last of
  its forms), the camera holds where it is (it doesn't follow the hero to a lamp) and the video
  ends `tail` seconds later (5), with the run. `run.txt` and the summary say when (`the fight is
  over (Vcwarrior fell) at 40.6 s of the video`). Only a fall after the video starts counts: a
  hero caught in the setup (by the pirates at the landing, say) and then teleported and healed
  doesn't end the clip, and the camera films again at once when the hero, up, is teleported
  beside a living subject.
  A form that comes back under the same name (VanCleef, who rises as Unbowed) is named once for
  each: `camera duel Edwin VanCleef|Edwin VanCleef`. The first fall then doesn't end the take: it
  runs on once the next form is up (the cameraman's `the fight is on again`, some 9 s after the
  fall), to that form's fall or the hero's. If nothing rises within 15 s (the secret that lets it
  wasn't found), the take ends there, and the summary says its next form never rose.
- **A teleport** (a setup's `.es goto`: the hero 15 yd or more away within half a second): the
  camera picks its position afresh where the hero is now, straight away, and for 2.5 s looks
  again every quarter second as the world there loads in, so it's on the fight from the video's
  first frame. Until the world there is in, it waits 6 yd from the fight rather than inside a
  rock that isn't solid yet; a far teleport (across the Deadmines) can take 3 or 4 s, so give the
  last `.es goto` that long before the `video` line if you can.
- **What it films**: `<scenario>.mp4` is the cameraman's window from the scenario's video time to
  its end, with sound (below). The stills (`<burst>-NN.jpg`) are frames of that video at the
  bursts' times, within a few tenths of a second; the hero's client takes none. `run.txt` has the
  hero's lines and the cameraman's (`camera: ...`).
- **A clean picture**: no interface, chat, names, floor marks, debug drawing or cursor. The
  cameraman's client hides it all; the scenario still says `.es debug quiet`.
- **Sound**: the cameraman's client writes its own mix to a file (benilla's `WOW_MIX_TAP`), which
  goes under the video. That mix is taken before the client's output, which is silent while its
  window isn't focused and is muted in Windows' mixer anyway (`mute`); the hero's client opens no
  sound device at all. Nobody at the PC hears the game.
- **Setup before the video, nothing after it**: travel, `.es clearfoes`, `.es spawn`, `.es goto`
  and `.es heal` go before the scenario's `video` time. A clip that needs `.es hurt`, `.es heal`
  or a teleport once the fight has started is "dev setup", and proves nothing about the fight:
  say so in its first comment line. From the video on, only the hero's own keys (`.es in ...`,
  `ActionDash`, `.es hop`) play.
- **Go to the logbook before `.es travel`**: `.es goto -9624 675` first. The book only reads
  within reach, and the hero starts each take wherever the last one left them, so a travel without
  it works only after a take that ended at the book (Nov 13: take 404 was refused the travel and
  fell through the open world for 7 minutes). A hero who starts below is sent up into the
  Deadmines' sky instead and the travel is refused ("Go to a lit lamp first."); the scenario's
  own `.es goto` then puts them where it wants, as nothing needs the lamp there.
- **`.es clearfoes <keep>` takes every other foe away until the server restarts**, every other
  boss included (take 405's "Took away 69 creatures"; Greenskin, Cookie, Gilnid, Rhahk'Zor and
  the Shredder were gone for takes 410 to 414). So every boss scenario starts below with
  `.es restock` (debug): every pack and boss stands again, as when a beaten run resets, with the
  lit lamps and found secrets kept. (`.es reset camp` is no help here: it shuts the book until the
  camp's first raid is beaten.)
- **A move's clip** (8 to 20 s) shows the boss's tell, the attack, the right answer, the boss's
  recovery and the punish: time the `video` line and the bursts around them.

`tools/visual-check/camera-greenskin.txt` is the first: the warrior and Captain Greenskin on the
landing's deck.

Gilnid's valve (the race, `docs/gilnid-encounter.md`) has its own set, all sharing
`_gilnid-valve-setup.txt` (the mine cleared but for him and his crew, markers off, and a timer
that presses the hero's keys a set time after his line is heard):

| Scenario | What it films |
| --- | --- |
| `gilnid-valve-stopped` | Dev setup, by the cameraman: his line, the marked channel, the run, a heavy blow at the valve, the mark going out, him open, the finish |
| `gilnid-valve-fire` | Dev setup, by the cameraman from above: nobody stops him; the whistle, the steam, the hero stepping out, the fire down the channel |
| `gilnid-valve-stopped-pov`, `gilnid-valve-fire-pov` | The same two on the hero's own screen |
| `gilnid-valve-stood` | Dev setup, the hero's own screen: a hero who stays in the channel to the end |
| `gilnid-valve-fight`, `gilnid-valve-fight-healed` | A real pull by a small player in the interface, no debug kills; the second healed throughout |
| `gilnid-valve`, `gilnid-ledges`, `gilnid-plank` | The older takes: the valve left alone seen from the side, the crew on their planks, his wait under a plank |

What it needs beyond the rest: a client with patch 0088 (`WOW_SPECTATE`; an older one films the
cameraman's own body) and 0090 for `wide` or a crew (an older one won't film with them), 0092 for the cut away from a wall, 0093 for the
longer hold, the cameraman hero (`setup` makes it), and a screen wide enough for both
windows side by side: the cameraman's (`resolution`) and the hero's (`hero_resolution`) on the
`monitor` screen. The run stops with an error rather than put one over the other.

How it works:

- The cameraman's client logs in first and says `.es watch <hero>`. The server then keeps its hero
  out of every rule: no creature sees it, no blow or area reaches it, nothing collides with it,
  and no count of heroes (scaling, aggro, a raid, an encounter, a dungeon's band) includes it. No
  other client is told of it, and it follows the hero it watches into their world (the camp or a
  dungeon) and stays near them, so the world around the fight is loaded. `.es watch off` makes it
  a hero again; logging out ends it too. It needs debug on (`.es debug quiet`), like the other
  dev commands.
- benilla's `WOW_SPECTATE=<subject>,<hero>,<shot>[,<crew>]` (client patches 0088 to 0090) takes the camera off the
  cameraman's body (free-fly, the body hidden) and frames the two each frame. The rig sets it, the
  cameraman's driver (it hides the interface and names, and says `.es watch`) and `WOW_MIX_TAP`.
- Then the hero's client starts beside it, and the scenario plays as any other; the video records
  the cameraman's window from the hero's cue still. When the hero's client quits, the cameraman's
  is closed, the sound goes under the video and the stills are cut from it.

## How it works, and its limits

The script starts the dev client with a settings folder of the run's own (`BENILLA_HOME`: the
bundled config, the window size, this repository's add-ons), so the add-on under test is the
branch's and none of the player's settings are touched. benilla's env probes do the rest:
`WOW_USER`, `WOW_PASS`, `WOW_CHAR`, `WOW_HOST`, `WOW_REALM` log in and pick the hero;
`WOW_PROBE_LUA` runs the scenario's driver in the interface once the hero is in the world;
`WOW_PROBE_CAM` holds the camera; `WOW_UNATTENDED` ends a run whose login is refused;
`WOW_PROBE_EXIT_AT` is the client's own time limit; `WOW_BG=0` keeps the window full size.

- The driver, not `WOW_PROBE_CHAT` and `WOW_LIVE_SHOT`, says the lines and takes the stills: those
  two count from when the client starts, not from world entry, so a slow login would send all
  the lines due by then in one frame and shoot the loading screen. The driver's stills are the
  game's own `Screenshot()`, with the "Screen captured" line kept out of them.
- The video starts when the driver's first (cue) still lands, so it's within a few tenths of a
  second of the plan. It's taken by Desktop Duplication (ffmpeg's `ddagrab`) from the window's
  rectangle on the screen the window is on, which works while the window is on top (as it is during a run) and
  the screen isn't locked: keep the PC awake and unlocked while checks run. GDI's grabs see only a
  few stale frames of a Vulkan window, so they're the fallback, not the way.
- With `monitor` set the client is told to make its window at the top-left of that screen's working
  area (`WOW_WINDOW_AT`, read by a client with that patch), so it never shows on another screen. A
  client without it makes its window where Windows puts it and answers a move only once it's
  started (about a second and a half in): the rig moves it then (its size kept), and puts it back
  there if the client moves it before the video starts. The video finds that screen's `ddagrab` output by its size, because DXGI numbers outputs in
  an order of its own, and takes the window's rectangle from the screen's own top-left. A number
  this PC has no screen for is said so in the run's output, and the window stays where Windows
  opens it. The stills are the client's own screenshots, so they don't depend on the screen.
- Trapper's Instinct answers a blow; the dummy never strikes, so its scenario shows the stance
  running out. Spawning a boar (commented in the file) shows the answer, but the boar stays.
- Training dummies don't move back after Reverse Grapple or Tripwire; `_setup` clears every dummy
  and puts up a fresh one each run, so nothing piles up.
