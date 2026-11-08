# Enemy temperaments (draft, Nov 7)

Corey, Nov 7: "not all enemies need to behave the exact same way … depending on the enemy their
behavior should completely change to be more immersive and fun gameplay."

Today every creature shares one `[ai]` table: one turn rule, one flank rule, one chance to give
ground. A **temperament** is a named set of those choices, and each creature picks one
(`temper = "brawler"` on its `[[mob]]`). The fairness rules stay for all of them: no blow too quick
to answer, a blade and a bolt never together, nothing on a reeling hero.

## The temperaments

| Temperament | How it fights | What it teaches |
| --- | --- | --- |
| **Brawler** | Comes straight in, takes its turn, punishes a greedy swing, now and then feints. | Parry, and don't over-swing. |
| **Backstabber** | Never fights you face to face if it can help it. Waits for you to be busy with another enemy, then circles to your back and strikes. Gives ground after its flurry. | Watch your back; turn to face it. |
| **Bruiser** | Plods forward and never backs off. A light hit doesn't stagger it. Slow, heavy blows. | Roll the heavy blows; don't trade. |
| **Coward** | Swarms with others. Below a third of its health it runs for help, or flees. | Finish the hurt ones before they bring friends. |
| **Commander** | Holds back while its pack fights. Its crack of the whip hastens the pack near it. Comes in when the pack thins. | Kill the leader first, or fight a faster pack. |
| **Caster** | Keeps its distance and casts between the blows up close. Pressed in melee, it steps back or blinks away. | Close the gap; bash the cast. |
| **Skirmisher** | Keeps range and shoots or throws. Closed on, it kicks you off and backs away. | Corner it; use cover against its shots. |

## The Deadmines roster

| Creature | Temperament |
| --- | --- |
| Defias Pirate | Brawler |
| Defias Blackguard | Backstabber |
| Goblin Woodcutter | Bruiser |
| Defias Miner, Strip Miner | Coward |
| Defias Overseer, Taskmaster | Commander |
| Squallshaper, Wizard, Evoker | Caster |
| Crossbowman, Dynamiter, Goblin Engineer, Craftsman | Skirmisher |
| Captain Greenskin | Brawler, a captain's pace |

Bosses get their own on top of their strings:

| Boss | How it fights |
| --- | --- |
| Mr. Smite | Bruiser: relentless, no retreat |
| Edwin VanCleef | A duelist: punishes every greedy swing, ripostes, steps out to make you come to him |
| Cookie | Erratic: wanders his kitchen between pots, hard to pin down |
| Gilnid | Bruiser who backs off to pour |
| Rhahk'Zor | Bruiser who roars and rushes |
| Marisa | Caster with a blade |
| Miner Johnson | Coward who blasts and runs |

## What each temperament sets

Each is a `[[temper]]` table in `survival.toml`. Anything it leaves out falls back to `[ai]`.

| Setting | What it does |
| --- | --- |
| `punish` | Comes at an opening, its turn or not. |
| `waits_turn` | Takes turns with the others on the same hero. |
| `flank` | While waiting, works round to the hero's back. |
| `backstab` | Prefers a hero who faces another enemy. |
| `retreat` | Chance to give ground after a string, and how far. |
| `poise` | A light hit doesn't stagger it. |
| `flee_below` | Health share at which it runs for help or away. |
| `rally` | A whip crack that hastens the pack near it. |
| `keep_away`, `kite` | How far off it stays, and whether it backs away when closed on. |

Not built yet: this note is for Corey's go-ahead and his changes to the roster.
