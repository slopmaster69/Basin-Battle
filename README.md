# Basin Battle and Elwynn Survival

Two co-op games for up to four players, played in the
[benilla](https://github.com/samwhosung/benilla) World of Warcraft 1.12.1 client with their own
server. Every player needs their own WoW 1.12.1 game files: nothing here contains or shares them.

- **Elwynn Survival** ([its own README](docs/elwynn-survival.md)): a soulslike survival game in
  Elwynn Forest. Attack, guard, parry and dodge with committed blows; pick five abilities from any
  of nine classes; gather, build and hold the camp at Westbrook against the Horde's raids; go below
  into the dungeons for timed Bookmaker's Wager runs; and fight other camps in Siege.
- **Basin Battle** ([its own README](docs/basin-battle.md)): a roguelike wave-defence game in
  Arathi Basin. Hold the Ancient against endless waves of undead, building your hero card by card.

## How it works

There are two halves, like a board game and its players:

| | What it is | Who needs it |
| --- | --- | --- |
| **The server** (the table) | Holds the world, the monsters and every saved hero. Runs both games at once, each on its own realm. | Only the host: one person. |
| **The game** (a seat) | benilla, the game client, with the add-ons. | Everyone who plays, the host too. |

So the steps are always the same: **one person hosts, everyone plays**. Playing alone is just
hosting and playing on the same computer. The server and the game must come from the **same
version** (the same release), or the add-ons and the server won't understand each other; on
Windows, Play and Host keep themselves up to date so they always do.

Heroes and worlds live on the host's computer (`saves/`), so a hero made on a friend's server stays
on that server.

## Windows

One download does everything: playing, hosting and updating.

1. From the [Releases](https://github.com/slopmaster69/Basin-Battle/releases) page, download the
   newest **Elwynn Survival for Windows.zip** and unzip it anywhere, e.g. `Documents\Elwynn
   Survival`. This is the last time you download it by hand: from then on it updates itself.
2. Double-click **Play** (the shield). If Windows says it protected your PC: More info → Run anyway.
   A splash with a painting comes up, says what it's doing, and the game opens, logged in on the
   group's server (the `server` line in `project.toml`) as a hero named after you. That's it.
   - It finds your WoW 1.12.1 install by itself, or asks for the folder once.
   - Move the mouse over the splash for **Play alone** (your own server, started quietly on this
     PC and stopped with the game) and the cog: your name, **someone else's server** (type the
     address they give you), the other game, where your WoW is, and **Make a test copy** (below).
   - When a new version is out, Play brings the whole folder up to date first (a progress line,
     under a minute). Your settings, keys and mouse speed are kept, and the worlds in
     `server\saves` are backed up first, to `server\saves-backups`.
   - Nothing is ever compiled on a player's PC, and there is nothing else to install.

**Hosting for friends:** double-click **Host** and leave its window open. The server comes
ready-made in the download, and its window shows the addresses friends join with (see
[Friends joining](#friends-joining)). Options go after it: `Host.exe --password ember`, or
`--game survival` for only Elwynn Survival. Allow `htl-server` on the network when Windows asks.
Host checks for a new version when it starts and after a warned restart (`Restart Basin
Battle.command`, or the minutes written in `restart.request`).

**Playtesting the newest main:** every merge to main publishes the same download as the `latest`
pre-release, minutes later. Under Play's cog, **Make a test copy** makes `<folder> (test)` beside
your game, on that channel: its Play updates itself to the newest main every time, with its own
heroes and saves, and your real folder never sees it.

**What's in the folder:** Play, Host and README.txt are all you use. `benilla.exe` and
`benilla-config` are the game, `server` is the server and your saved worlds, `version.txt` is what
Play updates from, and `settings.txt` is your choices. `play.log` says what Play did last time.

**Hosting from the source instead** (to try a branch, or change the server): get the code (below)
and double-click **Host Basin Battle.bat** in it. The first run installs Rust if it's missing (no
Visual Studio needed), asks for your WoW 1.12.1 folder once (kept in `settings-windows.txt`) and
builds the server (5-10 minutes). Players join it with Play as usual.

## Mac

1. Get the code: `git clone https://github.com/slopmaster69/Basin-Battle.git` (or the zip from the
   green **Code** button), and open Terminal in that folder.
2. One-time setup, 10-30 minutes the first time: `scripts/setup-mac.sh`. It installs Apple's
   command line tools and Rust if they're missing, downloads benilla, finds your 1.12.1 `Data`
   folder (or asks for it) and builds everything.
3. Host, and leave that window open: `scripts/host.sh` (password **hold**), or
   `scripts/host.sh --password ember`. Or double-click **Host Basin Battle.command**.
4. Play: double-click **Play Basin Battle.command**. It builds Play (the same launcher as the
   Windows download's, a few minutes the first time) and puts **Elwynn Survival.app** in this
   folder: open that from then on, or drag it to the Dock. Play's splash joins the group's server
   logged in, or, under the cog, a friend's address or your own server on this Mac. It plays
   through `scripts/play.sh`, so the client still rebuilds itself after an update; the folder
   itself is updated with `git pull`, not by Play.
   The plain way still works, in Terminal: `scripts/play-survival.sh` for Elwynn Survival or
   `scripts/play.sh` for Basin Battle. For a friend's game add their address:
   `scripts/play-survival.sh 192.168.1.20`.

To update by hand, `git pull` (Basin Battle.app does it when it opens) and double-click **Restart Basin Battle.command** (or give it a number of
minutes, or `cancel`): everyone playing sees a countdown, the worlds are saved, and the server
builds the newest version and starts again. The play scripts rebuild the client when its patches
change.

## Linux (a server only)

`scripts/host-linux.sh` builds and runs just the server, so it works on a headless machine or a
VPS (under `tmux` or `screen`).

- It needs a C compiler and `curl` (`sudo apt install build-essential pkg-config curl`); it
  installs Rust itself.
- It asks for the 1.12.1 `Data` folder once, or set it: `WOW_DATA=/srv/wow/Data scripts/host-linux.sh`.
- Open TCP ports 3724, 8085 and 8086 (`sudo ufw allow 3724,8085,8086/tcp`).

## Friends joining

The host's window prints the addresses to give out.

- **Same house or Wi-Fi:** the "Friends on your own network join with: 192.168.x.x" address.
- **Over the internet, the router way** (the group's default): forward TCP ports **3724**,
  **8085** (Basin Battle) and **8086** (Elwynn Survival) on the router to the host's computer, and
  give that computer a fixed address on the home network so the rule keeps pointing at it. Friends
  use the "Over the internet" address. The Windows host finds it by itself; on a Mac pass it:
  `scripts/host.sh --public-host 203.0.113.7`.
- **Two hosts behind one router:** the second needs its own ports. On Windows, put
  `login_port = 3725`, `world_port = 8087` and `survival_port = 8088` in its
  `settings.txt`, beside Play (on a Mac or Linux, `--login-port 3725 --world-port 8087 --survival-port
  8088`), forward those to it, and its address ends in `:3725`.
- **Over the internet, no router changes (Tailscale):** a private network between your computers,
  free for a few friends. Everyone installs [Tailscale](https://tailscale.com/download) and signs
  in, and the host shares their computer with each friend in Tailscale's admin console. Friends use
  the host's Tailscale address (100.x.y.z): the Mac host prints it, and on Windows it's in the
  Tailscale app. No `--public-host` is needed for them.

| Port | What |
| --- | --- |
| 3724 | Login (both games) |
| 8085 | Basin Battle's world |
| 8086 | Elwynn Survival's world |

## Troubleshooting

- **"Unable to connect"**: is the host's server window open? Right address? Over the internet, are
  the ports forwarded (or are you both on Tailscale)? Are you on the same release as the host?
- **Login says incorrect password**: everyone uses the host's password; the name can be anything.
- **"tell me where your 1.12.1 game data is" / "no WoW install found"**: the folder isn't a 1.12.1
  (build 5875) install; modern WoW Classic won't work. Delete `settings.sh` (Mac),
  `settings-windows.txt` (Windows server) or the game folder's `settings.txt` (Windows game) and
  pick again.
- **Only one game window**: it's titled "benilla". If you're unsure, close them all and start again.
- **Two servers at once**: they share ports, so only one can run on a computer (unless one is given
  other ports, as above).
- More for players (black window, anti-virus, sound, keys): the README.txt in the Windows zip.

## How it fits together

| Folder | What it is |
| --- | --- |
| `crates/htl-server` | The login and world servers benilla connects to (both games). |
| `crates/htl-sim` | Basin Battle's rules: waves, enemies, combat, the hub. No networking. |
| `crates/htl-survival` | Elwynn Survival's rules. |
| `crates/htl-map` | Builds the worlds from your 1.12.1 data: terrain, buildings, water, roads. |
| `content/` | Both games' content and numbers (`game.toml` and `classes/` for Basin Battle, `survival.toml`, `siege.toml` and the dungeons for Elwynn Survival). Edit and restart the server. |
| `addon/` | The two add-ons every player runs; each sleeps until its own server answers. |
| `patches/benilla/` | Our changes to the client, applied in order on the pinned benilla by the setup and play scripts (and the release build). |
| `scripts/` | Setup, host and play scripts; `dist/windows/` is what the Windows zip carries. |
| `docs/` | Each game's README, the writing style, and the class references. |

`cargo test` runs the rules' tests and end-to-end tests where benilla's own protocol code logs in,
plays and checks every packet the server sends. `scripts/check-addon.sh` runs the add-ons in
benilla's own UI script host against the lines a real session sends, and lists any Lua error.

## Working on it together

Several people and AI agents work on this repository at once. Before changing anything, read
[AGENTS.md](AGENTS.md): sync first, small branches, test and push often, merge through pull
requests once CI passes. A release is a tag on `main`; pushing it builds the Windows zip.

## Legal

Basin Battle and Elwynn Survival are an independent fan project, not affiliated with or endorsed
by Blizzard Entertainment. They ship no Blizzard content. World of Warcraft is a trademark of
Blizzard Entertainment, Inc. Our code is licensed under [MIT](LICENSE-MIT) or
[Apache 2.0](LICENSE-APACHE), at your option.
