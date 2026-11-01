# Basin Battle

A co-op roguelike wave-defence game for up to four players, played in the
[benilla](https://github.com/samwhosung/benilla) World of Warcraft 1.12.1 client. Endless waves of
undead march on the Ancient of the Basin in the middle of Arathi Basin. Build your Warrior, Priest,
Mage or Hunter as you go: pick class spells, upgrades and talents that change how they play, gear up from Militia greens to glowing legendaries, raid
the four outer bases, and see how deep you can get.

One player hosts a small server on their computer; everyone (the host included) plays with
benilla. Every player needs their own WoW 1.12.1 game files: nothing here contains or shares them.

The same server also runs **Elwynn Survival**, a separate experiment on its own realm: a footman
of Goldshire with an axe, a shield, a dodge and a bow, gathering, building a camp and holding it
against the Horde: orcs, trolls, tauren and Forsaken who fight in souls-like strings of blows,
three raids that each end with a champion, and the journal to lead the way. Its book also opens
**Siege**: one against one up to four each for themselves in Arathi Basin, each holding a camp
against the waves the others buy and send. See [docs/elwynn-survival.md](docs/elwynn-survival.md); play it with
`scripts/play-survival.sh` (or **Play Elwynn Survival.bat** on Windows).

## Get the code

```sh
git clone https://github.com/slopmaster69/Basin-Battle.git
cd Basin-Battle
```

Or download it as a zip from the green **Code** button on GitHub. Either way you get everything
except the WoW 1.12.1 game files, which every player brings themselves: the server, the add-ons,
the client patches (the setup script downloads benilla itself and applies them), and the
scripts. Windows players who only want to play can take the ready-made zip from the
[Releases](https://github.com/slopmaster69/Basin-Battle/releases) page instead.

## Host a game (Mac)

1. Open Terminal in this folder and run the one-time setup (10-30 minutes the first time):

   ```sh
   scripts/setup-mac.sh
   ```

   It installs Apple's command line tools and Rust if they're missing, downloads benilla, finds
   your 1.12.1 `Data` folder (or asks for it) and builds everything.

2. Start the server and leave that Terminal window open:

   ```sh
   scripts/host.sh                    # password: hold
   scripts/host.sh --password ember   # or choose one
   ```

   To restart it after an update without dropping anyone unwarned, double-click **Restart Basin
   Battle.command** (or run it with a number of minutes, or `cancel`): everyone playing sees a
   countdown, the worlds are saved, and this window builds the newest version and starts it again.

   The server's log goes to `logs/host-<date>.log`. After each wave it writes `[balance]` lines:
   the enemies' level, health and swing, and for each hero their health, armour, the damage they
   took (hits, average, biggest and from what), falls and damage dealt.

3. In a second Terminal window, start playing:

   ```sh
   scripts/play.sh
   ```

   At the login screen use **any name** and the server's **password**. Pick a class on the
   character screen and enter the world.

## Host a game (Windows)

Double-click **Host Basin Battle.bat** in this folder (or run `scripts\host-windows.ps1` in
PowerShell) and leave the window open while you play.

- The first run installs Rust if it's missing (a few minutes; no Visual Studio needed), asks for
  your WoW 1.12.1 folder once (remembered in `settings-windows.txt`) and builds the server
  (5-10 minutes). Later runs rebuild only what changed.
- Options go after it, as on the Mac: `"Host Basin Battle.bat" --password ember`.
- When Windows asks whether `htl-server` may use the network, allow it.
- To play on the same PC, use the Windows zip's **Play Basin Battle.bat** with the address
  `127.0.0.1`.

## Host a game (Linux)

Run `scripts/host-linux.sh` in a terminal (or under `tmux`/`screen` on a server). It builds only
the server, so it works on a headless machine or a VPS.

- It needs a C compiler and `curl` (`sudo apt install build-essential pkg-config curl` on
  Debian/Ubuntu); it installs Rust itself if it's missing.
- It asks for the 1.12.1 `Data` folder once, or set it: `WOW_DATA=/srv/wow/Data scripts/host-linux.sh`.
- Open TCP ports 3724, 8085 and 8086 in the firewall (`sudo ufw allow 3724,8085,8086/tcp`).

## Friends joining

**On the same network:** they run `scripts/play.sh <your computer's address>`, e.g.
`scripts/play.sh 192.168.1.20` (System Settings → Wi-Fi → Details shows your address).

**Over the internet:**

1. On your router, forward TCP ports **3724**, **8085** and (for Elwynn Survival) **8086** to your
   Mac.
2. Find your public address (search "what is my IP"), and host with it:
   `scripts/host.sh --public-host 203.0.113.7`
3. Friends play with that address. If macOS asks whether `htl-server` may accept incoming
   connections, allow it.

**Over the internet, without touching your router (Tailscale):** a private network between your
computers, free for a few friends. Nothing to forward, and nobody outside it can reach the server.

1. You and each friend install [Tailscale](https://tailscale.com/download) and sign in.
2. Invite them: in Tailscale's admin console, **Share** your Mac with each friend's account (or
   invite them to your network).
3. Host as usual (`scripts/host.sh`, or **Host Basin Battle.command**): it prints "Friends on your
   Tailscale network join with: 100.x.y.z".
4. Friends play with that 100.x.y.z address (on Windows, type it in the launcher). The server
   sends Tailscale players to the same address for the worlds, so `--public-host` isn't needed for
   them, and friends who forward ports can still join the usual way at the same time.

**Friends on Windows:** share `Basin Battle for Windows.zip` (in this folder once it's built) with
them, e.g. through a Dropbox link or the repository's Releases page. It holds the ready-made client, a launcher and a README. They
unzip it, double-click **Play Basin Battle.bat**, pick their own WoW 1.12.1 folder and type your
address. Nothing to install or build. The zip contains no WoW files: each friend needs their own
1.12.1 install.

## Playing

- **The interface:** a small add-on that comes with the game draws the card picker, your spell bar,
  the wave tracker at the top of the screen, an objective tracker on the right (what to do right
  now) and the big banners. Pick cards with a click or keys 1-4, reroll
  with R. The world pauses while anyone is choosing (up to a minute, then the first card is taken),
  so picks never pile up and nobody is hit while reading.
  The add-on sets up the keys once: A and D strafe (turn with the mouse or the arrow keys), spells
  on 1, 2, 3, 4, Q and E, and B opens the shop. Keys you've changed yourself are left alone;
  rebind Action Button 1-6 in Key Bindings to move the spell keys (the bar shows the new keys).
  Drag a spell onto another button (or Shift-click both) to swap their keys.
- **Health bars:** the add-on turns on friendly nameplates, so treants, springs, defenders and
  the party show their health (V adds the enemies'). Defences wear down when packs stop to
  attack them (enemy blows land x1.2 on them; bosses' at their plain weight).
- **Taking hits:** normal enemies hit heroes 1.2x their listed damage at wave 1, 0.08x more each
  wave (about 2.5x by wave 20), and enemy health compounds faster from wave 4, so the waves keep
  outgrowing the heroes' spells and gear; armour now counts (a
  full plate set takes about half of every blow, cloth about a quarter; the curve stretches with
  each Escalation as gear's armour grows). Warriors have the most health and take 10% less from
  everything. The one share-of-health strike is the rare Plague Butcher's marked cleave (40%).
- **Enemy roles:** Plague Sappers run past everything to blow a share off the Ancient's health
  (stop them on the way); Shrieking Heralds' war cry drives nearby enemies on (25% more damage,
  20% faster); bases are guarded by a swarm to push through. The endless stream pauses while a
  boss leads a wave.
- **Danger circles:** where an enemy's ability will land (or is burning), a glowing circle
  marks the ground at its full size. Hunters' traps show on the ground where they lie.
- **Falls hurt:** as in 1.12, a drop of more than about 28 yards takes health, and about 80 is
  the end of you.
- **Damage meter:** on the right, under the fronts: Party ranks everyone's damage this wave,
  with lines for the Base defences (treants and the Ancient) and the Soldiers (Basin Defenders);
  You breaks your own down by spell (pets and auto-attacks included), with a Reset.
- **Six spells:** cards' spells and the spells talents teach share the six keys. Once all six
  are full, no more new spells are offered: only upgrades and passive talents.
- **Start of a run:** pick one of three core spells of your class, one from each talent tree
  (a Mage chooses between Arcane Missiles, Fireball and Frostbolt). Your spells sit on the first
  six buttons of the action bar: press 1, 2, 3, 4, Q or E (or click) to cast them at your target, or at the
  nearest enemy when you have none. A short global cooldown keeps a rhythm; pressing a key just before it ends queues the spell.
  Mage and Priest nukes and heals (and Aimed Shot) have cast times: stand still until the bar fills,
  moving interrupts it. Arcane Missiles and Mind Flay are channelled: their missiles fly
  over two seconds while you stand still, and stepping away loses the rest. Presence of Mind, Inner Focus and Clearcasting make the next one instant.
  Casting by hand
  deals 15% more than letting them fire on their own (`.bb auto`, or the Auto/Manual switch above
  the bar, toggles that; without the add-on they always fire on their own). Right-click enemies to
  auto-attack as well. Mages, Priests and Hunters attack from range; Warriors build rage by
  fighting and spend it on their strikes.
- **Spells, upgrades and talents:** every level a window offers three cards: a new spell of your
  class (up to six), an **upgrade** for one you hold, a **class talent**, a **combination** (two
  spells you hold that go together) or, now and then, a **path**. Every class levels the same
  way: each spell has its own deck of about 15 upgrades and takes 10 of them in a run, and its
  last card is an **Epic** that waits for 5 of them. Rarity means something: Common cards are
  numbers, Uncommon change how you play, Rare light a key, reset or cast, Epic are capstones,
  Legendary the paths. Press N for **Your Build**. Hover a card to see what it does now and
  after. Closed the window? Type `.cards`. Every card of every class is listed in `docs/`
  (warrior-cards.md, mage-cards.md, priest-cards.md, hunter-cards.md); each class's `-roles.md` says what every spell is for.
- **The Warrior:** all
  Warrior skills are cards, and each skill has its own deck of about 16 upgrades of which it
  takes 10 in a run: chunky rank-ups, **choices** that close each other, follow-ups that open
  after their first step, upgrades that change how the skill plays, and an **Epic** capstone
  that comes only after 5 of its upgrades (Bladestorm, Spellbreaker, Avatar of Vengeance,
  Rage Unbound...). Some light up a key: the skill is ready at once and its next cast is
  empowered; the key pulses gold until you press it. When a card's effect goes off in a big way
  its name pops over the bar. **Class talents** come whatever you hold, and **combinations**
  come only while you hold both of their skills (Charge lands with a free Thunder Clap: 45 of
  them). **Rarity means something:** Common cards are numbers, Uncommon change how you play,
  Rare light a key, reset or cast, Epic are capstones; better cards get likelier as you level,
  Rare and Epic cards glow and chime, and 6 offers without a Rare guarantee one. Press N for
  **Your Build**. Every card is listed in [docs/warrior-cards.md](docs/warrior-cards.md)
  (written from the game's files by `tools/card_catalog.py`).
- **Every Warrior skill has one job, and the world asks for it** ([docs/warrior-roles.md](docs/warrior-roles.md)):
  elites shrug off 10% of every physical blow and bosses 20% until **Sunder Armor** strips it
  (for the whole team); **Thunder Clap** makes a pack attack 30% slower; **Hamstring** cripples
  one enemy (it must fight you, can't run to the Ancient, and takes 20% more from everyone);
  **Mortal Strike** wounds (no healing for 10 sec); **Heroic Strike** is off the global cooldown
  and feeds on rage; **Execute** spends your spare rage; **Death Wish** is as in WoW (more
  damage dealt, and taken).
- **Enemy casts to break:** Bonecasters cast Soul Bolt, Dark Menders cast Dark Mending (heals
  their pack), and every boss casts Dark Ritual. They come with a cast bar and a warning. A
  warrior with **Shield Slam** (the interrupt) sees every breakable cast in reach: the bar
  pulses **INTERRUPT!** and the Shield Slam key lights up; the slam breaks the cast
  (**INTERRUPTED**) and the caster reels. Players without an interrupt see only the casts aimed
  at them.
- **Warrior weapon paths:** one a run, as a level-up card that comes at random. **Path of the
  Shieldbearer** trades the greatsword for a one-handed sword and a shield (tougher, blocks),
  **Path of the Berserker** for a sword and an axe (faster, extra swings), **Path of the
  Greatsword** keeps it in both hands (bigger crits). A path teaches nothing by itself: it opens
  its two skills to the skill cards (Shield Slam and Shield Wall; Bloodthirst and Death Wish;
  Mortal Strike and Sweeping Strikes; only that path ever sees them, and they get keys of their
  own so a full bar never shuts them out) and its own deck of path cards, with an Epic after 4.
  The weapons that drop afterwards follow your path.
- **The Mage levels the same way now** ([docs/mage-roles.md](docs/mage-roles.md), every card in
  [docs/mage-cards.md](docs/mage-cards.md)): every spell is a card with its own deck and an Epic,
  and every spell has one job (Frostbolt chills, Arcane Missiles melts elites, Frost Nova is your
  escape, Fire Blast shatters the Frozen, Scorch leaves Fire Vulnerability, Blizzard holds a lane,
  Cone of Cold peels...). New: **Counterspell**, the Mage's interrupt (no path needed). A
  **Specialization** (Fire, Frost or Arcane) comes at random as a Legendary card and opens its
  spells (Pyroblast and Combustion, Ice Barrier and Cold Snap, Arcane Power and Presence of Mind)
  and its own path cards. Some lit keys make the next cast instant (Brain Freeze, Hot Streak).
- **The Priest levels the same way now** ([docs/priest-roles.md](docs/priest-roles.md), every card in
  [docs/priest-cards.md](docs/priest-cards.md)): Smite's damage mends the most hurt ally near you
  (Atonement), Shadow Word: Pain spreads, Mind Blast bursts, Holy Nova heals and hurts all around
  you, Power Word: Shield, Renew, Flash Heal and Prayer of Healing keep the team up, Psychic Scream
  scatters, Devouring Plague drains. A **Specialization** comes at random as a Legendary card:
  Holy (Lightwell and Greater Heal), Discipline (Power Infusion and Inner Focus) or Shadow
  (**Silence**, the Priest's interrupt, and Vampiric Embrace, shadow that heals the team).
- **The Hunter levels the same way now** ([docs/hunter-roles.md](docs/hunter-roles.md), every card in
  [docs/hunter-cards.md](docs/hunter-cards.md)): Arcane Shot is the shot on the run, Serpent Sting
  the poison, Claw your wolf's strike, Multi-Shot the spread, Concussive Shot the slow, Hunter's
  Mark the team's call (the marked takes more from everyone), Volley the rain, the two traps the
  ambush and the lockdown, Furious Howl the rally, Feign Death the escape, Scatter Shot the peel.
  A **Specialization** comes at random as a Legendary card: Beast Mastery (Bestial Wrath and
  Intimidation), Marksmanship (Aimed Shot and Rapid Fire) or Survival (Wyvern Sting and
  Deterrence).
- **Legendary cards** (the weapon paths and specializations, Escalation picks) stand in a turning
  sunburst, land last with a flash and glow brighter than the rest.
- **Shoulder to Shoulder:** heroes within 12 yards of each other deal 10% more damage and take
  10% less (the crown on your buff bar). Hold a bridge together, but spread out of marked ground.
- **Pets:** Hunters fight with a wolf that holds enemies' attention and comes back after it falls.
- **Levels and gear:** heroes start at level 1 and reach 60 around wave 30. Gear drops after waves
  and from elites and bosses into your bags, never onto you: the message says whether it beats
  what you wear there, and you right-click it in your bags to put it on (or keep the look you
  like). A piece above your level says when you can wear it. Each Escalation brings the next tier
  of your class set, and four or six set pieces unlock bonuses.
- **The lanes:** every lane runs from a base's flag to the Ancient, and each base sends its own
  dead: the Farm ghouls, plague rats and farmhands; the Stables skeletons and chargers; the Gold
  Mine gargoyles, tunnelers and crypt fiends; the Lumber Mill banshees, lost woodsmen and rotting
  treants. From wave 2 every base you haven't claimed also sends a steady stream between waves.
  Waves march quickly down the lanes and slow to their own pace when they reach the fight. The
  first wave is a warm-up with no elites.
- **Bases:** from wave 3, Blight Gates open at the Farm, Stables, Gold Mine and Lumber Mill. Kill a
  base's guardians (they drop next-tier gear, and grow with every wave you leave them), open its relic cache (a rare card and a set piece of
  your choice), then right-click its banner to claim it (a 6 second channel; hits slow it down a
  little, moving breaks it). Each claimed base gives the whole team a boon and thins its lane. The
  Hearthstone in your bags (or the rune button by the wave tracker) takes you home.
- **Roads and bridges:** enemies keep to Arathi's roads and never wade the river. Each base sends
  half its wave down each of the two roads leaving it, and every road into the Blacksmith crosses
  one of its two bridges, so waves from different bases meet at the crossroads and the bridges:
  hold a bridge and you hold several lanes at once. Enemies on the longer road walk faster, so both
  halves arrive together.
- **Bridge posts:** each bridge has a standard on the Blacksmith's side. Defences can be built
  there as well as by the Ancient: pick the place with the tabs on the Shop's defences row (bought
  at home like everything else). Treants flank the road, springs stand behind, defenders block the
  bridgehead.
- **Fronts:** under the objective tracker, a panel shows each bridge (and the Ancient): between
  waves, how many of the next wave come that way, which special enemies, and where the boss
  comes; during a wave, how many are on their way now. Click Hold to tell everyone you've got a
  bridge (your name shows on it), or Help! to call for help where you stand. Without the add-on:
  `.hold 1`, `.sos`, and the preview is in chat. When nobody is near the Ancient and it's hit,
  everyone hears where from ("under attack from the north-east!").
- **Marked strikes:** many enemies wind up a big hit on the ground first: the spot is marked, then the
  blow. Step out of it (a green "Dodged!" says you made it). Bosses and chargers also mark lines (step aside) and the Archlich a ring
  of frost that is safe only inside, by its feet, right after a Death Nova that sends you running
  out. Plague Bloats from wave 5 waddle up and burst where they stand; killing a
  caster before its strike lands cancels it.
- **Waves with a twist:** wave 7 (and 17, 27...) is the Swarm, twice as many enemies at half
  the toughness and no elites; wave 12 (22, 32...) is the Vanguard, a few enemies, every one an
  elite. The preview between waves says so.
- **Elites with a twist:** a wave's elites come Vampiric (heal as they hit), Frenzied (attack
  fast), Swift (run fast) or Thorned (hurt whoever hits them up close: fight them from range).
- **Boss fights:** every fifth wave a boss leads. At half health it calls its dead to its side
  (skeletons, bloats, gargoyles, bonecasters), and at a quarter it rages and hits much harder.
  A fallen boss leaves a **power** for each hero to choose: the boss's own (Kharos's crown raises
  your kills, Gutrender's hunger makes you bigger and drinks life, Vesh's wings speed you, Morvane's
  phylactery wards you) or one of your class's (Bladestorm, Meteor, Plaguebringer, Volley Master
  and more). Powers last the run, and many show on you: an aura's glow, or a bigger frame.
- **The map:** press M. The add-on marks enemies, elites and bosses, your defences, the bridge
  posts and your party on Arathi's own map, live, and a call for help pulses red where it was
  made.
- **Enemies with a job:** from wave 4 Skeletal Bonecasters shoot from range, from wave 6 Scourge
  Siegebreakers walk past you to wreck your defences, from wave 7 Dark Menders heal the others
  (kill them first), from wave 8 Ghoul Runners take the long road and run past everyone for
  the Ancient, from wave 9 Scourge Necromancers raise skeletons while they fight (kill them
  first), from wave 11 Bone Constructs fall apart into three skeletons when they die, and from wave 12
  Restless Wraiths drift after you, pulsing death around them: keep away or burn them down. The
  line under each name says what it does.
- **Victory:** hold wave 20, led by Archlich Morvane, and the run is won: a banner, 50 Renown, and
  a VICTORY mark on the boards. The waves go on after, endless, for a deeper record.
- **Escalation:** claiming all four bases escalates the run: enemies grow much stronger, a new
  affix joins, everyone picks a legendary, and the bases reset with fresh guardians. Legendaries
  include your class's own (Titan's Grip and Unbreakable for warriors, Archmage and Frostbinder,
  Saint and Void Lord, Pack Leader and Deadeye) and everyone's (Colossus, Stormcaller, Wrath of the
  Basin, Guardian of the Ancient, Avatar, Twin Spirits...); one you already hold comes up less
  often than new ones. They're in `content/powers.toml` and `content/cards.toml`.
- **The shop:** Bruni Ironhammer, the dwarf with the hammer by the Ancient, sells everything: go
  home between waves to buy. Near her (or the Ancient), B, the Shop button above your spell bar
  or a right-click on her opens the shop; away from home it tells you to come back. Every
  purchase is a card showing its price and what it does. Build its defences, which stand in the world
  around it: Treant Protectors (up to 4) hurl bolts at enemies in reach, Healing Springs (up to 2)
  heal everyone near them, Basin Defenders (up to 6) fight at its feet, Basin Riflemen (up to 4)
  shoot from behind the line, Basin Clerics (up to 2) heal heroes and soldiers near them, and
  Fortifying makes them
  all stronger. Enemies passing a defence stop to tear it down, but each one holds at most four at
  once: the rest march on past it to the next defence, a hero or the Ancient; a fallen one can be bought again,
  and everything standing is mended between waves. It also sells more health, Thornbark, weapon
  enchants (10% more power for your spells of that school), and, once a wave, the Wrath of the
  Ancient: every enemy within 35 yards of it rooted for 5 seconds and a quarter of its health torn
  away, for when the wave is at the door. Gold comes from kills and a bounty for every wave cleared.
- **Your soldiers:** the shop's soldiers tab trains them. **Promote** raises every Basin
  Defender a rank, each one stronger and dressed for it: Basin Footman (sword and shield), Knight
  of the Basin, then Stormwind Guard. **March Orders** (free) send a soldier from the Ancient to a
  bridge post or back: pick where under SEND TO, and they walk there along the roads, so send them
  before the wave. Riflemen and Clerics stay behind the line wherever they're posted.
- **Items (the shop's Hero tab):** five rings, necklaces, trinkets, cloaks, tabards or helms on
  offer to each hero, new ones every wave (or Restock for a little gold). They fill the slots the
  class sets leave empty (neck, two rings, two trinkets, back, tabard); a helm takes the set
  helm's place (and its share of the set bonus). Cloaks, tabards and helms show on you. A worn one
  can be **reforged** to the tier dropping now, so a favourite keeps up. Each changes how you
  play: a Heartwood
  Locket mends the Ancient as you fight, a Cracked Phylactery raises your kills as skeletons, an
  Amulet of the Glass Cannon trades toughness for damage, a Trapper's Kit widens a hunter's traps.
  Each card says what you wear in its slot now. Cloaks hang on your back. Bought ones go straight
  on if a slot is free, else into your bags: drag them onto your character to swap. They're all
  in `content/items.toml`.
- **Falling:** a fallen hero gets up at the Ancient after a few seconds (8, plus one a wave, at
  most 20). Only the Ancient falling ends a run.
- **Endless:** waves never stop growing. If the Ancient falls, the run ends and your depth is recorded: `.best` shows the boards. Every run earns **Renown**, which
  stays with your name: `.renown` buys rerolls, banishes, skips, wider offers and more. It never
  adds raw power.
- Chat commands: `.start`, `.status`, `.cards`, `.pick 1-4`, `.hearth`, `.best`, `.renown`,
  `.hold <n>`, `.sos`, `.restart`, `.bb auto`. Press N for talents.

## Tuning

Everything is in the `content` folder: [`game.toml`](content/game.toml) (classes, enemies, scaling,
bases, affixes, Renown), [`classes/`](content/classes) (each class's spells, upgrades and talent
trees), [`cards.toml`](content/cards.toml) (legendaries), [`powers.toml`](content/powers.toml) (class
legendaries and bosses' powers), [`items.toml`](content/items.toml) (the shop's items) and
[`gear.toml`](content/gear.toml) (set looks, names and stat budgets). Edit them and restart the
server. High scores and Renown are kept in `saves/basin-battle.toml`.

## How it fits together

| Folder | What it is |
| --- | --- |
| `crates/htl-sim` | The game rules: waves, enemies, combat, the hub. No networking. |
| `crates/htl-map` | Builds the arena from your 1.12.1 data: terrain, buildings, water, lanes. |
| `crates/htl-server` | The login and world servers benilla connects to (Basin Battle's and Elwynn Survival's). |
| `crates/htl-survival` | Elwynn Survival's rules. |
| `content/game.toml` | Game content and numbers. |
| `scripts/` | Setup, host and play scripts. |
| `patches/benilla/` | This folder's changes to the client (Elwynn Survival's action mouse and aiming camera, then its action animation: predicted swings, hit-stop, camera shake, the dodge roll, aim pitch, and a camera that stays out of bodies; then the action layer: clips that own the body with blends and an upper-body split, local lunges and dashes, reactions, lock-on and its camera, and the camera kick; then a blow's weight played the moment it connects, the lock-on as the target, shuffle steps, kits and voices on call; then hit-stops that never leave a stance stuck, flinches that leave a blow in hand playing, impacts that land with the blow, and the bow's aim on the locked foe), applied on top of the pinned benilla by the setup and play scripts, which rebuild the client when they change. |

`cargo test` runs the rules' tests and an end-to-end test where benilla's own protocol code logs
in, plays and checks every packet the server sends. `scripts/check-addon.sh` runs the add-on in
benilla's own UI script host against the lines a real session sends, and lists any Lua error.

Balance tools (run by hand, `cargo test --release -p htl-sim <name> -- --ignored --nocapture`):
`power_curve` (hits to kill, wave by wave), `class_dps_report`, `solo_survival_sweep` (16 seeds
per class; `BRIDGED=1` for two bridges, `DODGE=1` for bots that step out of marked ground),
`defence_share_report` (four bots: who does the killing, the Ancient's lowest health and hero
deaths per wave) and `solo_death_report` (`TRACE=1` to follow a bot). On the host's own map,
`cargo run --release -p htl-map --example roads -- <Data> soak 21` plays many waves to catch
panics and stuck enemies.

## Troubleshooting

- **"tell me where your 1.12.1 game data is"**: delete `settings.sh` and run again to re-pick the
  `Data` folder. It must be a 1.12.1 (build 5875) install, not the modern WoW Classic.
- **Login says incorrect password**: everyone uses the host's password; the name can be anything.
- **Friends time out**: check the ports are forwarded (3724, 8085, 8086) and that you hosted with
  `--public-host`.
  Or skip the router entirely and play over Tailscale (above, under "Over the internet, without
  touching your router").

## Working on it together

Several people and AI agents work on this repository at once. Before changing anything, read
[AGENTS.md](AGENTS.md): sync first, small branches, test and push often, merge through pull
requests once CI passes.

## Legal

Basin Battle is an independent fan project, not affiliated with or endorsed by Blizzard
Entertainment. It ships no Blizzard content. World of Warcraft is a trademark of Blizzard
Entertainment, Inc. Our code is licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE),
at your option.
