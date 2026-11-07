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
