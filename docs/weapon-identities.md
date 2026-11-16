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
`[[mob]]`'s `tags` (`undead`, `worgen`, `stone`, `mechanical`).

| Weapon | Mechanic | Field |
| --- | --- | --- |
| Footman's Sword | Parry and riposte (the duelist) | `parry`, `riposte` |
| Silver Sword | 50% more damage to undead and worgen; they stop mending and stay down once slain | `bane`, `bane_damage`, `silver` |
| Cruel Barb | Its heavy hooks the foe and pulls it 6 yd to you | `hook` |
| Pirate Cutlass | Its heavy is a 5 yd boarding leap, a gap closer | `boarding` |
| Sneed's Machete | No backstab; each light hit chops every foe in a 5 yd line | `no_backstab`, `line` |
| Cursed Felblade | The last blow of the light chain carries you behind the foe | `blink_behind` |
