# Writing for Elwynn Survival

How the words in the game read: window notes, buttons, hints, tooltips, chat messages,
announcements, the journal, the map and every `text` in `content/`. Follow it for anything a
player reads. Docs, comments and commit messages keep their own style (AGENTS.md).

The game sits on World of Warcraft 1.12 and fights like a soulslike, so it borrows from both:
**WoW's interface grammar for anything that does something**, and **short Souls-style flavor for
anything with a story**. Every line has one job, and its job decides how it's written.

## The four kinds of text

| Kind | Where | How it reads |
| --- | --- | --- |
| **Interface** | Window titles and notes, buttons, click hints, prompts, errors | Imperative, verb first, plain. Short. No story. |
| **Stats** | What an item, ability, station or building does | WoW tooltip lines: one effect per sentence, numbers as numerals. |
| **Flavor** | The story under a name: a chest's find, an item's past, a place | One line, at most two. Concrete, a little grim, never a mechanic. |
| **News** | Announcements, chat from the camp, the journal, banners | Who did what, or what to do next, in one plain sentence. |

A single `text` that needs both stats and flavor puts the stats first, then the flavor in quotes:

> Heavy attacks break stances 50% faster. "Mr. Smite never asked a door twice."

## Interface text

- **Verb first, imperative.** "Bank all", "Take up", "Place", "Repair". Not "Bank all you carry".
- **Buttons: one or two words, Title Case.** Upgrade, Move, Tear Down, Repair, Ring It, Not Yet.
- **Window notes: one or two short sentences.** Say what the window is for and how it works.
- **Click hints use the WoW pattern**: "Click to take 1. Shift-click for 5. Right-click for all."
- **Errors say what's wrong and, if there's a fix, what to do:** "Not enough Elwynn Timber (need 6,
  have 2)." "Too close to the Cooking Fire." "Not during a raid."
- **Keys stay as the server writes them, in brackets:** "(H)", "(F)". The add-on swaps in the key
  as it's bound now, so never spell a key out in words.
- **No semicolons, no colon-led fragments, no parenthetical asides** beyond a key or a number.

## Stats

- **Numbers, always.** "50% faster", "+20% health", "for 3 sec", "within 9 yd", "every 60 sec".
  Never "half again", "a fifth more", "twice as soon", "a tenth".
- **Units the way WoW writes them:** sec, min, yd, %. "1.5 sec", not "1.5 s" or "a second and a half".
- **One effect per sentence**, the strongest first. Name what's affected ("Heavy attacks",
  "Resting nearby", "Raiders who touch it").
- **Present tense, no "you" unless it's needed.** "Deals 36 damage to every enemy in a line." For
  an ability's how-to, "you" is fine: "Hold to charge. Release to throw."
- **Game terms are capitalized when they're the game's**: Valor, Draughts, stance, EX. The
  rest stay lowercase.

## Flavor

- **One concrete image, not a summary.** *"Wrapped in spider silk. Still warm."* beats "A grimoire
  bound in black that you found at Jasperlode."
- **No mechanics in flavor**, and no "you can". If it matters to play, it's a stat.
- **Proper nouns as the world has them**: the Defias, the Riverpaw, Westbrook, Mr. Smite.
- **No winks at the player**, no modern words, no exclamation marks.

## News

- **Who did what, past tense:** "Ayla built a Cooking Fire." "Bren tore down the Thorn Hedge."
  "The Warsong Raid is beaten. Spoils sent to storage: 6 copper, 6 timber."
- **The journal is quest objectives**: an imperative, then a short reason or where. "Build a
  Cooking Fire in camp (H)." "Bank copper from the Fargodeep mine yard."
- **The camp growing** speaks like a narrator, one sentence: "A quartermaster arrives from
  Goldshire with a wagon of supplies."
- **Banners**: a title of two to four words, and a short line under it.

## The word list

Use these words, and only these, for these things.

| Say | Not |
| --- | --- |
| **Storage** (the camp's chest, in the interface) | the camp's chest, the chest |
| **Bank** (put in storage) | put in the chest, goes in the chest |
| **Take** (from storage) | take out, take back |
| **Equip** / **Equipped** | put on, wear, take up, in hand |
| **Build** / **Place** (a station) | raise, put down, set down |
| **Upgrade** / **Move** / **Tear Down** | make it a, pick up, demolish |
| **Repair** / **Damaged** | mend, wrecked |
| **Station** (a workplace), **Defense**, **Building** | the camp's things |
| **Raid**, **raiders** | settlement attack, the Horde's attack |
| **Camp** | the settlement |
| **Hero** (another player), **you** (the player) | footman, survivor (except in flavor) |
| **Enemy** (in stats), **foe** (in flavor only) | creature, mob |
| **Damage**, **health**, **stamina** | blow (in stats), wound |
| **Gather** | chop, pick (except in flavor) |
| **Respawn** | get up, come back |
| **Ability**, **abilities** (what goes on a key; **spell** for a caster's) | card (Elwynn Survival; Basin Battle's run-cards are cards) |

US spelling throughout (defense, armor, color), as WoW 1.12 has it.

## Before and after

| Before | After |
| --- | --- |
| What everyone banks for the camp. Click a thing to take one, Shift-click for five, right-click for all of it; click gear to put it on. | Shared by the whole camp. Click to take 1, Shift-click for 5, right-click for all. Click gear to equip it. |
| Click: take it up (what you hold goes in the chest) | Click to equip. Your current weapon goes to storage. |
| Bank all you carry | Bank All |
| Mr. Smite's: the heaviest swing there is, and it breaks a stance half again as fast. | Heavy attacks break stances 50% faster. "Mr. Smite's own. The heaviest swing there is." |
| The Warlord's helm: takes a tenth off every blow, and hardly slows the stamina. | Reduces damage taken by 10%. Slows stamina regeneration by 3%. "Warlord Gorzak's." |
| Cots and a watch roster: heroes who fall are up again twice as soon, and with a fifth more of their health. | Heroes respawn 50% faster, with +20% health. |
| Stations and defences stand in the yards round the camp's fire. Materials come out of your bags first, then the camp's chest. | Build stations and defenses in the yards around the campfire. Materials come from your bags first, then storage. |
| Moonfeather, the great owl: the druid's trial. | The druid's trial. "Moonfeather has watched these falls for a hundred years." |
| Ayla builds a Cooking Fire. | Ayla built a Cooking Fire. |

## What never changes

The words are free to change; these are not:

- **Keys and protocol:** item, station, ability and site `key`s, command words (`.es bank`),
  line verbs and their fields (`ui.rs` lists them). Never put `~`, `^` or a tab in text.
- **Names** of items, places and characters, unless one reads wrong. A save keeps keys, not
  names, but the journal and tests often match names.
- **Key brackets** like "(H)", which the add-on rewrites.
- **Tests** that match a message: change the test with the message, in the same commit.
