# Weapon identities (Nov 7)

Corey, Nov 7: "If the Maul is good as is, we need to give identity to the other types of
weapons … we need 1h differences ofc too … love the idea of no blocking with dual wielding as
right click can now become your off hand attack. Combo's!"

## Where they stand

Every one-hander plays the same: 21 to 24 damage a second on one foe, a shield always in the other
hand (`Player.shield` is never empty), and only the mace with a trait (a 0.8 s stagger). The
two-handers are the only weapons with an identity, and the Maul's (48 to 60, three foes, a 1.2 s
stagger) is the one Corey likes.

## The identities

| Weapon | Identity | What makes it so |
| --- | --- | --- |
| Sword | The duelist's blade | A wider parry window (+0.1 s) and a riposte that hits 1.5 times as hard |
| Axe | The bleeder | Hits 2; each hit bleeds the foe for 3 seconds |
| Mace | The stance breaker | Its hits count twice towards breaking a foe's stance; its 0.8 s stagger |
| Dagger | The assassin | Fastest; 2.5 times the damage from behind or on a staggered foe |
| Spear, halberd | Reach | 5 yd; its thrust goes through a creature's raised shield |
| Greatsword | The sweeper | Widest arc, up to 4 foes; a light blow on you doesn't break its swing |
| Warlord axe | The executioner | Hits 3, staggers 1 s; twice the damage on a foe under a third of its health |
| Maul | Unchanged | Hits 3, staggers 1.2 s, breaks stance |

## The off hand

What's in the off hand changes how the right click fights:

| Off hand | Right click | Identity |
| --- | --- | --- |
| A shield | The guard: block, and parry on the moment | The wall: cheapest, widest block, the bash that breaks wind-ups |
| Nothing (1h alone) | The weapon's own guard: a costly block, but a parry window 0.1 s wider and a riposte 1.5 times as hard | The duelist: timing over safety |
| A second one-hander (dual wield) | An off-hand attack. No block, no parry: dodge or take it | The aggressor: longer combos, the most damage a second |

Dual wield combos: the light (left) and off-hand (right) attacks each have their own chain, and
alternating them builds a combo. Left, right, left, right ends in a finisher with both blades
that hits harder and staggers. Each off-hand hit costs less stamina than a main-hand hit.

## Building it

1. The identities of the weapons that exist (`[[weapon]]` fields, some new: `bleed`,
   `through_guard`, `swing_poise`, `execute`, `riposte`).
2. The off hand as its own slot: a shield, nothing, or a one-handed weapon. The rack equips either
   hand; saves keep `shield` and add `off_hand`; the client shows the off-hand weapon.
3. Dual wield: the right click as an off-hand attack with its own chain and the alternating
   finisher.

## The identity pass (Nov 15)

Corey, Nov 15: every weapon on the rack must be different in effect, animation or gameplay, not
just in damage a second; the ones with no purpose go. Done as a series of pull requests: removals,
a moveset per family, a unique effect per kept weapon, then a test bar that keeps it so.

### Removed

| Weapon | Was | Old saves get | Its foes |
| --- | --- | --- | --- |
| Apprentice's Staff | a staff with no bolt | Ember Staff | (no dungeon source) |
| Squallshaper's Staff | a polearm in staff clothing | Goldshire Halberd | the Squallshaper no longer carries it |
| Goblin Woodcutter's Axe | a traitless 1H axe | Footman's Axe | the Woodcutter no longer carries it |
| Engineer's Wrench | a stagger-only mace | Militia Mace | the Engineer and the Craftsman no longer carry it |

Saves are migrated in `save.rs` (`RETIRED_WEAPONS`): in hand, off hand, bag, chest, cache, honed copies.

### Kept for the unique-effect pass

Each of these is a boss's only weapon reward, so it stays (a boss never loses its weapon) and gets
a fitting unique effect in the third step instead of a removal: Fallen Champion's Greatsword,
Corpsemaker, Guillotine Axe, Loksey's Boar Spear, Cursed Felblade, Ironspine's Fist, Thornspike,
Cobrahn's Fang.

### How the families play (step 2)

Each family swings its own `[[moveset]]` (a weapon's `moveset =`), so the families read differently
before any trait does. The sets keep their family's damage a second (a slower step hits harder by
about as much as it takes longer; `a_family_set_keeps_the_familys_damage_a_second` pins it within 5%).

| Family | Set | How it plays |
| --- | --- | --- |
| 1H axe | `axe_one_hand` | Wide hacking cleaves: three chops stepping further in, the last a two-hander's wide swing (the grip's timing: the Footman's Axe is the starting weapon and the drills are tuned to its pace) |
| 1H mace | `mace_one_hand` | Overhead slams, every step slower and heavier than the last |
| 2H axe | `axe_two_hand` | Big diagonal cleaves, the widest closing the chain |
| Maul, hammers | `maul` | A slow overhead slam every blow, the longest coil on the heavy |
| Sword, dagger, greatsword, polearm, staff | the grip's | Unchanged |

### Each weapon's own mechanic (step 3)

A mechanic, not a number: the thing this weapon does that no other does. The fields are on
`WeaponDef`, what they do is in `sim/weapon_fx.rs`, and the tooltip's sentence is built from the same
numbers (`effect_text`, filled into the item's `{fx}`). The creature kinds a bane names are a
`[[mob]]`'s `tags` (`undead`, `worgen`, `stone`, `mechanical`). Tests keep it so
(`tests_weapon_identity.rs`): every weapon but a staff and the kept ones has a mechanic and not just
a bare stagger or posture, no two weapons swing the same set with the same traits, and this table
names every weapon and every mechanic.

Kept as they were: Footman's Sword, Axe, Claymore, Militia Mace, Poacher's Knife, Quarry Maul,
Gorzak's Cleaver, Goldshire Halberd and Zanjin's Spear, and the six staffs.


#### Swords

| Weapon | Key | Mechanic | Fields |
| --- | --- | --- | --- |
| Footman's Sword | `sword` | The duelist: a wider parry window and a riposte 1.5 times as hard | `parry`, `riposte` |
| Silver-Edged Longsword | `silver_sword` | 50% more damage to undead and worgen; they stop mending and stay down once slain | `bane`, `bane_damage`, `silver` |
| Cruel Barb | `cruel_barb` | Its heavy hooks the foe and pulls it up to 6 yd to you | `hook` |
| Pirate's Cutlass | `pirate_cutlass` | Its heavy is a 5 yd boarding leap, a gap closer | `boarding` |
| Sneed's Machete | `sneed_machete` | No backstab; each light hit chops every foe in a 5 yd line | `no_backstab`, `line` |
| Cursed Felblade | `cursed_felblade` | The last blow of the light chain carries you behind the foe | `blink_behind` |

#### One-hand axes

| Weapon | Key | Mechanic | Fields |
| --- | --- | --- | --- |
| Footman's Axe | `axe` | The bleeder: each hit bleeds the foe | `bleed` |
| Bearded Axe | `bearded_axe` | Its heavy hooks a shield aside: the enemy can't block for 4 sec | `guard_hook` |
| Bloodlust Edge | `bloodlust_edge` | Each kill speeds your swings 8%, up to 4 times, for 6 sec | `bloodlust` |
| Defias Miner's Pick | `miner_pick` | A posture breaker; 50% more to stone, golem and mechanical foes | `posture`, `bane` |

#### Maces

| Weapon | Key | Mechanic | Fields |
| --- | --- | --- | --- |
| Militia Mace | `mace` | The stance breaker: its hits count twice towards breaking a stance | `posture` |
| Hogger's Cudgel | `hogger_club` | Its heavy knocks the foe 5 yd back | `knock` |
| Cookie's Tenderizer | `cookie_tenderizer` | A hit tenderizes: the next hit on the foe from another ally deals 25% more | `tender` |
| The Smelter's Spanner | `gilnid_wrench` | Its heavy stuns mechanical foes and machines 4 sec (not a champion machine) | `machine_stun` |
| Hand of Righteousness | `hand_of_righteousness` | Each hit heals the most wounded ally within 15 yd, you too, 3% | `mend_ally` |
| Baron's Scepter | `barons_scepter` | A hit curses the foe; a cursed foe that dies bursts in shadow on those within 5 yd | `curse` |
| Ironspine's Fist | `ironspine_fist` | Each hit gives a 5 point shield, stacking to 30 | `iron_skin` |

#### Daggers

| Weapon | Key | Mechanic | Fields |
| --- | --- | --- | --- |
| Poacher's Knife | `dagger` | The assassin: 2.5 times the damage from behind | `backstab` |
| Hypnotic Blade | `hypnotic_blade` | Hits make the foe lose interest in you for 4 sec | `lull` |
| Meteor Shard | `meteor_shard` | A backstab bursts in fire (50%) on foes within 4 yd and is a shatter trigger | `shatter`, `burst` |
| Bazil's Shiv | `bazil_shiv` | Hits poison 3 a sec a stack, up to 5 stacks, for 6 sec | `poison` |
| Thornspike | `thornspike` | Hits interrupt a casting foe and stagger it 1.5 sec | `interrupt` |
| Cobrahn's Fang | `cobrahn_fang` | A backstab puts the foe to sleep for 3 sec; damage wakes it | `sleep` |

#### Two-handers

| Weapon | Key | Mechanic | Fields |
| --- | --- | --- | --- |
| Footman's Claymore | `greatsword` | The sweeper: widest arc, up to 4 foes; a light blow on you doesn't break its swing | `swing_poise` |
| Fallen Champion's Greatsword | `champions_greatsword` | Its heavy rallies allies within 8 yd (you too): 15% less damage for 5 sec | `rally` |
| Quarry Maul | `maul` | Unchanged: breaks stances, 3 foes, a 1.2 sec stagger, and the maul's slow overhead set | `stagger` |
| Thornroot Maul | `thornroot_maul` | Its heavy roots the foes it hits for 2.5 sec | `root` |
| Mograine's Might | `mograines_might` | Its heavy sends a holy shockwave ring (6 yd) for 0.8 of the blow | `shockwave` |
| Smite's Mighty Hammer | `smite_hammer` | The 3rd hit in a row on one foe staggers it 1.5 sec | `build` |
| Rhahk'Zor's Maul | `rhahkzor_maul` | Its heavy marks the spot; after 1 sec rocks fall for 0.6 of the blow within 2.5 yd | `rockfall` |
| Gorzak's Cleaver | `warlord_axe` | The executioner: twice the damage on a foe under a third of its health | `execute` |
| Targorr's Cleaver | `targorr_axe` | Up to 80% more damage as your health falls | `rage` |
| Corpsemaker | `corpsemaker` | Its hits cow the foe: 25% less damage and slower wind-ups for 4 sec | `cow` |
| Guillotine Axe | `guillotine_axe` | Its heavy does triple damage to a stunned, asleep, rooted, frozen or staggered foe | `guillotine` |

#### Polearms

| Weapon | Key | Mechanic | Fields |
| --- | --- | --- | --- |
| Goldshire Halberd | `halberd` | Reach: 5 yd, and its thrust goes through a raised shield | `through_guard` |
| Zanjin's Spear | `zanjin_spear` | The longest reach on the rack (5.4 yd) and the same thrust through a shield | `through_guard` |
| The Horseman's Scythe | `horsemans_scythe` | A 200 degree reaping sweep of up to 5; each foe struck heals you 1% | `reap_heal` |
| Loksey's Boar Spear | `loksey_boar_spear` | Its thrust slows the foe 40% for 2.5 sec | `snare` |

#### Staffs

| Weapon | Key | Mechanic | Fields |
| --- | --- | --- | --- |
| Ember Staff | `ember_staff` | Casts Ember Bolt; guards with a ward | `bolt` |
| Rime Staff | `rime_staff` | Casts Rime Bolt; guards with a ward | `bolt` |
| Arcane Staff | `arcane_staff` | Casts Arcane Bolt; guards with a ward | `bolt` |
| Acolyte's Staff | `acolyte_staff` | Casts Smite; right click aims Healing Gust | `bolt`, `alt` |
| Shadowcaller's Staff | `shadowcaller_staff` | Casts a shadow bolt; right click channels Dark Reckoning | `bolt`, `alt` |
| Staff of the Hollow Star | `hollow_star_staff` | Tethers foes with the void; right click is the Hollow Star | `bolt`, `alt` |
