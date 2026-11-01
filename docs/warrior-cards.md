# Warrior: every level-up card

_Generated from `content/classes/warrior.toml` by `tools/card_catalog.py`. Edit the content file, not this page._

- **18 skills**, up to 6 on your bar at once.
- **286 skill upgrades** (about 15 per skill). A skill takes **10** in a run, so no run sees a whole deck.
- Every upgrade also makes its skill hit 6% harder and costs 7% less rage, on top of what the card says.
- **40 class talents** (offered whatever skills you hold; some come back for more ranks).
- **45 combinations** (offered only while you hold both skills).
- **3 weapon paths** (one a run, at random, as a Legendary card). It teaches nothing by itself: it opens its own skills to the skill cards (on keys of their own, so a full bar never shuts them out) and its own **30 path cards**.
- **Rarity means something**: Common cards are numbers, Uncommon change how you play, Rare light a key, reset or cast, and each skill's **Epic** capstone comes only after 5 of its upgrades (a path's after 4 of its cards). Skills themselves are not rolled for rarity.

How to read the upgrade tables: **Rank-up xN** can be taken N times. **Choice: x** means only one upgrade with that choice can be taken for the skill. **After: Y** is offered once Y is taken. **Lights up a key** makes a skill's next cast ready at once and empowered (its key glows until you press it). What each skill is for: [warrior-roles.md](warrior-roles.md).

## Skills and their upgrades

### Heroic Strike

_Arms · bolt · 2.5 sec cooldown · 15 rage · can be your first skill_

Off the global cooldown: weave it between your other skills. Hits 3% harder for every 10 rage you hold.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Honed Strike | Common | Rank-up x2 | Deals 25% more damage. Can be taken twice. |
| Improved Heroic Strike | Common | Rank-up x2 | Costs 5 less rage: 10, then 5. Can be taken twice. |
| Rampant Rage | Common | - | Hits 3% harder for every 10 rage you hold (6% in all). |
| Cleave | Uncommon | Choice: form | Becomes a Cleave: also cuts 2 enemies next to your target. |
| Sweeping Cleave | Rare | After: Cleave | Cleave cuts every enemy around your target (up to 6 more). |
| Slam | Uncommon | Choice: form | Becomes a heavy Slam: 80% more damage, 1.5 sec longer cooldown. |
| Wrecking Slam | Rare | After: Slam, Triggered | Every Slam shakes the ground: 0.6 times your hit to every enemy within 6 yd of the target, stunned 1 sec. |
| Heroic Throw | Uncommon | Choice: form | Becomes a thrown blade: reaches 25 yd. |
| Ricochet Blade | Uncommon | After: Heroic Throw | The thrown blade bounces to 3 more enemies. |
| Overflow | Uncommon | Triggered | Cast with 70 rage or more, it gives back 15: pour it out without running dry. |
| Riposte | Uncommon | Lights up a key | A critical strike lights up the next Heroic Strike: ready at once, free, and 50% more damage. |
| Battle Trance | Rare | - | 20% more critical strike chance, and every crit refunds all 15 rage. |
| Weave | Uncommon | Triggered | Each Heroic Strike makes your next skills 8% stronger for 5 sec, stacking 3 times: weave it between them. |
| Overpower | Uncommon | Triggered | On a sundered enemy it strikes twice: a second blow for 0.6 times your hit. |
| Tempo | Uncommon | Triggered | Each hit takes 1 sec off your other cooldowns. |
| Heroic Fury | Epic | Capstone: after 5 upgrades | Every Heroic Strike strikes twice. |

### Rend

_Arms · bolt · 4 sec cooldown · 10 rage · can be your first skill_

The target Bleeds: Heroic Strike deals more to it.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Improved Rend | Common | Rank-up x2 | Bleeds 40% more. Can be taken twice. |
| Effortless Rend | Common | - | Costs no rage. |
| Open Wounds | Common | - | The cut itself hits for 0.6 times your hit more. |
| Fast Bleed | Uncommon | Choice: pace | Bleeds just as much in 4 sec instead of 12: big, quick ticks. |
| Lingering Wound | Uncommon | Choice: pace | Bleeds 50% more, over 18 sec. |
| Tearing Strike | Uncommon | - | Rends 2 more enemies next to your target. |
| Spray of Blood | Rare | After: Tearing Strike, Triggered | A rended enemy's death sprays blood: 0.8 times your hit to every enemy within 8 yd of you. |
| Deep Cuts | Uncommon | - | The bleed stacks up to 3 times. |
| Ritual Wound | Uncommon | After: Deep Cuts, Triggered | 30% chance to Rend again at once, for free (another stack). |
| Hemorrhage | Rare | - | When a rended enemy dies, the bleed jumps to 2 enemies nearby. |
| Hemophilia | Rare | After: Hemorrhage | Hemorrhage jumps to 4 enemies instead. |
| Blood Scent | Common | Triggered | A rended enemy's death gives you 15 rage and heals you 4%. |
| Rupture | Uncommon | Triggered | Rending an enemy that is already bleeding tears it open: 1 times your hit at once. |
| Gore | Uncommon | Triggered | Rending an enemy that is already bleeding takes 1.5 sec off your other cooldowns. |
| Thirsting Blade | Common | - | Heals you 3% of your max health for each enemy it rends. |
| Crimson Tempest | Epic | Capstone: after 5 upgrades | Rend cuts every enemy around your target (up to 8). |

### Thunder Clap

_Protection · nova · 6 sec cooldown · 20 rage · can be your first skill_

Everything it hits attacks 30% slower for 10 sec (and moves slower): packs hit you far softer.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Thunderous Power | Common | Rank-up x2 | Deals 30% more damage. Can be taken twice. |
| Improved Thunder Clap | Common | Rank-up x2 | Reaches 3 yd further. Can be taken twice. |
| Echoing Thunder | Common | Rank-up x2 | Cooldown 1.5 sec shorter: every 4.5, then every 3 sec. Can be taken twice. |
| Heavy Thunder | Uncommon | - | Their attacks slow by 45% instead of 30%. |
| Rolling Thunder | Uncommon | Choice: storm, Triggered | 35% chance to clap a second time at once, for free. |
| Aftershock | Uncommon | Choice: storm, Triggered | Each clap sends a second, wider shockwave: everything within 16 yd is slowed 50% and attacks 30% slower. |
| Gathering Storm | Uncommon | Choice: storm | Deals 10% more for every enemy caught after the first. |
| Demoralizing Clap | Uncommon | - | Everything it hits also deals 25% less damage for 10 sec: watch their hits shrink. |
| Concussion Clap | Rare | - | 30% chance to stun each enemy hit for 2 sec. |
| Shockwave | Rare | After: Concussion Clap | Concussion Clap stuns half of them instead. |
| Thunder Ward | Uncommon | - | Shields you for 3% of your max health for each enemy hit. |
| Rage of Thunder | Common | Triggered | Gain 3 rage for each enemy hit. |
| Braced by Thunder | Uncommon | Triggered | Clapping 4 or more enemies: you take 20% less damage for 6 sec. |
| Ancient's Thunder | Rare | Triggered | Within 25 yd of the Ancient each clap heals the Ancient 1% of its health. |
| Shaking Ground | Common | - | Slows their feet by 60% instead of 30%, and 4 sec longer. |
| Earthquake | Epic | Capstone: after 5 upgrades | Every clap stuns everything it hits for 1.5 sec. |

### Whirlwind

_Fury · nova · 8 sec cooldown · 25 rage · can be your first skill_

The more you catch, the harder it hits: 5% more for every enemy after the first.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Honed Spin | Common | Rank-up x2 | Deals 25% more damage. Can be taken twice. |
| Improved Whirlwind | Common | Rank-up x2 | Cooldown 2 sec shorter: every 6, then every 4 sec. Can be taken twice. |
| Wide Spin | Common | - | Reaches 3 yd further. |
| Bloodbath | Uncommon | - | Heals you 2% of your max health for each enemy hit. |
| Blood Spin | Uncommon | After: Bloodbath | Bloodbath heals 1.5% more for each enemy hit. |
| Cyclone | Rare | Choice: spin | 35% chance to spin a second time at once, for free. |
| Spin Cycle | Rare | After: Cyclone | Cyclone spins again 15% more often. |
| Tornado | Uncommon | Choice: spin | Deals 12% more for every enemy caught after the first (17% in all). |
| Whirling Wall | Uncommon | Choice: spin | Shields you for 3% of your max health for each enemy hit. |
| Fury of Blades | Common | Triggered | Gain 2 rage for each enemy hit. |
| Cutting Wind | Uncommon | Triggered | Each kill takes 1.5 sec off your other cooldowns. |
| Spinning Advance | Common | Triggered | After each spin you run 30% faster for 4 sec: on to the next pack. |
| Blade Flurry | Common | Triggered | After each spin your swings come 30% faster for 6 sec. |
| Endless Spin | Rare | Triggered | Each critical hit has a 15% chance to finish its cooldown. |
| Eye of the Storm | Uncommon | Lights up a key | Spinning through 3 or more enemies has a 30% chance to light up the next spin: ready at once, 50% more damage. |
| Bladestorm | Epic | Capstone: after 5 upgrades, Triggered | Every spin goes off three times. |

### Sunder Armor

_Protection · bolt · 1.5 sec cooldown · 10 rage_

Elites shrug off 10% of every physical blow, bosses 20%: each stack strips 4% of that armor for everyone, and makes your physical damage 5% stronger against it (5 stacks).

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Heavy Sunder | Common | Rank-up x2 | Deals 50% more damage. Can be taken twice. |
| Effortless Sunder | Common | - | Costs no rage. |
| Improved Sunder Armor | Uncommon | - | Applies 2 stacks: armor gone in 3 Sunders. |
| Sundering Wave | Uncommon | Choice: aim | Also sunders 2 more enemies next to your target. |
| Splintering | Uncommon | Choice: aim | Splinters fly: every enemy within 8 yd of the target is sundered too (and takes 40% of the hit). |
| Devastate | Rare | Choice: aim, Triggered | On an enemy already sundered it also strikes for 0.8 times your hit. |
| Armor Crush | Uncommon | - | Deals 25% more for each Sunder Armor stack already on the target. |
| Weak Point | Rare | After: Armor Crush | Armor Crush: 15% more per stack. |
| Armor Shatter | Common | Triggered | 25% chance to tear 2 extra stacks off an enemy already sundered. |
| Crack Armor | Rare | Triggered | Sundering an enemy already sundered has a 20% chance to crack it open: 1 times your hit and a 1.5 sec stun. |
| Focused Fury | Common | Triggered | Sundering an already sundered enemy gives you 4 rage. |
| Mocking Sunder | Common | Triggered | What it hits turns on you for 3 sec: the tank's sunder. |
| Sunder the Mighty | Common | - | Deals 50% more to elites and bosses. |
| Sunder Spree | Uncommon | Triggered | Its critical strikes take 1.5 sec off your other cooldowns. |
| Exposed Armor | Uncommon | - | A sundered enemy is laid open: it takes 10% more damage from everyone for 8 sec. |
| Shatter | Epic | Capstone: after 5 upgrades | Every Sunder applies all 5 stacks at once: armor gone in one blow. |

### Execute

_Fury · bolt · 3 sec cooldown · 10 rage_

After its cost it spends up to 30 more rage, each point 2.5% more damage (75% more with a full spend).

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Honed Execute | Common | Rank-up x2 | Deals 25% more damage. Can be taken twice. |
| Deep Pockets | Common | Rank-up x2 | Spends up to 20 more rage (50 in all). Can be taken twice. |
| Executioner | Uncommon | - | Works below 30% health. |
| Headsman | Rare | After: Executioner | Works below 40% health. |
| Decapitate | Uncommon | Choice: edge | 30% more critical strike chance. |
| Deathblow | Uncommon | Choice: edge | Its critical strikes deal 60% more. |
| Cleaving Execute | Uncommon | Choice: edge | 50% of each Execute splashes onto enemies within 8 yd of the target. |
| Momentum | Rare | Triggered | A kill refunds 20 rage and finishes its cooldown: execute the next one. |
| Sweeping Execute | Rare | - | Also executes a second wounded enemy next to your target. |
| Killing Rush | Rare | Lights up a key | Each kill has a 30% chance to light up the next Execute: free, and usable on any enemy at any health. |
| Bloodthirsty Blade | Uncommon | Triggered | Each kill: 8% more damage for 10 sec, stacking 5 times. |
| Culling Blow | Common | Triggered | A kill heals you 6% of your max health. |
| Chase Down | Common | Triggered | A kill gives 15 rage and 30% run speed for 4 sec: on to the next. |
| Giant Killer | Common | - | Deals 50% more to elites and bosses. |
| Final Verdict | Uncommon | Triggered | A kill takes 2 sec off your other cooldowns. |
| Massacre | Epic | Capstone: after 5 upgrades | Works below 50% health and spends up to 30 more rage. |

### Charge

_Arms · charge · 14 sec cooldown_

Defenders in reach rally to your target: they turn on it and hit it 20% harder.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Improved Charge | Common | Rank-up x2 | Gain 10 more rage. Can be taken twice. |
| Quick Charge | Common | Rank-up x2 | Cooldown 4 sec shorter: every 10, then every 6 sec. Can be taken twice. |
| Intercept | Uncommon | - | Stuns for 3 sec. |
| Trample | Uncommon | Choice: landing | Lands with a shockwave: 0.8 times your hit to every enemy within 6 yd, all stunned. |
| Earthshaker | Rare | After: Trample | Trample reaches 4 yd further and stuns 1 sec longer. |
| Skull Crack | Uncommon | Choice: landing | Lands a crushing blow: 2.5 times your hit, and 50% more to enemies above 80% health. |
| Shield Charge | Uncommon | Choice: landing, Triggered | Charge behind your shield: arrive with an absorb shield of 15% of your max health. |
| Warbringer | Rare | - | Reaches 10 yd further, and works from any distance. |
| Battle Rush | Common | Triggered | After a charge your swings come 30% faster for 6 sec. |
| Rallying Charge | Uncommon | Triggered | You and allies within 15 yd deal 12% more damage for 8 sec after a charge. |
| Juggernaut | Common | Triggered | After a charge you take 25% less damage for 5 sec. |
| Double Charge | Rare | Triggered | 30% chance its cooldown finishes at once: charge again. |
| Warpath | Uncommon | Triggered | Landing takes 3 sec off your other cooldowns. |
| Taunting Charge | Common | Triggered | Enemies within 10 yd of where you land turn on you for 4 sec. |
| Guardian Charge | Common | Triggered | Charging within 30 yd of the Ancient shields you for 10% of your max health. |
| Bulldoze | Epic | Capstone: after 5 upgrades, Triggered | Every Charge lands like a meteor: 1.5 times your hit to everything within 8 yd, stunned 2 sec. |

### Challenging Shout

_Protection · nova · 20 sec cooldown · 10 rage_

Pull them off the Ancient and your allies.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Booming Voice | Common | Rank-up x2 | Reaches 4 yd further. Can be taken twice. |
| Quick Shout | Common | Rank-up x2 | Cooldown 5 sec shorter: every 15, then every 10 sec. Can be taken twice. |
| Mocking Laughter | Common | - | They stay on you 4 sec longer. |
| Intimidating Shout | Uncommon | Choice: voice | Half the enemies that answer are stunned for 2 sec. |
| Terrifying Roar | Rare | After: Intimidating Shout | Intimidating Shout stuns every enemy that answers. |
| Battle Shout | Uncommon | Choice: voice, Triggered | Rallies you and allies within 15 yd: 15% more damage for 10 sec. |
| Commanding Shout | Uncommon | Choice: voice, Triggered | Steels you and allies within 15 yd: 15% less damage taken for 10 sec, and heals them 8%. |
| Demoralizing Shout | Uncommon | - | Enemies that answer deal 30% less damage for 12 sec. |
| Rallying Cry | Uncommon | - | Heals you 3% of your max health for each enemy that answers. |
| Battle Cry | Rare | - | Gain 4 rage for each enemy that answers. |
| Shield of Voices | Uncommon | - | Shields you for 3% of your max health for each enemy that answers. |
| Last Line | Uncommon | Triggered | Shouting within 25 yd of the Ancient heals it 2% of its health. |
| Challenge Accepted | Uncommon | Triggered | When 5 or more enemies answer, you take 25% less damage for 8 sec. |
| Bellow | Common | - | Gain 20 rage. |
| Echoing Challenge | Uncommon | After: Booming Voice, Triggered | Takes 4 sec off your other cooldowns. |
| Rallying Bellow | Epic | Capstone: after 5 upgrades, Triggered | You and allies within 15 yd take 25% less damage and deal 15% more for 8 sec. |

### Hamstring

_Arms · bolt · 6 sec cooldown · 5 rage_

Crippled: it must fight you (it can't run past you to the Ancient), and it takes 20% more damage from everyone for 10 sec.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Deep Cripple | Common | Rank-up x2 | Crippled enemies take 10% more damage from everyone (30% in all). Can be taken twice. |
| Long Leash | Common | - | The cripple lasts 5 sec longer. |
| Quick Hamstring | Common | - | Costs no rage, and comes back 2 sec sooner. |
| Throwing Axe | Uncommon | - | Thrown: reaches 25 yd. Catch the runners before they reach the Ancient. |
| Tendon Rip | Uncommon | Choice: cut | Also roots the target for 3 sec: it goes nowhere. |
| Kneecap | Uncommon | Choice: cut | Also stuns the target for 2 sec. |
| Shackle | Uncommon | Choice: cut | The crippled enemy deals 30% less damage. |
| Piercing Cut | Uncommon | - | Also cripples 2 more enemies next to your target. |
| Marked Prey | Rare | Triggered | Hamstringing an elite or boss gives you 20 rage and takes 3 sec off your other cooldowns. |
| Hunter's Rage | Common | Triggered | Each Hamstring gives you 8 rage. |
| Rallying Mark | Uncommon | Triggered | You and allies within 15 yd deal 10% more damage for 6 sec after each Hamstring: focus it down. |
| Lacerate | Uncommon | - | Leaves a bleed for 1.2 times your hit over 8 sec. |
| Exploit Weakness | Uncommon | Triggered | After each Hamstring you deal 15% more damage for 6 sec: cripple it, then cut it down. |
| Twin Cut | Uncommon | Triggered | 30% chance to cut again at once, for free. |
| Crippling Sweep | Epic | Capstone: after 5 upgrades | Cripples every enemy within 8 yd of the target too. |

### Revenge

_Protection · bolt · 5 sec cooldown · 5 rage_

Ready for a few seconds after you block or take a blow. The warrior in front gets the most of it.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Vengeance | Common | Rank-up x2 | Deals 25% more damage. Can be taken twice. |
| Quick Revenge | Common | Rank-up x2 | Cooldown 1.5 sec shorter. Can be taken twice. |
| Vengeful Fury | Rare | Lights up a key | A blow that takes 8% of your health or more lights up Revenge: ready at once, 50% more damage. |
| Retaliation | Uncommon | - | Heals you 3% of your max health for each enemy hit. |
| Spite | Uncommon | - | Deals 8% more for every 10% of your health you're missing. |
| Counterattack | Common | Triggered | Each Revenge shields you for 8% of your max health. |
| Grudge | Common | Triggered | Gain 4 rage for each enemy hit. |
| Unstoppable | Uncommon | Choice: wrath, Triggered | Its critical strikes finish its cooldown. |
| Avenger | Uncommon | Choice: wrath | 50% of each hit splashes onto enemies within 8 yd. |
| Wrath of the Wronged | Uncommon | Choice: wrath | Hits only your target, but for 2.6 times your hit. |
| Taunting Revenge | Common | Triggered | What it hits turns on you for 4 sec. |
| Bulwark Strike | Uncommon | Triggered | After each Revenge you take 15% less damage for 6 sec. |
| Blood for Blood | Uncommon | - | 30% of the damage it deals heals you. |
| Undying Revenge | Rare | Triggered | Below 35% health each Revenge heals you 10% of your max health. |
| Rippling Vengeance | Uncommon | Triggered | 30% chance to send a shockwave: 0.7 times your hit to every enemy within 8 yd of you. |
| Avatar of Vengeance | Epic | Capstone: after 5 upgrades, Triggered | Every blow you take has a 25% chance to unleash a free Revenge. |

### Bloodrage

_Fury · buff · 30 sec cooldown_

Costs 5% of your health: gain 30 rage over 5 sec.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Improved Bloodrage | Common | Rank-up x2 | Gain 15 more rage. Can be taken twice. |
| Quick Temper | Common | Rank-up x2 | Cooldown 7 sec shorter: every 23, then every 16 sec. Can be taken twice. |
| Unbridled | Uncommon | - | Costs no health. |
| Blood Fury | Rare | - | You also deal 25% more damage while it lasts, for 10 sec. |
| Boiling Blood | Rare | After: Blood Fury | Blood Fury: 15% more damage. |
| Frenzied Regeneration | Uncommon | Choice: blood, Triggered | Your blood answers: heals you 15% of your max health. |
| Berserker Fury | Uncommon | Choice: blood, Triggered | Your swings and cooldowns run 25% faster for 10 sec. |
| Death's Door | Uncommon | Choice: blood, Triggered | Below 50% health: 35% more damage for 10 sec. |
| Rush of Blood | Uncommon | Triggered | Takes 5 sec off your other cooldowns. |
| Adrenaline | Common | Triggered | Gain 25 rage at once. |
| Unstoppable Rage | Common | Triggered | You run 40% faster for 6 sec. |
| Blood Pact | Uncommon | Triggered | Heals allies within 15 yd 6%, and they deal 10% more damage for 10 sec. |
| Intimidating Rage | Uncommon | Triggered | Enemies within 8 yd are stunned for 2 sec. |
| Battle Ready | Common | Triggered | Every wave begins with 30 rage. |
| Pain Fuel | Common | Triggered | Blows that take 8% of your health or more give 10 rage. |
| Rage Unbound | Epic | Capstone: after 5 upgrades, Lights up a key | Your next 5 skills after Bloodrage cost no rage. |

### Sweeping Strikes

_Arms · buff · 30 sec cooldown · Greatsword path only_

Your next 5 strikes and swings also hit an enemy next to your target.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Extra Sweeps | Common | Rank-up x2 | 3 more strikes. Can be taken twice. |
| Quick Sweeps | Common | Rank-up x2 | Cooldown 6 sec shorter: every 24, then every 18 sec. Can be taken twice. |
| Sweeping Power | Uncommon | Triggered | You deal 15% more damage for 10 sec. |
| Whirling Steel | Common | Triggered | Opens with a sweep: 0.8 times your hit to every enemy within 8 yd. |
| Blade Dance | Uncommon | Triggered | Your swings and cooldowns run 25% faster for 10 sec. |
| Sweeping Rage | Common | Triggered | Gain 20 rage. |
| Arcing Sweep | Uncommon | Choice: sweep, Triggered | Takes 4 sec off your other cooldowns. |
| Endless Sweep | Uncommon | Choice: sweep | Lasts 10 sec longer, with 3 more strikes. |
| Battle Rhythm | Rare | Choice: sweep, Triggered | Each swing has a 4% chance to finish its cooldown. |
| Guarded Sweep | Common | Triggered | You take 15% less damage for 10 sec. |
| Lifesweep | Uncommon | Triggered | For 10 sec, 10% of the damage you deal heals you. |
| Stunning Sweep | Rare | Triggered | Enemies within 6 yd are stunned for 2 sec. |
| Taunting Sweep | Common | Triggered | Enemies within 10 yd turn on you for 5 sec. |
| Bloodthirsty Sweep | Common | Triggered | Heals you 8% of your max health. |
| Eternal Sweep | Epic | Capstone: after 5 upgrades | Sweeping Strikes lasts 40 sec longer, with 25 more strikes. |

### Mortal Strike

_Arms · bolt · 6 sec cooldown · 30 rage · Greatsword path only_

Mortal Wound: the target can't be healed for 10 sec (Dark Menders, boss rituals). 40% more to enemies above 80% health: the opener.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Mortal Edge | Common | Rank-up x2 | Deals 20% more damage. Can be taken twice. |
| Improved Mortal Strike | Common | Rank-up x2 | Costs 6 less rage. Can be taken twice. |
| Quick Strike | Common | - | Cooldown 1.5 sec shorter: every 4.5 sec. |
| Crushing Opener | Uncommon | - | Deals 40% more to enemies above 80% health (80% in all). |
| Festering Wound | Common | - | The Mortal Wound lasts 8 sec longer. |
| Deep Impact | Uncommon | Choice: blade | 40% of the hit splashes onto enemies within 8 yd, and wounds them too. |
| Cleave Through | Uncommon | Choice: blade | Also strikes and wounds a second enemy. |
| Mortal Blow | Uncommon | Choice: blade | 50% more damage: one enemy, hit as hard as anything in the game. |
| Grievous Wound | Uncommon | - | A wounded enemy also takes 15% more damage from everyone for 10 sec. |
| Overpowering | Uncommon | Triggered | Its critical strikes have a 50% chance to finish its cooldown. |
| Brutal Strike | Common | - | Its critical strikes deal 50% more. |
| Rage Return | Common | - | Its critical strikes refund 20 rage. |
| Battle Focus | Uncommon | Triggered | Its critical strikes take 2.5 sec off your other cooldowns. |
| Slayer | Common | - | Deals 40% more to elites and bosses. |
| Shockstrike | Rare | Triggered | Every strike sends a shockwave: 0.6 times your hit to enemies within 6 yd of the target. |
| Tactician | Common | Triggered | Its kills give you 25 rage. |
| Mortal Calamity | Epic | Capstone: after 5 upgrades | Every Mortal Strike wounds every enemy within 8 yd of the target (and hits them for 30%). |

### Death Wish

_Fury · buff · 60 sec cooldown · Berserker path only_

For 20 sec you deal 30% more damage, and take 20% more.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Deeper Wish | Common | Rank-up x2 | 10% more damage while it lasts. Can be taken twice. |
| Restless Wish | Common | Rank-up x2 | Cooldown 12 sec shorter: every 48, then every 36 sec. Can be taken twice. |
| Long Wish | Common | - | Lasts 8 sec longer. |
| Steady Nerves | Uncommon | - | You take only 10% more damage while it lasts (instead of 20%). |
| Reckless Abandon | Uncommon | - | Lean in: 20% more damage while it lasts, but 10% more damage taken. |
| Recklessness | Rare | Choice: wish, Triggered | 30% more critical strike chance for 12 sec. |
| Defiant Wish | Uncommon | Choice: wish, Triggered | The price paid back: you take 35% less damage for 12 sec (more than it costs). |
| Bloodlust | Rare | Choice: wish, Triggered | Your swings and cooldowns run 30% faster for 15 sec. |
| Fury Unleashed | Common | Triggered | Gain 35 rage. |
| Undying Wish | Uncommon | Triggered | Below 40% health it heals you 25% of your max health. |
| Death Spiral | Uncommon | Triggered | Opens with a spin: 1.5 times your hit to every enemy within 8 yd. |
| Bloody Wish | Uncommon | Triggered | For 15 sec, 12% of the damage you deal heals you. |
| Wild Heart | Uncommon | Triggered | Takes 8 sec off your other cooldowns. |
| Shared Fury | Rare | Triggered | Allies within 15 yd deal 12% more damage for 15 sec. |
| Death Waits | Rare | Triggered | Each wave has a 30% chance to finish its cooldown as it begins. |
| Wish Granted | Epic | Capstone: after 5 upgrades | Death Wish has no price, and comes back every 30 sec. |

### Bloodthirst

_Fury · bolt · 6 sec cooldown · 30 rage · Berserker path only_

The strike that heals.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Deep Thirst | Common | Rank-up x2 | Deals 20% more damage. Can be taken twice. |
| Improved Bloodthirst | Common | Rank-up x2 | Costs 6 less rage. Can be taken twice. |
| Greater Thirst | Common | Rank-up x2 | Heals 3% more. Can be taken twice. |
| Quick Thirst | Common | - | Cooldown 1.5 sec shorter: every 4.5 sec. |
| Thirst for Blood | Uncommon | Choice: thirst | Deals 50% more to bleeding enemies. |
| Bloody Cleave | Uncommon | Choice: thirst | Also strikes 2 more enemies, and each one heals you. |
| Vampiric Thirst | Rare | Choice: thirst | 30% of the damage it deals heals you too. |
| Surging Thirst | Rare | Lights up a key | Each Bloodthirst has a 25% chance to light up the next one: ready at once and free. |
| Killing Spree | Uncommon | Triggered | Each kill with it: 6% more damage for 15 sec, stacking 5 times. |
| Crimson Rush | Uncommon | Triggered | Its critical strikes take 2 sec off your other cooldowns. |
| Thirsting Blades | Common | Triggered | After each Bloodthirst your swings come 25% faster for 6 sec. |
| Enraged Thirst | Uncommon | Triggered | Its critical strikes enrage you: 20% more damage for 8 sec. |
| Unending Thirst | Uncommon | Triggered | Its kills finish its cooldown. |
| Feast | Common | Triggered | Its kills heal you 6% of your max health. |
| Blood Gift | Uncommon | Triggered | Each Bloodthirst also heals allies within 15 yd 3% of their health. |
| Blood Bath | Epic | Capstone: after 5 upgrades | Bloodthirst heals 10% more and has a 50% chance to strike twice. |

### Last Stand

_Protection · buff · 90 sec cooldown_

For 20 sec your max health is 30% higher (and you're healed for it).

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Greater Stand | Common | Rank-up x2 | 15% more max health while it lasts. Can be taken twice. |
| Ready Stand | Common | Rank-up x2 | Cooldown 20 sec shorter: every 70, then every 50 sec. Can be taken twice. |
| Long Stand | Common | - | Lasts 8 sec longer. |
| Unyielding | Uncommon | Triggered | Also shields you for 20% of your max health. |
| Desperate Stand | Rare | Choice: stand, Triggered | A blow that leaves you below 30% health finishes its cooldown (once a minute). |
| Defiant Stand | Uncommon | Choice: stand, Triggered | Enemies within 12 yd turn on you for 6 sec. |
| Rallying Stand | Rare | Choice: stand, Triggered | Heals allies within 15 yd 15%, and they take 15% less damage for 10 sec. |
| Stand Firm | Uncommon | Triggered | You take 20% less damage for 10 sec. |
| Fresh Wind | Common | Triggered | Heals you 12% more. |
| Vengeful Stand | Uncommon | Triggered | You deal 25% more damage for 10 sec. |
| Ancient Bond | Uncommon | Triggered | Within 25 yd of the Ancient it heals the Ancient 5% of its health. |
| Last Gasp | Common | Triggered | Gain 40 rage. |
| Unbreakable | Uncommon | Triggered | Enemies within 8 yd are stunned for 2 sec. |
| Iron Hide | Uncommon | Triggered | Takes 10 sec off your other cooldowns. |
| Stalwart | Common | Triggered | Every wave begins with a heal of 12% of your max health. |
| Immortal Stand | Epic | Capstone: after 5 upgrades, Triggered | Last Stand heals you to full. |

### Shield Slam

_Protection · bolt · 8 sec cooldown · 15 rage · Shieldbearer path only_

Interrupts: if an enemy within 20 yd is casting something you can break, your shield flies to it and the cast is broken (2 sec stun). Its key lights up while a cast can be broken.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Heavy Slam | Common | Rank-up x2 | Deals 25% more damage. Can be taken twice. |
| Quick Slam | Common | Rank-up x2 | Cooldown 2 sec shorter: every 6, then every 4 sec. Can be taken twice. |
| Long Arm | Uncommon | - | Interrupts casters up to 30 yd away. |
| Disrupting Slam | Rare | Triggered | Breaking a cast finishes its cooldown: ready for the next caster. |
| Spell Reflection | Rare | Choice: break, Triggered | Breaking a cast throws it back: the caster takes 2 times your hit. |
| Concussive Break | Uncommon | Choice: break, Triggered | Breaking a cast stuns the caster for 4 sec instead of 2. |
| Bashing Break | Uncommon | Choice: break, Triggered | Breaking a cast gives you 25 rage. |
| Guarded Break | Uncommon | Triggered | Breaking a cast shields you for 15% of your max health. |
| Sword and Board | Rare | Lights up a key | Each blow you block has a 30% chance to light up Shield Slam: ready at once, free, 50% more damage. |
| Punishing Slam | Common | Triggered | The target turns on you for 6 sec. |
| Brace | Common | Triggered | After each slam you take 15% less damage for 6 sec. |
| Ricochet | Uncommon | - | Also slams a second enemy. |
| Heavy Shield | Uncommon | - | 12% more damage for each Sunder Armor stack on the target. |
| Shockwave Slam | Rare | Triggered | Its critical strikes send a shockwave: 0.8 times your hit to enemies within 6 yd of the target, stunned 1.5 sec. |
| Spellbreaker | Epic | Capstone: after 5 upgrades, Triggered | Every broken cast stuns every enemy within 10 yd of the caster for 3 sec and refunds the cooldown. |

### Shield Wall

_Protection · buff · 120 sec cooldown · Shieldbearer path only_

For 10 sec you take 60% less damage.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Improved Shield Wall | Common | Rank-up x2 | Cooldown 25 sec shorter: every 95, then every 70 sec. Can be taken twice. |
| Long Wall | Common | Rank-up x2 | Lasts 4 sec longer. Can be taken twice. |
| Fortress | Uncommon | Triggered | Enemies within 15 yd turn on you for 8 sec. |
| Spiked Wall | Common | Triggered | Slams outward: 1.2 times your hit to every enemy within 8 yd. |
| Wall of Blades | Uncommon | Choice: wall, Triggered | You deal 25% more damage for 10 sec. |
| Protective Wall | Rare | Choice: wall, Triggered | Allies within 15 yd take 30% less damage for 10 sec. |
| Ancient's Wall | Rare | Choice: wall, Triggered | Within 30 yd of the Ancient it heals the Ancient 8% of its health. |
| Rage Wall | Common | Triggered | Gain 35 rage. |
| Healing Wall | Uncommon | Triggered | Heals you 20% of your max health. |
| Unbowed | Rare | Triggered | A blow that leaves you below 25% health finishes its cooldown (once every 90 sec). |
| Stone Wall | Uncommon | Triggered | Shields you for 25% of your max health. |
| Rallying Wall | Uncommon | Triggered | Heals allies within 15 yd 12% of their health. |
| Shield Burst | Uncommon | Triggered | Takes 12 sec off your other cooldowns. |
| Patient Wall | Uncommon | Triggered | Each wave has a 30% chance to finish its cooldown as it begins. |
| Counter Wall | Rare | Triggered | Each blow you take has a 20% chance to strike the attacker back for 1 times your hit. |
| Impenetrable | Epic | Capstone: after 5 upgrades | Shield Wall blocks 90% of the damage, and lasts 5 sec longer. |

## Class talents

| Talent | Rarity | Ranks | Rank 1 | Top rank |
|---|---|---|---|---|
| Tactical Mastery | Uncommon | 1 | You keep 20 rage between fights, and every wave begins with 20 more. | - |
| Anger Management | Common | 2 | You gain 2 rage every second while fighting. | You gain 4 rage every second while fighting. |
| Unbridled Wrath | Common | 1 | Your swings give 40% more rage. | - |
| Endless Rage | Common | 1 | Blows you take give 50% more rage. | - |
| Focused Rage | Common | 2 | Your abilities cost 15% less rage. | Your abilities cost 30% less rage. |
| Cruelty | Common | 1 | 8% more critical strike chance. | - |
| Impale | Common | 1 | Your abilities' critical strikes deal 30% more damage. | - |
| Weapon Mastery | Common | 2 | Your abilities and swings deal 8% more damage. | Your abilities and swings deal 16% more damage. |
| Deep Wounds | Uncommon | 2 | Your critical strikes make the target bleed for 50% of your hit over 6 sec. | Your critical strikes make the target bleed for 100% of your hit over 6 sec. |
| Enrage | Uncommon | 2 | After a critical strike you deal 10% more damage for 8 sec. | After a critical strike you deal 20% more damage for 8 sec. |
| Flurry | Uncommon | 1 | After a critical strike your next 3 swings come 30% faster. | - |
| Blood Craze | Uncommon | 1 | After a critical strike you regenerate 2% of your max health every second for 6 sec. | - |
| Sword Specialization | Uncommon | 2 | Your swings have a 8% chance to give you an extra swing at once. | Your swings have a 16% chance to give you an extra swing at once. |
| Sweeping Blows | Uncommon | 2 | Your swings have a 15% chance to cleave: 0.6 times your hit to the target and every enemy within 5 yd of it. | Your swings have a 30% chance to cleave: 0.6 times your hit to the target and every enemy within 5 yd of it. |
| Mace Specialization | Uncommon | 2 | Your swings have a 5% chance to stun the target for 1.5 sec. | Your swings have a 10% chance to stun the target for 1.5 sec. |
| Battle Flow | Rare | 1 | Your abilities' critical strikes have a 15% chance to finish their own cooldown. | - |
| Commanding Presence | Common | 1 | Your cooldowns run 15% faster. | - |
| Berserker Stance | Common | 1 | You deal 12% more damage, but take 5% more. | - |
| Defensive Stance | Common | 1 | You take 15% less damage, and enemies within 6 yd turn on you. | - |
| Relentless | Common | 1 | You run 15% faster. | - |
| Toughness | Common | 1 | You take 10% less damage. | - |
| Anticipation | Common | 1 | 15% more max health. | - |
| Defiance | Common | 2 | Enemies that come within 6 yd of you turn on you, leaving the Ancient and your allies alone. | Enemies that come within 12 yd of you turn on you, leaving the Ancient and your allies alone. |
| Shield Specialization | Uncommon | 2 | A 10% chance to block a blow: it does half damage and gives you 5 rage. | A 20% chance to block a blow: it does half damage and gives you 5 rage. |
| Healing Blocks | Uncommon | 1 | A 5% chance to block, and your blocks heal you 3% of your max health. | - |
| Shield Spikes | Uncommon | 1 | A 5% chance to block, and what you block is struck back for 0.6 times your hit. | - |
| Improved Battle Shout | Common | 1 | Your battle cry carries: you and allies within 30 yd deal 8% more damage. | - |
| Heroic Presence | Common | 1 | You and allies within 30 yd have 8% more max health. | - |
| Rallying Presence | Uncommon | 1 | Every wave begins with a rally: you and allies within 15 yd deal 12% more damage for 30 sec. | - |
| Carnage | Rare | 1 | Each kill: 4% more damage for 20 sec, stacking 5 times. | - |
| Second Wind | Rare | 1 | When a blow leaves you below 35% health you heal 20% of your max health (once every 30 sec). | - |
| Victorious | Uncommon | 1 | Each kill has a 40% chance to heal you 4% and give 8 rage. | - |
| Iron Will | Uncommon | 1 | A blow that takes 10% of your health or more shields you for 15% of your max health (once every 15 sec). | - |
| Bridge Warden | Uncommon | 1 | While you hold a bridge post, you take 15% less damage. | - |
| Warden of the Ancient | Uncommon | 1 | Each kill within 25 yd of the Ancient heals it 0.5% of its health. | - |
| Bloodied Edge | Uncommon | 1 | Below 50% health you deal 20% more damage. | - |
| Sudden Death (needs Execute) | Rare | 1 | Your abilities' hits have a 6% chance to light up Execute: free, and usable on any enemy at any health. | - |
| Victory Rush (needs Heroic Strike) | Rare | 1 | Each kill lights up Heroic Strike: ready at once, free, and twice the damage. | - |
| Revenge Ready (needs Revenge) | Rare | 1 | Each blow you block lights up Revenge: ready at once, 50% more damage. | - |
| Bloodsurge (needs Bloodthirst) | Rare | 1 | Whirlwind and Thunder Clap hitting 4 or more enemies light up Bloodthirst: ready at once, free. | - |

## Combinations

| Combination | Rarity | Skills | What it does |
|---|---|---|---|
| Unbound Fury | Rare | Bloodrage + Death Wish | Bloodrage has a 30% chance to finish Death Wish's cooldown, and Death Wish gives 30 rage. |
| Whirling Rage | Rare | Bloodrage + Whirlwind | Bloodrage lets loose a free Whirlwind. |
| Booming Challenge | Rare | Challenging Shout + Thunder Clap | Challenging Shout also lets loose a free Thunder Clap. |
| Run Down | Rare | Charge + Execute | Charging an enemy below 20% health lands with a free Execute. |
| Hunter's Leap | Rare | Charge + Hamstring | Charge lands with a free Hamstring. |
| Heroic Leap | Rare | Charge + Heroic Strike | Charge lights up Heroic Strike: ready at once, free, 50% more damage. |
| Concussive Force | Rare | Charge + Shield Slam | Charge lights up Shield Slam: ready at once, 30% more damage to the stunned target. |
| Thunderous Arrival | Rare | Charge + Thunder Clap | Charge lands with a free Thunder Clap. |
| Whirling Arrival | Rare | Charge + Whirlwind | Charge lands with a free Whirlwind. |
| Deathbringer | Rare | Death Wish + Mortal Strike | Death Wish lights up Mortal Strike (30% more damage), and Mortal Strike's kills finish its cooldown. |
| Bloodbath Finish | Uncommon | Execute + Bloodthirst | Execute heals you 5% like Bloodthirst, and Bloodthirst deals 50% more to enemies below 30% health. |
| Coup de Grace | Rare | Execute + Mortal Strike | Mortal Strike kills finish Execute's cooldown, and Execute's critical strikes finish Mortal Strike's. |
| Crippling Wounds | Rare | Hamstring + Rend | Hamstring lands with a free Rend. |
| Opening Strike | Rare | Heroic Strike + Execute | Heroic Strike's critical strikes light up Execute: free, and usable on any enemy. |
| Double Strike | Rare | Heroic Strike + Mortal Strike | Each Heroic Strike has a 20% chance to light up Mortal Strike: ready at once, 30% more damage. |
| Shattering Strikes | Uncommon | Heroic Strike + Sunder Armor | Heroic Strike also sunders, and deals 10% more per Sunder Armor stack. |
| Stand Alone | Rare | Last Stand + Challenging Shout | Last Stand lets loose a free Challenging Shout, and Challenging Shout heals you 2% for each enemy that answers. |
| Facing Death | Rare | Last Stand + Death Wish | Last Stand finishes Death Wish's cooldown. |
| Twin Fangs | Rare | Mortal Strike + Bloodthirst | Mortal Strike and Bloodthirst each have a 30% chance to finish the other's cooldown. |
| Mortal Wounds | Rare | Mortal Strike + Rend | Rend lights up Mortal Strike: ready at once, 50% more damage. |
| Crimson Tide | Uncommon | Rend + Bloodthirst | Bloodthirst deals 25% more to bleeding enemies and heals you 3% more when it hits one. |
| Bleed Out | Uncommon | Rend + Execute | Execute deals 40% more to bleeding enemies, and an Execute kill makes every enemy within 8 yd of you bleed for 1 times your hit. |
| Rending Sweep | Uncommon | Rend + Sweeping Strikes | Sweeping Strikes makes every enemy within 8 yd bleed for 1 times your hit. |
| Defiant Fury | Rare | Revenge + Bloodrage | Revenge gives 5 rage for each enemy it hits, and Bloodrage lights up Revenge. |
| Vengeful Thirst | Rare | Revenge + Bloodthirst | Revenge heals you 3% of your max health, and its kills finish Bloodthirst's cooldown. |
| Punishing Revenge | Rare | Revenge + Hamstring | Revenge slows what it hits 50% for 4 sec, and lands with a free Hamstring. |
| Retribution | Rare | Revenge + Shield Slam | Each Revenge has a 50% chance to finish Shield Slam's cooldown. |
| Bastion of Defiance | Rare | Shield Wall + Challenging Shout | Shield Wall lets loose a free Challenging Shout, and Challenging Shout shields you 3% for each enemy that answers. |
| Bulwark | Uncommon | Shield Wall + Last Stand | Last Stand also shields you for 20% of your max health, and Shield Wall heals you 15%. |
| Shield and Thunder | Rare | Shield Wall + Thunder Clap | Shield Wall lets loose a free Thunder Clap. |
| Exposed Throat | Rare | Sunder Armor + Execute | Execute deals 15% more per Sunder Armor stack, and sundering an enemy below 20% health lights up Execute (free). |
| Armor Breaker | Rare | Sunder Armor + Mortal Strike | Mortal Strike applies 2 Sunder Armor stacks, and each Sunder Armor has a 15% chance to light up Mortal Strike. |
| Sundering Slam | Rare | Sunder Armor + Shield Slam | Sunder Armor has a 15% chance to light up Shield Slam, and Shield Slam deals 8% more per Sunder Armor stack. |
| Iron Thunder | Uncommon | Sunder Armor + Thunder Clap | Thunder Clap sunders everything it hits, and Sunder Armor tears 2 extra stacks off slowed enemies. |
| Sweeping Death | Rare | Sweeping Strikes + Execute | Sweeping Strikes lights up Execute (free, usable on any enemy), and Execute strikes a second wounded enemy. |
| Rallying Thunder | Uncommon | Thunder Clap + Challenging Shout | Thunder Clap near the Ancient heals it 1% for each enemy hit, and Challenging Shout comes back 5 sec sooner. |
| Earthbind | Uncommon | Thunder Clap + Hamstring | Thunder Clap roots what it hits for 2 sec, and Hamstring deals 30% more to slowed enemies. |
| Thunderous Rend | Uncommon | Thunder Clap + Rend | Every enemy Thunder Clap hits bleeds for 0.8 times your hit over 6 sec. |
| Seismic Slam | Rare | Thunder Clap + Shield Slam | Shield Slam's critical strikes let loose a free Thunder Clap. |
| Blood Frenzy | Rare | Whirlwind + Bloodthirst | Whirlwind through 3 or more enemies lights up Bloodthirst: ready at once, free, 50% more damage. |
| Reaper | Rare | Whirlwind + Execute | Whirlwind deals 40% more to enemies below 20% health, and its kills finish Execute's cooldown. |
| Merciless | Rare | Whirlwind + Mortal Strike | Mortal Strike's critical strikes finish Whirlwind's cooldown, and Whirlwind deals 30% more to bleeding enemies. |
| Rending Whirl | Uncommon | Whirlwind + Rend | Every enemy Whirlwind hits bleeds for 0.8 times your hit over 12 sec. |
| Whirling Sweep | Rare | Whirlwind + Sweeping Strikes | Sweeping Strikes opens with a free Whirlwind, and Whirlwind reaches 2 yd further. |
| Storm and Thunder | Rare | Whirlwind + Thunder Clap | Thunder Clap lights up Whirlwind: ready at once, 50% more damage. Whirlwind slows what it hits 30%. |

## Weapon Paths and path cards

### Path of the Greatsword

Commit to the greatsword: you crit 5% more often, your crits deal 25% more, and your abilities' crits have a 25% chance to Overpower for 0.6 times your hit to everything within 5 yd. Opens Mortal Strike and Sweeping Strikes to your skill cards, and the Greatsword path cards. One weapon path a run.

| Path card | Rarity | Kind | What it does |
|---|---|---|---|
| Two-Hander Mastery | Common | Rank-up x2 | You deal 7% more damage. (rank 2: You deal 14% more damage.) |
| Heft | Common | - | 15% more max health, and you take 5% less damage. |
| Deep Wounds Mastery | Common | - | Your critical strikes make the target bleed for 80% of your hit over 6 sec. |
| Follow-Through | Uncommon | - | Your swing crits strike again at once for 1 times your hit. |
| Cut Them Down | Uncommon | Choice | Your abilities deal 30% more to enemies below 30% health. |
| Colossus | Uncommon | Choice | You deal 20% more damage to elites and bosses. |
| Cleaving Swings | Uncommon | - | Your swings have a 20% chance to cleave: 0.6 times your hit to every enemy within 5 yd of the target. |
| Heavy Blows | Uncommon | - | Your swings have an 8% chance to stun the target for 1.5 sec. |
| Crushing Blows | Uncommon | - | Your abilities' critical strikes deal 40% more. |
| Titan's Grip | Epic | Capstone: after 4 path cards | Your abilities deal 20% more damage, and you crit 10% more often. |

### Path of the Shieldbearer

Take up sword and shield: 12% less damage taken, 10% more health, a 15% chance to block, and every block gives 3 rage. Opens Shield Slam (your interrupt) and Shield Wall to your skill cards, and the Shieldbearer path cards. One weapon path a run.

| Path card | Rarity | Kind | What it does |
|---|---|---|---|
| Shield Block | Common | Rank-up x2 | 10% more chance to block. (rank 2: 20% more chance to block.) |
| Defender's Resolve | Common | Rank-up x2 | Your blocks heal you 2% of your max health. (rank 2: Your blocks heal you 4% of your max health.) |
| Shield Mastery | Common | - | Every block gives 8 more rage. |
| Bastion | Uncommon | Choice | Hold: you take 15% less damage and enemies within 8 yd turn on you. |
| Vanguard | Uncommon | Choice | Press: you deal 15% more damage, and what you block is struck back for 1 times your hit. |
| Taunting Shield | Uncommon | - | What you block turns on you for 5 sec, and every block gives 2 Sunder Armor stacks. |
| Bulwark Reflex | Uncommon | - | A blow that takes 10% of your health or more raises your shield: 30% less damage for 4 sec (once every 12 sec). |
| Guardian | Uncommon | - | Allies within 30 yd take 8% less damage. |
| Shield Bash | Uncommon | - | Every block has a 15% chance to stun the attacker for 2 sec. |
| Unbreakable Bulwark | Epic | Capstone: after 4 path cards | 15% more chance to block, and every block shields you for 4% of your max health. |

### Path of the Berserker

Two weapons: 12% faster attacks, a 20% chance for an extra swing, and your swing crits make you 5% faster for 6 sec (stacking 3 times). Opens Bloodthirst and Death Wish to your skill cards, and the Berserker path cards. One weapon path a run.

| Path card | Rarity | Kind | What it does |
|---|---|---|---|
| Flurry of Blows | Common | Rank-up x2 | Your attacks come 8% faster. (rank 2: Your attacks come 16% faster.) |
| Dual Wield Specialization | Common | Rank-up x2 | Your swings deal 15% more damage. (rank 2: Your swings deal 30% more damage.) |
| Unbridled Fury | Common | - | Your swings give 50% more rage. |
| Off-Hand Strike | Uncommon | - | 10% more chance for an extra swing. |
| Rampage | Uncommon | Choice | Each kill: 5% more damage for 20 sec, stacking 5 times. |
| Blood Hunger | Uncommon | Choice | Each kill heals you 5% of your max health and gives 10 rage. |
| Furious Blades | Common | - | Your swing crits give 8 rage. |
| Berserker Rage | Uncommon | - | A blow that takes 8% of your health or more sends you berserk: 20% more damage for 6 sec (once every 8 sec). |
| Whirling Blades | Uncommon | - | Your swings have a 12% chance to spin: 0.6 times your hit to every enemy within 5 yd. |
| Frenzy | Epic | Capstone: after 4 path cards | Your attacks and cooldowns run 25% faster, and you crit 5% more often. |

## Which skills combine

| Skill | Combines with |
|---|---|
| Heroic Strike | Sunder Armor (Shattering Strikes), Execute (Opening Strike), Mortal Strike (Double Strike), Charge (Heroic Leap) |
| Rend | Mortal Strike (Mortal Wounds), Whirlwind (Rending Whirl), Thunder Clap (Thunderous Rend), Hamstring (Crippling Wounds), Execute (Bleed Out), Bloodthirst (Crimson Tide), Sweeping Strikes (Rending Sweep) |
| Thunder Clap | Rend (Thunderous Rend), Charge (Thunderous Arrival), Challenging Shout (Booming Challenge), Hamstring (Earthbind), Shield Slam (Seismic Slam), Whirlwind (Storm and Thunder), Sunder Armor (Iron Thunder), Challenging Shout (Rallying Thunder), Shield Wall (Shield and Thunder) |
| Whirlwind | Rend (Rending Whirl), Bloodthirst (Blood Frenzy), Execute (Reaper), Sweeping Strikes (Whirling Sweep), Mortal Strike (Merciless), Thunder Clap (Storm and Thunder), Charge (Whirling Arrival), Bloodrage (Whirling Rage) |
| Sunder Armor | Heroic Strike (Shattering Strikes), Mortal Strike (Armor Breaker), Thunder Clap (Iron Thunder), Shield Slam (Sundering Slam), Execute (Exposed Throat) |
| Execute | Whirlwind (Reaper), Bloodthirst (Bloodbath Finish), Heroic Strike (Opening Strike), Rend (Bleed Out), Mortal Strike (Coup de Grace), Charge (Run Down), Sweeping Strikes (Sweeping Death), Sunder Armor (Exposed Throat) |
| Charge | Thunder Clap (Thunderous Arrival), Hamstring (Hunter's Leap), Shield Slam (Concussive Force), Execute (Run Down), Whirlwind (Whirling Arrival), Heroic Strike (Heroic Leap) |
| Challenging Shout | Thunder Clap (Booming Challenge), Last Stand (Stand Alone), Shield Wall (Bastion of Defiance), Thunder Clap (Rallying Thunder) |
| Hamstring | Charge (Hunter's Leap), Rend (Crippling Wounds), Thunder Clap (Earthbind), Revenge (Punishing Revenge) |
| Revenge | Shield Slam (Retribution), Bloodthirst (Vengeful Thirst), Bloodrage (Defiant Fury), Hamstring (Punishing Revenge) |
| Bloodrage | Death Wish (Unbound Fury), Revenge (Defiant Fury), Whirlwind (Whirling Rage) |
| Sweeping Strikes | Whirlwind (Whirling Sweep), Execute (Sweeping Death), Rend (Rending Sweep) |
| Mortal Strike | Rend (Mortal Wounds), Sunder Armor (Armor Breaker), Execute (Coup de Grace), Whirlwind (Merciless), Heroic Strike (Double Strike), Bloodthirst (Twin Fangs), Death Wish (Deathbringer) |
| Death Wish | Bloodrage (Unbound Fury), Last Stand (Facing Death), Mortal Strike (Deathbringer) |
| Bloodthirst | Whirlwind (Blood Frenzy), Execute (Bloodbath Finish), Revenge (Vengeful Thirst), Rend (Crimson Tide), Mortal Strike (Twin Fangs) |
| Last Stand | Shield Wall (Bulwark), Challenging Shout (Stand Alone), Death Wish (Facing Death) |
| Shield Slam | Revenge (Retribution), Thunder Clap (Seismic Slam), Charge (Concussive Force), Sunder Armor (Sundering Slam) |
| Shield Wall | Last Stand (Bulwark), Challenging Shout (Bastion of Defiance), Thunder Clap (Shield and Thunder) |
