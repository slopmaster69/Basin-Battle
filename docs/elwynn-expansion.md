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

## Stage 5: The fifth raid, "The Brotherhood's Price" [x]

Built as planned, with these differences, found checking the plan against the code:
- `turn_rate` is the creature's `[mob.attack] turn_rate`, and it only slows his turning while he winds up (between blows a creature faces its foe at once). At the plan's 90 a hero could not be sure to get round him in one wind-up. It is 70, which the test shows works at the sprint's 12 yd/s.
- The stomp only punishes hugging if nothing else fits up close, so his maul and sweep have `min_range = 3.6` and the stomp `max_range = 3.5`. A creature stops 4.35 yd from its foe with this reach, so at its natural distance it uses the maul.
- The weapon is the black two-hand maul (display 1478). The mason wears the blacksmith's look (1500): no dressed human creature display with a mason's clothes turned up.
- `ELWYNN_GOALS` is 8; no site was added. The stonemason stands at (-9648.5, 716), open ground by `elwynn_probe`.

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

# Round two: "The Lion's Seal" (stages 6 to 10)

Designed 16 Nov 2026 from origin/main @23ff9db1, built from the plan's approved defaults. Stages are ticked as they land.

Defaults for Corey's open questions, all reversible:
- Lescovar is the buyer. That is our own addition: in vanilla his treason is with the Defias.
- The invented names stay: the Mason's Cairn, the Parley Stones, the Blackrock Envoy, the Silent Blade and the marked stones.
- Remy's choice is camp-wide and one-time, and any hero can make it.
- A true escort stays deferred (it needs a `Foe::Mob` target).

Story spine: Gruff's Riverpaw were paid in Brotherhood silver. A tally stick points to Brannick's cairn. His road book names "R. at the Lion's Pride". Remy "Two Times" sold the watch rosters to a man in black. The Stonefield wagon note names a parley. At the parley Marzon the Silent Blade sells the Redridge road to a Blackrock envoy, and his satchel carries Lord Gregor Lescovar's seal.

Journal: every new goal goes just before "Every corner". `ELWYNN_GOALS` in `save.rs` goes 8, 9, 10, 11, 12, 15. "Every corner" equals the site count (36, then 37 in Stage 7, then 38 in Stage 10). Every placement was probed with `elwynn_probe` against the real data in `C:\WoW\Data` (its `near` mode), and a failing spot was moved nearby. The tests are in `tests_story2.rs`, with `tests_story.rs`'s old-save, goal-order and sites-equal-corner tests extended.

## Stage 6: Gruff Swiftbite [x] (content only)

Built as planned, with these differences, found checking the plan against the code:
- `evade` is a creature field, and `evade_heavy` (in `sim/mobs.rs`) is the mechanism: it runs as a hero's heavy begins, for a creature in `Chase` with `stagger_until <= clock`, within 4.5 yd and 60 degrees of the hero's facing. A parried Gruff reels, so he cannot step back, exactly as the plan says.
- His goal is placed with the other goals before "Every corner", not in his block (a goal's place in the journal is its place in the file). `ELWYNN_GOALS` is 9.
- His spots are all probed clear: (-9930, 600), (-10010, 660) and (-9880, 720), each more than 250 yd from the camp and at least 20 yd from Hogger's.

Rule: "He steps out of every heavy. Parry him, and the heavy lands while he reels." The Riverpaw Tribute (259) at (-9965, 628) stays shut while he lives.
## Stage 7: The Mason's Marks [x] (puzzle; engine change E1)

Built as planned, with these differences, found checking the plan against the code:
- **E1.** `use_chest` (`sim/explore.rs`) reads a chest's `after` through `chest_unlocked`: with a colon it is `camp_has`, a bare key is the camp's `kill:<key>` count as before. `validate()` accepts only `open:`, `found:` and `kill:` for a chest, checks the name is real, and refuses a chest that waits on itself. The locked text of a chest with a condition and no `locked` line is "<name> is shut."
- The stones' display is 235 (the plain runestone). The plan's 1364 has no game-object row in the real data (it is a creature display).
- `ELWYNN_GOALS` is 10. The site is a `landmark`, so "Every corner" is 37 for the 37 sites.
- Probed with `elwynn_probe near`: every stone is on open ground. The crown's footpads stood at (-8896, -824), which has a trunk or wall next to it, so they stand at (-8890, -822).

## Stage 8: Remy "Two Times" [x] (camp choice; engine change E4)

Built as planned, with these differences, found checking the plan against the code:
- **E4.** `Survival::choose` (`sim/explore.rs`, called by `.es choose` in `htl-server/src/survival/ui.rs`) checks, in this order, that the group is not yet decided ("That is already decided."), then that a person within 6 yd has a current line with that link. It counts `chose:<group>` and `chose:<group>:<option>`, then runs `grow_camp` at once so the people stand or go now. No save-format change: it all lives in `camp.counts`, and the add-on is untouched (its link buttons already send any command), so no `VERSION` bump.
- `CAMP_CONDITIONS` has `chose`, `GoalDef::KINDS` has `chose`, and `PropDef.until` removes a standing piece (and stops it being set out) once its condition holds.
- `validate()` checks each `.es choose` link is `group:option`, each `chose:` condition names an option some link offers, each goal of kind `chose` names a group, and each option is waited on by some line, person or chest (a choice with no consequence is refused).
- A chest's `after` can be `chose:...` too, so the bounty and the stash are chests that stand but stay shut, with their own words, until the choice. (A chest has no way to stand later. The plan's "stand only after arrest" is read as "open only".)
- Displays from `npcfind` on the real data: Remy is 1466 (a cloth swashbuckler's shirt over black, human male), Dughan is 1504 (copper mail), not the plan's fallbacks (1295 is a woman's look). The Stockade link uses the real key `stockade`.
- Remy stands at (-9444, 30): the plan's (-9450, 30) has a wall beside it. The bounty chest is at (-9463, 70), not (-9466, 78), which sits in a pocket of walls. Dughan (-9465, 75) and the stash (-9990, 75) are open ground. `ELWYNN_GOALS` is 11; no site was added, so "Every corner" stays 37.

## Stage 9: Hold the Stonefield Wagon [x] (timed defense with a fighting ally; engine E2 and E3)

Built as planned, with these differences, found checking the plan against the code:
- **E2 (hold).** A chest's `trial` now works without `teaches` (`open_chest` starts any trial). New `ChestDef` fields `hold`, `wave_text`, plus `hold_title`, `held_title` and `lost_text` for the banners and the news (the plan's fixed texts, as content). `Trial` has `started` and `called`. `step_trials` calls wave `i` of `n` at `i * hold / n` sec (`call_wave`, the old `start_trial` spawning) and passes once the hold has run, every wave has been called and none is up. A pass with no `teaches` is `finish_open`: the chest is spent and fills again in its `respawn`, the opener gets the loot. The existing give-up rule (no hero within 60 yd for 12 sec) resets the waves and says `lost_text`. A hold's wake-on-open `ambush` packs wake when the hold begins and stand down at the end.
- A wave is one pack, so a `trial` entry may name several joined with `+` (wave 2 is tunnelers plus a geomancer, wave 3 Fedfennel plus laborers). `validate()` checks every pack of every wave wakes on the chest, that a hold has waves, and that there are no more wave texts than waves.
- **E3 (ally).** `MobDef.shoots` (a `ShootDef`, with a new optional `spell`), only on a `friendly` mob (checked by `validate()`). `allies_shoot` (`sim/explore.rs`, run every tick with a hero in the world) shoots the nearest foe within range in `Chase` or `Windup` on a hero, with a clear line (`land::clear_line`), through `hurt_mob(r, 0, dmg)` as a tower does, so a creature it finishes counts for the camp. A friendly creature no longer wanders from its home, so one set by a pack stands where it was put. `engage` already skipped friendly creatures.
- The banner's second line is "Stay near it for 90 sec" (generic), not "Stay near the wagon for 90 sec".
- Fedfennel's scale is 1.3 in the content, and every boss is drawn `combat.boss_scale` times that, as Brannick is. His shield rule is the existing one (`block`, in `combat.rs`): Chase or Recover, within 70 degrees, not a heavy, shove or `through_guard`, not while a Bearded Axe hook holds his guard down. The wind-up and the side are the open moments. A kobold with a shield on its arm is unseen in the real client.
- Tommy Joe's and Fedfennel's looks and the shield are unchecked in the client. Tommy Joe wears display 1408 (human male, brown leathers) with a bow. `ELWYNN_GOALS` is 12; no site was added, so "Every corner" stays 37.
- Probed with `elwynn_probe near`: the farm spots, the wagon, the ally and the wave spawn are open ground, unchanged.

## Stage 10: The Parley Stones [x] (payoff; engine E5; one new weapon)

Built as planned, with these differences, found checking the plan against the code:
- **E5.** `PackDef.after` is checked in `wake_ambush` (a pack whose condition doesn't hold stays asleep, and `validate()` refuses a bad condition or an `after` on a pack that doesn't wake on a chest). The helpers' stand-down is new: a friendly creature woken by a chest's `ambush` stands down when the chest fills again (the hold's helpers stood down at the end of the hold). `WeaponDef.punish` adds 75% to a blow on a creature in `MobState::Recover` (in `weapon_share`), with its sentence in `effect_text`, `punish` in `unique_mechanics`, and a row in `docs/weapon-identities.md`.
- **`punish` alone does not make him answer.** The plan's `silent` temper (`punish = true`, `pace = 1.8`, `retreat = 0.5`, `retreat_yards = 4`) only changes whose turn it is: a creature presses an opening only once its cooldown is nearly run (`ai.press_early` is 1.2 sec), and a pace of 1.8 makes the cooldown long. So "starts his windup within 0.3 sec of a swing that missed" needed one more temper field, `answer` (an opening draws the blow at once within `PRESS_RANGE`, whatever the cooldown). Tempers do apply to bosses (`pace`, `punish`, and `retreat` when the temper sets it, as the code says); the test measures the cooldown he sets after a string (1.3 x 1.8 sec against an ordinary 1.3 sec).
- The fan of knives is a pattern of weight 0, begun by his 50% phase (`pattern`), so it comes once and is no enrage (`speed` and `might` stay 1). His stabs carry `parry_ends` (and `tests_boss_rules.rs` checks he does not reel from a parry).
- Marzon's own pack is `marzon` on the chest `parley_price` with the helpers `parley_dughan` and `parley_remy`, all `wake = "parley_price"`. The helpers are friendly shooters named for the people (Dughan, display 1504, a bow, 40 damage every 5 sec to 30 yd; Remy, 1466, a dagger, 6 damage every 0.8 sec to 16 yd). The envoy has no weapon of note (the Blackrock grunt's axe) and is a `coward`.
- The silent blade is a dagger (display 2699, backstab 2.5, damage 15 to 20) with `punish = 0.75` and nothing else of the pass's. It is Marzon's trophy.
- `ELWYNN_GOALS` is 15 and "Every corner" is 38 for the 38 sites.
- Probed with `elwynn_probe near`: the site centre, the guards (the archers' spot has a trunk or wall within 6 yd, none beside it), the envoy, the table, the satchel and the helpers' spots are open ground, unmoved.

Round two is built.

# Round three: "The Lord's Answer" (stages 11 to 13)

Designed 16 Nov 2026 from origin/main @8905cc8d, built overnight from the plan's defaults. Stages are ticked as they land.

Defaults for Corey's open questions, all reversible:
- The witness is invented: Aldric Pell, a Stormwind seal-cutter who cut Lescovar's ring. So are Crane (the Lion's Knife), Sable (the Lion's Hound) and Steward Venn.
- The escort ends at the Stormwind gate, a new landmark on the north road. Nothing of the city is spawned.
- A lost escort is not a death: Pell is struck down, the knives slip away, and he waits at his start to try again.
- The second choice (sell or keep the letter) is camp-wide and one-time, like Remy's.
- Sable hunts whichever way the camp chose, so the journal can always be finished.
- Stormwind's archers at the camp shoot raiders at its buildings too.
- No new weapon this round. The Horde column on the Redridge road and a night event are deferred.

Story spine: a letter is ink, so Stormwind needs the man who cut the seal to swear to it. Pell is walked to the city under Lescovar's knives. Lescovar then offers to buy the letter back, and sends his hunter whatever the answer.

Journal: every new goal goes just before "Every corner". `ELWYNN_GOALS` in `save.rs` goes 17, 18, 19. "Every corner" equals the site count (39 from Stage 11). Camps already past "The classes' trials" still skip the new goals: that migration is a call left for Corey. The tests are in `tests_story3.rs`, with `tests_story.rs`'s old-save, goal-order and sites tests extended.

## Stage 11: The Witness [x] (escort; engine change E6)

Built as planned, with these differences, found checking the plan against the code:
- **E6.** `ChestDef.escort` and `ChestDef.route` (`sim/explore.rs`: `escort_walks`, `escort_walker`, `hurt_ward`, `lose_trial`). The escort is a trial: `start_trial` wakes the walker and the chest's `ambush` helpers and calls wave 1 at once; `step_trials` moves him (`go`, at his walk speed, only with a hero within 20 yd and nothing whose foe is him) and calls wave `i` as he sets off from point `i * len / n`. `Foe::Mob` is new (`sim/mobs.rs`): `foe_pos`, the blow landing on him (`hurt_ward`, never `mob_died`), the leash (one sent after a standing walker never gives up for it), `retarget` (it keeps him), and the draw-off on a hero's blow (any creature on him, not only a raider). `Mob.ward` keeps whom it was set on. `validate()` checks the escort's pack is one friendly creature that wakes on the chest, that it has a route and waves and no `hold` or `teaches`, and that only an escort has a route.
- **`MobDef.hunts`**: drawn off its quarry, it gives that hero one string and goes back (at the string's end, in `Recover`). A blow on it while it winds up at its quarry turns that wind-up on the hero (the plan's "a light blow knocks the throat-cut aside" could not be: a boss's wind-up is never knocked aside by the rules, `boss_knock_aside`).
- A friendly shooter also shoots a foe on a walker (`allies_shoot`), so Dughan's watchman helps Pell.
- The waves reuse the Brotherhood's footpads and thugs, all fighters up close: a ranged creature's shot lands where its target stood and only hurts heroes there, so an escort's waves keep to melee.
- `lose_trial` is the old give-up, shared: a hold or an escort that is lost says its `lost_text`; a shrine's trial still says nothing.
- Displays (`npcfind` on the real data): Pell 1724 (purple cloth), Crane 3358 (black leathers), the watchman 1984 (Stormwind plate and tabard). Unchecked in the client.
- Wyle's line at `open:marzon_satchel` now ends with the pointer to Pell (it still starts as round two's test reads it), and Wyle, Remy and Dughan each have a line at `open:pell_satchel`.
- Two tests elsewhere (`tests_tactics`'s give-ground, `tests_warlord`'s tackle) drew their dice from the world's content, which this stage's packs move; each now seeds its own dice (both pass on 11 of 12 seeds).
- `ELWYNN_GOALS` is 17 and "Every corner" is 39 for the 39 sites.
- Probed with `elwynn_probe near`: the satchel, Pell's start, every point of the road, the wave spots and the gate are open ground. The route's fifth point moved from (-9150, 365), which has a wall within 6 yd, to (-9152, 360).

## Stage 12: The Lion's Bargain [x] (a second camp choice; engine change E7)

Built as planned, with these differences, found checking the plan against the code:
- **E7.** `PackDef.after` on any pack: `spawn_mob` keeps one whose condition doesn't hold unseen (`pack_due`), an ambusher lying in wait doesn't step out before it holds, and `grow_camp` (every 5 sec, and as a choice is made) brings into sight the packs whose time has come (`packs_come`). `validate()` dropped its "only for a pack that wakes" rule; round two's test now reads the parley's ambush error instead.
- `allies_shoot` also shoots a raider (any state but fallen), so the archers help in a raid.
- Venn, the purse and the archers stand outside the camp's building ground (55 yd from its heart, past the gate's lion statues), so nothing built can stand on them. Pell leaves only if the letter is sold (`until = "chose:letter:sell"`); kept, he stays at the gate with a new line.
- Displays: Venn 7011 (a black suit, `npcfind`), the archers 1984. Unchecked in the client.
- `ELWYNN_GOALS` is 18. No site was added.
- Probed with `elwynn_probe near`: Venn (-9673, 692), the purse (-9675, 688) and the archers (-9678, 674) and (-9678, 702) are open ground; the plan's first spots by the statues had walls beside them.

## Stage 13: Sable, the Lion's Hound [x] (roaming hunter; content on E7)

Built as planned, with these differences, found checking the plan against the code:
- Content only. Her pack is `hidden = 12`, `rare`, with three `spots` and `after = "chose:letter"` (E7 keeps her from stepping out before the camp answers, whichever answer). The `stalker` temper is the existing `backstab` rule (`sim/mobs.rs`: a backstabber waits unless its hero's back is to it or they are open) with `flank`, `punish`, `retreat 0.5` over 5 yd and `pace 0.9`; tempers apply to a boss.
- Her 50% turn is a `shove` of 4 yd (every hero beside her thrown down) with her line, not the plan's "unseen for a moment": a `run_to` needs fixed points and she wanders between spots. No adds, no enrage (`speed` and `might` stay 1).
- Three spots, not four: the Goldshire road east of the camp is Stitches' (his bridge end is at (-9744, 470)), and (-9752, 540) is on the stream's bridge. Her spots are (-9752, 640), (-9764, 790) and (-9705, 760), each more than 100 yd from the camp and 60 from Stitches' road.
- Display 1297 (a woman in black and red leathers, `npcfind`), two daggers. Unchecked in the client.
- Lines: Wyle, Marten and Pell (only there if the letter was kept) each have a last word at `kill:sable`.
- `ELWYNN_GOALS` is 19; no site was added, so "Every corner" stays 39.
- Probed with `elwynn_probe near`: the three spots are open ground; (-9752, 540) and (-9790, 700) failed (water, a wall) and were dropped.

Round three is built.
