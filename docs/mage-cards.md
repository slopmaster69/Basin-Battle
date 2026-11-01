# Mage: every level-up card

_Generated from `content/classes/mage.toml` by `tools/card_catalog.py`. Edit the content file, not this page._

- **18 skills**, up to 6 on your bar at once.
- **270 skill upgrades** (about 15 per skill). A skill takes **10** in a run, so no run sees a whole deck.
- Every upgrade also makes its skill hit 3% harder, on top of what the card says.
- **36 class talents** (offered whatever skills you hold; some come back for more ranks).
- **32 combinations** (offered only while you hold both skills).
- **3 specializations** (one a run, at random, as a Legendary card). It teaches nothing by itself: it opens its own skills to the skill cards (on keys of their own, so a full bar never shuts them out) and its own **33 path cards**.
- **Rarity means something**: Common cards are numbers, Uncommon change how you play, Rare light a key, reset or cast, and each skill's **Epic** capstone comes only after 5 of its upgrades (a path's after 4 of its cards). Skills themselves are not rolled for rarity.

How to read the upgrade tables: **Rank-up xN** can be taken N times. **Choice: x** means only one upgrade with that choice can be taken for the skill. **After: Y** is offered once Y is taken. **Lights up a key** makes a skill's next cast ready at once and empowered (its key glows until you press it). What each skill is for: [mage-roles.md](mage-roles.md).

## Skills and their upgrades

### Frostbolt

_Frost · bolt · 2.5 sec cooldown · can be your first skill_

The chill: frost hits harder on the chilled and the Frozen.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Ice Edge | Common | Rank-up x2 | Deals 25% more damage. Can be taken twice. |
| Frost Channeling | Common | Rank-up x2 | Casts 0.4 sec faster: 1.4, then 1.0 sec. Can be taken twice. |
| Permafrost | Common | - | Chills harder: 50% slower, for 6 sec. |
| Piercing Ice | Uncommon | - | Deals 40% more to enemies already Chilled: the second bolt hits harder than the first. |
| Frostbite | Uncommon | - | 30% chance to Freeze the target for 3 sec. |
| Splintering Ice | Uncommon | Choice: bolt | Shatters on impact: 50% of its damage hits enemies within 8 yd of the target, and Chills them. |
| Twin Bolts | Uncommon | Choice: bolt | Fires a second bolt at another enemy. |
| Ice Lance | Uncommon | Choice: bolt | Instant: no cast, a little less damage (25% less). Cast it on the run. |
| Brain Freeze | Rare | Lights up a key | Its critical strikes light up Frostbolt: ready at once, instant, 50% more damage. |
| Fingers of Frost | Rare | Triggered | Hitting a Frozen enemy finishes its cooldown. |
| Winter's Bite | Uncommon | - | Leaves Winter's Chill: frost spells have 2% more critical strike chance against the target per stack (up to 5). |
| Ice Shards | Common | - | Its critical strikes deal 50% more. |
| Cold Precision | Common | - | 20% more critical strike chance. |
| Frozen Core | Uncommon | Triggered | Each Frostbolt shields you for 3% of your max health. |
| Glacial Spike | Epic | Capstone: after 5 upgrades | Every Frostbolt Freezes its target for 3 sec and deals 50% more to it. |

### Fireball

_Fire · bolt · 3.5 sec cooldown · can be your first skill_

The big hit: the hardest of your basic spells.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Searing Fire | Common | Rank-up x2 | Deals 25% more damage. Can be taken twice. |
| Improved Fireball | Common | Rank-up x2 | Casts 0.5 sec faster: 2.0, then 1.5 sec. Can be taken twice. |
| Searing Burn | Common | - | Its burn is twice as hot, and stacks 3 times. |
| Blast Radius | Uncommon | Choice: ball | Explodes: 40% of its damage hits enemies within 8 yd of the target. |
| Twin Fireballs | Uncommon | Choice: ball | Hurls a second Fireball at another enemy. |
| Great Fireball | Uncommon | Choice: ball | One target, 60% more damage, 0.5 sec more to cast: the biggest ball of all. |
| Impact | Uncommon | - | 25% chance to stun the target for 2 sec. |
| Critical Mass | Common | - | 20% more critical strike chance. |
| Molten Fury | Uncommon | - | Deals 40% more to enemies below 20% health. |
| Kindling | Uncommon | Triggered | Its critical strikes take 2 sec off your other cooldowns. |
| Heating Up | Rare | Lights up a key | Its critical strikes light up the next Fireball: ready at once, instant. |
| Burning Soul | Common | - | Deals 30% more to elites and bosses. |
| Scorching Ball | Uncommon | - | Leaves a stack of Fire Vulnerability. |
| Pyroclasm | Uncommon | Triggered | Its kills set enemies within 8 yd burning for 1 times your hit. |
| Meteor | Epic | Capstone: after 5 upgrades, Triggered | Each Fireball has a 30% chance to call down a Meteor: 3 times your hit to every enemy within 8 yd of the target, stunned 2 sec. |

### Arcane Missiles

_Arcane · bolt · 5 sec cooldown · can be your first skill_

The elite melter: a volley that can't miss, 30% stronger against elites and bosses. Magic ignores armor.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Arcane Potency | Common | Rank-up x2 | Deals 25% more damage. Can be taken twice. |
| Rapid Missiles | Common | Rank-up x2 | Cooldown 1.5 sec shorter: every 3.5, then every 2 sec. Can be taken twice. |
| Improved Arcane Missiles | Common | Rank-up x2 | Fires 2 more missiles. Can be taken twice. |
| Giant Slayer | Uncommon | - | Deals 40% more to elites and bosses (70% in all). |
| Arcane Splinters | Uncommon | Choice: weave | Each missile arcs to enemies within 6 yd of the target for 35% of its damage. |
| Split Volley | Uncommon | Choice: weave | The missiles split between your target and 2 more enemies. |
| Focused Beam | Uncommon | Choice: weave | Every missile into one target: 40% more damage. |
| Missile Barrage | Rare | - | 30% chance to fire a second volley at once. |
| Unstable Missiles | Common | - | 15% more critical strike chance. |
| Arcane Drag | Uncommon | - | Its target is slowed 40% for 4 sec. |
| Arcane Charge | Uncommon | Triggered | Each volley: 4% more damage for your spells for 10 sec, stacking 4 times. |
| Arcane Concentration | Rare | Triggered | Each volley has a 20% chance to finish all your other cooldowns (once every 8 sec). |
| Arcane Exposure | Uncommon | - | Its target takes 10% more damage from everyone for 8 sec. |
| Opening Volley | Common | - | Deals 40% more to enemies above 80% health. |
| Arcane Torrent | Epic | Capstone: after 5 upgrades | Fires at 3 more enemies, and each volley has a 50% chance to fire again. |

### Arcane Explosion

_Arcane · nova · 4 sec cooldown · can be your first skill_

Instant, all around you: the one blast you can cast on the run.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Arcane Reach | Common | Rank-up x2 | Reaches 3 yd further. Can be taken twice. |
| Rapid Detonation | Common | Rank-up x2 | Cooldown 1 sec shorter: every 3, then every 2 sec. Can be taken twice. |
| Improved Arcane Explosion | Common | - | Deals 30% more damage. |
| Mana Burst | Uncommon | - | Dazes: enemies hit are slowed 40% for 3 sec. |
| Arcane Surge | Uncommon | - | Each enemy hit takes 0.3 sec off your other cooldowns (up to 2 sec a cast). |
| Warding Blast | Uncommon | - | Each enemy hit shields you for 2% of your max health. |
| Critical Mass | Common | - | 20% more critical strike chance. |
| Concussive Blast | Uncommon | Choice: burst | Stuns enemies hit for 1 sec. |
| Crowded Room | Uncommon | Choice: burst | 5% more damage for every enemy caught after the first. |
| Resonance | Uncommon | Choice: burst | 30% chance to go off twice. |
| Displacement Charge | Uncommon | Triggered | After each explosion you run 30% faster for 3 sec: blast and step away. |
| Arcane Vigor | Common | - | Heals you 2% of your max health for each enemy hit. |
| Arcane Spark | Rare | Triggered | Hitting 4 or more enemies finishes all your cooldowns (once every 6 sec). |
| Arcane Nova | Uncommon | - | Exposes what it hits: 10% more damage from everyone for 5 sec. |
| Supernova | Epic | Capstone: after 5 upgrades | Reaches 6 yd further, knocks the wind out of them (stunned 1.5 sec), and has a 50% chance to go off twice. |

### Frost Nova

_Frost · nova · 15 sec cooldown_

Frozen enemies can't move or attack, and the Ancient's defences hit them 30% harder. Your escape.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Improved Frost Nova | Common | Rank-up x2 | Cooldown 3 sec shorter: every 12, then every 9 sec. Can be taken twice. |
| Ring of Frost | Common | - | Reaches 4 yd further. |
| Deep Freeze | Common | - | Freezes 2 sec longer. |
| Shattering Nova | Uncommon | - | Deals 3 times its damage to enemies already Chilled. |
| Frost Warding | Uncommon | - | Each enemy frozen wraps you in ice: a shield of 4% of your max health. |
| Brittle | Uncommon | - | Frozen enemies take 20% more damage from everyone while frozen. |
| Guardian's Frost | Uncommon | Triggered | Cast within 25 yd of the Ancient, it freezes everything within 10 yd of the Ancient too. |
| Cold Shell | Uncommon | Choice: nova, Triggered | You take 30% less damage for 4 sec. |
| Frozen Flight | Uncommon | Choice: nova, Triggered | You run 50% faster for 3 sec: freeze and go. |
| Icy Veins | Uncommon | Choice: nova, Triggered | Your spells cast 20% faster for 6 sec. |
| Shatter Setup | Rare | Lights up a key | Lights up Fire Blast and Frostbolt: ready at once. |
| Winter's Touch | Common | - | Leaves 2 stacks of Winter's Chill. |
| Lingering Cold | Common | - | Enemies it froze stay 50% slower for 4 sec after. |
| Biting Cold | Common | - | Deals 3 times as much damage. |
| Absolute Zero | Epic | Capstone: after 5 upgrades | Freezes 4 sec longer, and frozen enemies take 40% more damage from everyone for 8 sec. |

### Fire Blast

_Fire · bolt · 7 sec cooldown_

Fire's only instant: Frost Nova, then Fire Blast.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Improved Fire Blast | Common | Rank-up x2 | Cooldown 1.5 sec shorter: every 4.5, then every 3 sec. Can be taken twice. |
| Incinerate | Common | - | 25% more critical strike chance. |
| Searing Blast | Common | - | Deals 30% more damage. |
| Frostfire | Uncommon | - | Against Frozen enemies: another 50% more damage, and 50% more crit chance. |
| Flame Propagation | Uncommon | - | Spreads the target's burns to 3 enemies within 10 yd. |
| Impact | Uncommon | - | Stuns the target for 2 sec. |
| Long Blast | Common | - | Reaches 10 yd further. |
| Ignition | Uncommon | - | Sets the target burning for 1 times your hit over 6 sec. |
| Hot Streak | Rare | Choice: streak, Triggered | Its critical strikes finish its cooldown. |
| Chain Blast | Uncommon | Choice: streak | Leaps to a second enemy. |
| Blast Wave | Uncommon | Choice: streak, Triggered | Its kills send a wave of flame: 1 times your hit to enemies within 8 yd of you, slowed 50%. |
| Fire Vulnerability | Uncommon | - | Leaves 2 stacks of Fire Vulnerability. |
| Fiery Finish | Common | - | Deals 50% more to enemies below 20% health. |
| Searing Brand | Common | - | Deals 40% more to elites and bosses. |
| Flame Lance | Epic | Capstone: after 5 upgrades, Lights up a key | Always a critical strike against Frozen enemies, and every critical strike finishes its cooldown and lights up Fireball. |

### Scorch

_Fire · bolt · 1.5 sec cooldown_

Anyone's fire spells, not just yours: the more Scorch, the hotter every fire on it.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Improved Scorch | Common | Rank-up x2 | Applies 2 stacks of Fire Vulnerability. Can be taken twice. |
| Quick Scorch | Common | - | Casts 0.4 sec faster: 0.6 sec. |
| Searing Scorch | Common | - | Deals 30% more damage. |
| Scorched Earth | Uncommon | Choice: scorch | Also scorches 2 more enemies near your target. |
| Living Scorch | Uncommon | Choice: scorch | Leaves a burn of 1 times your hit, stacking 3 times. |
| Scorch on the Run | Uncommon | Choice: scorch | Instant: no cast time. |
| Searing Heat | Uncommon | - | Deals 60% more to burning enemies. |
| Incinerate | Common | - | 20% more critical strike chance. |
| Firestarter | Uncommon | - | Spreads the target's burns to 2 enemies within 10 yd. |
| Molten Shields | Uncommon | Triggered | Each Scorch shields you for 2% of your max health. |
| Kindled Flames | Rare | Lights up a key | Its critical strikes light up Fire Blast: ready at once, 50% more damage. |
| Fanning Flames | Common | Triggered | Each Scorch takes 0.5 sec off your other cooldowns. |
| Brand of Fire | Common | - | Deals 40% more to elites and bosses. |
| Molten Ground | Uncommon | - | Its target is slowed 40% for 4 sec. |
| Inferno | Epic | Capstone: after 5 upgrades | Every Scorch applies 5 stacks and spreads the target's burns to 4 enemies. |

### Blizzard

_Frost · zone · 14 sec cooldown_

Holds a lane: park it on a bridge or a choke.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Improved Blizzard | Common | - | Chills harder: 65% slower. |
| Arctic Reach | Common | Rank-up x2 | 3 yd wider. Can be taken twice. |
| Deep Winter | Common | Rank-up x2 | Lasts 4 sec longer. Can be taken twice. |
| Hailstorm | Common | - | 40% more damage each second. |
| Gathering Storm | Common | - | Cooldown 4 sec shorter. |
| Frozen Ground | Uncommon | Choice: storm | Each second, a 12% chance to Freeze each enemy in it for 2 sec. |
| Winter's Grasp | Uncommon | Choice: storm | Enemies in it gain a stack of Winter's Chill each second. |
| Bitter Cold | Uncommon | Choice: storm | Enemies in it attack 30% slower too. |
| Icy Ward | Uncommon | Triggered | Each Blizzard shields you for 12% of your max health. |
| Swift Storm | Uncommon | - | Instant cast. |
| Cold Front | Rare | Triggered | Each Blizzard has a 25% chance to finish Frost Nova's cooldown. |
| Guardian Storm | Uncommon | - | Within 25 yd of the Ancient it deals 40% more. |
| Chilled to the Bone | Uncommon | - | Enemies it slows take 10% more damage from everyone. |
| Ice Shards | Common | - | 25% more critical strike chance. |
| Eternal Winter | Epic | Capstone: after 5 upgrades | Lasts 8 sec longer, and each second a 20% chance to Freeze each enemy in it. |

### Flamestrike

_Fire · zone · 14 sec cooldown_

The clump killer: a pillar of flame, then burning ground. Best on enemies that can't walk out of it.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Improved Flamestrike | Common | - | 20% more critical strike chance. |
| Pillar of Flame | Common | Rank-up x2 | The first blast deals 50% more. Can be taken twice. |
| Firestorm | Common | - | The flames last 4 sec longer. |
| Quick Flames | Common | - | Casts 0.7 sec faster: 1.3 sec. |
| Wide Pillar | Common | - | 3 yd wider. |
| Impact | Uncommon | Choice: pillar | The first blast stuns everything in it for 1.5 sec. |
| Burning Ground | Uncommon | Choice: pillar | Enemies in the flames gain a stack of Fire Vulnerability each second. |
| Roaring Flames | Uncommon | Choice: pillar | 40% more damage each second. |
| Frozen Kindling | Uncommon | - | Deals 60% more to Frozen and Chilled enemies. |
| Flash Fire | Uncommon | - | Instant cast. |
| Burnout | Uncommon | Triggered | Its kills take 3 sec off your other cooldowns. |
| Twin Pillars | Rare | - | 30% chance to call a second pillar at once. |
| Searing Pillar | Common | - | Deals 30% more to elites and bosses. |
| Flame Ward | Uncommon | Triggered | Each Flamestrike shields you for 8% of your max health. |
| Rain of Fire | Epic | Capstone: after 5 upgrades | Calls a second pillar half the time, and the flames burn 4 sec longer. |

### Cone of Cold

_Frost · cone · 9 sec cooldown_

The peel: get them off your allies, then walk away.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Improved Cone of Cold | Common | Rank-up x2 | Deals 35% more damage. Can be taken twice. |
| Arctic Winds | Common | - | Reaches 5 yd further. |
| Quick Freeze | Common | Rank-up x2 | Cooldown 2 sec shorter: every 7, then every 5 sec. Can be taken twice. |
| Wide Cone | Common | - | A wider cone: 160 degrees. |
| Frostbite | Uncommon | Choice: cone | 35% chance to Freeze each enemy hit for 3 sec. |
| Numbing Cold | Uncommon | Choice: cone | Enemies hit attack 40% slower for 6 sec. |
| Crippling Cold | Uncommon | Choice: cone | Slows 80%, for 8 sec. |
| Shattering Cone | Uncommon | - | Deals double damage to Frozen enemies. |
| Icy Retreat | Uncommon | Triggered | You run 40% faster for 3 sec: blast and back off. |
| Winter's Breath | Common | - | Leaves a stack of Winter's Chill. |
| Blast of Winter | Uncommon | - | 5% more damage for every enemy caught after the first. |
| Cold Reflexes | Rare | Triggered | Hitting 4 or more enemies finishes Frost Nova's cooldown. |
| Frost Armor | Common | - | Each enemy hit shields you for 2% of your max health. |
| Ice Shards | Common | - | 25% more critical strike chance. |
| Frozen Wastes | Epic | Capstone: after 5 upgrades | Freezes everything it hits for 3 sec. |

### Blink

_Arcane · blink · 12 sec cooldown_

Your way out.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Improved Blink | Common | Rank-up x2 | Cooldown 3 sec shorter: every 9, then every 6 sec. Can be taken twice. |
| Far Blink | Common | - | Blinks 8 yd further. |
| Arcane Wake | Uncommon | Choice: wake | Leaves an explosion where you stood: 1 times your hit to enemies within 8 yd. |
| Frost Wake | Uncommon | Choice: wake | Freezes enemies within 8 yd of where you stood for 3 sec. |
| Fire Wake | Uncommon | Choice: wake, Triggered | Leaves flames where you stood: enemies within 8 yd burn for 1.5 times your hit. |
| Accelerated | Common | - | You run 40% faster for 4 sec after. |
| Displacement | Rare | - | Gives you Clearcasting: your next spell is instant and doesn't trigger its cooldown. |
| Prismatic Cloak | Uncommon | Triggered | You take 40% less damage for 3 sec after. |
| Arcane Restoration | Common | Triggered | Heals you 8% of your max health. |
| Temporal Shift | Uncommon | Triggered | Takes 3 sec off your other cooldowns. |
| Arcane Momentum | Uncommon | Triggered | Your spells deal 20% more damage for 5 sec after. |
| Instinct | Rare | Triggered | A blow that leaves you below 40% health finishes its cooldown (once every 20 sec). |
| Mirror Shield | Common | Triggered | Shields you for 10% of your max health. |
| Blink Strike | Uncommon | Lights up a key | Lights up Arcane Explosion: ready at once, 50% more damage. |
| Shimmer | Epic | Capstone: after 5 upgrades, Lights up a key | Each Blink lights up the next: ready at once. |

### Counterspell

_Arcane · bolt · 12 sec cooldown_

The interrupt: if an enemy within 30 yd is casting something you can break, the cast is broken and the caster reels (2 sec). Its key lights up while a cast can be broken. Otherwise it lashes your target.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Improved Counterspell | Common | Rank-up x2 | Cooldown 2 sec shorter: every 10, then every 8 sec. Can be taken twice. |
| Long Reach | Common | - | Breaks casts up to 40 yd away. |
| Arcane Lash | Common | - | Its strike deals 3 times as much. |
| Silenced | Uncommon | Choice: counter, Triggered | Breaking a cast stuns the caster 4 sec instead of 2. |
| Backlash | Uncommon | Choice: counter, Triggered | Breaking a cast throws it back: the caster takes 3 times your hit. |
| Arcane Shatter | Uncommon | Choice: counter, Triggered | Breaking a cast blasts everything within 8 yd of the caster for 1.5 times your hit. |
| Quick Wit | Rare | Triggered | Breaking a cast finishes its cooldown: ready for the next caster. |
| Arcane Insight | Uncommon | Triggered | Breaking a cast finishes your other cooldowns. |
| Spell Ward | Uncommon | Triggered | Breaking a cast shields you for 15% of your max health. |
| Rallying Silence | Uncommon | Triggered | Breaking a cast: you and allies within 15 yd deal 15% more damage for 8 sec. |
| Exposed Mind | Uncommon | - | Its target takes 20% more damage from everyone for 8 sec. |
| Arcane Bind | Common | - | Its target is slowed 60% for 5 sec. |
| Spellbreaker's Mark | Common | - | Deals 60% more to elites and bosses. |
| Mind Lash | Uncommon | - | Its strike stuns the target for 2 sec even when nothing is cast. |
| Spellsteal | Epic | Capstone: after 5 upgrades, Triggered | Breaking a cast steals its power: 30% more damage and 20% faster casts for 10 sec, and the cooldown comes back. |

### Pyroblast

_Fire · bolt · 14 sec cooldown · Fire path only_

The sniper's shot: the biggest single hit a mage has.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Hot Hands | Common | Rank-up x2 | Cooldown 3 sec shorter: every 11, then every 8 sec. Can be taken twice. |
| Searing Pyre | Common | - | Its burn is twice as hot. |
| Pyromaniac | Common | Rank-up x2 | Deals 25% more damage. Can be taken twice. |
| Quick Pyre | Common | - | Casts 1 sec faster: 3 sec. |
| Inferno | Uncommon | Choice: pyro | Explodes: 50% of its damage hits enemies within 8 yd of the target. |
| Pyroclasm | Uncommon | Choice: pyro | Stuns the target for 3 sec. |
| Molten Core | Uncommon | Choice: pyro | 25% more critical strike chance, and its crits deal 50% more. |
| Pyro Opener | Uncommon | - | Deals 50% more to enemies above 80% health. |
| Hot Streak | Rare | Lights up a key | Each Pyroblast has a 35% chance to light up the next: ready at once and instant. |
| Fuel the Fire | Uncommon | Triggered | Its kills finish its cooldown. |
| Searing Brand | Common | - | Leaves 3 stacks of Fire Vulnerability. |
| Wildfire | Uncommon | - | Spreads the target's burns to 4 enemies within 10 yd. |
| Dragonslayer | Common | - | Deals 40% more to elites and bosses. |
| Fiery Momentum | Uncommon | Triggered | Its critical strikes take 4 sec off your other cooldowns. |
| Living Sun | Epic | Capstone: after 5 upgrades | Its burn is 3 times as hot and spreads to every enemy within 10 yd when the target dies. |

### Combustion

_Fire · buff · 60 sec cooldown · Fire path only_

Each fire spell you cast adds 10% critical strike chance to your fire spells, until 3 of them crit.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Improved Combustion | Common | Rank-up x2 | Each fire spell adds 5% more critical strike chance (15%). Can be taken twice. |
| Quick Combustion | Common | Rank-up x2 | Cooldown 15 sec shorter: every 45, then every 30 sec. Can be taken twice. |
| Firestorm Heart | Uncommon | Triggered | You deal 20% more damage for 10 sec. |
| Ignition Point | Rare | Choice: combust, Lights up a key | Lights up Pyroblast: ready at once and instant. |
| Combustion Wave | Uncommon | Choice: combust, Triggered | Sends a wave of flame: 2 times your hit to enemies within 10 yd of you. |
| Conflagration | Uncommon | Choice: combust, Triggered | Sets every enemy within 15 yd burning for 2 times your hit. |
| Fire Starter | Uncommon | Triggered | Finishes Fire Blast's and Scorch's cooldowns. |
| Blazing Speed | Common | Triggered | Your spells cast 20% faster for 10 sec. |
| Searing Heat | Uncommon | Triggered | Every enemy within 15 yd gains 3 stacks of Fire Vulnerability. |
| Molten Armor | Common | Triggered | Shields you for 15% of your max health. |
| Kindle the Pyre | Uncommon | Triggered | You and allies within 15 yd deal 10% more damage for 10 sec. |
| Critical Mass | Common | Triggered | 15% more critical strike chance for 10 sec. |
| Slow Burn | Common | - | Lasts 10 sec longer. |
| Burning Waves | Uncommon | Triggered | Every wave has a 25% chance to begin with Combustion ready. |
| Phoenix Flames | Epic | Capstone: after 5 upgrades, Triggered | Your spells crit 50% more often for 8 sec, and Combustion comes back 20 sec sooner. |

### Ice Barrier

_Frost · shield · 25 sec cooldown · Frost path only_

Stand in it a little longer.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Thick Ice | Common | Rank-up x2 | Absorbs 10% more of your max health. Can be taken twice. |
| Quick Barrier | Common | Rank-up x2 | Cooldown 5 sec shorter: every 20, then every 15 sec. Can be taken twice. |
| Reflective Ice | Uncommon | Choice: barrier | 30% of what it absorbs strikes back at the attacker. |
| Shattering Barrier | Uncommon | Choice: barrier | Explodes when it breaks: 2 times your hit to enemies within 8 yd. |
| Icy Veins | Uncommon | Choice: barrier, Triggered | Your spells cast 25% faster for 8 sec. |
| Frost Armor | Uncommon | Triggered | Freezes enemies within 8 yd of you for 3 sec. |
| Cold Confidence | Common | Triggered | You deal 15% more damage for 10 sec. |
| Chilling Armor | Common | Triggered | Enemies within 10 yd are slowed 50% for 5 sec. |
| Glacial Embrace | Uncommon | Triggered | You and allies within 15 yd take 15% less damage for 8 sec. |
| Glacial Instinct | Rare | Triggered | A blow that leaves you below 35% health finishes its cooldown (once every 30 sec). |
| Winter's Patience | Common | Triggered | Takes 3 sec off your other cooldowns. |
| Fingers of Frost | Rare | Lights up a key | Lights up Frostbolt: ready at once, instant, 50% more damage. |
| Lasting Ice | Common | - | Lasts 15 sec longer. |
| Winter's Watch | Common | Triggered | Every wave begins with Ice Barrier ready. |
| Ice Block | Epic | Capstone: after 5 upgrades | Absorbs 40% more of your max health and comes back 10 sec sooner. |

### Cold Snap

_Frost · special · 60 sec cooldown · Frost path only_

Finishes the cooldowns of all your frost spells.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Quick Snap | Common | Rank-up x2 | Cooldown 12 sec shorter: every 48, then every 36 sec. Can be taken twice. |
| Snap Barrier | Common | Triggered | Shields you for 12% of your max health. |
| Snap Freeze | Uncommon | Triggered | Freezes enemies within 10 yd of you for 3 sec. |
| Icy Veins | Uncommon | Choice: snap, Triggered | Your spells cast 30% faster for 10 sec. |
| Deep Cold | Uncommon | Choice: snap, Triggered | Your frost spells deal 25% more for 10 sec. |
| Shatter Point | Uncommon | Choice: snap, Triggered | 30% more critical strike chance for 10 sec. |
| Arctic Snap | Rare | Triggered | Finishes Blink's and Counterspell's cooldowns too. |
| Frozen Restoration | Common | Triggered | Heals you 12% of your max health. |
| Bitter Snap | Common | Triggered | Enemies within 12 yd gain 3 stacks of Winter's Chill. |
| Winter's Embrace | Uncommon | Triggered | Allies within 15 yd take 15% less damage for 8 sec. |
| Snap Wave | Uncommon | Triggered | Every wave has a 30% chance to begin with Cold Snap ready. |
| Emergency Snap | Rare | Triggered | A blow that leaves you below 30% health finishes its cooldown (once a minute). |
| Snap Bolt | Uncommon | Lights up a key | Lights up Frostbolt: ready at once, instant. |
| Frozen Time | Common | Triggered | Takes 5 sec off your other cooldowns too. |
| Deep Freeze | Epic | Capstone: after 5 upgrades, Triggered | Comes back 20 sec sooner, and freezes every enemy within 15 yd for 4 sec. |

### Arcane Power

_Arcane · buff · 90 sec cooldown · Arcane path only_

For 15 sec: 30% more damage, and your cooldowns recover 30% faster.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Greater Arcane Power | Common | Rank-up x2 | 10% more damage while it lasts. Can be taken twice. |
| Quick Power | Common | Rank-up x2 | Cooldown 20 sec shorter: every 70, then every 50 sec. Can be taken twice. |
| Lasting Power | Common | - | Lasts 5 sec longer. |
| Arcane Surge | Rare | Choice: ap, Triggered | Finishes your other cooldowns. |
| Arcane Celerity | Uncommon | Choice: ap, Triggered | Your spells cast 25% faster while it lasts. |
| Arcane Brilliance | Uncommon | Choice: ap, Triggered | 20% more critical strike chance while it lasts. |
| Arcane Empowerment | Uncommon | Triggered | You and allies within 15 yd deal 12% more damage for 15 sec. |
| Missile Rush | Uncommon | Lights up a key | Lights up Arcane Missiles: ready at once, 50% more damage. |
| Arcane Blast | Uncommon | Triggered | Bursts out: 2 times your hit to enemies within 10 yd. |
| Mana Shield | Common | Triggered | Shields you for 15% of your max health. |
| Arcane Swiftness | Common | Triggered | You run 30% faster while it lasts. |
| Arcane Tide | Uncommon | Triggered | Every wave has a 25% chance to begin with Arcane Power ready. |
| Presence of Power | Uncommon | Triggered | Finishes Presence of Mind's cooldown. |
| Arcane Fortitude | Common | Triggered | Heals you 10% of your max health. |
| Arcane Ascendance | Epic | Capstone: after 5 upgrades | 40% more damage while it lasts, and it lasts 10 sec longer. |

### Presence of Mind

_Arcane · buff · 45 sec cooldown · Arcane path only_

Your next spell is instant, doesn't trigger its cooldown and deals 20% more.

| Upgrade | Rarity | Kind | What it does |
|---|---|---|---|
| Quick Mind | Common | Rank-up x2 | Cooldown 10 sec shorter: every 35, then every 25 sec. Can be taken twice. |
| Focused Mind | Uncommon | Triggered | You deal 20% more damage for 8 sec. |
| Mind of Fire | Rare | Choice: mind, Lights up a key | Lights up Pyroblast and Fireball: ready at once, instant, 30% more damage. |
| Mind of Frost | Rare | Choice: mind, Lights up a key | Lights up Frostbolt and Cone of Cold: ready at once, instant, 30% more damage. |
| Mind of Arcana | Rare | Choice: mind, Lights up a key | Lights up Arcane Missiles and Arcane Explosion: ready at once, 30% more damage. |
| Temporal Mind | Uncommon | Triggered | Takes 5 sec off your other cooldowns. |
| Mind Over Matter | Common | Triggered | Finishes Blink's cooldown. |
| Mental Barrier | Common | Triggered | Shields you for 10% of your max health. |
| Clarity | Common | Triggered | 25% more critical strike chance for 8 sec. |
| Quickened Thought | Uncommon | Triggered | Your spells cast 30% faster for 8 sec. |
| Prepared Mind | Uncommon | Triggered | Every wave begins with Presence of Mind ready. |
| Calm Mind | Common | Triggered | Heals you 8% of your max health. |
| Shared Insight | Uncommon | Triggered | Allies within 15 yd attack 15% faster for 8 sec. |
| Slow Time | Uncommon | Triggered | Enemies within 12 yd are slowed 50% for 5 sec. |
| Time Warp | Epic | Capstone: after 5 upgrades, Lights up a key | Lights up every spell you hold: all ready at once, instant, 20% more damage. |

## Class talents

| Talent | Rarity | Ranks | Rank 1 | Top rank |
|---|---|---|---|---|
| Arcane Focus | Common | 2 | Your arcane spells deal 6% more damage. | Your arcane spells deal 12% more damage. |
| Fire Power | Common | 2 | Your fire spells deal 6% more damage. | Your fire spells deal 12% more damage. |
| Piercing Ice | Common | 2 | Your frost spells deal 6% more damage. | Your frost spells deal 12% more damage. |
| Elemental Precision | Common | 2 | 5% more critical strike chance. | 10% more critical strike chance. |
| Arcane Mind | Common | 2 | Your spells cast and your cooldowns run 6% faster. | Your spells cast and your cooldowns run 12% faster. |
| Arcane Meditation | Common | 1 | Your cooldowns run 12% faster. | - |
| Arcane Concentration | Uncommon | 1 | Each spell you cast has an 8% chance to give you Clearcasting: your next spell is instant and doesn't trigger its cooldown. | - |
| Magic Attunement | Common | 1 | Your arcane spells have a 10% chance to go off twice. | - |
| Ignite | Uncommon | 2 | Your fire critical strikes set the target alight for 15% of the crit's damage over 4 sec. Ignites stack. | Your fire critical strikes set the target alight for 30% of the crit's damage over 4 sec. Ignites stack. |
| Critical Mass | Common | 1 | Your fire spells have 8% more critical strike chance. | - |
| Master of Elements | Uncommon | 1 | Your fire critical strikes take 0.6 sec off your other fire spells' cooldowns. | - |
| Flame Throwing | Common | 1 | Your fire spells reach 5 yd further and their areas are 15% wider. | - |
| Ice Shards | Common | 1 | Your frost critical strikes deal 30% more damage. | - |
| Shatter | Common | 2 | Your spells have 15% more critical strike chance against Frozen enemies. | Your spells have 30% more critical strike chance against Frozen enemies. |
| Permafrost | Common | 1 | Your chills slow 10% more and last 2 sec longer. | - |
| Arctic Reach | Common | 1 | Your frost spells reach 5 yd further and their areas are 15% wider. | - |
| Winter's Chill | Common | 1 | Your frost spells have a 50% chance to leave Winter's Chill: 2% more frost critical strike chance against the target per stack, up to 5. | - |
| Magic Absorption | Common | 2 | 8% more max health, and you take 5% less damage. | 16% more max health, and you take 10% less damage. |
| Frost Armor | Uncommon | 1 | Whatever strikes you is chilled: 40% slower for 4 sec. | - |
| Molten Armor | Uncommon | 1 | Whatever strikes you has a 30% chance to burn for 0.8 times your hit. | - |
| Mana Shield | Uncommon | 1 | A blow that takes 10% of your health or more shields you for 20% of your max health (once every 15 sec). | - |
| Blazing Speed | Uncommon | 1 | A blow that takes 8% of your health or more makes you run 50% faster for 4 sec. | - |
| Ice Block | Rare | 1 | When a blow leaves you below 25% health you're encased in ice: 80% less damage for 3 sec, and enemies within 8 yd frozen (once a minute). | - |
| Evocation | Rare | 1 | Every wave begins with all your cooldowns ready. | - |
| Arcane Intellect | Common | 1 | You and allies within 30 yd deal 6% more damage. | - |
| Dampen Magic | Common | 1 | You and allies within 30 yd take 5% less damage. | - |
| Amplify Magic | Common | 1 | You and allies within 30 yd attack and cast 5% faster. | - |
| Arcane Hunger | Uncommon | 1 | Each kill: 4% more damage for 20 sec, stacking 5 times. | - |
| Wand Specialization | Common | 1 | Your wand deals 25% more damage. | - |
| Warden of the Ancient | Uncommon | 1 | Each kill within 25 yd of the Ancient heals it 0.5% of its health. | - |
| Elemental Mastery | Common | 2 | Your spells deal 4% more damage. | Your spells deal 8% more damage. |
| Spell Echo | Rare | 1 | Your spells' critical strikes have a 12% chance to finish their own cooldown. | - |
| Arcane Missiles! (needs Arcane Missiles) | Rare | 1 | Your spells' hits have a 6% chance to light up Arcane Missiles: ready at once, 50% more damage. | - |
| Fingers of Frost (needs Frostbolt) | Rare | 1 | Your frost hits on Frozen enemies have a 25% chance to light up Frostbolt: ready at once, instant, 50% more damage. | - |
| Firestarter (needs Flamestrike) | Rare | 1 | Fire Blast and Scorch kills light up Flamestrike: ready at once and instant. | - |
| Spell Lock (needs Counterspell) | Common | 1 | Counterspell breaks casts up to 10 yd further. | - |

## Combinations

| Combination | Rarity | Skills | What it does |
|---|---|---|---|
| Arcane Barrage | Uncommon | Arcane Missiles + Arcane Explosion | Arcane Missiles strikes a second enemy, and Arcane Explosion deals 30% more. |
| Arcane Torrent | Rare | Arcane Power + Arcane Missiles | Arcane Power lights up Arcane Missiles (50% more damage), and Arcane Missiles deals 25% more to elites and bosses. |
| Arcane Speed | Rare | Arcane Power + Blink | Arcane Power finishes Blink's cooldown, and Blink comes back 3 sec sooner. |
| Arcane Exit | Rare | Blink + Arcane Explosion | Blink lands with a free Arcane Explosion. |
| Blazing Retreat | Rare | Blink + Fireball | Blink lights up Fireball: ready at once, instant. |
| Frozen Escape | Rare | Blink + Frost Nova | Blink lands with a free Frost Nova. |
| Winter's Grip | Uncommon | Blizzard + Frostbolt | Blizzard slows 20% more, and Frostbolt deals 30% more to chilled enemies. |
| Endless Winter | Rare | Cold Snap + Blizzard | Cold Snap lets loose a free Blizzard. |
| Cold Calculation | Rare | Cold Snap + Counterspell | Cold Snap finishes Arcane Missiles' and Counterspell's cooldowns. |
| Winter's Resolve | Uncommon | Cold Snap + Ice Barrier | Cold Snap heals you 15%, and Ice Barrier absorbs 20% more. |
| Flash Point | Rare | Combustion + Fire Blast | Combustion finishes Fire Blast's cooldown, and Fire Blast's critical strikes take 3 sec off Combustion's. |
| Combustion Wave | Rare | Combustion + Flamestrike | Combustion lets loose a free Flamestrike. |
| Winter's Wall | Uncommon | Cone of Cold + Blizzard | Cone of Cold leaves Winter's Chill, and Blizzard deals 40% more. |
| Arcane Lockdown | Rare | Counterspell + Arcane Missiles | Breaking a cast lights up Arcane Missiles: ready at once, 50% more damage. |
| Spell Lash | Rare | Counterspell + Fire Blast | Breaking a cast finishes Fire Blast's cooldown, and Fire Blast deals 50% more to stunned enemies. |
| Frozen Silence | Uncommon | Counterspell + Frost Nova | Counterspell Freezes its target for 3 sec. |
| Kindling | Rare | Fire Blast + Scorch | Fire Blast spreads the target's burns to 2 enemies, and Scorch's critical strikes finish Fire Blast's cooldown. |
| Pyromaniac | Rare | Fireball + Pyroblast | Fireball's critical strikes light up Pyroblast: ready at once, instant. |
| Scorched Earth | Uncommon | Flamestrike + Blizzard | Flamestrike and Blizzard both cover 3 yd more ground and deal 20% more. |
| Arcane Frost | Rare | Frost Nova + Arcane Explosion | Frost Nova lights up Arcane Explosion (50% more damage), and Arcane Explosion deals 60% more to Frozen enemies. |
| Deep Freeze | Uncommon | Frost Nova + Cone of Cold | Cone of Cold deals 60% more to Frozen enemies, and Frost Nova comes back 3 sec sooner. |
| Shatter Combo | Rare | Frost Nova + Fire Blast | Frost Nova lights up Fire Blast: ready at once, 50% more damage. |
| Frozen Pyre | Rare | Frost Nova + Flamestrike | Frost Nova lights up Flamestrike (ready at once, instant), and Flamestrike deals 50% more to Frozen enemies. |
| Ice Spray | Uncommon | Frostbolt + Cone of Cold | Cone of Cold and Frostbolt both deal 30% more to chilled enemies. |
| Frostfire | Uncommon | Frostbolt + Fireball | Frostbolt leaves Fire Vulnerability, and Fireball deals 40% more to chilled enemies. |
| Frozen Fire | Uncommon | Frostbolt + Pyroblast | Fireball and Pyroblast deal 50% more to Frozen enemies, and Frostbolt's freezes last 1 sec longer. |
| Frost Barrier | Rare | Ice Barrier + Frost Nova | Ice Barrier lets loose a free Frost Nova, and Frost Nova shields you for 10% of your max health. |
| Quick Thinking | Rare | Presence of Mind + Counterspell | Presence of Mind finishes Counterspell's cooldown, and Counterspell breaks casts 10 yd further. |
| Mind of Fire | Uncommon | Presence of Mind + Pyroblast | Pyroblast deals 30% more, and Presence of Mind comes back 10 sec sooner. |
| Searing Touch | Rare | Scorch + Fireball | Scorch's critical strikes light up Fireball: ready at once, instant. |
| Pyre | Uncommon | Scorch + Flamestrike | Flamestrike's flames leave Fire Vulnerability, and Scorch deals 40% more to burning enemies. |
| Searing Pyre | Rare | Scorch + Pyroblast | Each Scorch has a 15% chance to light up Pyroblast (instant), and Pyroblast leaves 2 Fire Vulnerability stacks. |

## Specializations and path cards

### Specialization: Fire

Specialize in Fire: your fire spells crit 10% more often, and their crits set the target alight (Ignite: 30% of the crit over 4 sec). Opens Pyroblast and Combustion to your spell cards, and the Fire path cards. One specialization a run.

| Path card | Rarity | Kind | What it does |
|---|---|---|---|
| Fire Mastery | Common | Rank-up x2 | Your fire spells deal 6% more damage. (rank 2: Your fire spells deal 12% more damage.) |
| Incineration | Common | - | Your fire spells have 8% more critical strike chance. |
| Searing Ignite | Common | - | Your Ignites burn 25% more of the crit. |
| Burning Soul | Uncommon | Choice | Steady: 10% more max health and 10% less damage taken. |
| Pyromania | Uncommon | Choice | Reckless: your fire critical strikes deal 40% more. |
| Master of Flame | Uncommon | - | Your fire critical strikes take 1 sec off your other fire spells' cooldowns. |
| Blast Wave | Uncommon | - | A blow that takes 10% of your health or more sends a wave of flame: 1.5 times your hit to enemies within 8 yd, slowed 50% (once every 8 sec). |
| Molten Shields | Uncommon | - | Whatever strikes you has a 40% chance to burn for 1 times your hit. |
| Hot Streak | Rare | - | Your fire critical strikes have a 10% chance to light up Pyroblast: ready at once and instant. |
| Living Bomb | Uncommon | - | Your fire spells' kills set enemies within 8 yd burning for 0.8 times your hit. |
| Firelord | Epic | Capstone: after 4 path cards | Your fire spells deal 15% more damage and crit 10% more often. |

### Specialization: Frost

Specialize in Frost: your spells crit 30% more often against Frozen enemies, your frost spells deal 20% more to chilled ones, and you take 10% less damage. Opens Ice Barrier and Cold Snap to your spell cards, and the Frost path cards. One specialization a run.

| Path card | Rarity | Kind | What it does |
|---|---|---|---|
| Frost Mastery | Common | Rank-up x2 | Your frost spells deal 6% more damage. (rank 2: Your frost spells deal 12% more damage.) |
| Shatter Mastery | Common | - | Your spells have 25% more critical strike chance against Frozen enemies. |
| Icy Veins | Common | - | Your spells cast and your cooldowns run 10% faster. |
| Ice Armor | Uncommon | Choice | Hold: you take 15% less damage, and whatever strikes you is slowed 50%. |
| Deep Winter | Uncommon | Choice | Strike: your frost spells leave a stack of Winter's Chill every time. |
| Permafrost Mastery | Common | - | Your chills slow 15% more and last 3 sec longer. |
| Frozen Core | Uncommon | - | A blow that takes 10% of your health or more freezes enemies within 8 yd for 3 sec (once every 12 sec). |
| Fingers of Frost | Rare | - | Your frost hits on Frozen enemies have a 15% chance to light up Frostbolt: ready at once, instant, 50% more damage. |
| Arctic Mastery | Common | - | Your frost spells reach 5 yd further and their areas are 20% wider. |
| Frostbite | Uncommon | - | Your frost hits have a 10% chance to Freeze the target for 3 sec. |
| Winter's Heart | Epic | Capstone: after 4 path cards | Your frost spells deal 15% more, and every frost hit has a 10% chance to Freeze. |

### Specialization: Arcane

Specialize in Arcane: your cooldowns run 15% faster, each spell has a 10% chance to give you Clearcasting (the next is instant and doesn't trigger its cooldown), and your arcane spells deal 10% more. Opens Arcane Power and Presence of Mind to your spell cards, and the Arcane path cards. One specialization a run.

| Path card | Rarity | Kind | What it does |
|---|---|---|---|
| Arcane Mastery | Common | Rank-up x2 | Your arcane spells deal 6% more damage. (rank 2: Your arcane spells deal 12% more damage.) |
| Deep Concentration | Uncommon | - | 8% more chance of Clearcasting. |
| Arcane Celerity | Common | - | Your spells cast and your cooldowns run 8% faster. |
| Arcane Instability | Uncommon | Choice | Reckless: you deal 10% more damage and crit 5% more often. |
| Arcane Fortitude | Uncommon | Choice | Steady: 15% more max health and 10% less damage taken. |
| Arcane Echoes | Uncommon | - | Your arcane spells have a 12% chance to go off twice. |
| Arcane Flow | Common | - | Your cooldowns run 10% faster. |
| Arcane Surge | Uncommon | - | Your critical strikes take 1 sec off your other cooldowns. |
| Missile Barrage | Rare | - | Your spells' hits have an 8% chance to light up Arcane Missiles: ready at once, 50% more damage. |
| Slow | Uncommon | - | Your arcane spells slow what they hit 30% for 4 sec. |
| Archmage | Epic | Capstone: after 4 path cards | Your spells deal 10% more damage and your cooldowns run 15% faster. |

## Which skills combine

| Skill | Combines with |
|---|---|
| Frostbolt | Fireball (Frostfire), Blizzard (Winter's Grip), Cone of Cold (Ice Spray), Pyroblast (Frozen Fire) |
| Fireball | Frostbolt (Frostfire), Scorch (Searing Touch), Pyroblast (Pyromaniac), Blink (Blazing Retreat) |
| Arcane Missiles | Arcane Explosion (Arcane Barrage), Counterspell (Arcane Lockdown), Arcane Power (Arcane Torrent) |
| Arcane Explosion | Arcane Missiles (Arcane Barrage), Blink (Arcane Exit), Frost Nova (Arcane Frost) |
| Frost Nova | Fire Blast (Shatter Combo), Cone of Cold (Deep Freeze), Blink (Frozen Escape), Counterspell (Frozen Silence), Ice Barrier (Frost Barrier), Arcane Explosion (Arcane Frost), Flamestrike (Frozen Pyre) |
| Fire Blast | Frost Nova (Shatter Combo), Scorch (Kindling), Counterspell (Spell Lash), Combustion (Flash Point) |
| Scorch | Fireball (Searing Touch), Flamestrike (Pyre), Fire Blast (Kindling), Pyroblast (Searing Pyre) |
| Blizzard | Flamestrike (Scorched Earth), Frostbolt (Winter's Grip), Cone of Cold (Winter's Wall), Cold Snap (Endless Winter) |
| Flamestrike | Blizzard (Scorched Earth), Scorch (Pyre), Frost Nova (Frozen Pyre), Combustion (Combustion Wave) |
| Cone of Cold | Frost Nova (Deep Freeze), Blizzard (Winter's Wall), Frostbolt (Ice Spray) |
| Blink | Frost Nova (Frozen Escape), Arcane Explosion (Arcane Exit), Arcane Power (Arcane Speed), Fireball (Blazing Retreat) |
| Counterspell | Arcane Missiles (Arcane Lockdown), Frost Nova (Frozen Silence), Fire Blast (Spell Lash), Presence of Mind (Quick Thinking), Cold Snap (Cold Calculation) |
| Pyroblast | Fireball (Pyromaniac), Scorch (Searing Pyre), Presence of Mind (Mind of Fire), Frostbolt (Frozen Fire) |
| Combustion | Fire Blast (Flash Point), Flamestrike (Combustion Wave) |
| Ice Barrier | Frost Nova (Frost Barrier), Cold Snap (Winter's Resolve) |
| Cold Snap | Ice Barrier (Winter's Resolve), Blizzard (Endless Winter), Counterspell (Cold Calculation) |
| Arcane Power | Arcane Missiles (Arcane Torrent), Blink (Arcane Speed) |
| Presence of Mind | Pyroblast (Mind of Fire), Counterspell (Quick Thinking) |
