# Warrior: every skill has a job

The third pass on the Warrior's cards. The feel pass made every card felt; this one makes every
skill *different*, gives rarity a meaning, and turns the weapon paths into decks of their own.
The full list of cards is in [warrior-cards.md](warrior-cards.md) (generated from the game's
files); why the cards feel the way they do is in [warrior-feel-pass.md](warrior-feel-pass.md).

## The rule: one job each, and you notice when it's missing

A skill's upgrades now make it better *at its own job*. Heroic Strike no longer bleeds or
taunts, Whirlwind no longer sunders or slows, Charge no longer sunders: those belong to other
skills. And the world now asks for each job, so going without one is felt (without making a run
impossible: everything can still be killed, just slower or at a cost).

| Skill | Its job | What you feel without it |
|---|---|---|
| Heroic Strike | The rage dump: **off the global cooldown**, weave it between everything; hits 3% harder for every 10 rage you hold (and 30% on bleeding enemies). | Rage piles up with nowhere to go. |
| Rend | The bleed: what Heroic Strike, Execute and Bloodthirst feed on. | Bleed bonuses sit idle. |
| Thunder Clap | The crowd breaker: everything it hits **attacks 30% slower** for 10 sec (and moves slower). | Packs hit you at full speed: a wave of skeletons hurts. |
| Whirlwind | The pack killer: **5% more for every enemy** caught after the first. | Packs take longer. |
| Sunder Armor | The armor breaker. **Elites shrug off 10% of every physical blow, bosses 20%**; each stack strips 4% of that for everyone (the hunter's arrows and the soldiers' swords too). | Elites and bosses take noticeably longer, for the whole team. |
| Execute | The finisher: after its cost it **spends up to 30 more rage**, each point 2.5% more damage. | Low-health enemies linger. |
| Charge | The opener: 25 yd, a stun, 15 rage, and defenders rally to your target. | Getting to the casters and runners takes longer. |
| Challenging Shout | The Ancient's guard: everything within 12 yd turns on you. | Leaks reach the Ancient. |
| Hamstring | The **lockdown**: one enemy, crippled for 10 sec: 70% slower, it must fight you (it can't run past you to the Ancient), and it takes **20% more damage from everyone**. | Runners get past you; the elite the team is burning takes longer. |
| Revenge | The counter: ready after you block or take a blow, hits two. | The warrior in front has less to hit back with. |
| Bloodrage | Rage now, for blood. | Slow starts. |
| Last Stand | The second life. | One bad pull ends you. |
| Mortal Strike *(Greatsword)* | The **wound**: the target **can't be healed for 10 sec**, and it hits 40% harder on enemies above 80% health: the opener on elites and bosses. | Dark Menders and boss rituals heal what you've hurt. |
| Sweeping Strikes *(Greatsword)* | The cleave window. | |
| Bloodthirst *(Berserker)* | The strike that heals. | |
| Death Wish *(Berserker)* | As in WoW: **30% more damage, 20% more damage taken** for 20 sec. Its upgrades lean in (Reckless Abandon) or steady you (Steady Nerves). | |
| Shield Slam *(Shieldbearer)* | The **interrupt**: if an enemy within 20 yd is casting something that can be broken, your shield flies to it and the cast is broken. | Soul Bolts land, Dark Menders heal their pack, bosses finish their rituals. |
| Shield Wall *(Shieldbearer)* | The fortress: 60% less damage. | |

Mortal Strike is no longer "another big hit": it is the answer to healing, and the opener.
Heroic Strike is the rhythm between other skills, Execute is the finisher that empties your rage.

## Enemy casts worth breaking

New enemy spells, cast with a cast bar and a warning (no marks on the ground):

- **Soul Bolt** (Skeletal Bonecaster): a 2.5 sec cast that strikes one hero hard (10% of their health).
- **Dark Mending** (Dark Mender): heals every enemy within 15 yd of the Mender by 12%.
- **Dark Ritual** (every boss, from 14 sec into the fight): heals the boss and everything near it.
- **Death Nova** can now be broken too.

A warrior holding Shield Slam sees every breakable cast in reach light up: the enemy's cast bar
says **INTERRUPT!** and pulses, and the Shield Slam key lights up until the cast lands or is
broken. Breaking one shows **INTERRUPTED** on the cast bar and the card's name over your bar;
the caster reels (stunned 2 sec), and its marks vanish. Players without an interrupt see only the
casts aimed at them, so it isn't noise for everyone else. Mortal Strike's wound stops Dark
Mending and Dark Ritual from healing what it cut.

## Rarity means something

- **Common**: numbers (25% a rank, a shorter cooldown, a lower cost).
- **Uncommon**: changes how you play (a choice, a new effect, a new target).
- **Rare**: build-definers: a key that lights up, a reset, a free cast.
- **Epic** *(new)*: one capstone per skill (Heroic Fury, Crimson Tempest, Earthquake,
  Bladestorm, Shatter, Massacre, Bulldoze, Rallying Bellow, Crippling Sweep, Avatar of
  Vengeance, Rage Unbound, Immortal Stand, Spellbreaker, Impenetrable, Mortal Calamity, Eternal
  Sweep, Blood Bath, Wish Granted), offered only after the skill has taken **5 upgrades**. Each
  weapon path has one too, after 4 of its cards.
- **Legendary** stays run-wide (Escalations, relics).

Skills themselves aren't rolled for rarity: they come when the bar needs them. The odds of the
better cards climb with level (Epics from level 10), boss and relic offers lean Rare and Epic,
and a **safety net** guarantees a Rare (or better) after 6 offers without one. Rare and Epic
cards have their own glow and sound.

## Weapon paths: random, and a deck of their own

A weapon path is still a level-up card that turns up at random (in a test of 40 runs, anywhere
from level 2 to level 60; most around the 20s). Taking one:

- re-arms you (greatsword, sword and shield, or two weapons) and gives the path's passives;
- **teaches nothing by itself**, but opens the path's two skills to the skill cards
  (Shieldbearer: Shield Slam, Shield Wall; Greatsword: Mortal Strike, Sweeping Strikes;
  Berserker: Bloodthirst, Death Wish). They ride on keys of their own (7 and 8), so a full bar
  never shuts them out, and they come soon after the path;
- opens the path's **10 path cards**: rank-ups, a choice (Bastion or Vanguard; Rampage or
  Bloodlust; Executioner or Colossus) and an Epic capstone (Unbreakable Bulwark, Frenzy,
  Titan's Grip) after 4 of them.

Without the path, its skills never show up: no Shield Slam without the shield.

## A team notices its warrior

- Sunder Armor strips armor for everyone hitting the target.
- Thunder Clap slows a whole pack's attacks, the ones hitting your allies too.
- Hamstring's cripple makes its target take 20% more from everyone; Rallying Mark adds 10%
  team damage.
- Shouts and stands can rally the team: Battle Shout, Commanding Shout, Rallying Bellow,
  Rallying Stand, Protective Wall, Blood Pact, Shared Fury; and the Shieldbearer's Guardian.
- Shield Slam breaks the casts aimed at your healer.

## Not done yet

- Only the Warrior has an interrupt; the other classes don't break casts yet.
- Armor makes physical damage weaker against elites and bosses for every class (hunters
  included) until someone sunders them.
