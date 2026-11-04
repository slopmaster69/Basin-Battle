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
version** (the same release), or the add-ons and the server won't understand each other.

Heroes and worlds live on the host's computer (`saves/`), so a hero made on a friend's server stays
on that server.

## Windows

Everything comes from the [Releases](https://github.com/slopmaster69/Basin-Battle/releases) page.
Use the newest release, and take both parts from the same one.

**To play on someone else's game** you only need the game:

1. Download **Basin Battle for Windows.zip** and unzip it anywhere, e.g. `Documents\Basin Battle`.
2. Double-click **Play Elwynn Survival.bat** (or **Play Basin Battle.bat**). If Windows says it
   protected your PC: More info → Run anyway.
3. The first time, pick your WoW 1.12.1 folder (the one with WoW.exe in it).
4. Type the host's address and press Enter. Next time Enter keeps it.
5. Log in with **any name** (it becomes your hero) and the host's password (**hold** unless they
   changed it).

**To host** (for yourself, for friends, or both), you also need the server:

1. From the same release, download **Source code (zip)** and unzip it, e.g. to
   `Documents\Basin Battle server`.
2. Double-click **Host Basin Battle.bat** in it and leave that window open while anyone plays; close
   it to stop the server. It runs both games.
   - The first run installs Rust if it's missing (no Visual Studio needed), asks for your WoW
     1.12.1 folder once (kept in `settings-windows.txt`) and builds the server (5-10 minutes).
     Later runs rebuild only what changed.
   - Allow `htl-server` on the network when Windows asks.
   - Its window shows the addresses friends join with (see [Friends joining](#friends-joining)).
   - Options go after it: `"Host Basin Battle.bat" --password ember`, or `--game survival` for
     only Elwynn Survival.
3. **To play on your own server**, start the game as above and type the address **127.0.0.1**.

**To update**, close the game and the server, then take both parts from the new release: unzip the
new game over the old game folder (your settings, mouse speed and keys are kept), and unzip the new
source over the old server folder or into a fresh one. Keep the server folder's `saves` (your
worlds) and `settings-windows.txt`; copy them over if you used a fresh folder. Back up `saves`
first.

## Mac

1. Get the code: `git clone https://github.com/slopmaster69/Basin-Battle.git` (or the zip from the
   green **Code** button), and open Terminal in that folder.
2. One-time setup, 10-30 minutes the first time: `scripts/setup-mac.sh`. It installs Apple's
   command line tools and Rust if they're missing, downloads benilla, finds your 1.12.1 `Data`
   folder (or asks for it) and builds everything.
3. Host, and leave that window open: `scripts/host.sh` (password **hold**), or
   `scripts/host.sh --password ember`. Or double-click **Host Basin Battle.command**.
4. Play, in a second window: `scripts/play-survival.sh` for Elwynn Survival or `scripts/play.sh`
   for Basin Battle. For a friend's game add their address: `scripts/play-survival.sh 192.168.1.20`.

To update, `git pull` and double-click **Restart Basin Battle.command** (or give it a number of
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
- **Over the internet, the router way:** forward TCP ports **3724**, **8085** (Basin Battle) and
  **8086** (Elwynn Survival) on the router to the host's computer. Friends use the "Friends on the
  internet" address. The Windows host finds it by itself; on a Mac pass it:
  `scripts/host.sh --public-host 203.0.113.7`.
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
- **Two servers at once**: they share ports, so only one can run on a computer.
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
