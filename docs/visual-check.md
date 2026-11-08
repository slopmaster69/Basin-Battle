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
   | `resolution` | `1280x720` | the client's window while it runs |
   | `format`, `width` | `jpg`, `1280` | the stills: JPG (or `png`), scaled down past this width |
   | `video` | `screen` | `screen` records the game window's rectangle by Desktop Duplication (it sees every frame); `title` and `desktop` are GDI's grabs, which see only a few stale frames of benilla's Vulkan window; `off` none |
   | `ffmpeg` | `Documents\visual-check\ffmpeg\ffmpeg.exe` | |
   | `settle` | `6` | seconds the world gets to load in before a scenario starts |
   | `login_timeout` | `120` | seconds the client gets to log in |

5. **The test heroes**: run the `setup` scenario once (`Visual Check.bat setup`). It makes a dwarf
   hunter and a human warrior with the names in the settings on the test account, through the
   character screen (benilla's `WOW_PROBE_CHARCREATE`); a hero already there is left alone. Any
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
  rectangle on the main screen, which works while the window is on top (as it is during a run) and
  the screen isn't locked: keep the PC awake and unlocked while checks run. GDI's grabs see only a
  few stale frames of a Vulkan window, so they're the fallback, not the way.
- Trapper's Instinct answers a blow; the dummy never strikes, so its scenario shows the stance
  running out. Spawning a boar (commented in the file) shows the answer, but the boar stays.
- Training dummies don't move back after Reverse Grapple or Tripwire; `_setup` clears every dummy
  and puts up a fresh one each run, so nothing piles up.
