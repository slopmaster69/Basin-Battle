# Elwynn Survival

A small survival and action-combat experiment beside Basin Battle, on the same benilla client and
the same host. You are a footman of Goldshire: fight with aimed, committed blows, a shield and a
dodge; gather timber, meat and Fargodeep copper; bank it at the camp; build a cooking fire
and a smithing anvil in the meadow; and, when you're ready, ring the bell and hold the camp
against the Horde's raids. Past the camp the whole of Elwynn Forest is open, from Northshire to
the Westfall bridge and the Redridge road: its places are marked on the world map, and each has
its foes, its goods, its chests and secrets, and the shrines and champions that teach the camp
its cards.

It is a prototype for playtesting the brief's first loop, not a finished game.

## Running it

The host runs both games at once. Each has its own realm on the one login:

| Game | Realm | World port |
| --- | --- | --- |
| Basin Battle | `Basin Battle` | 8085 |
| Elwynn Survival | `Elwynn Survival` | 8086 |

```sh
scripts/host.sh                      # both games (password: hold)
scripts/host.sh --game survival      # only Elwynn Survival (or --game basin)
scripts/play-survival.sh             # play it on this Mac (scripts/play.sh is Basin Battle)
scripts/play-survival.sh 192.168.1.20   # a friend's game
```

Windows players double-click **Play Elwynn Survival.bat** in the Windows folder (Play Basin
Battle.bat is the other game). Over the internet the router must forward TCP **3724, 8085 and
8086**.

The survival world is saved in `saves/elwynn-survival.toml` (a server option, `--survival-save`,
moves it). Delete that file to start the world over. Its tuning is `content/survival.toml`
(`--survival-content` to try another copy).

Both add-ons are installed for every player; each one sleeps until its own server answers its
hello, so neither changes the other game's keys or screen.

## Controls

The controls follow the soulslike keyboard and mouse: the mouse steers (in action mode the view
turns the hero, as in a shooter), left mouse attacks, right mouse raises the guard, Space is the
dash button (tap to dodge, hold to sprint), and the five abilities sit on Q, E, 1, 2 and 3.

The Elwynn layout is set for the session whatever your saved bindings say, and your own bindings
come back when you leave. The Controls card (K, or the key icon bottom right) lists every control
as it is right now, read back from the game's bindings. **Click a key on it to change it**: the
next key or mouse button you press, with Shift, Ctrl or Alt as held, takes its place (Esc leaves
it, Backspace puts the Elwynn key back). A key taken from another action leaves that one unbound,
and the card says so. Changed keys are remembered for the next session without touching your
saved bindings; "Elwynn keys again" forgets them. The card can also switch to your own saved
bindings (Elwynn Survival's actions are under their own heading in Key Bindings), let the middle
button lock on, put the health bars by the hero or top left, size them (100, 125 or 150%),
turn the floor markers off or on (a creature's blow coming, ground about to burn, holy ground),
and set the **mouse speed** (− and +: from a tenth of the stock speed to one and a half times it,
over both the game's look speed and its sensitivity, whose lowest is still quick; kept with the
add-on's settings and put back each time you play). A new layout version starts everyone once on
the Elwynn keys (a line in the chat says so), and a key saved for an action that's gone is
dropped. Hints in the server's words ("(H)") show the key as it is now.
Every prompt on the screen (the ability keys, the gear cross, what G would do) shows the key in
effect.

| Input | What it does |
| --- | --- |
| W A S D | Move and strafe |
| The mouse | In action mode, it steers: the view turns the hero. A window up (the bags, a chest, the board, the card, the map) frees the cursor until it closes; Z frees it on purpose |
| Left mouse | Attack. Blows **chain**: press again during one's follow-through and the next of the weapon's set comes at once, each with a short **lunge** into it. Attacking while sprinting is a **running attack** that lunges much farther. Pressed during a dodge, the **attack out of the dodge** comes as the roll ends. Jump while a dodge still dashes (about 0.2 s in) for a **Corey roll**: the leap carries the dash's speed, and an attack pressed in the air strikes as you land, a long heavy **leap attack**. Corey rolls chain: land, roll, jump again. While an ability is aimed: **cast** it (held: charge it) |
| Shift + left mouse | **Heavy attack**: slower, 1.7 times the damage and 1.6 times the stamina. It doesn't stop a creature's wind-up (hitting first only trades blows); landed just after the creature's own blow, it staggers it (never a boss) |
| Right mouse (hold) | Raise your **guard**. Raise it just as a blow lands to **parry**: nothing gets through, no stamina, and the attacker reels. Raised while your own blow still winds up, it **feints**: the blow is called off and its stamina comes back (once it has landed, no). Late in a swing's follow-through it cuts the rest short. While an ability is aimed: call it off |
| Right + left mouse | **Shield bash** (with a two-hander, a kick); with Shift, the **kick**: breaks a guard harder |
| Mouse button 4 | Tap: **dodge** as you let go, the way you're moving (15 stamina, and it takes at least 18 to start one). Hold: sprint. It works with the mouse freed (Z) too |
| Shift (hold) | Sprint too: 12 against 7 (over the 11 the hero's own Sprint clip needs, so a strafe doesn't flicker between it and Run), only 3 stamina a second: a way across the world, not a fight's cost |
| Q | The **ranged** ability (Hurled Axe to begin with) |
| E | The **close** ability (Sundering Blow) |
| 1 | The **utility** ability (Charge) |
| 2 | The **defense** ability (Retaliation) |
| 3 (hold) | The **ultimate** (Onslaught): held a quarter second it goes (a line fills along the key), so a stray press never spends the whole Valor bar |
| Q, E, 1 or 2 held (0.35 s), or Ctrl + it | Its **charged-up EX version**, for Valor: an aimed one comes up as itself and turns EX as the hold passes (the key shows the EX and clicks); one that goes at once (a weapon art, a stance, a nova, holy ground) waits to see which you meant. Without the Valor for the EX version, the key is a plain press at once (nothing waits on a hold you can't pay for) |
| Space | Jump: 8 stamina, and not without it |
| F | Use what's in front of you (the prompt over the dock says what): a chest, a station, the weapon rack, timber, copper, the bell, your dropped supplies. While placing a station, put it down |
| R | Sheathe or draw your weapon |
| T | Use the quick item: a roast or stew (heals), a salve, a tonic, a sharpening stone (while placing, turn the station) |
| G | The next quick item |
| X | Walk or run (pressed in the moment a move roots you, it goes once the root is off) |
| Num Lock | Autorun (put it on a mouse button on the card if you like) |
| V | The health bars over heads (foes' and friends') on or off |
| B | The bags: the backpack and all four satchels |
| H | Build: the menu, then the station stands in front of you with the mouse free, a ring on the ground where it goes (green: it can; red: it can't, and the banner says why); only you see it until it's built, so it never stands in a friend's way. F puts it down, R or T turns it, H cancels |
| N | The **board** of abilities |
| J | The camp's journal |
| Tab | **Lock on** to the foe in front of you (the one you've been trading blows with first, then a hero to fight, then a creature fighting someone; a creature minding its business last); Tab again lets go (middle mouse too, if the card says so) |
| Ctrl + Tab | The next foe in view, each in turn (locks on if you weren't) |
| Z | Free the mouse to click things in the world (Z again to fight) |
| K | The Controls card |
| C, M | Character, world map |

With Shift held (sprinting), the movement keys, Space, the ability keys (Ctrl + Shift for the
EX versions), F, R, T, G and Tab do the same as without it; with Ctrl held (for an EX version),
the movement keys, Space, F, R, T, G and X do too (Ctrl+S is the game's own sound toggle
otherwise).

A locked-on body turns to its foe at about 290° a second (patch 0013), and a locked swing at a hero
turns toward them only within 50° (a creature: 90°), so rolling past a hero locked on to you gets
you behind for a moment.

**Poise and ripostes**: your heavy attack, from its start until it lands, isn't knocked aside by
a light blow. A body reeling from any stagger you caused (a bash, a kick, a wind-up knocked aside,
a heavy blow in its recovery, a knockdown) takes a riposte: half again damage.

**Attack windows**: a creature's wind-up goes through your blows, a heavy one too, so getting your
swing in first only trades blows with it. Dodge it or guard it (or parry it), then strike in the
window after it, while it recovers: a heavy blow there staggers an ordinary creature, and every
blow there counts half again towards breaking a champion's or a boss's stance. Only a weapon made
for it (the mace, the maul, the bearded axe, a few champions' weapons) knocks a wind-up aside,
and then not that creature's next one for 8 seconds, and never a boss's; a bash or a kick still
knocks a creature reeling, but a boss only out of a cast (`[combat]`: `heavy_knocks_aside`,
`knock_aside_wary`, `boss_knock_aside`, `recovery_stagger`, `recovery_posture`, `shove_bosses`). Stamina doesn't come back while you swing or
dodge; its pause starts as they end.

**Commitment and cancels**: a blow can't be cut short while it winds up or lands. Past the first
45% of its follow-through (65% for a heavy attack) a dodge or the guard cuts the rest short; an
attack pressed in the last three quarters of a light blow's follow-through chains at once
(`[combat]` in `content/survival.toml`, with the blows of each weapon in its `[[moveset]]`).

Swings have a **soft lock**: with nobody inside the weapon's arc, a blow turns toward a foe up to
35 degrees past it (`[lock]` in `content/survival.toml`), so a swing a little wide still finds who
it was meant for.

## Abilities

Five keys, each a row of the **board** (N), and each row nine cards, one from each of the nine
classes. Click a card to put it on its row's key; any card goes with any other, so a build can be
one class's fantasy or a mix (a Warrior's Charge with a Mage's Meteor and a Priest's shield).

| Key | Row | Warrior | Paladin | Hunter | Rogue | Priest | Shaman | Mage | Warlock | Druid |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Q | **Ranged** | Hurled Axe · *Rending Axe* | Avenger's Shield · *Holy Shock* | Aimed Shot · *Multi-Shot* | Fan of Knives · *Crippling Throw* | Penance · *Mind Blast* | Lava Burst · *Chain Lightning* | Fireball · *Pyroblast* | Seed of Corruption · *Chaos Bolt* | Living Seed · *Entangling Roots* |
| E | **Close** | Sundering Blow · *Unstoppable Blow* | Consecration · *Hallowed Ground* | Explosive Trap · *Freezing Trap* | Backstab · *Ambush* | Holy Nova · *Psychic Scream* | Thunderstorm · *Thundercrack* | Cone of Cold · *Frost Nova* | Shadowfury · *Shadowflame* | Swipe · *Typhoon* |
| 1 | **Utility** | Charge · *Heroic Leap* | Divine Steed · *Blessing of Freedom* | Disengage · *Net and Disengage* | Shadowstep · *Blinding Step* | Leap of Faith · *Mind Grip* | Earthbind Totem · *Earthgrab Totem* | Blink · *Frost Blink* | Demonic Circle · *Dread Circle* | Feral Leap · *Pounce* |
| 2 | **Defense** | Retaliation · *Shield Wall* | Blessing of Protection · *Lay on Hands* | Flare · *Feign Death* | Smoke Bomb · *Vanish* | Power Word: Barrier · *Power Word: Shield* | Healing Stream Totem · *Grounding Totem* | Ice Block · *Cold Snap* | Drain Life · *Death Coil* | Barkskin · *Ironbark* |
| 3 | **Ultimate** | Onslaught | Sanctuary | Kill Shot | Death Mark | Void Rift | Earthquake | Meteor | Summon Infernal | Hurricane |

In italics, each card's **charged-up EX version**: hold its key a quarter second (or Ctrl with
it) to cast that for Valor instead. An ultimate has none: it is the bar's whole worth.

**The classes**, in a line each:

- **Warrior**: bulldoze: close the gap, smash through, break guards. Hurled Axe flies out through
  every foe and comes back to wherever you stand, striking each again (walk so its way back crosses
  the fight; its EX, Rending Axe, drags what it strikes toward you). Charge bowls through everyone
  in its line, staggering each (or Heroic Leap, landing in a burst that throws everyone back);
  Sundering Blow; Retaliation against blades; Onslaught.
- **Paladin**: shield your friends, punish who touches them. Avenger's Shield ricochets to two
  more foes and silences them; its EX, Holy Shock, heals the friend under your reticle (or you) and
  frees them. Consecration and Sanctuary lay healing ground. Divine Steed is a burst of speed out of
  roots, and its key again within 3 s throws Hammer of Justice, a stun: ride in, then punish
  (its EX, Blessing of Freedom, frees and speeds a friend). Blessing of Protection halves what a
  friend takes for 3 s, the one defense you give away; Lay on Hands is 40% of their health at once.
- **Hunter**: set up, then snipe. Aimed Shot is a draw: the longer you hold it, the harder,
  farther and faster the arrow, and fully drawn it goes through the whole line (a heavy blow on you
  spoils it); Multi-Shot is five arrows in a fan. Traps arm and wait (Explosive: a burst that throws
  back and burns; Freezing: a freeze, and setting it makes your next Aimed Shot a free Multi-Shot
  that shatters the frozen). Disengage leaps you straight back (its EX nets the foe in front);
  Flare is lobbed onto a pack and blinds them till they're struck, Feign Death drops you as if dead
  and every creature and lock lets go; Kill Shot is a 1 s draw that goes through a 40 yd line and
  finishes anything under a quarter of its health.
- **Rogue**: in and out before they turn round. Fan of Knives, three poisoned knives that all land
  up close (the EX throws one blade through two); Shadowstep in, and its key again within 3 s steps
  you straight back (its EX blinds everyone where you come out); Backstab, three times from behind;
  Smoke Bomb, a cloud at your feet that hides you and your friends in it and slows foes, Vanish
  hides you at once (your next blow counts as from behind); Death Mark.
- **Priest**: steer the light. Penance, a lance of light held 1.5 s and swept as you turn, burns
  the foes in it and heals the friends in it (its EX, Mind Blast, bursts at the reticle 0.6 s later
  and terrifies); Holy Nova heals you and your friends as it burns foes; Leap of Faith pulls the
  first body it touches to you, friend or foe; Power Word: Barrier, a dome where you and your
  friends take a quarter less and can't be staggered (its EX shields a friend, or you); Void Rift
  drags everyone in it to its middle.
- **Shaman**: shape the field. Lava Burst, molten rock lobbed over everyone in the way to land
  where its ring shows and leave the ground burning (its EX, Chain Lightning, leaps on twice);
  Thunderstorm throws everyone round you 8 yd away; totems to slow or root, to heal, or to
  swallow a projectile (Grounding); Earthquake stuns and slows a field.
- **Mage**: freeze, then shatter. Fireball grows with its charge, and its EX, Pyroblast, is a
  1.5 s hold for a huge one; Cone of Cold slows (and freezes the already slowed), Frost Nova
  freezes all round you and makes your next Fireball a free Pyroblast for 5 s; Blink; Ice Block;
  Meteor falls 1.5 s after you call it on a 7 yd circle: 70 damage.
- **Warlock**: rot. Seed of Corruption sticks in a foe and bursts round them 2 s later, leaving
  everyone near rotting (its EX, Chaos Bolt, is a 1 s hold that tears through every foe in a
  line); Shadowfury stuns a spot a moment later, its EX breathes shadowflame; Demonic Circle draws
  a way out at your feet and the key again steps back to it (its EX screams where you left); Drain
  Life outlasts them, every point it drains healing you, and Death Coil sends a foe fleeing as it
  heals you; Summon Infernal crashes down and stands burning.
- **Druid**: grow and pounce. Living Seed's two seeds strike a foe or settle on a friend and mend
  them for 4 s (its EX roots); Swipe, and Typhoon throws a cone of foes back; Feral Leap bounds
  onto them (Pounce stuns); Barkskin, and Ironbark gives it away, to the friend under your reticle
  with a heal besides (no one there: you); Hurricane hurls everyone in it out from its middle
  every second.

**How they move bodies**: a **knockback** throws someone away from where it came from (a burst's
middle, a projectile's flight, the caster), a **pull** draws them toward it (Leap of Faith, Mind
Grip and Void Rift); both cut short what they had in hand, and fly heroes on their own client.
Nothing moves a body in Ice Block, Onslaught, Sanctuary or Barkskin. A **leap** goes through the
air to the reticle's point (or straight back, a Disengage) and strikes what's there as it lands;
Demonic Circle sets an **anchor** to step back to; **traps** arm, wait (two at once) and spring on
the first foe to step near; a **wind-up** (Shadowflame, Typhoon) holds you a moment and is lost to
a stun. Projectiles may **pierce** (Crippling Throw, Chaos Bolt), **leap on** (Chain Lightning),
**burst**, or **stick** and go off a moment later round whoever carries them (Seed of
Corruption); ground may linger, pull, throw out or slow every second. A dash may run **through**
every foe in its way, striking each as it passes (Charge). Help aimed at a **friend**
lands on the friend under the reticle (or the one nearest it, within 15°), else on you, and gives
them only its heal, cleanse and boons; a **beam** is held a few seconds as a strip from you along
your aim, sweeping as you turn: every foe in it takes its damage each tick and every friend in it
is mended (a stun, silence, stagger or roll ends it, and so does its key again).

**Combos** (engine support; the cards that use them come with the class redesign): a card with a
**recast** arms its key for a few seconds after it goes, and the key again fires its second
part for free (a blink there and back, a throw after a slam); the key glows green with the seconds
left. A card that **empowers** another makes that card's next press its EX version for no Valor
(Frost Nova on E making Fireball on Q its EX); the empowered key glows, its seconds in the corner.

**Beta mechanics** (engine support for the beta page's generic cards): a projectile may **yank**
its caster after it (a grappling hook: to the wall or the ground it meets, or beside the foe it
strikes, in a fast dash a wall stops short; a miss does nothing) or **swap** places with what it
strikes (a foe, a creature or a friend; never a champion, a totem, or anything steadfast or
untouchable, which is only struck). A knockback may **stun against a wall** (`wall_stun`): a body
thrown into a wall or up against a steep step is stunned as it hits, as long as any stun (half on
a hero, kept off by the grace). A buff may leave an **echo** (`rewind`): its recast, or a few
seconds, brings you back to it with the health you had then if that's more than now. A trap may
**launch** instead of springing (a jump pad: you or a friend who steps on it is thrown that far
the way you face; foes don't set it off, and it lasts its life, once a second a hero). The boon
**cheat death** turns a blow that would fell you into one that leaves you at 1 health, ends, and
mends a share of your health over 3 s. A stance with a **mirror** turns a foe's projectile (a
hero's, or a creature's arrow or bolt) back at whoever threw it, as hard as it was; blows and
ground go through. A projectile may **tether** the foe it strikes for a while, a link drawn
between you: once it strays past the tether's reach it's dragged back to you and stunned, and the
tether ends. A shield may **pop** as it breaks or runs out (a burst round whoever held it), and
lingering ground may **burst as it ends** (a black hole's collapse), each the burst of a small
nova of its own.

**Seeing it coming** (the action-combat client, eighth round): everything that lands later is
drawn on the ground where it will land, red when it's a foe's, blue when it's yours or a
friend's, filling as it comes: a sky-fall's ring (Mind Blast, Meteor, Earthquake, Void
Rift, Hurricane, Shadowfury, Summon Infernal), a leap's landing, a wind-up's cone. While you aim,
your own ability's shape lies under you along the view: a projectile's path, a cone, a ring round
you, or the circle where it will land. Traps and anchors show as green circles.

**Combos**, inside a class and across them:

- **Freeze, then shatter**: Frost Nova, Freezing Trap, Cold Snap, Frost Blink or a Cone of Cold on
  a slowed foe freeze; fire (Fireball, Pyroblast, Meteor), a Sundering Blow or a Multi-Shot on them
  shatters for half again.
- **Pull into the pain**: Void Rift, Leap of Faith or Mind Grip drag foes onto a trap, into
  Consecration, an Earthquake or under a Meteor.
- **Push into the trap**: Thunderstorm, Typhoon or Heroic Leap throw a foe onto
  an Explosive or Freezing Trap; Hurricane keeps them off a point.
- **Hold them for the slow ones**: a root (Entangling Roots, the Net, Earthgrab, Mind Grip), a
  stun (Divine Steed's Hammer of Justice, Shadowfury, Pounce, Earthquake) or a freeze makes Mind
  Blast and Meteor land.
- **From behind**: Vanish, Smoke Bomb, Blinding Step and Shadowstep set up Backstab, and Fan of
  Knives Marks them so it crits; a kill under Death Mark readies everything again.
- **Stay alive**: Power Word: Barrier, Barkskin and Healing Stream stack with Holy Nova and
  Consecration; Grounding Totem eats the big projectile (a Pyroblast, a hammer, a Death Mark).

**PvP**: every card keeps to a budget a test holds it to (`tests_classes.rs`): a ranged poke does
at most about 45 a hit and 9.5 a second over its cooldown, a utility little damage on a cooldown
of 10 s or more, an ultimate the whole Valor bar and 40 s; no card holds a hero in hard crowd
control (a stun, a freeze, a fear, an incapacitate) more than 1.5 s, nor roots them longer,
nor throws them more than 10 yd; an EX costs 25 or 50 Valor and is better, not a different
league. Every hard crowd control is followed by the 3 s grace, and heroes get half the
duration creatures do.

- **Weapon arts** (Sundering Blow, Backstab) are blows with the weapon in hand, through the same
  rules as any swing: their own wind-up and reach, a share of the weapon's damage, stamina, and a
  guard and a parry answer them (the Unstoppable Blow aside: no guard stops it, no parry turns it,
  and its target sees the weapon go red).
- **Defense** is a read and a bait in a duel: Retaliation blocks melee (not bolts), Flare
  blinds whoever is coming till they're struck, Smoke Bomb hides you (a creature loses you, a lock
  lets go), Drain Life out-heals them, Ice Block lets nothing in but you can't act.
- **Full charge**: a charged spell held to the top flares on you (everyone sees it: the class's
  flash and sound), the reticle bursts and the screen's edge flares in its colour, and Aimed
  Shot, Fireball and Frostbolt fly much faster.

Hover a key with the mouse free (or a card on the board) for what it does in words and numbers:
how it's used, its damage, reach, charge, wind-up, stamina and cooldown, and the same for its
charged-up version with its Valor. This build has every card from the start (`[dev] full_tree =
true`), the Warrior's cards on the keys (`[loadout]`); without it a hero has the Warrior's and
unlocks the rest. Abilities off the board (the bow's Aimed Shot, Frostbolt, Flamestrike,
Nature's Grasp, Fire Ward) keep their rules (`bench = true`) for putting back on
it.

**Aiming**: an aimed ability comes up on its key (the camera over the shoulder, the reticle in the
middle, its name under it); left mouse casts it along the reticle's line, held to charge it (a
bar fills, gold once it's past the least charge); right mouse or the key again calls it off.
Projectiles fly along that line in three dimensions and strike the first body, ground or wall
they meet, uphill, downhill and in caves alike. A ground ability lands where the line meets the
ground (on the action-combat client, its circle lies there as you aim). Instant ones (a weapon
art, a stance, a nova, ground around you) go at once.

**Valor** (the gold bar over the abilities, in segments of 25; you start with 25): earned by blows
landed (light 2, heavy 6, an ability 6), a parry (10), a dodge through a blow (8), a counter that
catches something (15) and a kill (3). A charged-up EX version costs 25 or 50 of it (a gold pip
on its key, lit when you have enough) and shares its base's cooldown; an ultimate takes it all.

**Crowd control**: stuns, freezes, roots, slows, panic, silence, weakness and burning, shown over
the middle of the screen as they hold you. On heroes every one lasts half as long, and after a
stun, incapacitate or panic you're unshakable for 3 seconds (`[cc]`).

The numbers live in `[[ability]]` in `content/survival.toml`. For testing, with `.es debug` on (or
in this build): `.es tree` (the board; `full`, `reset`), `.es slot <q|e|1|2|3> <ability>` (a bench
ability too, on its row's key), `.es valor <n|full>`,
`.es cd off|on`, `.es heal`, `.es dummy [guard|roll|counter <school>]` (a training dummy in front
of you), `.es cc log` and `.es pvp on|off`. `.es debug` also draws projectiles' bodies and every
ground area on the ground.

### The beta page

The board has two tabs: **Classes** (the nine columns above) and **Beta**, five rows (the same
keys) of up to four generic cards any class can use, there to be tried out. Click one: it's yours
for free and goes on its row's key in place of your class's card there (`.es tree take <card>`
works for them anywhere, no debug needed). Their EX versions work as any card's; a dev build owns
them all; a save keeps them; Siege leaves them out (its fresh kit, its prices, its trainer). They
keep to their row's damage and crowd-control budgets but count toward no class. In content they're
`school = "beta"`, at the end of the `[[ability]]` list; the board's `TR` line gives each card its
page last (1 or 2), so an add-on from before them simply doesn't show them.

| Key | Card · *EX* | What it does |
| --- | --- | --- |
| Q | Ricochet · *Pinball* | A spike that bounces to three more foes within 8 yd, each bounce ×1.25 (7, 9, 11, 14). EX: five bounces, each stunning 0.75 s. |
| Q | Gravity Lance · *Collapse* | A heavy bolt: every foe within 5 yd of the one it strikes is dragged into it, all take 14. EX: 6 yd, 16, and they're slowed 70% for 2 s. |
| E | Whirlwind · *Bladestorm* | Spin 2 s, still moving at about 70%: 5 every half second to everything within 4 yd in front and to your sides. EX: 3 s, and nothing staggers you. |
| E | Launcher · *Juggle* | An uppercut: 10, stunned 0.8 s and thrown up; E again within 1.5 s leaps you onto them for 14 in 3 yd (Slam). EX: 14, 1.2 s, a Slam of 18 that throws them 4 yd. |
| E | Siphon Blade · *Soul Rend* | A 4 yd slash, 12 to each foe, and you heal 60% of all it deals. EX: wider, 16, and you heal all of it. |
| 1 | Blink Dagger · *Phase Strike* | Blink 12 yd, and again within 3 s. EX: each blink lands with 8 to everyone within 3 yd. |
| 1 | Phase Dash · *Searing Dash* | Dash 10 yd through foes, untouchable: each takes 4 and burns 4 a second for 3 s. EX: 14 yd, and they're slowed 40% too. |
| 2 | Riposte · *Counterstrike* | A 0.8 s parry: the first blow or bolt is turned aside, and an attacker within 6 yd is staggered and bleeds 7 a second for 4 s. EX: 1.2 s, and it stuns them 1.2 s. |
| 2 | Bubble · *Hard Bubble* | A shield on the friend under the reticle, or on you: soaks 40 for 4 s. EX: 70. |
| 3 | Black Hole | Lobbed 26 yd: for 3 s it drags every foe within 7 yd 3 yd to its middle each second, 6 a second. |
| 3 | Earthsplitter | A 30 yd fissure along your aim, drawn for everyone, bursts 0.8 s later: 30 and stunned 1.5 s. |
| 3 | Avatar | 8 s of Onslaught, 20% faster and 20% less damage taken. |

Where the engine can't do what the design asked yet, a card does the closest thing and says so in
its comment: Ricochet's EX stuns on every bounce, not just the last; Collapse slows instead of
rooting (a root or stun after a pull stops a creature where it stands); Whirlwind is a beam, so it
reaches round your front and sides but not right behind you; Siphon Blade steals no boons; Phase
Dash burns the foes it passes instead of leaving a trail; Riposte's cut is a bleed and there's no
step behind the attacker; Avatar's extra damage is Onslaught's unblockable heavies (no boon adds
damage). Ultimates have no EX version (no key for one, and the bar is spent). Bubble's pop and
Black Hole's collapse, and the cards that need new engine pieces (Grappling Hook, Displacer Orb,
Wall Slam, Rewind, Jump Pad, Cheat Death, Reflect, Soul Chain), come with them.

### The action-combat client

Some of this needs a change to the client itself, which this folder carries as patches
(`patches/benilla/`) on top of the pinned benilla: `scripts/setup-mac.sh` and
`scripts/play.sh` apply it and rebuild the client once whenever it changes (a few minutes), and
`scripts/play-windows.ps1` does the same on Windows. On it:

- the mouse steers in action mode (the game's own mouse-look, held), let go while a window is up;
- the left button is the add-on's alone, so it never drags the camera;
- an aimed ability brings the camera in over the right shoulder (`SetAimCamera`), and casts go
  along that camera's line (`GetActionAim`), so the reticle is where they go;
- a ground ability's circle lies on the ground where it will land (`SetAimMarker`), and a blow
  coming down lays a red circle where it will fall (`SetGroundMarker`), following the ground;
- a creature's slow heavy blow holds its coil and snaps onto its impact;
- a creature's swing trails its last stroke into the blow (amber: a guard stops it, red: it
  doesn't), not its whole wind-up; a beast with nothing in its hands trails the part of it that
  strikes, a paw or its head: the ninth round;
- a creature's blow is drawn on the ground under it as it winds up (`SetTelegraph`): its own
  shape (a wedge of its arc, or a ring all round) out to the reach it lands at, turning and
  moving with it, and a fill growing from the creature to the rim, meeting it as the blow lands.
  Red: no guard stops it, dodge. Orange: it can be blocked, and the whole shape turns gold for
  your parry's window (raise the guard then). No stroke round it: soft fills whose edges fade
  out. A blow meant for someone else is fainter, and has no growing fill;
- a blow meant for you tells you when to answer it, with the floor markers on or off: the
  striker glows over its wind-up (amber: a guard stops it, red: it doesn't), and a reaction's
  time before your window opens the glow flashes bright: raise the guard now to parry (with a
  ping), or roll now for a red one. The stroke into the blow wooshes, heavier for a slower
  wind-up;
- a raised guard's ShieldBlock goes up once and holds there (on the upper body while moving),
  where it used to loop;
- heroes' abilities are drawn the same way (`SetTelegraph` strips and a friend's blue,
  `SetTelegraphAt` for a shape laid at a spot), and an aimed ability's shape lies under the hero
  along the view (`SetAimMarker` `line`, `cone`, `self`): the eighth round.
- a clip can come from another character model (`PlayUnitAnimation`'s `from`): a move's or a
  creature's `from` in the content (`"ScourgeFemale"`) plays that model's take on the unit's own
  body, retargeted by its key bones: the tenth round. `/esclip <clip> [take] [Model]` tries one on
  your hero, and `scripts/race-clips.sh` lists every model's attack clips from your game data.
  `scripts/clip-census.sh` measures what each does (a flip, a leap, a spin, a quick thrust…), and
  `tools/clip-census/assign.py logs/clip-census.md` picks every humanoid foe's borrowed clips from
  that: quick takes for a flurry, overheads and chops for a heavy, leaps for a lunge, flips and
  spins for a boss's signature blows alone (a regular foe spins only on a blow all round), each
  take timed near its step, and a boss never repeating one.

On a stock benilla the add-on falls back to a click-catcher over the world: the same keys, right
mouse guards and looks around, and aiming keeps the normal camera. The Controls card says which
client you're on.

### The screen

- **The dock, bottom centre**: one dark plate with health (red), stamina (green) and Valor (gold,
  in its segments) over the five ability keys, every bar exactly as wide as the five keys. A pale
  chunk shows what a blow just took before it drains, and healing rises off the health in green;
  the stamina bar blinks red when you're too winded for what you tried. Each key shows its icon,
  its column's colour along its foot, its cooldown draining, its charges, a gold pip while there's
  Valor for its EX version (Ctrl held, or the key held a moment, shows the EX itself), a line
  filling while you hold it, and its key on a cap under it; the ultimate glows when the bar is
  full. Small to the health's left, a shield lit while your guard is up and crossed swords lit
  when a swing would reach someone; over it, what your meals and salves are doing. Just above the
  dock, a bar while eating, gathering, casting, channelling or charging, and what G would do. The
  card sizes the dock (100, 125 or 150%), or puts health and stamina top left instead (there they
  grow with what they hold).
- **Top centre**: a champion's bar when one fights near (its name over it), and under it the bar
  of the foe you're trading blows with (a creature or a hero), for 8 seconds after the last blow;
  never the champion twice. With the health bars top left, these sit a little lower.
- **Left, above the chat**: the equipment cross: your weapon (right, left mouse), shield or
  two-handed guard (left, right mouse) and the quick item (down, T, with how many), each with its
  key, and the bow (up) when it's on a key. It keeps above the chat wherever the chat sits.
- **Middle**: while aiming, the reticle (grey while the ability can't go); a hit marker when a
  blow of yours lands; the crowd control on you. The screen's edge flashes red when you're hurt,
  and pulses below 30% health.
- **Top right**: the raid, resting at the camp, the way to your dropped supplies, and the kill
  feed (who felled whom).
- **Bottom right**: icon buttons for the bag (with how many of its 80 slots are used), character,
  map, controls, game menu and help; hover one for its name and key.
- **Right, above those**: the damage meter, from your first fight on. **Fight** shows you and the
  heroes fighting near you in this fight (or the last one: a lull of five seconds ends one), the
  most damage first, with how long it has run; **You** shows your own damage by card since you
  last reset it, your per-second over your time in fights and what you took. **Healing** turns
  both to healing (a friend you mend carries you into their fight). Only what landed counts: not
  overkill, not a heal past full. Drag it to move it, click it to fold it, `/esmeter` hides it,
  **Reset** (or `.es meter reset`) starts your own count over.
- **Falling**: "You have fallen" across a dark band, and when you'll be up again.

Health bars float over creatures' and other heroes' heads. Messages show briefly on screen
rather than in the chat, which stays for talking. The action bar (with its spellbook, talents,
quest log and social buttons), the minimap, the player and target portraits and the cast bar are
hidden while you play Elwynn Survival, and P, L, O and 5 to 0 do nothing; all of it comes back on
leaving.

### Animations and the weight of a blow

On the action-combat client (the patch's second part) the client plays clips itself, so:

- **Your own swings and dodges start the moment you press**, not a round trip later; the server's
  word follows and isn't played twice (a swing it turns down, for want of stamina, still showed).
- **The clips own the body**: an action clip fades in over the walk or stand under it (80 ms) and
  out again (160 ms), plays on its own clock so the clip's impact keyframe lands on the server's
  blow, and moves to the upper body alone when you're moving, so a swing on the run doesn't freeze
  the legs. The dodge's tumble starts only once the dash is under way, and puts back exactly what
  it turned: the root's height, once left behind by the roll, is what left heroes standing in the
  air.
- **Lunges and the dodge run on your own client**: the step into a blow and the dodge's dash
  are your own movement, sped up for a moment, with nothing waiting on the server; others see the
  pace through the server.
- **Swings use the game's real attack clips**, timed so the clip's own impact keyframe lands when
  the server's blow does: Attack1H (17), a dagger's Attack1HPierce (85), Attack2H (18), a
  polearm's Attack2HL (19); the heavy attack's Special1H (57) or Special2H (58). Each leaves a
  **weapon trail**: a faint pale streak, a heavy blow's ember.
- **Hit-stop**: a blow that lands holds both bodies still for a moment (60 ms, 115 heavy, 150 on a
  parry, 50 on a block), and **the camera shakes** for whoever is in it, harder for the one struck.
- **The dodge tumbles**: the whole body rolls head over heels, about the hips, the way it dashes.
  The dash itself is your own run, sped up for a quarter second (run and run-back speed), so it
  follows the ground like any step. It used to be a knockback, and a throw that fast is a flight:
  one into a tree's roots could come to rest up in them, leaving the hero standing in the air.
- **Struck bodies react**: a light blow flinches (CombatWound), a heavy one the bigger flinch
  (CombatCritical), a stagger stuns, each leaning the body back away from where it came from. A
  block shows the shield block; a parried attacker reels.
- **Impact visuals and sound**: a heavy blow, a bash, a parry and a riposte flash the game's own
  spell impacts (Heroic Strike's, Shield Bash's, Disarm's, Backstab's) on the struck; every swing
  whooshes (small, medium or large by the weapon) as it comes; a heavy blow punches the camera in.
- **The bow follows your aim**: while aiming, the upper body leans with the camera's pitch.
- **The camera never ends up inside another body**: it stops short of other units the way it
  stops short of walls. A body lying dead is no wall to it, and one standing right behind you is
  passed through rather than pulling the view into your own head.
- **The lock-on camera** holds the locked foe in view past your right shoulder, a little farther
  back, while the hero keeps turning to face them. Aiming (the bow, an aimed ability) while
  locked on hands the view back to the mouse: the reticle goes where you put it, the body turns
  with it, and the shot flies along the reticle's line; let the aim go and the lock takes the
  view again.
- **Shots go where the reticle is**: an aimed shot carries the reticle's line from your client
  (where the view's centre looks, round and up or down), so it flies along it even if the server
  last heard you facing a little differently, and passes over or under whoever the line misses. A
  shot rises or drops to whoever it's lined up on (up a slope, onto a deck above), so long as
  nothing solid stands between on that slant (a floor between doesn't let one through to someone
  below). A miss flies on at the reticle's height instead of diving into the ground.
- **No long thin lines**: a bow's string isn't drawn while its tips are farther apart than a bow
  can be (a stale pose stretched it across the screen).

**Fairer hits over the internet**: the server times each hero's round trip (a ping every two
seconds). Your swing is judged against where the others were when you saw them (up to a quarter
second back), and a blow on you waits for your round trip (up to a fifth of a second) before it
lands, so a dodge or guard you'd already pressed when you saw it coming counts. Between a
client's reports (a heartbeat comes twice a second) the server carries a running hero on along
the keys they hold, so it judges blows against where they are, not where they were half a second
ago. `.es net` lists everyone's round trip and the allowances.

**Stances never stick** (the fifth part, `patches/benilla/0005-*`): the hit-stop holds each clip
at speed 0 instead of pausing it. A paused clip that the game's own animation picked away from
mid-stop (the gait changing as a swing's slow walk ended, a block or a reel) was never faded out,
and it played on at full weight over everything after it: a frozen swing, block or reel blended
over the walk and run for the rest of the fight. The lock-on's two shuffle steps, and clips
replaced twice in one frame, stop the one they replace too. A flinch on a body still winding up
its own blow leaves that swing playing (the blow still lands, so it must show) and carries the
hit in the lean and the hit-stop instead; once the blow has landed, the flinch plays. Every blow's
number, blood and impact sound come as it lands (the game held them for an impact key the action
layer's clips never pass, and showed only the number at the next blow).

### The fourth round: weight the moment a blow connects

On the client with the patch's fourth part (`patches/benilla/0004-*`):

- **Your own blow lands at once**: as your clip reaches its impact keyframe, the client checks
  what the blade is crossing (your blow's own reach and arc) and plays the hit-stop, the shake and,
  for a heavy blow, a bash, a kick or a chain's last blow, its flash, without waiting a round trip.
  The server's word follows and isn't played twice (its flash isn't sent back to you).
- **The lock-on is your target**: its selection circle marks who you're locked on to.
- **Turning in place while locked on steps the feet round** (ShuffleLeft and ShuffleRight) instead
  of spinning on the spot.
- **A dodge to the side or back is a quick step**, the Dodge clip over legs running the dash;
  forward is the tumble. A little dust as it ends.
- **Knockdowns**: a guard broken by a heavy blow or a kick, or a heavy creature's blow landing,
  puts a hero on the ground (Knockdown, its whole fall and rise over 1.5 seconds), with a cry and
  the dust.
- **Parries** play the parrying sweep (Parry1H) on the one who parried.
- **Voices**: your shout on a heavy blow, a bash and a kick; a cry from whoever is knocked down or
  staggered. A heavy creature's blow lands as a crushing one: the big blood spurt and the cry.
- **More flashes**: a heavy blow shows Heroic Strike's or Slam's impact, the chain's last blow
  and a running attack Cleave's, a body hitting the ground a dust nova.
- **Each weight of blow shakes and holds differently**: a bash thuds, a kick shoves, a running
  attack carries its weight, a heavy blow punches the camera in.
- A blow stepping in keeps its own legs (they slide the little way rather than flick to a run and
  back); the run speeds up with a dash already, as the game scales the run by speed.

Others' swings and rolls come from the server as clips too (`AN`, `RO`), and every blow as an
impact (`IM`); a stock client gets the emotes instead. Casts (a throw, a spell) play their clips
on everyone, the caster too (`AN` with the kind `cast`), and gathering and eating show on your own
body as others see them. The bow (Aimed Shot) plays its own clips everywhere:
drawing and holding while it charges (everyone sees it), the release as it looses. The draw takes
a second, as the hero's own LoadBow does (no shot before the arrow is on the string, 0.45 s), and
the release plays out (0.7 s) before the next arrow comes up. Walking with the bow drawn, the legs
walk: the drawn pose is withdrawn while you move (others see you walk) and comes back when you
stop; on the action-combat client your own upper body holds the draw (HoldBow) meanwhile. Arrow hits land
as shots, so the archer keeps the bow out and the struck flinches. A heavy attack shows as a
critical with the bigger flinch and number, and throws a struck hero back. On a stock client the
swings are emotes (54 for a one-handed heavy attack, 37 and 38 two-handed) and the dodge is a dash
with Sprint's whoosh.

`.es fx emote <id>`, `.es fx spell <id>`, `.es fx cast <id> [ms]` and `.es fx pose <id>` play a
look on yourself, for trying others. The calls the add-on makes (`PlayUnitAnimation`, `HitStop`,
`ShakeCamera`, `RollUnit`, `SetCameraUnitCollision`; the third part's `StopUnitAnimation`,
`ReactUnit`, `ActionDash`, `SetLockOn`, `CameraKick`; the fourth's `ActionProbe`, `ActionKit`,
`PlayUnitVocal`, `TakeActionMarkers`, `ActionMoveKeys`) are in `patches/benilla/`, documented in
its `action_combat.rs`.

### Tuning in play

`.es tune <path> <value>` changes a value of `content/survival.toml` while the game runs, for
everyone at once (blows, chains and windows reach the clients' own play straight away); a path
names tables and keys with dots, and a list's entry by its number or its `key`:

- `.es tune combat.chain_after 0.3`, `.es tune combat.max_grace 0.15`, `.es tune dodge.secs 0.5`
- `.es tune moveset.one_hand.light.2.lunge 1.4`, `.es tune bash.stamina 12`
- `.es tune mob.brute.special.chance 0.5`, `.es tune weapon.axe.damage [9, 14]`

`.es tune` alone lists what's been changed, `.es tune <path>` shows a value, `.es tune keep` writes
the changes into `survival.toml` (its comments and layout stay as they are) and `.es tune reload`
reads the file again. Values that change the world's make-up (new items, stations) need a
restart.

## Friendly fire (PvP testing)

Friendly fire is on (`[pvp]` in `content/survival.toml`): swings and arrows strike other heroes
by the same rules as creatures: blocked by a guard facing them, **parried** by a guard raised
just in time (the attacker reels for 0.9 seconds, open to a riposte that does half again), dodged
in the dodge's window. A heavy weapon (two-handed, mace, bearded axe) knocks a hero's wind-up
aside. A felled hero gets up a second later where they fell, at full health, with everything
they carried (no sack), and the camp hears who felled whom. Set `friendly_fire = false` and remove
`respawn_secs`, `respawn_health` and `keep_supplies` to go back to co-op rules.

Chat commands, if you'd rather type: `.bank`, `.take <item> <n>`, `.equip <item>`, `.arm <weapon>`
(at the rack), `.build [station]`, `.place`, `.rotate`, `.cancel`, `.craft <recipe>`, `.status`,
`.where`, `.debug` (marks your swing's reach on the ground; `.raid reset` and `.es spawn <creature>`
work while it's on: `.es spawn warlord` calls Warlord Gorzak up in front of you to practise on, `.es spawn grunt 3` three grunts).

## Weapons to try

The **Weapon Rack** beside the camp chest hands out every weapon for free while testing
(`armory = true` in `content/survival.toml`):

| Weapon | Hands | Feel |
| --- | --- | --- |
| Footman's Axe | one, with a shield | Balanced; strikes up to two |
| Bearded Axe | one | Slower and heavier; a blow mid wind-up knocks the attack aside (not a boss's, and not twice in 8 s) |
| Footman's Sword | one | Fast and cheap; one foe at a time |
| Militia Mace | one | Knocks a winding-up foe's attack aside (not a boss's, and not twice in 8 s) |
| Poacher's Knife | one | Very quick, short reach, light damage |
| Footman's Claymore | two | Slow, wide sweeps that strike up to four; guards with the blade |
| Goldshire Halberd | two | Long, narrow reach; guards with the haft |
| Quarry Maul | two | Very slow, very heavy; breaks wind-ups; guards with the haft |
| Militia Buckler / Reinforced Shield | shield | The reinforced one covers more and blocks for half the stamina |

Two-handed weapons have no shield: their guard is narrower and dearer, but parries work the same.

Bows and crossbows aren't on the rack: ranged fighting is off the board (Aimed Shot is benched), though the Hunting Bow and Militia Crossbow are still defined for the camp's towers and older saves.

## Enemies near the camp

Every foe has `combat.foe_health` (1.6) times the health its own entry gives (critters, dummies
and totems as they are): fights last about 60% longer than the bare numbers, since the classes'
cards hit hard. Their blows (and burns) do `combat.foe_damage` (1.35) times the damage written.

**More heroes, harder fights.** For each hero in a foe's fight past the first (anyone standing
within 25 yards of it, or on it from farther off), it has 70% more health (`hero_health`) and its
blows do 10% more (`hero_damage`). It grows as heroes join, keeping the damage it has taken,
and stays grown until it's home again. Below, every hero in the mines counts the same way
(`hero_health` in `deadmines.toml`); the arena sends more foes instead.

The Horde has taken the Fargodeep mine yard and camps by the road to the west. Its fighters are
orcs, trolls, tauren and Forsaken with real weapons (their own race's models and clips, the
weapons and shields in their hands), and they fight like you do: each attack is a **pattern**, a
string of blows picked by how far off you are, every blow with its own wind-up (its clip slowed
to it on the action-combat client, a mark on the ground where it lands) and a recovery to punish.

| Enemy | Where | Holds | Its patterns |
| --- | --- | --- | --- |
| Warsong Grunt | the mine road and yard | axe and shield (blocks light blows between attacks) | two quick cuts; a shield bash; a slow glowing overhead that waits out an early dodge; a charge from a few strides off |
| Warsong Overseer | the mine yard | two-handed axe (poised) | a wide cleave; a double chop, the second quicker; an overhead slam through the guard; a leaping chop; a kick to open the guard, then the cleave |
| Warsong Shaman | the mine yard | staff | lightning where you stood when the cast ended; two bolts, the second quick on the first; crowded, a thunderclap all round that throws you down, then it backs off |
| Darkspear Berserker | the camp to the west | axe, no shield | a flurry of three (parry the first and the rest never come); a leap; a spin that strikes all round |
| Darkspear Witch Doctor | the camp to the west | staff | frost where you stood; a volley of three, quicker each time; crowded, a burst of frost all round (through the guard), then it backs off |
| Bloodhoof Brave | the road south of the meadow | great spiked mace (poised) | a crushing overhead; a sweep across everything in front; a kick; a stampede from far off |
| Forsaken Deathguard | the meadow south of the camp | sword and shield (blocks half the light blows between attacks) | a long thrust; two cuts and a thrust; a shield bash; a delayed overhead |
| Darkspear Spearman | the camp to the west | a spear: long and narrow (step aside, not back) | a long thrust; two thrusts, the second farther; close in, a sweep of the haft; a leaping thrust |
| Forsaken Cutthroat | the meadow with the Deathguard | two daggers, and quick to step out of a heavy blow | a stab; a flurry of four (parry the first); a lunge; a feint held back, glowing |
| Warsong Archer | the mine yard | a bow | an arrow to where you stood; two, the second quick on the first; crowded, a kick to make room, then it backs off |
| Captain Grashak | with the Warsong Raid (the first), last | axe and the Horde's shield (blocks over half the light blows between attacks) | three cuts; a bash and a cut; a glowing overhead held back; a shield charge from far off that throws you down; at two fifths of his health he rallies, quicker, and charges at once |
| Shadow Hunter Zanjin | with the Darkspear Raid (the second), last | a spear, and hexes | two thrusts; a sweep all round when you crowd him; a hex of shadow from afar; a leaping thrust; at half his health the loa take him: quicker, and three hexes in a row |
| Warlord Gorzak | with the Horde Warband (the third raid), last | great two-handed axe (poised) | two phases: see below |

**Chases and leashes.** A creature's leash is how far from its home its foe may be: a hero
shooting from a little past it still draws the fight to them (it gives up once they are six yards
past it, or it is twelve past it itself). On its way home it **evades** anyone still out past
its leash (their blows and shots find nothing, and say so), and is whole again once home; come
back within its leash and it's a fair fight again. Creatures **spread round their foe** instead
of standing in one another, and one that can find no way to its foe (up on a ledge it can't
reach) stands rather than walking through the wall, and gives up after a few seconds.

**How they fight a band of you** (`[ai]`, after Bloodborne's beasts and hunters). A creature
doesn't just run at the first hero it saw:

- **Whom it goes for.** Every second or so between strings it looks round: at whoever has hurt it
  lately (the memory fades), at a hero with their guard down (eating, working, gathering, in a
  swing's follow-through, drawing back or casting a spell, reeling), at the most hurt, at the
  nearest, and away from a hero already beset (a pack spreads over a band). It keeps its foe
  unless another is clearly better, so it doesn't turn every second. A raider on its way to a
  station turns on a hero who comes near it.
- **It presses an opening.** A heal, a work, a swing's follow-through or a spell drawn back within
  7 yards: it comes at once, up to a second early and past the two-at-a-time turn.
- **It reads your guard.** Against a raised guard it's three times as likely to reach for a
  string with a blow no guard stops, or a heavy one that tears at it.
- **It gives ground.** After a string, now and then (three times in ten; not a boss, nor one
  that shoots), it backs off a few yards facing you: come to it, or wait it out.
- **It works round you.** Waiting its turn, it circles toward your back, the shorter way round.
- **No metronome.** Its cooldown between strings is give or take a quarter.

Shield-bearers take light blows on the shield from in front between their attacks: hit them as
they wind up or recover, from the side, or with a heavy attack, a bash or a kick. **Poise**: a
heavy fighter (the Overseer, the Brave) isn't knocked out of its wind-up even by a weapon made for
it unless the blow is heavy; a bash or a kick still does it, and every stagger you cause opens it
to a **riposte** (half again damage, Backstab's flash). A heavy fighter's blow that lands puts you on the ground.

Each raid ends with its **champion**, who comes up the road on his own after the rest, with his bar
across the bottom of the screen while he fights near you, and leaves a trophy in the chest the
first time the camp kills him: **Captain Grashak** with the Warsong Raid (**Grashak's Shield**,
the widest and lightest to hold up), **Shadow Hunter Zanjin** with the Darkspear Raid
(**Zanjin's Spear**, a two-handed spear with the longest reach there is), and **Warlord Gorzak**
with the Horde Warband.

**Warlord Gorzak** comes with the last wave of the Horde Warband: a champion (the skull on his
portrait, his bar across the bottom of the screen while he fights near you), 1200 health and a
great two-handed axe. He cleaves wide, strings two chops into a delayed glowing overhead (it waits
out a dodge pressed in panic), kicks the guard open for an overhead through it, leaps from far
off, and stomps everyone who hugs him onto the ground. At half his health he roars a war cry all
round him (through the guard: dodge it) and his blows come a fifth quicker; then he adds a
whirlwind of three turns (a long breath after it to punish), a fury of four blows ending in a
stomp, and two leaps one after the other. The first time the camp kills him he leaves **Gorzak's
Cleaver** (a two-handed axe that staggers what it hits) and the **Warlord's Helm** in the chest.
The warband never brings more than one of him, however often it comes back.

**Taking their camps.** Kill every Horde fighter in the Fargodeep mine yard (the yard and the
grunts on the hill above it) or at the Darkspear camp and it's taken: a banner, none come back there
for ten minutes, and the first time the camp's chest gets their stores (tin, bronze and linen from
the mine; leather and herbs from the Darkspear). The journal asks for both.

**Resources over the whole forest.** Beyond the handful by the camp (the first goal's fallen
timber, a few herbs, the mine yard's veins), timber and silverleaf stand beside trees and
peacebloom, mageroyal, earthroot, briarthorn and boulders lie in the open all over Elwynn
(`spread` on a `[[nodes]]`: the zone cut into squares that many yards a side, a spot tried in
each where the ground is Elwynn's own by the terrain's area ids, dry, level and off the roads,
clear of the camp's grounds and the ring, then `count` of them picked as evenly as can be and at
least `spacing` apart; `by_trees` beside a trunk). On the host's data the probe finds the 80
timber piles in 39 of Elwynn's 50 300-yard squares, the 60 peacebloom in 43. A world saved before
lays these out fresh on its next start (they are new kinds); the older far piles are gone.

**Walking.** A creature walks straight only where a walker could (nothing solid in the way, no
slope too steep: `Land::walk_clear`); else it goes round by a search of the land (tree trunks
all but closed to it, no corner cut past a wall or a trunk, nothing steeper than it can climb),
a long walk (a raid's march) a leg of 140 yards at a time, and a way searched is kept while its
goal hardly moves. A chase with no way to its foe (up on a ledge) stands, and gives up. Packs
stand clear of trunks and walls, and a startled deer bolts to open ground. Nodes stand clear of
trunks and walls by a stride. The probe reports nodes against a trunk or wall and creatures that
walk into one or up a sudden rise while a hero stands at each place: on the host's data, 128
nodes and five creatures before, one vein (set by hand) and one brush against a house's corner
after.

**Inside the buildings.** A house's or a barn's inside is walked (the land takes its floor, not
its roof: no floor more than 3.5 yards over the terrain but a bridge's over the water), and
whoever has moved in stands there: kobolds in the Maclure house and the Stonefield house and
barn, the Brotherhood asleep in the Maclure barn's hay, their conjurer in the Brackwell parlour
and archers in its barn, a prowler pair and a black bear in the Eastvale farms, a smuggler's drop
west of the camp, restless spirits in the orchard house by Mirror Lake, and the Brotherhood
drinking in the Lion's Pride Inn with the innkeeper's strongbox in the back. Each has something
hidden or kept there (a secret that shows itself only inside, or a chest that fills again).
Nothing inside sees a hero through the walls. The probe's `inside x y...` finds a building's
rooms (the middle and two far corners).

**Down in the mines.** Each cave and mine (Fargodeep, Jasperlode, Echo Ridge, the dens) is its
own ground with floors below floors over its box, the hill's slope and its tunnels both, as the
Deadmines are: a creature there stands, walks and searches its way on its own floor, sees only
along its tunnels, and a walk into or out of one goes only by its mouth. Packs down there give
their floor's height (`z`), nodes theirs (`deep`). The kobolds hold Fargodeep's tunnels from the
mouth to the flooded bottom (and a cache there), Echo Ridge's from the mouth down to Snivvle and
his hoard, Jasperlode's first tunnels, with spiders deeper in the dark and Mother Fang at the
far end with the Black Grimoire; silver, copper and tin veins in all three. The probe's `caves`
lists each cave's spots underground and checks they walk out to the land.

**Roaming the wilds.** `[[roam]]` groups are spread over the whole forest at a world's start,
each a pack where it stands: wolves and prowlers by the trees, boars and black bears in the
clearings, Defias pairs off the roads, deer, rabbits and squirrels everywhere, clear of the camp
(82 groups on the host's data, 499 creatures in all).

**Patrols.** Two Warsong grunts walk the mine road towards the camp and back, and two Darkspear
berserkers hunt the forest's edge from their camp, waiting a while at each end; a fight stops a
patrol until it's over (`route` and `wait` on a `[[pack]]`).

**Danger.** A blow that no guard stops (the Overseer's slam, the bear's mauling, the Warlord's
war cry, the frost burst and the like) turns the creature's weapons red, pulsing, for its wind-up
(a creature with none glows red all over): dodge it. On a stock client, a red **!** over your
hero instead; a hazard (a cannon's blast, a spell's ground) keeps the **!** everywhere.

**Posture.** The heavy fighters and the champions have a stance to break: enough damage within six
seconds (110 for the Overseer, 100 for the Brave, 130 for the Captain, 150 for the Shadow Hunter,
240 for the Warlord) and it reels for a couple of seconds, open to a riposte ("Stance broken!").
Fighting together, keep the pressure on.

**Backstabs.** A blow on a creature from behind does half again its damage, with Backstab's
flash: get round the ones circling you, or the one busy with your friend.

**Healing is an opening.** Use a roast, stew or salve with a foe close and on you, and it comes at
you at once: back off first, or eat behind a friend.

**Crowds.** No more than two Horde fighters strike at you at once. The rest circle at a few
strides, facing you and stepping sideways (walking, not running), and each takes its turn when
one of the two is cut down or falls back to recover. A fighter between attacks also circles a
little rather than standing in place. A shield-bearer raises its shield (the guard stance) while
it waits and lowers it to attack, so you can see when a light blow will be blocked. Quick
fighters (the Berserker most of all, then the Deathguard and the Grunt) sometimes see a heavy
attack coming and step back out of it, so feint with a light blow or catch them in recovery.

The beasts of the forest are wildlife, hunted for meat and hides, and fight in patterns too:
**boars** to the south-west (they only fight back: a gore, two gores, a charge from farther off),
pairs of **Timber Wolves** (a quick bite, good for learning the parry; snapping, two quick bites
and a harder one; a leap from farther off) and a **Young Forest Bear** (a slow swipe; two swipes,
left and right; its mauling through the guard; a charge). Like the Horde, no more than two of them
go for you at once.

Every number of every pattern is in `[[mob.pattern]]` in `content/survival.toml`, and tunable in
play: `.es tune mob.grunt.pattern.0.steps.1.windup 0.3`.

## Elwynn beyond the camp

The world is the whole zone now (`htl-map`'s `elwynn.rs` reads every tile of it: about 1,600 by
2,700 yards, loaded in under two seconds on the host). The **world map** (M) shows all of Elwynn
explored from the start, and marks every place worth going to: a found place in full colour with a
tick, one not yet found greyed; its tooltip says what's there. Along the top, the camp's tally:
places found, chests opened, secrets come upon. The first hero to walk into a place finds it for
the camp (a banner, and a little into the chest); the journal counts them. With no supplies of
yours to point to, the corner's arrow points the way to the nearest place not yet found (within
600 yards), and says how far.

| Place | What's there |
| --- | --- |
| Northshire Abbey and its vineyards | the priest's shrine; Defias thieves; Garrick Padfoot (rare); Brother Malach (the priest's champion); a relic behind the Abbey |
| Echo Ridge Mine | kobolds; Taskmaster Snivvle (rare) and his hoard; copper, tin, silver |
| Crystal Lake | murlocs; Grizzlefin the Old (rare); fish and pearls; a bear den; a sunken strongbox (secret, guarded) |
| Jasperlode Mine | kobold geomancers and tunnelers; spiders that drop out of the dark; Mother Fang (rare); silver; the warlock's relic |
| The Tower of Azora | the mage's shrine; Defias conjurers; living embers; the Unbound Flame (the mage's champion); mageroyal; the mage's relic in the dens nearby |
| Heroes' Vigil (on Stone Cairn Lake's island) | the paladin's shrine; Forsaken defilers and darkcasters; Defiler Morvane (the paladin's champion); a grave whose dead rise |
| The Stone Cairn | the shaman's shrine; rocklings; Rumbler (the shaman's champion); earthroot, stone |
| The burnt farm | the warlock's shrine; the cult, its imps and voidwalker; Summoner Vexx (the warlock's champion) |
| Brackwell Pumpkin Patch | the rogue's shrine; the Brotherhood's camp; Morgan the Collector (the rogue's champion); Princess (rare); pumpkins; a strongbox in the barn (guarded) |
| Eastvale Logging Camp | the hunter's shrine; prowlers and black bears; timber |
| Ridgepoint Tower and Blackfang's den | the Blackrock: outrunners, shadowcasters, Rak'gash the Herald and the tower's armory; Old Blackfang (the hunter's champion) |
| Jerod's Landing | Defias smugglers and their crate (guarded); murlocs; fish; the rogue's relic under the dock |
| The Maclure Vineyards and Stonefield | kobolds raiding the farms; Goldtooth (rare); cows, chickens, sheep (wool); Ma Stonefield's savings (secret) |
| Forest's Edge and Westbrook | the Riverpaw gnolls and Hogger (rare, wandering between their camps); briarthorn; the Westbrook armory (once Hogger falls); the paladin's relic at the garrison |
| Mirror Lake, its orchard and Thunder Falls | the druid's shrine; Moonfeather (the druid's champion); the druid's relic in the orchard, the shaman's behind the falls |

**Characters.** Each login can have up to ten characters, made on the client's own creation
screen: a name (letters only, two to twelve, no one else's), any of the eight races, either sex,
skin, face, hair, its colour and facial hair, and a class. The 1.12 client only offers each race
the classes it had in 1.12 (a human can't be a hunter, druid or shaman: those need a dwarf or
night elf, a night elf or tauren, an orc, tauren or troll). Every race is a camp survivor (the
same faction); the camera and the swing clips were tuned on a human man, so the other models may
sit a little off. Each character is its own hero, kept apart in the save under `account/name`: its
gear, bags, Valor, look and cards. The camp's chest, stations, journal, raids, the places found
and the chests opened are the camp's, shared by everyone. A save from before characters keeps
each account's first footman (a human warrior) on the list as it was. A character can be deleted
from the select screen when it isn't in the world.

**Cards are learned out in Elwynn.** A new hero knows the Q and E cards of the class picked on the
creation screen (`start.cards`), on those keys; 1, 2 and 3 are empty until learned. Each hero
learns for themselves:

- each class's **shrine** (a place marked as a sanctum) teaches its Q and E cards (another class's
  Q and E, for most): fight through what holds the ground to reach it, then pass its **trial**.
  Using it wakes its guardians (two or three in the class's spirit: gnolls at the Garrison's
  Banner, the Abbey's restless dead, Azora's embers, the cairn's rocklings, the cult's imps...);
  once they've all fallen, every hero within 40 yards learns its cards. It stays, and each hero
  who wants its cards calls its trial up again; walk away (60 yards, 12 seconds) and the
  guardians go back to waiting;
- a hidden **relic** (a secret: unseen until a hero comes within a few strides) teaches its 1 and 2
  the same way, after its keepers' trial; what's in it goes to the first to pass it;
- the class's **champion** teaches its 3 to every hero within 40 yards when it falls (and whoever
  struck the last blow): the champion is its own trial.

The board (N) says where each card not yet learned is taught. For testing, `.es debug` and then
`.es goto <place>` (`northshire`, `azora`, `forests_edge`...) sets you down at a place. `full_tree` in `[dev]` (off now) gives
every hero every card, as before; heroes saved before keep what they had, and `.es debug` lets a
card be taken off the board.

**Starting over (testing).** With `.es debug` on: `.es reset me` sets your hero back to their first
day (at the camp, their class's Q and E, the axe, buckler, bow and a roast; still the same name,
class and look). `.es reset camp` starts the camp's whole world over (the chest emptied, stations
gone, the journal at its first goal, no raid beaten, no place found nor chest opened, every
creature and resource back): the save is first written and copied beside itself as
`elwynn-survival.toml.before-reset-<time>`, so copying that back over the save (with the server
stopped) undoes it. The heroes keep what they carry; `.es reset me` each for a true blank slate.
The Deadmines aren't touched.

**Chests** (`[[chest]]`): some in plain sight and full again after a while (dens, the farms, the
smugglers' crate), some shut until the camp has killed their guardian (the Westbrook and Ridgepoint
armories, the champions' hoards), some **secret**, and some guarded by an **ambush** that steps out
as they're opened (the Brackwell barn, the smugglers, the sunken strongbox, the footman's grave).
What's in one goes to the opener's bags (what won't fit, to the camp's chest). Ambushers also lie in
wait on the roads (the Northshire road, the east road's bridge, the west road) and step out as a
hero comes near; spiders drop out of the dark at Jasperlode.

**Rares** wander between spots and come back somewhere else a while after they fall; their fall,
and their return (and where they've been seen), is told to everyone: Hogger, Garrick Padfoot, Taskmaster Snivvle, Grizzlefin, Mother Fang, Goldtooth,
Princess. Every champion and rare leaves a trophy in the chest the first time (Hogger's Cudgel,
Rak'gash's pauldrons, Morgan's vest, silver, pearls...). Patrols walk the roads; rabbits, deer,
squirrels and the farms' beasts never fight back and run from a blow (some give meat or wool).
The camps look lived in (`[[prop]]`): the Brotherhood's fires and wagons, the kobolds' carts and
warning signs, the murlocs' altars, the gnolls' cages, the cult's candles, the Blackrock's fires.

**New goods** and what they make:

| Resource | Where | Makes |
| --- | --- | --- |
| Raw Mud Snapper | fishing schools at the lakes' and river's edges (G: a few seconds); murlocs | Grilled Mud Snapper (heals 40%), Fisherman's Feast |
| Silver Ore | the deeper veins at Jasperlode, Echo Ridge, Ridgepoint; kobolds, elementals, hoards | Silver-Inlaid Helm, Silver-Edged Longsword |
| Mageroyal | round the Tower of Azora and in Northshire | Clarity Draught (abilities back a quarter sooner, 90 s) |
| Earthroot | the stony ground at Stone Cairn and Ridgepoint | Earthroot Poultice (an eighth more off every blow, 2 min) |
| Briarthorn | Forest's Edge, Brackwell | Briarthorn Oil (every blow a fifth harder, 90 s) |
| Wool Cloth | the Brotherhood, gnolls, Stonefield's sheep | Woolen Gambeson (chest, 14%), Padded Leggings, Wool Bandage, the Explorer's Pack |
| Spider Silk | Jasperlode's spiders | the Explorer's Pack (resources stack two and a half times as high) |
| Brackwell Pumpkin | the Brackwell patch | Brackwell Pumpkin Pie, the Feast |
| Shimmering Pearl | murlocs, fishing, hoards | the Silver-Inlaid Helm |

The journal gains steps for all of it: scout four places, learn at a shrine, fish for supper, two
secrets, silver, Hogger, the far reaches (eighteen places), an explorer's pack, the classes' trials
(sixteen cards), Ridgepoint, and every corner.

Far from every hero a creature at rest sleeps (it doesn't think or wander), so the wide land costs
little: a tick with a hero at any of the places takes about a quarter of a millisecond.

## The Deadmines

Read the Smuggler's Logbook by the camp's chest (after the first raid) to go down (its Proving
Ring page is open from the start: see below). The run goes
backwards through the instance, lamp to lamp (`content/deadmines.toml`); resting at a lamp fills
your Miner's Draughts and puts every foe but the bosses back at their posts.

| Area | Lamp | Foes | Boss (trophy) |
|---|---|---|---|
| The cove's east beach | The Smugglers' Landing | pirates, a squallshaper | Cookie (Cookie's Tenderizer) |
| The scaffold round the Juggernaut | The Dry Dock | pirates, a crossbowman, a squallshaper | Mr. Smite (Smite's Mighty Hammer); off the way on the ship: Captain Greenskin (elite) and Edwin VanCleef (Cruel Barb, Blackened Defias Armor), fought when you're ready |
| The Cannon Gallery | The Cannon Gallery | pirates, goblin woodcutters, an engineer throwing dynamite, a Blackguard over a strongbox | — |
| The Foundry | The Foundry's Back Door | Blackguards on the ramp, engineers on the ring | Gilnid (The Smelter's Spanner): the doors fog over while he fights, the smelter's channels overflow on a beat, and at half he calls two woodcutters and pours iron three ladles at a time; his back door opens when he falls |
| The workshop and the Mast Room | The Mast Room's North Door | woodcutters, an engineer, a strongbox | Sneed's Shredder, and when it breaks Sneed himself climbs out (Sneed's Machete); if he wins, the shredder is back whole at the next rest |
| The galleries and the mouth | The Galleries | miners, overseers (shield and cutlass), dynamiters (three sticks at a time), two strongboxes | Rhahk'Zor (Rhahk'Zor's Maul): he keeps the front door shut; under two fifths, a roar that throws everyone down |

Past Rhahk'Zor, walk out of the front door into Moonbrook to end the run. The Brotherhood's own
leathers (vest, shoulderpads, leggings and the red mask) drop now and then from the miners and
overseers.

**Boss loot.** Each boss wears a piece of armour of its own besides its weapon: Cookie's
Stew-Spattered Trousers, Smite's Plated Vest (heavy plate), Greenskin's Tricorne, VanCleef's
Pauldrons, Gilnid's Smelting Pauldrons, Sneed's Grease-Stained Leggings, Rhahk'Zor's Helm. One
of a boss's pieces (its weapon or its armour) falls for certain each time it's beaten, each
other piece a quarter of the time (`boss_extra_chance`).

**Running it again.** `reset_minutes` (30) after a run's first boss falls (or its first
strongbox is emptied), once nobody is below, the bosses and their crews stand again and the
strongboxes fill: the Deadmines can be farmed. The lamps lit, the gates and the shortcuts stay as
they are. `.es reset camp` starts the Deadmines over with the camp (no lamp lit, no door open,
whoever's below set down at the first lamp; their save is kept aside as well).

**The journal below** is the camp's: the goal shown there, and its count, are the camp's
(mirrored every tick; the journal moves on only at camp). After the Logbook, the next goal is to
kill Cookie once.

## Secrets and notes

Notes lie where they were dropped below; read one (the use key) and it opens on a parchment page
that stays till you close it, with a clue in red ink while the secret it speaks of is still
hidden. Every hidden spot has a marker to search (a heap of sacks, a pumpkin on a grave, a book
in rusted chains): come near it and you're nudged once a run, search it and it gives up what it
kept. The camp remembers every note read and secret found, but the book doesn't list or count
secrets: they're found down there, not ticked off. Its page for each dungeon has a **Notes** row
(`.es lore <dungeon>`): the rumours the notes read give of what's still hidden, and every note
to read again (`.es note <note>`).

## The Bookmaker's Wager (timed runs) and the Bookmaker's Board

Each dungeon's page in the book has **The Bookmaker's Wager** (`.es wager <dungeon>`): a run
against the clock at a level, as a mythic keystone's. Taking one (`.es wager stockade 3`, or click
a level) starts the dungeon fresh for whoever takes it: bosses up, secrets hidden, shortcuts
shut (given back as they were once everyone's out). The clock starts when the first hero leaves
the first lamp and stops when the dungeon's last boss falls (every boss that stands from the
start; secrets' bosses and rares are extra); each fall adds 10 seconds. Lamps only take you home
during a wager. Friends join it by taking the same level from the book while it runs.

- **Levels.** Every camp starts open to +1 on each dungeon. Won in time, the next level opens;
  in four fifths of the time, two levels; in three fifths, three (up to +20).
- **Tougher every level.** Foes have 9% more health and hit 6% harder each level (compounding).
  From +2, +4 and +7 the week's three terms come in, one each, and they change how a fight is
  fought: *The Call* (a pack still fighting after five seconds calls the nearest pack at rest) or
  *Quickening* (bosses hit a tenth harder for every twenty seconds a fight lasts); *Rallying* (a
  foe's fall heals its pack a little and makes it hit harder) or *Frenzied* (foes below a third
  of their health hit a third harder); *Smouldering* (fallen foes leave burning ground) or
  *Eruptions* (the ground under one of you erupts now and then). Four weeks round, starting
  Mondays.
- **Better pay.** Plunder for every hero there (6 + 3 a level; half when over time), and in time
  the dungeon's own gear into the camp's chest: one piece and one more every three levels, the
  finer pieces (a perk, a set's) from +3, one for certain from +6.
- **Times to beat.** By the dungeon's size (8 minutes, 0.4 a pack, 1.5 a boss, rounded to 2:30),
  or `wager_minutes` in its file: the Deadmines 35:00, the Stockade and Ragefire 27:30, ..., the
  Library and the Cathedral 20:00. First guesses: tune them from the board.

**The week's news.** The first time a hero comes in each week (once the camp has the book), a page
says the week's three terms and last week's three best runs on the board; `.es news` shows it again.

**The fighting record.** Every time a hero fells another (the Proving Ring, friendly fire, a fight
below; not Siege, whose heroes are made for the match), the camp keeps it: `.es record` lists the
heroes by how many they've felled, and their falls to heroes.

**The board.** Every wager won is kept in `saves/wager-board.toml`; `.es board <dungeon>` shows the
runs in time, highest level first, then quickest (`.es board <dungeon> solo`, `duo` or `group`: that
size of party only, a link on the page). One server keeps the board for everyone (the
one friends join: it runs without `--board`). Anyone hosting their own sets `HTL_BOARD` in
`settings.sh` (or `--board`) to that server's address and login port (`1.2.3.4:3724`); theirs
then sends its runs there every minute and fetches everyone's back. Runs go over plain HTTP on
the login port (no new port to forward); the keeper takes short requests only, a few a minute from
one address, checks every run (a known dungeon, a sane level and time, names that are only names,
the game's signature) and keeps the best 200 of each dungeon. `HTL_BOARD_NAME` (`--board-name`)
names your camp on it (a made-up "Camp 3F2A" otherwise).

## The Proving Ring

The Smuggler's Logbook has a page that's open from the start: the Proving Ring, a ring of
braziers on the level field south of Fargodeep (`[arena]` in `content/survival.toml`). Read it and
you're set down inside, a battle-glade banner behind you. Ten seconds later the first wave walks
in from the edge, and they never stop coming:

- each wave is worth more than the last (the foes' `cost`; half as much again for each hero past
  the first), up to seven at once, after which they only get tougher (5% more health a wave);
- new kinds join as the waves go on (boars, wolves and grunts first; Darkspear berserkers and
  archers from the second; cutthroats and spearmen from the third; shamans, witch doctors and
  bears from the fourth; Deathguard, braves and overseers later);
- Elwynn's own foes join too: kobolds and Riverpaw runts from the first wave, murlocs from the
  second, footpads and prowlers from the third, and on to Blackrock outrunners and black bears;
- every fifth wave brings a champion (Captain Grashak, Shadow Hunter Zanjin, Hogger, Warlord
  Gorzak, Rak'gash the Herald, round again) with half a wave beside him, and beating it pays linen, leather and copper
  into the camp's chest;
- a wave beaten gives everyone in the ring 30% of their health back and eight seconds before the
  next; the banner calls the next one at once (`.es arena next`).

The wave and the foes left of it, or the next one's countdown, sit in the top right corner where
the raid's count goes. Your run ends when you fall (nothing you carry is dropped: you get up at the
camp), when you take the banner home (`.es travel camp`) or when you walk out of the ring; when the
last hero is out, the ring clears. Anyone can join a run under way from the book. The camp keeps
its best wave (shown on the page), and a new best from the fifth wave on is announced.

## Siege: camps against camps in Arathi Basin

The Smuggler's Logbook has a second page open from the start: **the Siege of Arathi Basin**
(`content/siege.toml`). Read it and you're in Trollbane Hall, overlooking the basin, as a fresh
hero of your class: its Q and E, the plain sword, buckler and bow. Your own hero (gear, cards,
bags) waits at camp, untouched, until you come home by the muster banner (or `.es travel
camp`). The server needs Arathi Basin's ground from your 1.12 data; without it, the page says
Siege isn't open.

**The muster.** At the banner in the hall, join a way to play: **one against one** (the
Stables against the Farm), **two against two** (two heroes sharing a camp: one Hearth, two
purses), **three** or **four each for themselves** (the Gold Mine and the Lumber Mill too). Once a
way's seats are full it begins fifteen seconds later, and in that time each hero is told which
camp they'll hold and where it is; **Begin now** starts it with whoever's waiting (two at least),
and a camp with nobody in it isn't played.

**A match.** Each camp is a **Hearth** (4000 health) with its building ground round it (40 yards),
a weapon rack (free), a **Mercenary Ledger** and a **Battle Trainer**. The first wave comes after 45 seconds, then one
every 55 seconds whether the last is beaten or not, twenty in all, unless everyone is **ready**
sooner: F at your own Hearth (or `.siege ready`; again takes it back), and once every hero still
in the match is, the next wave comes in 5 seconds (the corner shows who's ready). Each wave comes
in at every camp's gate (out along the way to the middle; marked red on the ground before the first
wave and as each warning comes), in groups of three six seconds apart:

- the **base wave** everyone faces: two or three to begin with, growing to five or six (boars,
  wolves, kobolds and Riverpaw runts first; murlocs, Defias, prowlers, brutes later; Blackrock,
  bears and Forsaken by the end), more for a camp with two heroes in it;
- and whatever its rivals bought for it at their ledgers, with a warning ten seconds before
  ("Coming at you").

A match's foes have 60% of their usual health and 70% of their usual damage (everyone starts a
match with two cards), toughening 4% a wave.

Foes go for a hero near them, then the camp's defences (palisades, barricades, watchtowers), then
the Hearth. One at a defence or the Hearth that a hero strikes finishes the blow it's in and turns
on that hero (the rest of its string at the Hearth is dropped). Every wave also brings each hero
still standing a Roast Meat and a Healing Salve (up to three of each in the bags): the match's only
food. A camp whose Hearth falls is out; the last camp standing wins, or the healthiest
Hearth once the last wave's time is up (a draw if two are even). Twenty seconds later everyone
is back in the hall, fresh, for the next.

**Gold** is the match's only money (the corner shows it, and every camp's Hearth):

| Gold from | |
|---|---|
| Your start | 100 |
| Your income, paid as each wave begins | 20, and more for every send you've bought |
| Each foe you fell | its health times 0.08 (a wolf 5, an overseer 26, Hogger 72) |
| The Blacksmith's veins, in the middle | 12 a gather |
| A rival you fell | 15% of theirs |

It buys defences (H: a palisade 15, a barricade 20, a watchtower 60, on your own camp's ground
only; mending a wrecked one is half), more spells (the **Battle Trainer** behind your Hearth
sells your class's other cards, cheapest first, and one bought goes straight on its key if that
key is free; any card of any class is also on the board, N; 60 for utility, 70 defence, 80 a
ranged or close spell, 150 an ultimate), and
**sends** at the ledger. Each send marches on the rival you're sending to (the ledger's top row:
Next turns to the next rival camp standing) with their next wave, and raises your income for the
rest of the match. A hero can send at most 4 creatures for the first wave, one more for each
wave after (8 by the fifth, 23 by the last); the ledger's top row shows how many you've sent of
how many, and a send over the limit says so ("Wave 2's limit: 3 of 5 creatures sent, this is 4"):

| Send | Gold | Income | From wave |
|---|---|---|---|
| 3 Kobold Laborers | 15 | +1 | 1 |
| 4 Murloc Foragers | 25 | +2 | 2 |
| 3 Timber Wolves | 30 | +2 | 2 |
| 2 Defias Footpads | 40 | +3 | 3 |
| 2 Mine Spiders | 45 | +3 | 4 |
| A Riverpaw Brute | 60 | +4 | 4 |
| 2 Defias Conjurers | 70 | +5 | 5 |
| 2 Darkspear Berserkers | 80 | +5 | 6 |
| 2 Riverpaw Mystics | 90 | +6 | 7 |
| An Elwynn Black Bear | 100 | +6 | 7 |
| A Bloodhoof Brave | 110 | +7 | 8 |
| A Warsong Overseer | 120 | +8 | 8 |
| Hogger (once a match) | 400 | +25 | 10 |
| Warlord Gorzak (once a match) | 700 | +40 | 12 |

**Rivals and partners.** In a match, heroes of rival camps strike each other (a fall gets you up
at your own Hearth eight seconds later, whole, and costs you 15% of your gold to whoever felled
you); partners in two against two never do, and nobody does in the hall. Walk out to invade a
rival's camp, or fight over the middle's veins, but your own camp holds alone meanwhile.

Commands, if you'd rather type: `.siege join 1v1` (or `2v2`, `1v1v1`, `1v1v1v1`), `.siege
leave`, `.siege start`, `.siege send <kobolds|murlocs|wolves|footpads|spiders|brute|conjurers|
berserkers|mystics|bear|brave|overseer|hogger|warlord>`, `.siege target [camp]`, `.siege`.

## Resources, stations and what they make

**Where to build.** The camp's building ground is the meadow south and west of the chest, 46
yards across its middle each way (`build_center`, `build_radius`), ringed by torches. While you
place a station the add-on lays the whole ground out in green on the ground, and a spot outside
it says how far and which way the ground is. `elwynn_probe <Data> <rules> x build [x y r]...`
maps a building ground on the real land (a cell every 3 yards: where a cooking fire goes, and
why not where it doesn't); the one in the rules takes a fire on 606 of its 673 cells.

**The work takes time.** Making something at a station takes 2.5 s (gear 1.6 times as long),
building one 4 s (a watchtower half again), mending a wrecked one 3 s (`[work]`). You stand at it
with a bar ("Making Roast Boar"); walking off or a blow stops it, and what it costs is paid as
it's done. The station goes up where its ghost was put. Each station's making wears a 1.12
recipe's cast; building and mending wear Mining's.

| Resource | Where | Used for |
| --- | --- | --- |
| Elwynn Timber | fallen wood by the trees west of the camp | every station, mending |
| Raw Meat | boars, wolves, the bear | roasts, stew |
| Rough Hide | wolves, boars, the bear | leather (tanning rack) |
| Linen Scraps | the Horde's fighters, its supply crates | salves, armor linings, packs |
| Peacebloom | the meadows around the camp | salves, stew, tonics |
| Silverleaf | in the shade of the forest's trees (and the Warsong shamans) | tonics |
| Rough Stone | boulders along the mine road | the cauldron, the workbench, sharpening stones |
| Fargodeep Copper | the mine yard's veins | the anvil, bronze, the cauldron |
| Fargodeep Tin | the veins at the back of the mine yard (and the Overseer) | bronze |

The Horde keeps its own supplies in its camps: **Horde Supply Crates** (in the mine yard, at the
Darkspear camp and in the Deathguard's meadow: linen, and maybe tin, bronze, leather or a
sharpening stone) and **Horde Field Kits** (in the mine yard and the Darkspear camp: a salve, and
peacebloom, maybe a tonic). Prying one open (G) takes a couple of seconds under the guards' noses,
and it's back ten minutes later.

| Station | Costs | Makes |
| --- | --- | --- |
| Cooking Fire | 6 timber | Roast Meat (heals 45%), Hearty Stew (heals half, and *Well Fed*: stamina back a fifth faster for 3 minutes) |
| Smithing Anvil | 8 timber, 4 copper | Reinforced Shield, Bearded Axe, Bronze Bars (2 copper, 1 tin), Bronze-Ringed Mail, Bronze Helm |
| Tanning Rack | 6 timber, 2 hides | Cured Leather (2 hides), Hide Jerkin, Leather Cap, Linen Pack |
| Herbalist's Cauldron | 4 stone, 2 copper | Healing Salve (quick: a tenth at once, 30% more over 10 s), Silverleaf Tonic (stamina back half again as fast for a minute) |
| Workbench | 8 timber, 3 stone | Rough Sharpening Stones (every blow a tenth harder for 2 minutes), Leather Rucksack |

**Armor** is worn on the chest and the head, over the plain outfit, and shows on the hero. It
takes its share off every blow that lands (the HUD shows the total): Hide Jerkin 10%, Leather Cap
4%, Bronze-Ringed Mail 20% (stamina back a tenth slower), Bronze Helm 7% (a little slower). Made
armor is put on at once; what it replaces goes to the chest, where `.equip` (or the chest's
window) swaps it back.

**Packs** stack resources higher in the bags: the Linen Pack half again, the Leather Rucksack
twice as high (so a trip carries more home).

**The quick item**: R (or 1) uses it, **G** picks the next usable thing in the bags (a roast, a
stew, a salve, a tonic, a sharpening stone); the equipment cross shows it with how many are left.
Right-clicking one in the bags uses that one. What it's still doing (Well Fed, Salve, Tonic, Honed)
shows under the stamina bar with its time left.

## The loop

The camp's **journal** (top right; **J** opens the whole of it) gives the next step and pays into the chest when the camp
(everyone together) has done it: timber banked, a cooking fire, roasts, copper from the mine, an
anvil, Warsong grunts felled, the first raid held, a tanning rack, armor worn, a cauldron, salves,
the second raid, the Darkspear camp, the Overseer, the mine yard, bronze, a palisade, a workbench, a watchtower, the warband and its Warlord. The steps:

1. You start by the camp chest on the east edge of Goldshire's meadow with an axe, a buckler, a
   hunting bow and one roast. The weapon rack beside it has everything else to try.
2. Timber lies beside the trees west of the camp: G at a pile (a short gather). Boars roam the
   meadows south-west: they leave meat and hide. Peacebloom grows in the meadows, silverleaf in
   the forest's shade.
3. Bank what you carry at the camp chest (G). Your bags hold 80 stacks: the backpack's 16 and four 16-slot satchels. The chest's window is a grid
   of what's in it: click a thing to take one, Shift-click for five, right-click for all; click gear
   (the gold mark) to put it on. The build menu and each station list what they make with what it
   costs, red where you're short; hover over anything for what it does.
4. Build a **Cooking Fire** (6 timber) in the meadow beside the camp; cook raw meat into roasts.
5. The **Fargodeep mine yard** is south-west down the road, held by the Warsong: copper veins
   among grunts, a shaman and the Overseer, tin at the back. Boulders along the road give stone.
6. Build a **Smithing Anvil**, then a **Tanning Rack** and a **Herbalist's Cauldron**: armor,
   salves, tonics, bronze (see the tables above).
7. The **Alarm Bell** (G) calls the next raid once the camp has what it needs: the **Warsong Raid**
   (a cooking fire and an anvil), then the **Darkspear Raid** from the west (a tanning rack and a
   cauldron), then the **Horde Warband** with **Warlord Gorzak** (a workbench and a palisade), then the
   **Blackrock Assault** down the east road with **Rak'gash the Herald** (a watchtower and a
   palisade); once all four are beaten the last comes back, a raider more of each kind every time. The road they'll come by is marked;
   they come a few at a time and go for the stations, the defences first. Kill them all for the
   spoils; if everyone is down for long, the camp falls: the stations they reached burn and can be
   mended at the station.
8. **Defences**: a **Palisade** (10 timber, 2 stone) that raiders break before they reach what it
   guards, a **Spiked Barricade** (6 timber, 2 copper) that they go for too, and bleed at, and a
   **Watchtower** (12 timber, 6 stone, 2 bronze) whose archers shoot the nearest raider within 22
   yards every two seconds (raiders go for it first, too).

With co-op rules, falling (or dying) puts you back at the camp after a few seconds at 60% health. What you carried
for the camp stays in a sack where you fell (or the last firm ground you stood on); your gear and
food stay with you. Only you can pick your sack up, all at once; each death makes a new sack, and
sacks last across logouts and restarts. The corner of the screen points the way to the nearest
one.

Resting at the camp heals you quickly (not during a raid); in the field health trickles back after
a while out of combat.

## How it's built

| Part | Where |
| --- | --- |
| The rules (no networking, no files) | `crates/htl-survival`: `sim/combat.rs` (actions, stamina, block, dodge, bow), `sim/mobs.rs` (creatures, telegraphs, packs, leashing), `sim/world.rs` (camp, nodes, stations, placement, caches), `sim/raid.rs`, `save.rs` |
| Tuning | `content/survival.toml` |
| The world server | `crates/htl-server/src/survival/`: `mod.rs` (sessions, login, movement, saving), `play.rs` (events to packets), `draw.rs` (players, creatures, objects, items), `ui.rs` (add-on lines and commands), `land.rs` (Elwynn for the rules) |
| Elwynn's land | `crates/htl-map/src/elwynn.rs`: terrain, Goldshire's buildings as walls and floors, water, the cobbled roads, every tree trunk, read from the host's own 1.12.1 data |
| Realms | `crates/htl-server/src/realmd.rs` lists both games; `lib.rs` `start_games` runs them |
| The add-on | `addon/ElwynnSurvival/` (keys, the click-catcher, bars, windows, placing, banners) |

### Capability ledger (what the client and server offered, and what was done)

| Area | Found | Done |
| --- | --- | --- |
| Input, camera, cursor | benilla latches a mouse press over the world for the camera from the raw button (`player/camera.rs`, `WorldMouse`), whatever `BUTTON1`/`BUTTON2` are bound to; a press a UI frame takes never reaches the world, except during a look session (every press is the world's then); the right button's release ends `MouselookStart`'s look; the world map hides `UIParent`; reparenting a frame from a hidden parent to a shown one leaves it undrawn until it is shown again | A full-screen, bottom-layer frame takes world clicks in action mode: a left press is a swing (or a shot), a right press starts `MouselookStart` and aims, its release `MouselookStop`. A left click during the look is also the world's, so benilla's both-buttons run moves the hero forward while it's held (a short lunge on the click). The profile's keys are set for the session only and put back when you leave the world |
| Facing and movement | The client moves itself and reports position, facing (every 0.1 rad) and its movement-key flags | The server trusts position and facing (as Basin Battle does); facing aims swings and shots; the key flags give the dodge its direction |
| Attacks and hits | No server-side melee targeting is used: `CMSG_ATTACKSWING` is refused | Swings are the add-on's inputs; the server runs ready, wind-up, active and recovery phases and hits everything in a cone in reach, not behind walls or trees, once |
| Animation | `SMSG_EMOTE` plays an `Emotes.dbc` animation: checked on the real data: 36 Attack1H, 35 AttackUnarmed, 43 ShieldBlock, 34 CombatCritical, 15 Roar. `SMSG_ATTACKERSTATEUPDATE` with `0x10000` shows a result without a second swing; victim states 1 hit, 2 dodge, 5 block | Swings play as an emote when they start; the result (number, Block, Dodge) lands at the active moment |
| Dodge | `SMSG_MOVE_KNOCK_BACK` makes the client fly its own player against the world's collision | The dodge is a short knockback-hop (no roll animation exists to use) with a server-owned avoidance window |
| Aiming | No world-point aim from the camera reaches the server | The action-combat client sends the view centre's facing and elevation with each cast (`GetActionAim`); projectiles fly that 3D line from the eye and shoulder, stepped against bodies, ground and walls; the missile is the spell's own, flown to where the line ends |
| Enemies | — | Wind-ups with a turn limit and a final lock, a danger circle where heavy and ranged blows land, recovery windows, packs that link, leashing home |
| Objects and stations | Game objects draw from a display id; a "goober" (type 10) sends `CMSG_GAMEOBJ_USE` on right-click | Chest, bell, timber, copper, stations, sacks, fires are game objects; the station being placed shows with a green ring (good) or a red circle (refused) |
| Inventory | Item objects in pack slots; a use spell makes food usable from the bags | Bags (16 stacks) shown as real items; gear worn and visible to others |
| Persistence | — | One versioned TOML file: camp chest, stations, nodes (by real time), sacks, a raid in progress, every hero by account. Saved every 5 s when something changed and when a hero leaves |
| Mode | Realm list by name; `WOW_REALM` picks one | Two realms on one login; each add-on wakes only on its server's reply |

### Looking at the HUD

`scripts/snap-addon.sh [folder]` lays the HUD out in the add-on harness at the client's own UI
size (768 high) for a fight, the health bars top left, the board of abilities and the Controls card
(`tools/addon-harness/snap.txt`), and draws each to a PNG (`tools/addon-harness/snap.py`): the
add-on's own art as it is, the game's icons as labelled tiles, text in a near font. A layout
check for overlaps and spacing, not the game's drawing.

### Tests

`cargo test` runs the rules' 186 tests (every combat, gathering, building, death and raid rule
the brief lists, the chains, cancels, bash, kick, lock-on, the lag allowances, poise, knockdowns,
ripostes, creatures' patterns, lunges, shields, crowds and backsteps, the casters' blasts, the
Warlord's two phases, trophies and a two-minute soak of his fight, posture, backstabs, danger
warnings, patrols, taking the Horde's camps, the watchtower, the journal kept by title in saves,
armor, packs, the quick item, the journal, raid tiers, the server carrying a runner on, and
tuning) and `crates/htl-server/tests/survival_client.rs`, where benilla's own protocol
code logs in to the survival realm and every packet the server sends must parse:

- both realms are listed with their own ports; Basin Battle still plays as before, and each
  add-on's hello is ignored by the other server;
- a hero's look, stamina bar and gear; the add-on's hello and HUD; a swing at nothing (its
  animation, stamina spent), the guard slowing you, the dodge's hop, the bow drawn; gathering
  timber; banking it; withdrawing; the build menu, a good spot, placing a cooking fire (ghost and
  ring gone); cooking a roast; a long fall: down, a sack where you fell, up at the camp;
- the axe biting a boar and the shield blocking its charge;
- the world kept across a restart (the fire, and the hero where they left); a placement off the
  building ground refused with nothing paid;
- two heroes: each sees the other, a two-use timber pile used by both at once gives two gathers
  and is gone for both, what one banks shows in the other's open chest, stations built by each
  are seen by both, and the bell: asked, warned, the road marked, raiders arrive.
- the action-combat client's lines: its blows (`MV`), a swing's clip, take, lunge and kind,
  the struck hero's reaction on both screens, a heavy blow's flash, the lock-on and letting go,
  the round-trip ping, and a dodge that comes to nobody as a speed burst but that others see;
- the fourth round: blows with their reach and friendly fire, a heavy blow's flash sent to the
  struck but not back to the striker, a step aside seen as one, and tuning in play;
- a champion called up with `.es spawn` (debug only) and its bar's line.

`crates/htl-survival/src/tests_board.rs` covers the board's new cards: Sundering Blow's wind-up,
share of the weapon and stagger, the guard it crushes and the Unstoppable Blow no guard stops (its
target warned), Backstab from behind and on a Mark, Ambush's step, Holy Shock healing a friend and
freeing them (and Avenger's Shield striking a foe), holy ground burning foes and healing friends (and Hallowed Ground's
softened blows), Onslaught, the Death Mark and its reset, Sanctuary, Cold Snap and the shatter,
Dread Step, an old save's keys put on the board's rows, the five rows of four, Lay on Hands,
Shield Wall, and a full charge's flare and faster arrow (Aimed Shot). `tests.rs` has Tab switching
round the foes in view and held letting go, a jump's stamina and a roll's least stamina.

`crates/htl-survival/src/tests_moves.rs` covers the moving mechanics on abilities made for the
test: a knockback from a burst's middle (creatures and heroes), a pull to the caster (a friend
too, unhurt), a leap landing where it was aimed and a Disengage back with its net forward, a strip
after its drawn wind-up (and lost to a stun), a trap arming and springing, a Demonic Circle's
anchor, piercing, chaining and bursting projectiles, shields, mending and haste, a veil, a
Grounding Totem eating a bolt, Evasion and a vortex; `tests_recast.rs` covers recasts (a blink
there and back, an aimed recast, the window running out, no cooldown for the recast), a dash
through two foes and an empowered EX version; `tests_beta_mechanics.rs` covers the beta mechanics
(a hook to a wall and to a foe, and its miss; an orb swapping with a creature and a friend, not a
champion or a steadfast foe; a wall stun; an echo with its recast and by itself; a jump pad; a
cheated death; a mirror on a rival's bolt and a creature's arrow; a tether dragging a foe back; a
shield's and a black hole's bursts; and what the rules turn away). `tests_classes.rs` casts every card of the
nine classes and its EX version on creatures and in a duel, and holds the board to its PvP
budgets.

`crates/htl-survival/src/tests_explore.rs` covers Elwynn past the camp on places and chests made
for the test: a place found by walking there (once, its worth into the chest), a shrine teaching
each hero who uses it and standing for the next, a new character knowing its class's Q and E on
those keys (and keeping its class, look and owner in its save), a champion teaching everyone who
fought it, a hero and the camp starting over, a secret unseen until a hero comes near, an ambush
waking on its chest's opener and one on the road as a hero comes near, a chest shut until its
guardian falls and full again in time (not under a hero's nose), a rare at its spots (and its
news), a harmless creature running and a far one asleep, the Clarity Draught and the poultice, and
a save keeping what was found and opened, and resources spread over a whole land (beside trees or in the open, in every quarter, clear of the camp). `survival_client.rs` checks the world map (every
explored-zone word set on the hero's own create, and the add-on told of every place with whether
it's found); characters made on the creation screen (names refused too short, too long, with
other than letters or taken; a class the game hasn't; two listed with their race, class, sex and
looks; one played, drawn as made with its class's Q and E; deleted by its own account only); an
old save's footman listed as a human warrior; and `.es reset me` and `.es reset camp` (debug only,
the old save kept beside it).

`crates/htl-survival/src/tests_arena.rs` covers the Proving Ring: its page open from the start,
the first wave on its time and on the heroes in the ring, a wave beaten mending them, each wave
worth more and tougher with a champion every fifth (and his pay into the chest), a fall that drops
nothing and clears the ring, and the ways out (the banner, walking out, a save made mid-run waking
at the camp).

`cargo run --release -p htl-server --example survival_probe -- <Data> out.png` lays the world out
on the real land and reports and draws it (nodes, packs, building ground, a walk to the mine, tick
times). On the host's data: Elwynn loads in about 7 s, 1,472 one-yard spots of the meadow take a
cooking fire, the timber sits beside real trees, a raid's walk to the camp plans in about 30 ms,
and a tick with the mine's creatures fighting takes well under a millisecond.

`cargo run --release -p htl-server --example elwynn_probe -- <Data> content/survival.toml out.png`
lays out the whole zone from a content file and reports every pack, creature, object and chest
standing in a wall, deep water or on a cliff, times a walk from the camp to every place and a tick
with a hero at each, and draws it (`... out.png near <x> <y>` prints a small map round a spot). On
the host's data: the zone loads in 1.8 s (about 600 MB at its peak), 280 creatures and 190 objects,
and a tick takes about a quarter of a millisecond wherever the hero stands.

`scripts/check-addon.sh` runs both add-ons in benilla's UI script host against the lines real
sessions send: no Lua errors; keys taken on the server's answer and given back on leaving.

### Not tested yet

The second playtest (a PvP session) showed a hint spammed into the chat and the middle of the
screen on every unaimed click, a lunge forward on every click while aiming, the camera inside the
other hero's head, a long red ribbon behind every dodge, words where icons were wanted, and small
bars that were hard to read. This pass answers those. Still to see with your own eyes:

- the over-the-shoulder aim (2.4 yards back, 0.75 right, 0.25 up) and whether the reticle sits on
  the arrow's line; the slower look while aiming;
- the bow's drawn and held pose from Auto Shot's start, on you and on others, and that the release
  follows on the loose;
- the heavy attack's swing (Special1H and Special2H on the action-combat client) and its
  knock-back;
- the attack clips' timing (fitted to each weapon's wind-up), the trails' colours, how long the
  hit-stop holds and how hard the shake is; the roll's look and its pivot at the hips;
- the upper body's lean with the aim, and the camera stopping short of other bodies;
- Space as dodge on release (a tap) and sprint (a hold), and whether 0.28 s is the right line;
- the guard's pose: the ShieldBlock animation raised once and held with the shield up (the
  tenth part, `patches/benilla/0010-*`: looped, it put the shield up again every second), on the
  upper body while moving; whether 0.38 of the clip is the shield fully up; a two-handed weapon
  keeps its ready stance;
- remote play: everything is decided on the server; on the action-combat client your own swing
  and dodge start at once, but others' (and every hit-stop) come a round trip later; it hasn't
  been tried over the internet.
- the third part (the action layer, move sets, lock-on, fairer hits): the chains' rhythm and
  lunge lengths, the bash and kick (ShieldBash 59 and Kick 95), the reactions' leans, the impact
  flashes and whooshes, the camera kick, the lock-on camera's swing and framing, the clips moving
  to the upper body while running, and whether the dodge still ever leaves a hero in the air. The
  rules, the server's lines and the add-on's handling of them are tested; the client's side is
  built for Windows and the Mac but not yet seen in play.
- the fourth part: whether the local blow test agrees with the server's (a flash on a blow the
  server then judged a miss, or none on one it judged a hit), the shuffle steps, the Dodge clip
  over a sideways dash, the knockdown's timing, the voices' volume, the specials' flashes, and the
  server carrying runners on (a hero who stops dead may be judged a few steps on for a moment).

- the Horde round: the Horde models with their weapons and shields in hand, each pattern's clips
  (stretched to its wind-up) and flashes, the lunges, the shield-bearers' guard stance between
  blows, the circling (a walk sideways, facing you), the backsteps, the casters backing off, the
  Warlord's look (a full-armored orc, his great axe), his whirlwind (the looping Whirlwind clip
  over each wind-up), stomp, war cry and the bar across the bottom of the screen, the patrols'
  walks, the crates' and the watchtower's models (the tower is Elwynn's own, shrunk to a quarter),
  and the tower's arrows (shot from a game object, which the client may draw from its foot).

- the Elwynn round: the world map's marks and tally on the client (laid out in the add-on
  harness only), every place's art showing explored, the new creatures' models and clips (kobolds,
  murlocs, gnolls, Defias, Forsaken, cultists and demons, elementals, spiders, the owl, the
  critters), shrines and chests standing on the ground (a building's floor may hide one), fishing
  schools at the water's edge, and how hard each place's foes feel for its distance from the camp.

### Substitutions and changes to the brief's defaults

- No roll animation exists in 1.12, so the dodge is a short low dash, and on the action-combat
  client the body tumbles over it procedurally; the avoidance window is real.
- Gathering timber, copper, tin and stone shows Mining's cast look; herbs kneel as Herb Gathering
  does and crates are worked open as Opening does (`gather_spell` on a node); crafting shows a
  bandage's sparkle.
- Aimed abilities go along the reticle's line only on the action-combat client; a stock client
  casts along your facing, level.
- The mouse steers in action mode (the playtest asked for right mouse as the guard, so the
  game's own right-button look became always on).
- Enemies come back after they're killed when nobody is near their spot (a few minutes); nodes come
  back by real time (timber 2.5 min, copper 5 min), saved, so logging out doesn't refresh them.
- A raid in progress is saved as its remaining raiders; after a restart (or when the first hero
  comes back online) it gives 10 seconds' warning and goes on.

### Next risk

Whether the shoulder camera and the reticle make aimed shots land where the eye says, and
whether the heavy attack and the soft lock make melee read as committed and fair against another
hero. That's the next playtest.
