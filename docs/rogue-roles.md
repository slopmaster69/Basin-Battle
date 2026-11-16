# Rogue: every skill has a job

The Rogue, the fourth class of the design (Warrior, Priest, Mage, Rogue), rebuilt the way the
Warrior, the Mage and the Priest were: three trees, every skill one job, its numbers with their
reasons, and a build plan of small pull requests at the end. It lives in Elwynn Survival, where
the dodge, posture and the beat are. This page is the design the pull requests in the
[build plan](#build-plan) follow. Built so far: entry 1, the core (two blades, no guard, the
perfect dodge and the Shadow Cut), entry 2, Shadow (Distract and Sap, Shadowstep to a back,
Cloak and Dagger, Shrouded Step), entry 3, Venom (Coated Blade, Eviscerate and Rupture, Garrote
and Feint, Leeching Venom and Choking Cloud), and entry 4, Blades (Flourish, Darting Knife and
Pinning Knife, Blade Flurry and Hemorrhage, Quickstep and Afterimage, Evasion and Adrenaline Rush,
Blade Dance) with the board's third choice, and entry 5, the dual-wield pairs (Twin Fangs, Blade
and Shiv, Crossed Swords, Breaker's Pair, Iron Knuckles) with the fist weapons; see
[Decided while building](#decided-while-building).

## How the Rogue plays

The owner, Nov 15: "Rogue should be focused on dodging, not parrying. If anything they shouldn't
be able to block, and be focused on dual wielding as their main mechanic."

- **Two blades, never a shield.** The Rogue fights with a weapon in each hand. The right mouse
  button swings the off hand. There is no guard, no block and no parry. Defense is evasion:
  the dodge, steps, smoke and shadow.
- **The dodge is the core.** A dodge pressed in a blow's last moment is a **perfect dodge**: the
  body flickers, breath comes back, and the next attack steps behind whoever swung and lands in
  its back. Dodges weave into the two-blade string without breaking it.
- **Two hands, one rhythm.** Main hand, off hand, main hand, off hand, then both blades at once.
  The off hand is the quick, light hand you can always dodge out of. The main hand commits.
- **The back is the payoff.** Any hit in a creature's back is a backstab (#171). The whole kit is
  about getting there quickly and often: steps that land behind, dodges that turn you round a
  foe, and ways to make a foe look away.
- **Different from the other three.** The Warrior blocks and holds the line, the Priest heals and
  protects, the Mage casts from range. The Rogue never blocks, lives up close and in motion, and
  wins by position and timing.

## Which systems this uses

[warrior-roles.md](warrior-roles.md), [mage-roles.md](mage-roles.md) and
[priest-roles.md](priest-roles.md) describe **Basin Battle**: run-cards with decks, rarity, Epics,
Weapon Paths and Specializations, levelled up in a wave-defense run. Elwynn Survival has none of
that. It has the **board** (five keys: Q ranged, E close, 1 utility, 2 defense, 3 ultimate; each
key a card and its EX for Valor), the weapons on the rack, and the action-combat core. The Rogue
lives in Survival because the owner's direction is about the dodge, so the roles docs' shape is
kept and their systems are mapped onto Survival's:

| Roles docs (Basin Battle) | The Rogue (Elwynn Survival) |
| --- | --- |
| Every skill has one job, a table of jobs | The same: a table per tree |
| Specialization (Fire, Frost, Arcane) | A **tree**: the Rogue's card on each of the five keys (board choice 1, 2 or 3) and a **trait** |
| Weapon Path (re-arms you, opens its skills) | A **dual-wield pair**: what is in your two hands, each pair with one rule of its own |
| A skill's deck of upgrades, rarity, Epics | None. Survival has no run-cards; adding them is not needed for the Rogue |
| Lit keys, choices, combos | Survival's own: recasts, empowers, EX versions |

Two small additions, both needed and both reusing what is there:

- **A third choice on the board's Rogue column.** Hunter and Priest already show two choices per
  key (`choice = 2`). Three trees need a third. The content check (`content.rs`: `choice` 1 or 2,
  at most 18 cards a row) and the add-on's board (rows two cards high) grow to three for the
  Rogue only.
- **A tree's trait is on while 3 of your 5 keys hold that tree's cards.** Survival has no
  Specialization card to give a passive, and the board mixes freely. Three of five keys is most
  of the bar: a pure tree has its trait, a 3 + 2 mix has one trait, never two. Traits are the
  Rogue class's own (a hero created as a Rogue), as Specializations are a class's own.

What is the Rogue's and what is anyone's: a hero **created as a Rogue** has the footwork (the
perfect dodge's payoff, the dodge-feint, the light off hand) and the trees' traits. Any hero may
still put a Rogue card on a key (the board mixes classes), but without the Rogue's body. Cards
that need two blades say so, the way Shield Wall needs a shield (`needs_shield`).

## What the Rogue has today that breaks it, and how each changes

Today a Rogue hero starts with the same Footman's Axe and Buckler as everyone (`[start]` in
`content/survival.toml`), raises a guard on the right mouse button, and parries. Its five cards
(Fan of Knives, Backstab, Shadowstep, Smoke Bomb, Death Mark) never block, but its hands do.

| Today | Breaks the direction because | Becomes |
| --- | --- | --- |
| A shield (Buckler) at the start, any shield from the rack | It blocks | A Rogue **can't equip a shield**, as in WoW 1.12. It starts with two Poacher's Knives (`dagger`). The rack and the bags refuse a shield or a two-hander with "Rogues can't use shields." / "Rogues can't use two-handed weapons." |
| Two-handers and staffs | One weapon, no off hand, and a staff guards with a ward | A Rogue equips **daggers, one-handed swords, one-handed maces and fist weapons** only (WoW 1.12's Rogue weapons; axes go too). Bows stay |
| The guard (right mouse held) | Block | **The guard key is the off-hand blade**: a tap swings it (the dual-wield blow that exists, `off_press` in `sim/combat.rs`). Held, it does nothing. See the decision below |
| The parry (guard raised just in time) | The Rogue mustn't parry | Gone. A **perfect dodge** pays instead (25 stamina and 10 Valor for a parry become 20 stamina and 10 Valor) |
| The riposte off a parry (×1.5 on a reeling foe) | Comes from the parry | Gone with the parry. The rule's **riposte on any stagger you cause** stays (`riposte_any_stagger`): the two-blade finisher's 0.8 sec stagger opens one |
| The parry's posture fill (25% of a boss's posture) | Without parries the Rogue can't break a boss's stance | A perfect dodge fills **12%** of a boss's or champion's posture |
| The feint (guard raised in your own wind-up calls the blow off) | Lives on the guard | The **dodge** pressed in your own wind-up calls the blow off, stamina back, and the dodge goes |
| Shield bash and kick (right + left mouse) | A shield's tools | None: right + left is an attack, as dual wield has it now. The Rogue goes round a guard, not through it (a creature's shield blocks only from the front) |
| Retaliation (a Warrior card that blocks) and the beta page's Riposte (a parry stance) | A Rogue could still block or parry through a borrowed card | On a Rogue they refuse: "Rogues can't block." / "Rogues can't parry." Other borrowed cards work |
| Shadowstep goes to the reticle's point | That is the Mage's Blink | It lands **behind the enemy under the reticle** (Ambush's step), else at the point. The step back is kept |
| A Rogue save with a shield or a two-hander | Would hold what a Rogue can't | It loads with that item in the bags and the off hand empty (an empty off hand jabs; see below). Old card keys load as today (`RENAMED` in `save.rs`, `an_old_save_with_the_rogues_old_cards_loads_with_their_successors`) |

**The guard key decision: the off-hand blade, not a dodge or a step.** Why:

1. The owner asked for it on Nov 7: "love the idea of no blocking with dual wielding as right
   click can now become your off hand attack. Combo's!" ([weapon-identities.md](weapon-identities.md)).
   It is built.
2. Dual wielding is the main mechanic, and a rhythm between two hands needs two buttons.
3. The dodge already has its key (Mouse 4). A second dodge on right click would be two keys for
   one thing; a short step on it would be a weaker dodge.
4. What the guard gave besides blocking moves to the dodge: the feint, and the cut-short of a
   blow's follow-through.

A Rogue with an empty off hand (a save fixed up, a blade sold) jabs with the empty hand on right
click: an off-hand blow at half what the main-hand weapon would deal, so the rhythm never goes
away and right click never guards.

## The core: footwork and two blades

These are the Rogue class's, whatever its cards. Numbers live in `[dodge]`, `[valor]` and a new
`[rogue]` table in `content/survival.toml`, each with its reason beside it.

### The perfect dodge

The dodge today: 0.45 sec, 15 stamina (18 to start), blows that land 0.04 to 0.32 sec after it
starts miss, and rolling through a blow gives 10 stamina and 8 Valor.

- **Perfect**: the blow lands 0.04 to **0.18 sec** after the dodge starts (pressed in its last
  moment). Why 0.18: the parry's window is 0.25 sec (0.35 with a sword), but a dodge outside the
  perfect window still evades up to 0.32 sec, so being early costs only the payoff, not the blow.
  0.14 sec of perfect is about 70% of a parry's: harder to earn, never punished. Judged as the
  parry is, with the hero's round trip (`combat.max_grace`, `max_rewind`).
- **It pays**:
  - **20 stamina** back (a plain dodge-through gives 10; a parry 25): the dodge cost 15, so a
    perfect one leaves you 5 up, and a plain one 5 down.
  - **10 Valor** (a parry's; a plain dodge-through stays 8).
  - **The Shadow Cut**: for **0.6 sec**, either attack button steps you behind the one who swung
    (up to **6 yd**, Ambush's step, `sim/combat.rs` "Where an Ambush steps") and strikes as that
    hand's next blow, from behind (a backstab by the rule: ×1.5, a dagger ×2.5). 0.6 sec is about
    a dagger blow's whole length (0.18 wind-up, 0.24 recovery): one press, not a held option. 6 yd
    covers the dodge's 4 yd dash and a reach. A spot behind that is in a wall or off a ledge: no
    step, the blow is the attack out of the dodge.
  - **Posture**: 12% of a boss's or champion's posture. No stagger: the Rogue's reward is the
    step, not a reel.
- **How it reads with floor markers off**: the creature's own wind-up is the tell, as for every
  hero. The payoff reads on your own body and screen: the body flickers see-through for 0.3 sec
  (the Stealth aura, 5215, as Prowl wears it), Vanish's puff of smoke (spell 1856) where the blow
  fell, and a short stop on your screen (a `perfect_dodge` beat, or `readied` if an older add-on
  can't take a new one). The Shadow Cut shows the step's puff at both ends and the backstab's own
  look (the blood burst, kit 220, sound 1152, the `backstab` beat), which the owner chose in #171.
- **What it doesn't do**: slow time. One world runs for everyone in it; a slow-motion for one
  hero would slow the fight for the whole band. The stop on your own screen stands in.
- **Heroes too**: a perfect dodge of another hero's blow pays the same (PvP). The Shadow Cut puts
  you behind a hero, whose locked swing turns only 50°. No crowd control, so the PvP budget holds.
- **Not the Hunter's**: Call of the Wild primes on any dodge through a blow (`prime_wild`). That
  stays as it is: Hunter work is not part of this.

### The dodge-feint and the light off hand

- **Dodge-feint**: a dodge pressed while your own blow still winds up calls it off, gives its
  stamina back (as the guard's feint does now), and the dodge goes. Today a dodge can't start in
  a wind-up at all.
- **The off hand never commits you**: a dodge cuts an off-hand blow's follow-through short from
  the moment it lands (`cancel_after` 0 for the off hand). The main hand keeps 0.45 (0.65 for a
  heavy). So the hands feel different: the off hand is the safe poke, the main hand the commitment.

### The two-blade rhythm

Built today for anyone with two blades: left and right alternating build a run (`dual_run`), the
4th blow is the **finisher** with both blades (×2.2, a 0.8 sec stagger, clip 17), and the next
blow must come within 0.8 sec of the last one's end (`DUAL_*` in `sim/mod.rs`). The Rogue keeps all
of it and adds:

- **A dodge doesn't break the run**: the 0.8 sec is counted from the dodge's end when you dodged
  between blows. Weave: left, right, dodge, left, finisher.
- **The Shadow Cut counts as the next hand** in the run. A perfect dodge in the middle of a run
  takes you behind the foe and on towards the finisher.
- **The finisher from behind is a backstab** (the rule already does this for any blow) and opens
  a riposte on whatever it staggered.
- The rhythm is the hands', judged by the chain window that exists (`chain_after`). It is not the
  beat: the beat (`sim/rhythm.rs`, a schedule shown on screen) is the Priest's Penance and the
  Mage's arcane volley.
- **Seen**: the off hand swings `AttackOff` (clip 87) and the finisher both blades (`Attack1H`,
  clip 17) on the hero's own model; the client shows the weapon in each hand.

### Backstabs, as the game already has them

Any blow on a creature from more than 120° off its facing is a backstab (`from_behind` and
`BACKSTAB_ARC` in `sim/combat.rs`; arts check it in `connect_art`, missiles in `ability_on_mob`):
×1.5, or the weapon's own (`backstab = 2.5` on a dagger). Two rules keep it from being a lock:
each backstab in the last 8 sec takes a quarter off the next one's bonus (a third with three
heroes on it: `backstab_bonus`), and a boss or champion with more than one hero on it may wheel
round on whoever stabs it (`backstabbed`). The Rogue's kit is built on these, never around them:
spreading backstabs over foes beats stacking them on one, and a boss's turn is the Rogue's cue
to dodge, not a bug to remove.

## The three trees

Each tree fills all five keys, so a pure build works. Cards keep the board's budgets (`tests_classes.rs`): a
ranged poke at most about 45 a hit and 9.5 a second, a utility little damage on a cooldown of 10
sec or more, an ultimate the whole Valor bar and 40 sec, no hard crowd control over 1.5 sec on a
hero, an EX 25 or 50 Valor. Each EX does something its base can't and is never a stronger copy
(#378, #379).

What the other three already do, so no Rogue skill does it: **Warrior**: block and brace
(Retaliation, Shield Wall), the boomerang throw (Axe Throw, Grasping Axe's pull), the cone stun
(Shockwave), the charge into a foe (Charge, Heroic Leap), the spin (Bladestorm), the rally (Rallying
Cry), the guard-breaker (Sundering Blow); in Basin Battle, Sunder Armor, Mortal Strike's wound and
Execute's rage. **Priest**: heals and shields for friends, the parry given to a friend (Power Word:
Shield), the sidestep that slips a blow and slows the striker (Fade), rings and bolts on the beat
(Penance), the star there and back (Divine Star), the pull of a friend (Leap of Faith), fear and
holds (Psychic Scream, Void Tendrils), the drained team heal (Vampiric Embrace). **Mage**: the
blink to a point (Blink), freeze then shatter (Frost Nova, Ice Lance), the slow cone (Cone of Cold),
charged bolts (Fireball), the sheep at range (Polymorph), the ring that throws back (Blast Wave),
the sky-fall (Meteor), the untouchable stand (Ice Block).

### Shadow: get behind, and be gone

Board choice 1 (the Rogue column's first line). Built on the cards the owner has kept since #40:
Backstab and Ambush ("loved"), Shadowstep there and back, Smoke Bomb and Vanish. Inspiration: the
assassin's dive and reset (League of Legends), the shinobi's back-step (Sekiro). Best pair: Twin
Fangs.

**Trait: Shrouded Step.** A step that lands you behind an enemy (Shadowstep, Blinding Step,
Ambush, the Shadow Cut, Cloak and Dagger) hides you for **1 sec** (the `veil`). Creatures lose you
for that second: no new wind-up at you. Bosses see through it, as they see through Smoke Bomb and
Vanish (#75). Why 1 sec: one dagger blow and a step out; long enough to land, too short to camp in.

| Key | Skill · *EX* | Its job | Numbers, and why | With floor markers off | Not a copy of |
| --- | --- | --- | --- | --- | --- |
| Q | **Distract** (new) | Make them look away. | A pouch lobbed up to 25 yd (a lob, as Lava Burst flies). Every enemy within 6 yd of where it lands turns to face it for **2.5 sec**: it keeps its foe, starts no new blow, and a blow from its front ends it for that enemy (a backstab doesn't). A boss **glances**: turns to it for 0.8 sec, once every 15 sec, never mid-string. No damage. 10 sec cooldown. Why: 2.5 sec is a Backstab and two blows from behind; 0.8 sec is one dagger blow on a boss. Heroes: no effect (their facing is their own client's). | The pouch's flight and a glittering burst where it lands (a spell visual), and the enemies' bodies turning to it: the turn is the tell. | Hunter's Scatter Shot and Mage's Polymorph take a foe out; Distract only turns it. The Warrior's taunts pull a foe to you; this pushes its eyes away. |
| Q | *Sap* (EX, 25) | Take one out before the fight. | A 0.2 sec close art, 60% weapon. Only from behind or out of a veil; from the front it is just the cut. Incapacitates **6 sec** (1.5 on a hero), any damage ends it. Never a boss, never a creature already fighting. Why 6 sec: time to deal with the rest of its pack. | The creature slumps (the incapacitate's look, Blinding Step's) where it stood. | Polymorph is at range on anyone; Sap needs its back. |
| E | **Backstab** · *Ambush* (kept) | The payoff, and the dive. | Backstab: 0.25 sec, 120% weapon, ×3 from behind, a Mark crits for 50% more, 7 sec cooldown. Ambush: steps behind the enemy under the reticle (12 yd), 140% weapon, always from behind, 50 Valor. Unchanged: loved since #40. | The stab's clip and the backstab's blood burst on the creature. | Sundering Blow breaks a guard from the front; these want the back. |
| 1 | **Shadowstep** · *Blinding Step* (changed) | Get behind, and back out. | 20 yd, 12 sec cooldown, breaks roots and slows. **Lands behind the enemy under the reticle** (within 15° of its line, Ambush's find), else at the reticle's point. Again within 3 sec: straight back to where you began. Blinding Step (25): the same, and every enemy within 4 yd of where you come out is incapacitated 3 sec (1.5 on a hero). | Vanish's puff where you leave and where you land. | The Mage's Blink goes to a point and away; Shadowstep goes to a back. |
| 2 | **Smoke Bomb** · *Vanish* (kept) | Break the fight's grip. | Smoke: 5 yd, 4 sec, you and friends hidden, enemies inside 30% slower, 16 sec cooldown. Vanish (25): hidden 5 sec, 30% faster for 2 sec, the next blow counts as from behind. Bosses see through both (#75). | The cloud on the ground and the bodies inside fading. | Ice Block makes you untouchable in place; Smoke hides you and you keep fighting. |
| 3 | **Cloak and Dagger** (new) | Every blow from behind, for a moment. | **6 sec**, 100 Valor, 40 sec cooldown. Each attack press first steps you behind the nearest enemy within **8 yd**, then strikes as that hand's blow, from behind. It prefers one you haven't struck yet in it. Up to 10 steps. Each step is a Shrouded Step. Why: the backstab's decay makes spreading the steps over a pack the strong play; on a lone boss it is about six backstabs with the bonus falling each time. | The Rogue vanishing and appearing at one back after another, a puff at each. | Bladestorm spins in place; Charge and the Stoop go in once. This one hops from back to back. |

Skills in the tree: the trait and nine cards (Distract, Sap, Backstab, Ambush, Shadowstep,
Blinding Step, Smoke Bomb, Vanish, Cloak and Dagger).

### Venom: build it on, cash it in

Board choice 2. Built on Fan of Knives (three poisoned knives that Mark, #40, #44), Kidney Shot
(#378) and Death Mark's reset (kept since #40). Inspiration: marks that build and a finisher that
spends them (League of Legends' assassins), poisoned blades (Where Winds Meet). Best pair: Blade
and Shiv. It is the tree that works from the front when the back is closed (a stomp at the back:
Rhahk'Zor's within 3.5 yd).

**Trait: Coated Blade.** Each off-hand hit, and each Fan of Knives knife, lays **1 Venom** on the
enemy: up to **5**, for **8 sec**, refreshed by each new one, 1.5 damage a second each (7.5 at 5).
At 5 its poison look deepens: the body tells you it is ripe. The off hand delivers; the main hand
never does. Why: an off-hand blow lands about every other blow, so 5 Venom takes about 5 sec of
string; 7.5 a second is about a third more on a coated enemy, the tree's reward for staying on one.
Built like the Shiv's poison stacks (`weapon_fx.rs`, `POISON_STACKS`), with its own numbers, kept
apart from Bazil's Shiv's own poison: a Shiv in the off hand lays both, side by side.

| Key | Skill · *EX* | Its job | Numbers, and why | With floor markers off | Not a copy of |
| --- | --- | --- | --- | --- | --- |
| Q | **Fan of Knives** · *Kidney Shot* (kept) | The coat from range, and the stun. | 3 knives, 8 each, 16 yd, a 20° fan (all 3 point-blank), poison 3 a second for 4 sec, Mark 6 sec, 2 charges of 8 sec. With the trait each knife also lays 1 Venom. Kidney Shot (25): a close stab, 40% weapon, stun 3 sec (1.5 on a hero), 1 sec longer on a Marked enemy. | The knives in flight, the poison's green on the body. | Multi-Shot is a fan at range; these want you close. |
| E | **Eviscerate** (new) | The cash-in, from any side. | A two-blade art (clip 17), 0.3 sec wind-up, **80% weapon + 35% per Venom**, which it uses up. 6 sec cooldown, 14 stamina. Needs two blades. At 5 Venom: 255%. Why: Backstab from behind is 360%, so Eviscerate is less, but from the front, and only after the build. | Both blades come down at once; the ripe poison look goes out. | Execute (Basin's Warrior) spends the hero's own rage on a low enemy; Eviscerate spends what is on the enemy, at any health. |
| E | *Rupture* (EX, 25) | The team coats. | A cut, 60% weapon, uses no Venom. For **8 sec** every hero's blow on the enemy lays 1 Venom, as your off hand does. Why: the band hits often; 8 sec fills the coat for one Eviscerate. | The enemy bleeds (Rend's blood burst look) while it lasts. | Soul Swap (Warlock) copies rot onto others; Rupture lets others add to it. |
| 1 | **Garrote** (new) | The interrupt, from behind. | A 0.3 sec art. **From behind only**: breaks a cast (Thornspike's interrupt, `weapon_fx.rs`), silences 3 sec (1.5 on a hero; a boss loses only the cast), bleeds 4 a second for 6 sec, lays 2 Venom. From the front it is a 50% cut, nothing more. 14 sec cooldown. Why: the other interrupts (Avenger's Shield; in Basin Battle, Shield Slam, Counterspell, Silence) reach from range; the Rogue's asks for the back. | The cast bar breaks, the caster clutches its throat (the silence's look). | Range interrupts; this one is positional. |
| 1 | *Feint* (EX, 25) | Hand the enemy to a friend. | A 4 yd hop back (the dodge's back step). Every enemy within 8 yd that is on you **loses interest for 2 sec** (the Hypnotic Blade's lull, `LULL_SECS`); one with another hero near turns to them. A boss halves your threat instead. Why: the enemy that turns to your friend shows you its back. | The enemies turning away from you to someone else. | The Warrior's shouts pull enemies onto him; Feint pushes them off you. |
| 2 | **Leeching Venom** (new) | Live off the coat. | **6 sec**: every tick of your Venom on any enemy heals you as much. 18 sec cooldown. At 5 Venom on one enemy: 45 over the 6 sec; on two, 90. Why: a defense that pays only while the coat is up, so the tree's defense is its offense. | A green glow on you (the poison look on yourself), and the healing numbers. | The Priest heals friends; this heals only you, and only from your poison. |
| 2 | *Choking Cloud* (EX, 25) | Make their blows slow to come. | A 5 yd cloud at your feet, 4 sec. Enemies inside wind up **40% slower** (Corpsemaker's `cowed`) and gain 1 Venom a second. Why: slower wind-ups are wider perfect-dodge chances: defense through the dodge, not a wall. | A green cloud on the ground (a lingering spell look) and the enemies coughing in it, their wind-ups visibly longer. | Smoke Bomb hides; Blizzard and Cone of Cold slow movement; this slows the blow. |
| 3 | **Death Mark** (kept) | The reset. | A bolt, 20 damage, 34 yd. For 8 sec the enemy takes 30% more from everyone; a kill under it readies every cooldown. 100 Valor, 40 sec cooldown. Kept since #40. With a full coat it is the tree's big burst: Eviscerate into a Marked, ripe enemy. | The mark's look on the enemy. | No other class resets its cooldowns on a kill. |

Skills in the tree: the trait and nine cards (Fan of Knives, Kidney Shot, Eviscerate, Rupture,
Garrote, Feint, Leeching Venom, Choking Cloud, Death Mark).

### Blades: the dance

Board choice 3 (new). Every clean evasion feeds the blades. Inspiration: the fast two-blade
strings and footwork of Phantom Blade and Where Winds Meet, the short-cooldown dodges and
counters of Battlerite, Supervive and Bloodline Champions, Sekiro's timing windows. Best pair:
Crossed Swords.

**Trait: Flourish.** Each perfect dodge gives **1 Flourish**: up to **5**, for **6 sec**,
refreshed by each new one. Every blow is **6% harder and 4% quicker** per Flourish (30% and 20% at
5). A blow that lands on you takes them all. Why: five perfect dodges in a fight is the top of the
skill; losing them all to one hit is the risk that makes them worth having. Built like the
Bloodlust Edge's stacks (`BLOODLUST_STACKS`), with its own trigger.

| Key | Skill · *EX* | Its job | Numbers, and why | With floor markers off | Not a copy of |
| --- | --- | --- | --- | --- | --- |
| Q | **Darting Knife** (new) | Keep the rhythm from range. | A knife, 9 damage, 22 yd, **3 charges** of 5 sec. A knife that strikes counts as a hand in your two-blade run: two knives, step in, two blows, and the finisher comes sooner. Why: 9 per 5 sec is well under the poke budget; its worth is the rhythm, not the damage. | The knife's flight (a missile look) and its glint on the enemy. | Axe Throw comes back; Fan of Knives fans; this one feeds the hands. |
| Q | *Pinning Knife* (EX, 25) | Keep its back open. | A knife, 9 damage. For **3 sec** the enemy **can't turn** (1.5 on a hero): it still strikes whatever is in front of it. A boss turns at half its speed instead (`turn_rate`). Why: 3 sec is time to get round and land two blows; a boss keeps turning, slowly. | A knife stuck in the enemy, and its body staying put while you circle it. | Roots and freezes stop the feet; this stops the turn. |
| E | **Blade Flurry** (new) | The whole two-blade run, at once. | A string of 4 alternating cuts, **0.3 sec apart**, each stepping 1 yd in; the 4th is the run's finisher (×2.2, both blades). A dodge cancels it between any two cuts. 8 sec cooldown, 20 stamina. Needs two blades. Why: a run by hand takes about 2.4 sec; this is 1.2 sec, but you still have to see the dodge coming. | Both hands visibly alternating (clips 87 and 17) and the finisher's stagger on the enemy. | Bladestorm and Whirlwind spin round you; Flurry goes one way and stops when you dodge. |
| E | *Hemorrhage* (EX, 25) | Open it for the team. | A cut, 60% weapon. For **8 sec** the enemy forgets its backstabs: no backstab on it loses bonus to the ones before (`backstab_bonus`), anyone's. It may still wheel on whoever is at its back. Why: lets a band take turns at one back without the decay; the wheel stays, it is the boss's answer. | The enemy bleeding (Rend's blood look) while it lasts. | Sunder Armor (Basin) strips armor for everyone; Hemorrhage keeps the back open. |
| 1 | **Quickstep** (new) | More dodges than stamina allows. | A 5 yd dash along the movement keys (back with none), **2 charges** of 10 sec, no stamina. It evades as a dodge does, so a perfect one pays (and gives Flourish). It cancels your own blow at any point, wind-up included, stamina back. Why: two charges is a chain of three evasions with the dodge; 10 sec keeps it a utility. | The dash and the body's flicker on a perfect one, as the dodge. | Fade is one 5 yd sidestep that slows the striker; Blink is a 20 yd teleport. Quickstep is charges of dodge that cut your own blow. |
| 1 | *Afterimage* (EX, 25) | Leave a decoy. | A Quickstep that leaves a **shade** where you started, for 2 sec. Every enemy on you turns to the shade (Distract, 1.5 sec; a boss glances, 0.8 sec). Why: the shade is where they last saw you; their back is to wherever you went. | The shade: a creature body with a rogue's look (a Defias Cutpurse's display), see-through under the Stealth aura, and the enemies turning to it. | No one else leaves a body behind. |
| 2 | **Evasion** (new, the name back from #40, not the old stance; key `blade_evasion`) | Make the perfect dodge easier, for a while. | **5 sec**: the perfect window widens from 0.18 to **0.28 sec**, and a dodge costs 8 stamina, not 15. 20 sec cooldown. Not a stance: you keep moving and striking. Why: inside the 0.32 sec evade, so it widens the reward, never the dodge itself. | A lasting shimmer on the body (Evasion's own look in 1.12). | The old Evasion and Ice Block stand still and soak; this keeps you dancing. |
| 2 | *Adrenaline Rush* (EX, 50) | The all-in. | **6 sec**: blows and dodges cost no stamina. Why 50 Valor: it is the strongest moment of the tree that isn't the ultimate. | A burst on you as it starts and a lasting glow. | The Warrior's Rallying Cry gives a band full stamina once; this is yours, for a while. |
| 3 | **Blade Dance** (new) | Every dodge a perfect one. | **8 sec**, 100 Valor, 40 sec cooldown. Every dodge that evades a blow counts as perfect (its Shadow Cut, stamina, Valor, Flourish), dodges cost nothing, and each throws a free cut at whoever swung (60% weapon, from behind). Why: the ultimate is the tree's whole idea at full strength; it hurts only what attacks you. | The body flickering through the whole 8 sec, a counter-cut and a backstab burst on every striker. | Bladestorm hurts everything round you; Retaliation blocks and sweeps. Blade Dance only answers blows that miss you. |

Skills in the tree: the trait and nine cards (Darting Knife, Pinning Knife, Blade Flurry,
Hemorrhage, Quickstep, Afterimage, Evasion, Adrenaline Rush, Blade Dance).

### New effects the trees need

Each is one effect kind, added where its first card is built, reused by the rest:

| Effect | What it does | First used by | Also by |
| --- | --- | --- | --- |
| `distract` | The enemy faces a point for its time: keeps its foe, starts no blow, a blow from its front ends it. A boss glances (0.8 sec, once every 15 sec, never mid-string). No effect on heroes | Distract | Afterimage |
| `venom` | Stacks to 5, 8 sec, damage a second per stack, a deeper look at 5 | Coated Blade | Eviscerate, Garrote, Rupture, Leeching Venom, Choking Cloud |
| `pinned` | The enemy can't turn (a boss: half its turn rate) | Pinning Knife | |
| `flourish` | Stacks on the hero: damage and swing speed per stack, lost to a hit | Flourish | Quickstep, Blade Dance |

## Weapon paths: the dual-wield pairs

A Rogue's hands are its weapon path. Each weapon keeps its own mechanic in its own hand
([weapon-identities.md](weapon-identities.md): a Bazil's Shiv still poisons, a Cobrahn's Fang
still puts to sleep from behind). The pair adds one rule, judged on the two hands' families. The
Rogue's families: dagger, one-handed sword, one-handed mace, fist.

| Pair | Hands | Its rule | Why | Best with |
| --- | --- | --- | --- | --- |
| **Twin Fangs** | dagger + dagger | A Shadow Cut doesn't count toward the enemy's backstab memory. The quickest run (both wind-ups a dagger's 0.18 sec) | The perfect dodge is the hardest timing in the kit; its backstab stays whole | Shadow |
| **Blade and Shiv** | sword + dagger | The finisher takes the sword's reach (3.0 yd) and the dagger's backstab (×2.5) | A finisher that lands from a step farther off, and still stabs | Venom |
| **Crossed Swords** | sword + sword | The finisher is a cross-cut: it strikes every enemy in a 150° arc in reach | #60 found the Rogue falls to a big pack; this is its answer | Blades |
| **Breaker's Pair** | mace + any blade | The finisher counts 3 times toward a champion's posture (the mace's own ×2 on its blows stays) | With no parry, the Rogue needs a stance-breaker for bosses | Any |
| **Iron Knuckles** (new family) | fist + fist | A run of 6 quicker blows (wind-up 0.15 sec); the finisher wheels a non-boss enemy round so its back is to you (it turns back at its own rate). A boss: the finisher's posture counts twice | The fists' job is making backs | Shadow, Blades |

The Rogue starts with Twin Fangs (two Poacher's Knives). Fist weapons are new `[[weapon]]` and item
entries: a `fist` family, a moveset with `AttackUnarmed` (clip 16; 35 is NPCGoodbye), on the rack where the other
weapons are, with a 1.12 fist weapon's display id checked in an ignored test against `WOW_DATA`.

## A team notices its Rogue

- Distract, Feint and Afterimage turn enemies away from the Rogue and toward their friends.
- Rupture lets every friend's blow coat the enemy; Death Mark's 30% is for everyone.
- Hemorrhage keeps one enemy's back open for the whole band.
- Garrote breaks the cast aimed at the healer.
- Smoke Bomb hides friends in it.

## What I built on

- **#40** (the class spell redesign): the Rogue "repositions: in and out before they turn round";
  keep Backstab and Ambush, and Death Mark's reset. #40 also called the old Evasion "a timed stance
  that roots you", which the new Evasion is not.
- **#44**: Fan of Knives Marks so Backstab crits; Shadowstep and Blinding Step step back.
- **#60**: the solo sim's Rogue plan (Shadowstep in, Backstab) and its finding that the Rogue falls
  in the big pack: Crossed Swords and Choking Cloud answer it.
- **#75**: bosses see through Smoke Bomb and Vanish. Kept, and Shrouded Step follows it.
- **#171**: any hit in a creature's back is a backstab, by anyone, with the blood burst the owner
  chose (kit 220). Every back-skill here goes through that rule.
- **#378 and #379**: an EX does something its base can't, niche and true to the class; Kidney Shot
  is Fan of Knives' EX; Prowl shares the Rogue's veil.
- **Weapon identities (Nov 7 and Nov 15)**: no blocking with two blades, the right click as the off
  hand, the run and its finisher; the dagger as the assassin (×2.5 from behind); every weapon's own
  mechanic.
- **The owner's direction of Nov 15**, above, which overrides anything older.

## Not decided here

- **The Rogue in Basin Battle.** Basin Battle's fourth class is the Hunter; this page is Survival's
  Rogue only. A Basin Battle Rogue would be its own page.
- **Looks that need the client**: a ribbon trail on each blade, and a true copy of the hero's own
  body for Afterimage and the Shadow Cut. The server's looks carry everything until a client patch
  is allowed (the last pull request below). Built in entry 6 (client patch 0103); the server's
  looks stay for a client without it.

## Decided while building

What the design left open, decided in the pull request that built it:

- **The core (entry 1)**:
  - An old Rogue save whose **main hand** holds what a Rogue can't (a two-hander, an axe, a staff)
    loads with that weapon in the bags and the Rogue's start blade (a dagger) in the hand, so the
    Rogue always has a blade to fight with. The off hand is left empty (it jabs), as the table says.
  - A shield, a two-hander or an axe a Rogue **forges** at a station goes to Camp Storage with
    "Rogues can't use shields." (or the weapon's words), never into the hands.
  - The **character screen** shows a Rogue's off-hand blade (or nothing), as the world does, and
    never an old Rogue save's shield. Other classes' look there is left as it was.
  - The perfect dodge's **sound** is Vanish's (`SOUND_PERFECT_DODGE`, 3324 in `sim/cues.rs`); the
    id is to be checked when it is first filmed.
  - The **`perfect_dodge` beat** is in the add-on (`Cues.lua`, add-on 70): a 90 ms stop on the
    hero alone (a parry's is 150: a slip, not a clash), no shake, and the screen's corners
    flashed violet for 0.6 sec, the Shadow Cut's time. An older add-on ignores a beat it doesn't
    know, so it shows only the server's looks.
  - A **Shadow Cut** steps squarely behind the one who swung, or half a radian to either side of
    that (still in its back, never round its flank). If none of those is clear, within 6 yd and
    in sight, there is no step and the cut is spent: the press is the attack out of the dodge.
  - Mid-run, the attack pressed **out of a dodge** is the run's next blow (a light one), not the
    roll's attack, so the run goes on.
- **Shadow (entry 2)**:
  - A card's **tree** is its `choice` on the Rogue column (1 Shadow, 2 Venom, 3 Blades): no new
    field. Fan of Knives and Death Mark are choice 2 now. A new Rogue still knows Fan of Knives (a
    class's first cards are its whole Q and E rows), and the Dead Drop teaches Distract, Backstab
    and Fan of Knives, so Fan of Knives stays learnable by other classes until Venom gives it a
    place. Morgan the Collector teaches Cloak and Dagger beside Death Mark: the Rogue's boss
    teaches both of its ultimates.
  - **Shrouded Step is not Vanish's veil.** The veil drops a creature's fight and ends at the first
    blow; the shroud does neither. For its 1 sec, a creature (not a boss) whose foe is the Rogue
    stands and starts no blow, keeping its fight, and one at rest doesn't notice the Rogue. It
    lasts through the Rogue's own blows, which is the point: land one and step out. It is shown
    with the Stealth look (5215), as the perfect dodge's flicker. A step counts when it lands
    behind an enemy: Shadowstep and Blinding Step on a back, Ambush, the Shadow Cut and each Cloak
    and Dagger step. Bosses see through it (#75).
  - **Shadowstep uses Ambush's find** (the enemy under the reticle, else the one locked on, else
    the nearest within 20 deg of the reticle's line), not a second, 15 deg cone: one rule for
    every step to a back.
  - **Distract**: the creature's own turn is the whole tell; no look is laid on it. One winding up
    a blow finishes it first, then turns. A distracted creature at rest notices nobody, so a pack
    can be walked round. A blow "from its front" is any not from its back (its side too), by
    anyone or anything (a burn's tick from someone in front). The pouch draws no ring and warns no
    hero: it hurts nobody. Its look is 1.12's Distract (spell 1725), to be checked when first
    filmed.
  - **A boss's glance**: "mid-string" is a string begun (`pattern`) or a wind-up. A glance refused
    mid-string doesn't spend the 15 sec.
  - **Sap's "already fighting"**: a creature not at rest before the blow (chasing, striking,
    walking home). A creature notices a hero in its reach whatever way it faces, so Sap needs it
    not to see you first: out of a veil (Smoke Bomb, Vanish), in Shrouded Step's second, or while
    it is Distracted. Its look is 1.12's Sap (spell 6770) and the incapacitate's stun stars (853).
  - **Cloak and Dagger** steps only when the Rogue is free to strike (not in the ultimate's own
    0.35 sec cast, nor mid-wind-up), as the Shadow Cut. A press with no back in reach is a plain
    blow and spends no step. In a duel it steps behind heroes too. Its steps are Shrouded Steps
    only with the trait. Its 10 steps are the effect's `amount`.
  - **Truly needs a client patch**: nothing. A shadow trail between the two ends of a step would
    add to it (PR 6).
- **Venom (entry 3)**:
  - **Venom is laid on creatures only.** A duel's blows land a round trip late (`strike_fair`),
    and an 8 sec coat on a hero would sit outside the PvP budget. On a hero, Eviscerate is its 80%,
    Garrote silences 1.5 sec and bleeds; nothing coats.
  - **The off hand's coat includes the two-blade finisher**, whichever button pressed it: both
    blades strike, so the off blade is in it. One Venom a blow at most: under Rupture the Rogue's
    off hand lays 1, not 2.
  - **A coat has one owner**, the latest hero to lay it; what a friend's blow lays under Rupture is
    the Rupture's Rogue's. Its ticks are the owner's damage, and Leeching Venom heals its owner.
    Two Rogues on one enemy share one coat.
  - **The ticks** are 1.5 a stack with the fractions carried (`Cc::left`): one stack ticks 1, 2, 1,
    2. Its looks: Poison State (12881, state kit 221) on the body while it holds, Poison Cloud's
    (3815, state kit 288) in its place once ripe at 5, with a burst of the drip as it turns, and at
    every tick a drip (kit 805, `Poison_ImpactDot_Med_Base`) and its number. All to be checked when
    first filmed.
  - **Beside a dagger's own poison.** Of the daggers (#413), only Bazil's Shiv poisons: its
    `poison` is its own effect, stacks by its own rule (3 a sec a stack, 5, 6 sec) and shows as it
    did. A Shiv in the off hand lays both, side by side; Eviscerate spends only Venom and leaves the
    Shiv's poison be. Nothing in `weapon_fx.rs` changed.
  - **Eviscerate spends the Venom only when its blow lands**: one that finds nothing (slipped by a
    creature gone home, dodged by a dummy) leaves the coat on.
  - **Rupture** counts melee blows, any hero's (weapon blows and weapon arts), not missiles. Its own
    cut lays none. It shows as Rend's blood burst (kit 220) every 1.5 sec on the body, no aura.
  - **Garrote** breaks a cast from behind with 0.5 sec of reeling (the cast cut, not a stun), a
    boss's too. Its silence, on a creature, keeps it from starting a cast for 3 sec (`caster_waits`):
    a Rogue's own mark (`garroted`), so Avenger's Shield's silence is left as it was. A boss shrugs
    the silence off (it is control) but still bleeds and takes the 2 Venom. The bleed is a burn
    shown as Rend's blood burst every second (aura 772, which draws nothing, in place of a burn's
    fire). Its clip is the stab (85), as Sap's and Kidney Shot's.
  - **Feint** is a leap backward, 4 yd, 0.3 sec in the air (`lift = 3`). A creature on the Rogue
    within 8 yd turns on the nearest other hero it can see within 10 yd of it (`[rogue] feint_turn`),
    else stands; either way it takes no notice of the Rogue for 2 sec (the lull). One winding up a
    blow at the Rogue finishes it first, then looks away. A boss halves the Rogue's threat and stays.
  - **Choking Cloud's slow is its own effect, `choked`**, not Cowed: Cowed's slow is a fixed 15%,
    it also softens blows, and it is the Warrior's. A boss isn't slowed (as Cowed), but takes the
    Venom. The slow holds 1.5 sec past each second's touch (Smoke Bomb's), so it lasts from tick to
    tick. Its cloud is the Poison spell's (28158, `Poison_Cloud_Grobbulus`), to be checked when
    filmed for its size.
  - **Leeching Venom** wears Poison State's green on the Rogue and starts with Deadly Poison's
    green hands (2818).
  - **The trait is said in words** on Eviscerate and Fan of Knives ("With 3 Venom abilities on your
    keys..."), since nothing else tells a player the off hand coats.
  - **Where they're learned**: Eviscerate is a first card (a class's whole Q and E rows) and the
    Dead Drop teaches it too; Morgan's Stash teaches Garrote and Leeching Venom beside Shadowstep
    and Smoke Bomb.
  - **Truly needs a client patch**: nothing. A green sheen or drip on the off-hand blade while the
    trait is on, and a trail on Eviscerate's two blades, would add to it (PR 6).
- **Blades (entry 4)**:
  - **Flourish comes only with the trait** (3 of 5 keys on Blades' cards, a hero created as a
    Rogue), from any perfect dodge: the dodge's, Quickstep's, Blade Dance's. It makes the hands'
    blows harder and quicker (every blow of the run, Blade Flurry's cuts, the Shadow Cut), not
    weapon arts or other abilities. "A blow that lands" is any blow, a creature's or a hero's, that
    deals the Rogue damage (`strike_player`); a burn's tick or a spell doesn't take it.
  - **How Flourish reads**: Slice and Dice's look (5171) on the body while any holds, for everyone;
    on the Rogue's own screen, five violet diamonds under the reticle (`Rogue.lua`, the `SK
    flourish` line, add-on 71), one lit with a chime as each comes, all five breathing when full,
    and bursting with a crack when a blow takes them or they run out. An older add-on shows only
    the body's look. The look is to be checked when first filmed.
  - **Darting Knife's hand**: a knife (Darting or Pinning) that hits a creature carries the
    thrower's two-blade run one hand on, never to its finisher (at most the 3rd), and the next blow
    of either hand goes on from it. It holds the run 1.5 sec (`[rogue] knife_window`), not the
    run's 0.8, so two knives, a few steps in and two blows reach the finisher. A knife on a hero
    counts for nothing. Its look is 1.12's Throw (2764).
  - **Pinning Knife**: a pinned creature stands where it is and keeps its facing. While its foe is
    in front of it and in reach it still thinks and strikes, where it stands and as it faces; a
    wind-up begun keeps no turn. A boss turns at half speed in its wind-ups (`[rogue]
    pinned_boss_turn`) and moves as ever. A hero's facing is their own client's: on a hero it is
    only the knife ("Heroes ignore it"). Its look is Crippling Poison's (3409), to be checked when
    filmed; the body staying put as the Rogue circles it is the tell.
  - **Blade Flurry** is paid at once (20 stamina); its cuts are the hands' own blows (Flourish
    counts), cost nothing, and give nothing back to a dodge pressed in one's wind-up. Between two
    cuts means from the moment a cut has struck (the off hand's rule for every cut) or in a cut's
    wind-up (the dodge-feint): either way the string ends. Each cut's 1 yd step is the client's
    lunge (`lunge`), as the weapon's own lunges are. Its look as it begins is 1.12's Blade Flurry
    (13877).
  - **Hemorrhage**: while it holds, a backstab on the enemy is not remembered, so each keeps the
    whole bonus, and none counts after it ends. The wheel round on whoever stabs (`backstabbed`) is
    still rolled. Its look is Rend's blood burst every 1.5 sec, as Rupture's.
  - **Quickstep is a dodge** (`Action::Dodge`, the dodge's own evade and perfect windows) that costs
    nothing and dashes at 20 yd a second for the dodge's 0.25 sec (5 yd; with no movement key, the
    dodge's own hop back). It goes from inside the Rogue's own blow at any point (the wind-up's
    stamina back and the run ended, as the dodge-feint; later, nothing back) and from inside a
    dodge (the chain). Rooted, it refuses ("Can't move"). It has no spell look: the dash and, on a
    perfect one, the flicker are its look.
  - **Afterimage's shade** is a creature (`rogue_shade`: a Defias Footpad's body, 2338, under the
    Stealth look, 5215) set down as a totem that never pulses, 10 health, gone after 2 sec. "Every
    enemy on you" is every creature whose foe is the Rogue, however far; each is Distracted toward
    the shade for 1.5 sec (a boss glances, by Distract's rule). Vanish's puff where the Rogue was.
  - **Evasion's key is `blade_evasion`**: the old `evasion` key stays Smoke Bomb's in old saves
    (`RENAMED`). Under it the perfect window runs to its `amount` (0.28, checked to sit inside the
    dodge's evade window) and a dodge costs and needs 8 stamina, not the 18 a dodge needs to start.
    Its look is 1.12's Evasion (5277), to be checked when filmed.
  - **Adrenaline Rush** is its own effect (`adrenaline`), not the Warrior's `fury`: it makes the
    hero tireless as a fury does (blows, weapon arts and stamina abilities free) and its dodges
    free, and adds no damage. Its look is 1.12's Adrenaline Rush (13750).
  - **Blade Dance**: for a Rogue, every dodge that evades (0.04 to 0.32 sec) is perfect. The
    counter-cut is 60% of a weapon roll times the backstab's bonus (a dagger's 2.5, with the
    backstab memory's decay unless Hemorrhage holds), with the backstab's burst, sound and beat; on
    a hero (a duel), 60% with no backstab. A non-Rogue who borrows it has free dodges only (the
    perfect dodge is the class's). The body wears the Stealth look the whole 8 sec.
  - **The board's third choice** goes only to an add-on that lays it (71 and on, `.es hello`'s
    version kept with the connection): an older add-on reads any choice but 2 as the first and
    would lay Blades over Shadow, so it is sent the first two only and told to update. The Rogue's
    column draws its cards shorter, three in a row's height (`FitCard`); other columns are as they
    were.
  - **Charges**: a cooldown is shared by its charges (Fan of Knives' 8 over 2), so "3 charges of 5
    sec" is Darting Knife's 15 over 3 and "2 charges of 10 sec" Quickstep's 20 over 2: each charge
    comes back in the time the table says.
  - **Where they're learned**: Darting Knife and Blade Flurry are first cards (a class's whole Q
    and E rows) and the Dead Drop teaches them too; Morgan's Stash teaches Quickstep and Evasion;
    Morgan the Collector teaches Blade Dance beside Death Mark and Cloak and Dagger.
  - **Truly needs a client patch**: nothing. A copy of the hero's own body and gear for the shade,
    a blade trail on Blade Flurry's cuts, and a knife left standing in a pinned enemy would add to
    it (PR 6).

- **The pairs (entry 5)**:
  - **Which hands make which pair**: families by the item subclass (15 dagger, 7 sword, 4 mace, 13
    fist), in either hand. "A mace and any blade" is a mace with anything a Rogue holds, a second
    mace or a fist too: no pair should leave a mace's hands with nothing. **A fist with a dagger or
    a sword is no pair**: each hand keeps its own mechanic and the run is the plain 4. Why: the
    table names none, and a sixth rule would be a guess. An empty off hand (it jabs) is no pair.
    Only a hero created as a Rogue has one; a Warrior's two daggers are as they were.
  - **Twin Fangs**: a Shadow Cut's backstab is not remembered **and** takes its whole bonus
    whatever the creature remembers ("its backstab stays whole"). The wheel round on whoever stabs
    a boss is still rolled, as under Hemorrhage.
  - **Blade and Shiv**: the finisher, whichever hand presses it, reaches as far as the longer hand
    (the sword's 3.0 yd) and stabs with the better hand's bonus (the dagger's 2.5). A Sneed's
    Machete keeps its own "no backstab" in its own hand.
  - **Crossed Swords**: the cross-cut is the finisher's arc opened to `[rogue] cross_arc` (150) and
    no cap on how many it strikes; the three blows before it strike one, as ever.
  - **Breaker's Pair, and posture by hand**: a Rogue's blow counts toward posture by **the hand
    that struck it** (a mace's 2 from either hand), not the main hand's as for everyone else, so the
    mace's ×2 stays on its own blows wherever it is. Breaker's Pair's finisher counts
    `[rogue] breaker_heft` (3) whichever hand pressed it, not 3 × 2. Other classes are untouched (a
    test pins a Warrior's off-hand mace at the main hand's weight).
  - **Iron Knuckles' wheel is a Distract** toward a point 5 yd beyond the creature, away from the
    Rogue, for `[rogue] wheel_secs` (1.0): the creature turns at once (its body is the tell, as
    Distract's), starts no blow, a backstab leaves it be and a blow from its front ends it; then it
    turns back. A champion (not a boss) is wheeled; a boss never, its posture counts
    `knuckle_boss_heft` (2) instead. The run of 6 also sets how far a Darting Knife carries it (the
    5th) and where Blade Flurry starts so its 4 cuts still end in the finisher.
  - **Clip 16, not 35**: the page said AttackUnarmed is 35; in 1.12's AnimationData 35 is
    NPCGoodbye and AttackUnarmed is 16 (tools/clip-census). The fists' set is the dwarf's
    AttackUnarmed (take 0) for its jabs, the dagger's overhead for the heavy, and the finisher plays
    16 for both fists (`[rogue] knuckle_clip`).
  - **The fist weapons**: a `fist` family with its own set (within 5% of the one-hander's damage a
    second, as every family's), a 0.15 sec wind-up, and `backstab = 2.0` as its family identity
    (the identity test needs one; "the fists' job is making backs"). Two on the rack for anyone,
    each with a mechanic no other weapon has, both for the dodge: **Footpad's Knuckles** (display
    15720, `hand_1h_knuckle_a_01`) gives 3 stamina back a hit (`wind`); **Cutpurse's Claw** (display
    4687, `hand_1h_claw_a_01`) does 50% more within 1 sec of a dodge that slipped a blow (`slip`,
    any dodge that evades, not only a perfect one). The off hand shows the same model as the main
    hand: 1.12 keeps left-hand fist models as their own displays, to be checked when filmed.
  - **How a player learns the pair**: equipping a hand at the rack tells a Rogue its pair in one
    line ("Your pair is Iron Knuckles. ..."), built from the numbers (`pair_words`). No add-on
    change: the server's chat line carries it.
  - **Truly needs a client patch**: nothing. A ribbon on both blades during the finisher, and a
    left-hand fist model in the off hand, would add to it (PR 6).

- **Looks the server can't make (entry 6, client patch 0103)**:
  - **One patch, the thirty-second round**: `HasActionCombat()` 32, the add-on's hello `ac32` and
    the server's `CLIENT_NEWEST` 32 (`ROGUE_LOOKS_LEVEL`); add-on `VERSION` and `ADDON_NEWEST` 72.
    0103 was the next free number (0100 to 0102 on `main`, no open pull request claiming it).
  - **The blade's ribbon is 1.12's own weapon trail, not a second ribbon.** The page named patch
    0030's missile ribbons as the model, but a blade already has its ribbon: the swing trail the
    client draws from each weapon's own tip and base (`weapon_trail`, one a hand), which
    `PlayUnitAnimation` armed on both hands in one colour. The patch lets that trail name a colour
    a hand, `"<main AARRGGBB>/<off AARRGGBB>"`, `-` for a hand that trails nothing. Why: a second
    ribbon on a blade would draw twice, and the trail already follows the blade's own shape.
  - **The trail rides the blow's own `AN` line**, in its trail field, not a line of its own: the
    trail must start with the clip, and a second line could arrive after it. Only an `ac32` client
    is sent the two-colour field; an older one gets the one colour as before (both blades). The
    main hand's blow trails the right blade alone in the blow's own colour (the light blow's pale
    `50F2EEE4`, a heavy's ember); the off hand's the left blade alone in a deeper steel
    (`5A7C86B4`, half the pale's brightness, so the hands read apart in the rhythm); the two-blade
    finisher both, each its own colour at a heavy blow's alpha (`78F2EEE4` / `787C86B4`), whichever
    button pressed it. Only a hero created as a Rogue: a Warrior's two blades trail as before.
  - **The one new line is `AI`, the afterimage's start and stop** (`ui.rs`'s `afterimage_line`):
    `AI <guid> <secs> <alpha> <RRGGBB> [<x> <y> <z> <stand-in>]`, and `AI <guid> 0` to take the
    hero's copies away. It starts on the server's own triggers, a sim event
    (`Event::Afterimage`): a perfect dodge, for the flicker's time (`[rogue] flicker_secs`, 0.3: no
    new number), and Afterimage's shade, for its `shade` (2 sec). It stops as the shade goes for
    any reason (its time out, struck down, a second Afterimage, its Rogue gone).
  - **The copy is patch 0027's afterimage** (the frozen copy of a unit's own body and what it
    holds that Bladestorm's cuts leave), which the add-on can now ask for: `PlayUnitAfterimage`.
    The perfect dodge's is left where the body is as the line arrives (a moment into the dodge:
    the blow fell then), pale (`C8C8FF`) at 0.55, fading the whole way. The shade's stands at the
    shade's spot (its rig moved along the ground, the body's own height kept), Shadow's violet
    (`A08CDC`) at 0.6, keeping most of its alpha until near its end (the appear ramp's cubic, run
    downwards), and the server's Defias body there is hidden on that client while it stands (its
    stand-in). The enemies still turn to the server's shade, at the same spot, so what the player
    sees and what the enemies turn to agree. A copy whose stand-in goes first goes with it, even
    if the stop line is lost.
  - **The copy's pose is the body's as the line arrives**: for the shade, the Rogue at the start
    of the Quickstep's dash; for a perfect dodge, mid-dodge. To be checked when filmed.
  - **The flicker and the shade's body stay** for every client: the flicker plays on the body as
    before (the copy is left behind it), and a client without the patch sees the Defias shade.
  - **Our own blows that the add-on plays before the server's line** (a left click): once the
    server's own blows have come a hand each, a guessed blow trails the main hand alone, and a
    finisher the guess got wrong (the same clip, both blades) is played again with both. An older
    add-on on the new client trails both blades on its guesses, as before.
  - **Not done here**: the Shadow Cut's step leaves no copy of its own (the perfect dodge's copy is
    left the same moment); the off-hand fist's model is as it was (nothing in the patch sends a
    left-hand display).
  - **The client's own guards**: `PlayUnitAfterimage` is on the list of globals past 1.12
    (`reference_surface.rs`) and in the VM surface golden, both in the patch.

## Build plan

Six pull requests, in order. Each is handed to one subagent with only this page to go on, so each
entry says all it needs. The sections above are the design; an entry names the ones it builds.

**Standing rules for every one**: read `AGENTS.md` and `docs/writing-style.md` first. Branch from
the latest `origin/main`. Every number in `content/` carries its reason in a comment beside it.
Every word a player reads follows the style guide (stats: one effect a sentence, numbers as
numerals, "sec" and "yd"; Survival says "ability", not "card"). Each new ability gets a test that
its words say what its data does (as `kidney_shots_words_say_what_the_data_does`). Tells read with
floor markers off; a floor marker is only ever an optional extra behind the add-on's setting.
Keep Hunter behaviour exactly as it is. Keep old saves loading (`tests_old_saves.rs` and
`an_old_save_with_the_rogues_old_cards_loads_with_their_successors` stay green). Checks:
`cargo build --release -p htl-server`, `cargo test --release` (report the counts; rerun a
real-time end-to-end failure alone), `scripts/check-addon.sh` if the add-on changed (then bump its
`VERSION` and the server's `ADDON_NEWEST` in `crates/htl-server/src/survival/ui.rs` together),
`scripts/check-pr.sh`. Add a visual-check scenario file for how it should be filmed later
(`tools/visual-check/<name>.txt`, listed in `docs/visual-check.md`), and don't run it. In the
pull request: what changes for the player, the rules with their numbers, what was tested, and
what nobody has seen in the client yet (the `needs playtest` label). Update
`docs/elwynn-survival.md`'s Rogue line and this page's "Nothing here is built yet" as parts land.

### 1. The core: two blades and the perfect dodge

Branch `claude/rogue-core-footwork`. Builds: [What the Rogue has today that breaks it](#what-the-rogue-has-today-that-breaks-it-and-how-each-changes)
and [The core](#the-core-footwork-and-two-blades). The guard key is the off-hand blade (decided).

- **Files**:
  - `content/survival.toml`: `[dodge]` `perfect_to = 0.18`, `perfect_refund = 20.0`,
    `perfect_posture = 0.12`; `[valor]` `perfect = 10.0`; a new `[rogue]` table (`cut_secs = 0.6`,
    `cut_reach = 6.0`, `off_cancel_after = 0.0`, the weapon families allowed); the Rogue's start
    gear (two `dagger`, no shield) beside `[start]`.
  - `crates/htl-survival/src/content.rs`: those fields, with defaults so an older content file loads.
  - `crates/htl-survival/src/sim/combat.rs`: the perfect branch in `strike_player` (a creature's
    blow) and in the hero-on-hero dodge check (`dodged`, near "dodged in a dodge's"); the Shadow
    Cut (reuse Ambush's step-behind spot); the dodge-feint in `Input::Dodge` (today a dodge can't
    start in a wind-up); the off hand's cancel in `cancelable`; the run kept over a dodge and the
    Shadow Cut counted as a hand in `start_swing_with`; the empty-off-hand jab.
  - `crates/htl-survival/src/sim/mod.rs`: the Player fields (when the Shadow Cut is open, whose
    back), constants.
  - `crates/htl-survival/src/sim/abilities.rs`: Retaliation and the beta Riposte refuse on a Rogue
    (the `needs_shield` refusal is the pattern); a `needs_two_blades` flag beside `needs_shield`
    for the trees' cards.
  - Equipping (`equip_from_chest` in `sim/world.rs`, and wherever the bags equip): a Rogue can't
    equip a shield, a two-hander, a staff or an axe, with the words in the table above.
  - `crates/htl-survival/src/save.rs`: a Rogue save holding a shield or a two-hander loads with it
    in the bags.
  - `crates/htl-survival/src/sim/heroes.rs` (`fresh_save`): the Rogue's start gear.
  - `crates/htl-survival/src/sim/cues.rs` and `addon/ElwynnSurvival/Cues.lua`: the `perfect_dodge`
    beat, if an older add-on ignores a beat it doesn't know (check); otherwise use `readied` and
    leave the add-on alone.
  - New `crates/htl-survival/src/tests_rogue_core.rs` (listed in `lib.rs` with the other test
    modules).
- **Tests**:
  `a_rogue_starts_with_two_daggers_and_no_shield`,
  `a_rogue_cant_equip_a_shield_a_two_hander_or_an_axe`,
  `a_rogues_old_save_with_a_shield_loads_with_it_in_the_bags`,
  `a_rogues_right_click_swings_the_off_hand_and_never_blocks_or_parries`,
  `a_rogue_with_an_empty_off_hand_jabs_on_right_click`,
  `a_dodge_whose_blow_lands_by_0_18_sec_is_perfect_and_a_later_one_only_evades`,
  `a_perfect_dodge_gives_20_stamina_and_10_valor_and_a_plain_one_10_and_8`,
  `an_attack_within_0_6_sec_of_a_perfect_dodge_steps_behind_the_striker_and_backstabs`,
  `no_shadow_cut_when_the_spot_behind_is_in_a_wall`,
  `a_perfect_dodge_fills_12_percent_of_a_bosss_posture_and_never_staggers_it`,
  `a_perfect_dodge_of_a_heros_blow_pays_too`,
  `a_perfect_dodge_pressed_within_the_round_trip_still_counts`,
  `a_dodge_in_your_own_wind_up_calls_the_blow_off_and_gives_its_stamina_back`,
  `a_dodge_cuts_an_off_hand_blow_short_at_once_and_a_main_hand_blow_only_after_45_percent`,
  `a_dodge_between_blows_keeps_the_run_and_a_shadow_cut_counts_as_the_next_hand`,
  `retaliation_and_riposte_refuse_on_a_rogue`,
  `a_warrior_with_two_blades_has_no_perfect_dodge_payoff` (the footwork is the class's),
  `the_hunters_call_still_primes_on_any_dodge_through_a_blow`.
- **Shown with the server's looks**: the off hand's `AttackOff` (87) and the finisher's two blades
  (17); the Stealth aura (5215) for 0.3 sec as the flicker; Vanish's puff (spell 1856) where the
  blow fell and at both ends of the Shadow Cut's step; the backstab's blood burst (kit 220), sound
  1152 and beat; the dodge's own tumble. Scenario `rogue-perfect-dodge.txt`: a Rogue and a grunt
  (`.es target grunt 4`), a dodge late into its overhead, then an attack.
- **Truly needs a client patch**: nothing. A blade trail on the two weapons and an afterimage of
  the hero's own body would add to it (PR 6).

### 2. Shadow

Branch `claude/rogue-shadow-tree`. Builds: [Shadow](#shadow-get-behind-and-be-gone), the
`distract` effect, and the tree-trait rule (3 of 5 keys hold the tree's cards, the Rogue class
only).

- **Board**: Shadow is choice 1 on every row: Distract (Q), Backstab (E), Shadowstep (1), Smoke
  Bomb (2), Cloak and Dagger (3). Fan of Knives (Q) and Death Mark (3) move to choice 2 (they are
  Venom's). A fresh Rogue's keys come from choice 1 (`fresh_abilities`), and its first cards
  (`[start] cards = ["ranged", "close"]`) become Distract and Backstab: check `first_cards` and
  the rogue's shrine (`shrine_rogue` teaches `fan_of_knives` and `backstab`: make it teach
  `distract` and `backstab`, and keep `fan_of_knives` owned for saves that have it).
- **Files**: `content/survival.toml` (`distract`, `sap`, `cloak_and_dagger` abilities; Shadowstep's
  and Blinding Step's landing; the shrine; the `[rogue]` trait numbers); `crates/htl-survival/src/content_abilities.rs`
  (the `distract` effect kind in the list of effects; the tree a card belongs to, or derive it
  from `school = "rogue"` and `choice`); `crates/htl-survival/src/sim/cc.rs` or `sim/moves.rs`
  (the `distract` control and the boss glance); `sim/mobs.rs` (a distracted creature faces its
  point and starts no blow); `sim/abilities.rs` (Shadowstep behind the foe under the reticle,
  reusing Ambush's find; Cloak and Dagger's step on each attack press; the 3-of-5 trait check;
  Shrouded Step's 1 sec veil after a step behind); new `crates/htl-survival/src/tests_rogue_shadow.rs`.
- **Tests**: `distract_turns_every_enemy_near_its_landing_to_face_it_for_2_5_sec`,
  `a_distracted_enemy_starts_no_blow_and_one_from_its_front_ends_it`,
  `a_backstab_does_not_end_a_distraction`, `a_boss_glances_for_0_8_sec_once_every_15_sec_never_mid_string`,
  `distract_does_nothing_to_a_hero`, `sap_from_behind_incapacitates_6_sec_and_from_the_front_is_a_cut`,
  `sap_never_takes_a_boss_or_a_creature_already_fighting`,
  `shadowstep_lands_behind_the_enemy_under_the_reticle_else_at_the_point`,
  `shadowstep_still_steps_back_within_3_sec`,
  `cloak_and_dagger_steps_behind_the_nearest_enemy_before_each_blow_for_6_sec`,
  `cloak_and_dagger_prefers_an_enemy_not_yet_struck`,
  `shrouded_step_hides_the_rogue_1_sec_after_a_step_behind_and_a_boss_sees_through_it`,
  `the_shadow_trait_is_on_with_3_shadow_cards_on_the_keys_and_off_with_2`,
  `a_non_rogue_with_3_shadow_cards_has_no_trait`, `distract_and_sap_and_cloak_and_daggers_words_say_what_the_data_does`,
  and the PvP budget test (`tests_classes.rs`) passing with the new cards.
- **Shown with the server's looks**: a lobbed missile for the pouch (Lava Burst's lob, another
  missile look) and a spell visual where it lands; the creatures turning (their own bodies);
  Blinding Step's incapacitate look for Sap; Vanish's puff at each step; the veil's fade.
  Scenario `rogue-shadow.txt`: Distract between two grunts, a Backstab on the one that turned,
  Cloak and Dagger through three.
- **Truly needs a client patch**: nothing.

### 3. Venom

Branch `claude/rogue-venom-tree`. Builds: [Venom](#venom-build-it-on-cash-it-in) and the `venom`
effect. Needs PR 2 merged (the choice 2 move and the trait rule).

- **Board**: choice 2 on every row: Fan of Knives (Q), Eviscerate (E), Garrote (1), Leeching Venom
  (2), Death Mark (3).
- **Files**: `content/survival.toml` (`eviscerate`, `rupture`, `garrote`, `feint`,
  `leeching_venom`, `choking_cloud`; Fan of Knives' knives lay Venom with the trait); `content_abilities.rs`
  (the `venom` effect; Eviscerate's per-Venom damage; `needs_two_blades` on Eviscerate);
  `sim/weapon_fx.rs` (Venom beside the Shiv's poison stacks, apart from them; the lull Feint
  reuses); `sim/combat.rs` (the off hand lays Venom with the trait; Garrote's from-behind rule
  through `from_behind`); `sim/tactics.rs` (Feint: the lull, the turn to another hero, a boss's
  threat halved); new `tests_rogue_venom.rs`.
- **Tests**: `each_off_hand_hit_lays_1_venom_and_a_main_hand_hit_none`,
  `venom_stacks_to_5_for_8_sec_and_ticks_1_5_a_stack`, `venom_shows_ripe_at_5`,
  `eviscerate_deals_80_percent_and_35_per_venom_and_uses_it_up`,
  `eviscerate_requires_two_blades`, `rupture_lets_every_heros_blow_lay_venom_for_8_sec`,
  `garrote_from_behind_breaks_the_cast_silences_and_bleeds`, `garrote_from_the_front_is_only_a_cut`,
  `garrote_on_a_boss_breaks_only_the_cast`, `feint_hops_back_and_enemies_on_you_lose_interest_for_2_sec`,
  `a_feinted_enemy_with_another_hero_near_turns_to_them`, `leeching_venom_heals_what_your_venom_deals_for_6_sec`,
  `choking_cloud_slows_wind_ups_40_percent_and_lays_venom_each_sec`,
  `fan_of_knives_lays_venom_only_with_the_trait`, the words tests, and the PvP budget.
- **Shown with the server's looks**: the poison aura (1978, Fan of Knives') for Venom and a deeper
  poison aura at 5; Rend's blood burst for Rupture and Garrote's bleed; the silence look; the
  cloud as a lingering spell on the ground (as Smoke Bomb's `linger_spell`); enemies turning away
  for Feint. Scenario `rogue-venom.txt`: a string on a grunt to 5 Venom, Eviscerate; Garrote on a
  casting kobold geomancer from behind.
- **Truly needs a client patch**: nothing.

### 4. Blades, and the board's third choice

Branch `claude/rogue-blades-tree`. Builds: [Blades](#blades-the-dance), the `pinned` and
`flourish` effects, and the third choice. Needs PR 2 (the `distract` effect for Afterimage).

- **Board**: choice 3 on every row: Darting Knife (Q), Blade Flurry (E), Quickstep (1), Evasion
  (2), Blade Dance (3). The content check allows choice 3 for the Rogue column only and one more
  card in each row (`content.rs`: `choice` 1 or 2, "at most" `SCHOOLS.len() * 2` a row). The
  server's board line already sends the choice (`ui.rs`); the add-on's board
  (`addon/ElwynnSurvival/ElwynnSurvival.lua`, "Each row holds a class's two choices") lays a third
  card under the second in the Rogue's column. An older add-on reads any choice but 2 as 1, so the
  Blades cards would cover Shadow's: bump `VERSION` and `ADDON_NEWEST` together, so an older add-on
  is told to update.
- **Files**: `content/survival.toml` (`darting_knife`, `pinning_knife`, `blade_flurry`,
  `hemorrhage`, `quickstep`, `afterimage`, `evasion`, `adrenaline_rush`, `blade_dance`;
  `evasion` is a new key: check the old `evasion` key in `save.rs`'s `RENAMED` (it maps to
  `smoke_bomb`) and give the new card a different key, such as `blade_evasion`, so old saves
  still load as they do); `content.rs` and `content_abilities.rs` (choice 3; `pinned`,
  `flourish`); `sim/combat.rs` (Flourish on a perfect dodge and lost on a hit; Blade Flurry's
  string and its dodge cancel; Quickstep as a dodge that cuts your own blow; Blade Dance's every
  evasion as perfect and its counter-cut; Darting Knife counted as a hand); `sim/mobs.rs`
  (`pinned`: no turn; a boss's half `turn_rate`); `sim/combat.rs` `backstab_bonus` (Hemorrhage);
  Afterimage's shade as a short-lived creature body (as totems are placed) with Distract on the
  enemies on you; `ui.rs` and the add-on board; new `tests_rogue_blades.rs`.
- **Tests**: `a_perfect_dodge_gives_a_flourish_up_to_5_for_6_sec`,
  `each_flourish_makes_blows_6_percent_harder_and_4_percent_quicker`,
  `a_blow_landing_on_the_rogue_takes_every_flourish`, `a_darting_knife_that_strikes_counts_as_a_hand`,
  `pinning_knife_stops_an_enemy_turning_for_3_sec_and_a_boss_turns_at_half_speed`,
  `blade_flurry_cuts_4_times_0_3_sec_apart_and_ends_in_the_finisher`,
  `a_dodge_cancels_blade_flurry_between_cuts`, `hemorrhage_stops_backstabs_on_it_losing_bonus_for_8_sec`,
  `quickstep_has_2_charges_costs_no_stamina_and_evades_like_a_dodge`,
  `quickstep_cancels_your_own_wind_up_and_gives_its_stamina_back`,
  `afterimage_leaves_a_shade_and_the_enemies_on_you_turn_to_it`,
  `evasion_widens_the_perfect_window_to_0_28_sec_and_a_dodge_costs_8`,
  `adrenaline_rush_makes_blows_and_dodges_free_for_6_sec`,
  `blade_dance_counts_every_evading_dodge_as_perfect_and_cuts_the_striker`,
  `the_board_takes_a_third_rogue_choice_and_no_other_class_a_third`,
  `an_old_save_with_evasion_still_loads_as_smoke_bomb`, the words tests, the PvP budget, and
  `scripts/check-addon.sh`.
- **Shown with the server's looks**: a missile look for the knives and an impact glint; the run's
  clips; the shade as a creature body (a Defias Cutpurse's display) under the Stealth aura;
  Evasion's own 1.12 spell look as a lasting aura; the flicker on every evasion during Blade
  Dance. Scenario `rogue-blades.txt`: Quickstep and dodges through a Darting grunt's string to 5
  Flourish, Blade Flurry, Blade Dance against two grunts.
- **Truly needs a client patch**: nothing. A copy of the hero's own body and gear for the shade
  would be better than a Defias's (PR 6).

### 5. The dual-wield pairs and the fist weapons

Branch `claude/rogue-weapon-pairs`. Builds: [Weapon paths](#weapon-paths-the-dual-wield-pairs).
Needs PR 1.

- **Files**: `crates/htl-survival/src/sim/weapon_fx.rs` (the pair's rule, judged on both hands'
  families when the finisher or a Shadow Cut lands); `sim/combat.rs` (the finisher's reach and
  backstab from the better hand for Blade and Shiv; the cross-cut arc for Crossed Swords; the
  posture count for Breaker's Pair; Iron Knuckles' run of 6 and the wheel round); `content/survival.toml`
  (a `fist` family, a `[[moveset]]` `fist` with `AttackUnarmed`, clip 35, wind-up 0.15; two fist
  weapons as `[[weapon]]` and items, on the rack where `dagger` is offered, each with its own
  mechanic per the identity rule in `docs/weapon-identities.md`); `docs/weapon-identities.md`
  (the new rows; its test keeps the table and the content in step); new `tests_rogue_pairs.rs`;
  an ignored test that the fist weapons' display ids exist in the client's data (`WOW_DATA`).
- **Tests**: `twin_fangs_shadow_cut_adds_nothing_to_the_backstab_memory`,
  `blade_and_shiv_finisher_reaches_3_yd_and_backstabs_for_2_5`,
  `crossed_swords_finisher_strikes_every_enemy_in_150_degrees`,
  `breakers_pair_finisher_counts_3_times_toward_posture`,
  `iron_knuckles_run_is_6_and_its_finisher_turns_a_non_boss_away`,
  `iron_knuckles_finisher_on_a_boss_counts_twice_toward_posture`,
  `a_pairs_rule_needs_both_hands` (a dagger and an empty hand is no pair),
  `each_weapons_own_mechanic_still_works_in_either_hand`, and the weapon identity tests.
- **Shown with the server's looks**: both weapons in hand; the finisher's clip 17 (fists: clip 35);
  a wheeled enemy's own turn. Scenario `rogue-pairs.txt`: one finisher per pair on a grunt.
- **Truly needs a client patch**: nothing.

### 6. Looks the server can't make (optional, client patch)

Branch `claude/rogue-client-trails`. Only when the coordinator's prompt allows a client patch;
otherwise skip it. Builds the [Not decided here](#not-decided-here) looks.

- **What**: a short ribbon trail on each blade during a Rogue's blows (pale on the main hand, a
  darker shade on the off hand, both on the finisher), and an afterimage: a see-through copy of
  the hero's own model and gear left for 0.3 sec on a perfect dodge and for 2 sec as Afterimage's
  shade. Patch 0030's missile ribbons are the model for the trail.
- **Files**: one new patch with the next free number in `patches/benilla/` (0103, built; never
  edit another), a server line to start and stop it (`ui.rs`'s line formats), and the
  server's fallback kept for a client without the patch.
- **Tests**: the server line's format test; the whole patch series applied on a clean benilla
  checkout and the client built (`scripts/play.sh` does it). Ready only once the whole series has
  built (`AGENTS.md`): a patch that doesn't build stops the owner's game from starting.
- **Shown with the server's looks**: n/a; this is the part that needs the client. Scenario
  `rogue-trails.txt` films it (and, on an older client, the fallback beside it).
