# Elwynn expansion plan: "The Roads Are Sold"

Stages are ticked in their headings as each PR lands. Designed 15 Nov 2026 from origin/main @738a62ae. Approved for building overnight by the main session.

Defaults for Corey's open questions, all reversible:
- The story spine is the Brotherhood selling Elwynn's roads to the Horde.
- The invented names stay.
- Stitches does NOT attack stations.
- New goals go just before "Every corner".
- Talking has no choices yet.
- Bile uses a green ground spell look if one reads; otherwise Fire, which is flagged for Corey.

## What exists

- **Map.** It already covers the whole zone (`htl-map/src/elwynn.rs`, x -10160..-8580, y -1700..960), with 34 `[[site]]`s. Three gaps are empty:
  - the south riverbank, x~-10060, y -400..-1300;
  - the woods between Eastvale and Blackfang's Den, (-9660, -1200);
  - the woods east of Jasperlode, (-9260, -800).
- **Quests from content alone.**
  - The journal is `[[goal]]`, one linear list. Kinds: kill, open, explore (site key), raid, clear. Counts are camp-wide.
  - Chest `after = <mob>`: the chest stays shut until that mob is killed.
  - Pack `wake = <chest>`: the pack stays unseen until that chest is opened, then is set on the opener.
  - `route` makes a patrol. `rare = true` announces the mob's fall.
  - A chest's `text` is its story.
- **Bosses in the overworld** (`sim/bosses.rs`).
  - Phases: `[[mob.phase]]` with summon, speed, might, shove, burn, run_to (+ wait_for / arrive_text), debris, rocks_round.
  - Second form: then / then_after / then_text.
  - Other fields: unparryable, posture, temper (commander, coward, bruiser...), aggro_text, death_text.
- **NPCs.** Camp NPCs are `[[prop]]` with `npc = true`: Hale 1299, Tobin 1357, Garrick Stone 1500, Wren 1439, Maren 1295, Marten 1364, Wyle 1276, sentries 1499. They grow with `after`. No gossip exists in Survival.
- **Dialogue window.** `Event::Letter` (parchment: title, text, command links) is shown by the add-on's `Letter.lua` as the `NT` line.
- **Dungeons.** They hook in only through the camp's book. The Smuggler's Logbook opens the Deadmines; the book's ladder opens SFK after two first-tier clears.
- **Models.**
  - Defias: thug 2323, footpad 2338, conjurer 2345, lookout 2357.
  - Worgen: Wolfguard 202, Darksoul 657, Moonwalker 729, worg 557, Razorclaw 104.
  - Objects: chest 259, lockbox 5743, armory 41, scroll 163.

**Rules.**
- Every boss has its own story and "a rule a hero learns". No stock 50%-adds or 25%-enrage.
- Every warning must read with the floor markers off (`docs/floor-markers.md`).
- Player text follows `docs/writing-style.md`.
- **Journal safety.** `camp.goal` is saved as an index. Put every new goal IMMEDIATELY BEFORE "Every corner", and raise that goal's count to the new number of sites.

## Stage 1: The camp talks [x] (small engine change, the base for the rest)

Built as planned, with these differences, found checking the plan against the code:
- `kill:` and `open:` read the camp's own counts (`kill:<mob>`, `open:<chest>`), which the sim already keeps.
- A person is usable only while a line holds, not whenever they have lines.
- `validate()` also refuses a condition whose name isn't real (a station, site, creature or chest), for props and talk lines.
- Talk lines are `[[prop.talk]]` tables after the prop. Links are `command~label` strings.
- The add-on `VERSION` and `ADDON_NEWEST` went to 69.

**Engine.**
- Add `PropDef.talk: Vec<TalkDef { after: Option<String>, text, links: Vec<String> }>`.
- An NPC prop with talk lines is usable; its `use_prompt` reads "Talk to".
- `interact` sends `Event::Letter { kind: "talk", title: name, place: subname, text, links }`, using the last line whose `after` holds.
- `camp_has` gains `kill:<mob>` and `open:<chest>`.
- `validate()` refuses an unknown condition prefix.

**Add-on.**
- In `Letter.lua`: `SEALS.talk = "seal_blue"`, and `FOOT.talk = "Talk to them again as the camp grows."`.
- Bump the add-on `VERSION` and the server's `ADDON_NEWEST` together (see AGENTS.md).

**Lines.** These are examples; add more for the other camp NPCs in the same voice.
- Hale, `built:cooking`: "Keep the chest fed and I'll keep you fed. Goldshire sends what it can spare, which is less every week."
- Garrick Stone, `built:smithy`: "Bring me Fargodeep copper and I'll give you an edge. The Warsong are sitting on the best of it."
- Scout Marten, `raids:2`: "The Darkspear came through woods I'd have sworn were empty. Someone showed them the way."
- Marshal Wyle, `raids:3`: "Three raids. Three roads. Every one the road we'd left bare. Someone is selling Elwynn by the mile. Start at the Lion's Pride."

**Files.**
- `content.rs`
- `sim/explore.rs` (camp_has, talk)
- `sim/world.rs` (interact, use_prompt)
- `sim/mod.rs` (object view: usable)
- `content/survival.toml`
- `addon/ElwynnSurvival/Letter.lua`
- `docs/elwynn-survival.md`

**Tests** (in a new `tests_story.rs`).
- An NPC with talk lines is usable and prompts "Talk to"; one without them isn't.
- The line changes after `raids_won` rises and after a `kill:` count is set.
- `camp_has("kill:hogger")` goes from false to true.
- `validate` rejects `"bogus:x"`.
- A content test checks that every `after`, `wake`, goal `what` and talk `after` names a real mob, chest, site or station.
- Run `scripts/check-addon.sh` if it can run.

**Risk.** Only the use key reaches an NPC, not right-click. Leave right-click out.

## Stage 2: The Brotherhood's Hand [x] (questline, content only)

Built as planned, with these differences, found checking the plan against the code:
- The goals sit before "Every corner", which was goal 34 of 34 in `survival.toml` (the dungeon files' goals follow it). The save loader already finds a goal by its title. For a save with no title (a finished journal, read by its place) `ELWYNN_GOALS_AT` and `ELWYNN_GOALS` in `save.rs` carry the places past the new goals along. Raise `ELWYNN_GOALS` by the goals each stage adds. A test checks every old goal still loads as itself.
- "Every corner" was set to 35 while there were 34 sites, so it could not be finished. It is now 35 for the 35 sites, and a test holds it to the number of sites.
- `then`, `run_to`, `wait_for`, `arrive_text`, `wake`, `route`, `rare` and `burn` all exist as the plan says. A phase's `run_to` points take a height (x, y, z), so the ford's two points carry their ground heights, read from the real land with `elwynn_probe`.
- Voss's "rule" is a `commander` temper (holds back while others of his fight are on the hero), and his turn at 35% is a `[[mob.phase]]`.
- The boat is a yell and a place at the water. No boat model was used.
- Placement: the plan's (-10040, -480) is 160 yd from the river, so the ford is at (-9950, -530), 20 yd from the water's edge. Probed against the real land: nothing flagged.

**The chain.**
1. **Defias Messenger** (`bh_messenger`, 2338, not elite, respawns after 900 sec). It patrols with `route` from Goldshire (-9460, 40) to Jerod's Landing (-9940, -145).
2. **The Messenger's Drop** (lockbox 5743) at Jerod's dock, about (-9935, -150), with `after = "bh_messenger"`. Text: "Orders in a Defias cipher, and a map of Elwynn's roads. Westbrook's three are marked in red, with a price beside each."
3. **New site, Smugglers' Ford** (`kind = "foes"`), about (-10040, -480). It holds thugs, lookouts and a conjurer, plus props: fires 396 and a wagon 3678.
4. **The Paymaster's Strongbox** (259) with `ambush = ["bh_paymaster", ...guards]`.
5. **Paymaster Voss** (mini-boss).
   - Look and kit: 2323 at scale 1.15, a one-hander with a dagger in the off hand, `temper = "commander"`.
   - Rule: "Kill the hirelings or him, and don't wait."
   - At 35%, a `run_to` phase across the ford with `wait_for = 10`; he makes his last stand at his boat.
   - Lines: aggro "You're not on the list.", phase "Hold them. You're paid till sundown.", arrive "The boat leaves without me, then."

**Goals.** "The messenger" (kill), "The drop at the dock" (open), "The Smugglers' Ford" (explore), "The Paymaster" (kill).

**Talk.** Wyle, at `kill:bh_paymaster`: "The Brotherhood paid the Horde's way, and the orders came from below Moonbrook." Link: `.es lore <deadmines key>~The Deadmines`.

**Tests.**
- The drop stays shut until the messenger dies.
- Opening the strongbox wakes the Paymaster on the opener.
- At 35% he runs to his last point and fights again once a hero is within 10 yd.
- The goals advance in order.
- The ambush waits again after the chest refills.

**Risk.** The ford's coordinates are unchecked. Run `elwynn_probe` if the WoW data is present; otherwise say so in the report.

## Stage 3: Nightbane Crossing (new region, mini-boss, SFK hook) [x]

Built as planned, with these differences, found checking the plan against the code:
- A rare pack with a second form announced the first form's fall, and put the second form back as a creature of its own. In `mobs.rs` the fall is now told only when no form rises, and what comes back is the first form (`form_of`), after its own respawn time, once the last form is dead.
- Fenros's `then` form is `fenros_risen`, which holds the trophy and the goal's kill. Only the risen form is a `boss` (with the health bar): a parry makes a boss fill its posture instead of reeling, so the first form is not one, and a parry on him cuts his string short as the plan says.
- Watcher Dodds uses the Westbrook sentry's display (1499). No creature viewer was available to pick a better one.
- The crossing's site is of kind `foes`. `ELWYNN_GOALS` in `save.rs` is 6. "Every corner" is 36 for the 36 sites.
- Placement is at the plan's (-10060, -1100) and (-9660, -1200), the third spot (-9830, -1175), all open ground by `elwynn_probe`.

**Locations.**
- New site **Nightbane Crossing**, about (-10060, -1100).
- A `[[camp]]` "the Nightbane Crossing", taken by killing every worgen there.
- Fenros wanders, using `spots`, between there and (-9660, -1200).

**Creatures.**
- Nightbane Prowler (657), temper backstabber.
- Nightbane Shadow Weaver (729), temper caster.
- Bleak Worg (557).

**NPC: Watcher Dodds of the Night Watch.** A prop with `after = "found:nightbane"`. Find a display with npcfind, or reuse a human guard display already in the repo.
- First line: "The Night Watch holds Darkshire, barely. The worgen cross here now, and Fenros leads them."
- After Fenros dies: "Fenros got up once. They do that. These wolves came south from Silverpine, from a keep your book should name." Link: `.es lore shadowfang~Shadowfang Keep`.

**Rare boss: Fenros** (202, scale 1.3, `rare`).
- First form: quick claw strings (parry the first and the rest never come) and a leap from range.
- Then `then = "fenros_risen"`, `then_after = 4`, `then_text = "Not... yet."`.
- The risen form is `unparryable`, slower and heavier.
- Rule: "What you learned is taken away: parry the first form, dodge the second."

**Goals.** "The Nightbane Crossing" (clear), "Fenros" (kill fenros_risen).

**Tests.**
- The camp is taken when its packs die.
- The second form rises after 4 sec with its yell.
- A parry against the risen form fails.
- The lore link names a real dungeon key.

## Stage 4: Stitches on the Goldshire road (world boss, content only) [x]

Built as planned, with these differences, found checking the plan against the code:
- `boss`, `respawn` and `unparryable` are creature fields (the pack only takes `rare` and the `route`). `burn` is a field of a `[[mob.phase]]`, as is the line he says at 50%.
- The display is 1693, the big Flesh Golem (`mobfind`), which is the Duskwood abomination's model. No creature template was available to check it is the one creature 412 wears.
- The route is read off the real road (the probe's drawing of it), from the bridge at (-9744, 470) to (-9474, 120) north of Goldshire's crossroads. Every point is 209 yd or more from the camp, and open ground by `elwynn_probe`.
- The bile is Fire's look (the flames are the bile). No green ground look could be seen from here, so this is flagged for Corey. Every blow of his is `heavy`, so every one leaves bile once he burns.
- The thrown cleaver uses the Hurled Axe's missile look (spell 16075).
- `ELWYNN_GOALS` is 7. No site was added: "Every corner" stays 36.

**Location.** A pack with a `route` along the cobbled road from the south bridge toward Goldshire's crossroads, kept more than 110 yd from the camp. Settings: `rare`, `boss`, `respawn = 2400`. Use Stitches' own display (creature 412).

**Mechanics.**
- `unparryable`, posture about 320.
- A blockable cleaver chop.
- A 360 degree gut swing, glowing red when crowded (`blockable = false`; dodge it).
- A lurching charge (lunge) from range.
- A thrown missile.
- At 50%, `burn`: his heavy blows leave bile where they land.
- Rule: "Too big to parry. Every heavy blow leaves bile, so keep him walking."

**Goal.** "Stitches" (kill).

**Tests.**
- He moves along the route with a hero near.
- His fall is announced.
- Fire or bile objects appear where his heavy blows land after 50%.
- A parry fails against him.

## Stage 5: The fifth raid, "The Brotherhood's Price" [ ]

**The raid.** A new `[[raid.tier]]`.
- `needs = ["barracks", "infirmary"]`, `from = [-9690, 420]`.
- Raiders: defias_thug x5, footpad x3, lookout x2, conjurer x2, and a champion.

**Champion: Brannick the Mason.**
- Look and kit: 2323, scale 1.3, a poised two-hand maul, `boss`, `turn_rate` about 90.
- A 360 degree stomp at 3.5 yd or less, to punish hugging. At 50%, `burn` (pitch).
- Rule: "He turns slowly: work round him for backstabs, but not too close."
- Lines: aggro "I built this wall. I know where it breaks.", phase "Pitch on the stones, lads."

**Camp growth.** Gate it on `after = "kill:brannick"`, not `raids:5`. It brings a Stormwind stonemason NPC with talk lines.

**Goal.** "The Brotherhood's Price" (raid, what = "4").

**Tests.**
- `plan(4)` is the new tier.
- `plan(5)` gets +1 raider each and the title suffix "(2)".
- A hero circling him ends up at his back, and the backstab multiplier applies.
- The props stand only after the kill.

## For every stage

- Keep each stage in its own PR, with its TOML in one block under a header comment.
- Update `docs/elwynn-survival.md`: the places table, rares, the camp-growth table and the raid list.
- Label every PR "needs playtest". NO filming.
- Run `elwynn_probe` if the WoW data is present; otherwise list placements as unverified.
- No new client patch. All models are stock 1.12.
