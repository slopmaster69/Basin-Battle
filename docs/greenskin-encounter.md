# Captain Greenskin: break the firing line

Corey's revised design (Nov 8, "Deadmines bosses: review and redesign", section 2). The choice a
hero makes: **run to a gunner with the captain chasing, or duel the captain and step out of the
lines.** The code is `crates/htl-survival/src/sim/firing_line.rs`; the line's hit test is
`LineShot` in `combat.rs`; the numbers are in `content/deadmines.toml` (`dm_greenskin`,
`dm_deck_gunner`).

## The eight beats

1. **Setup.** Two Defias Deck Gunners (85 health, HumanMale, the short blunderbuss in hand) stand
   at their posts before anyone pulls, one each side of the quarterdeck: (-58, -806.5) and
   (-58, -824.5), 9.4 and 10.3 yd from the captain's home at (-62, -815, 41.5). They are posted
   once a hero comes within 40 yd, and they are pulled with him: pull him, or hurt, fell or
   provoke either gunner, and the whole crew comes.
2. **The order.** "Hold the quarterdeck!" and he advances, with no blow yet. A second later he stops, faces his
   first gunner and points his cutlass (EmotePoint), holding it 0.8 sec, and yells
   "Steady... FIRE!" (2 sec after his pull line, through the shared line queue). The first load
   starts a second after he points.
3. **The shot.** One gunner at a time, 3.6 sec apart, never on the same beat. Load (LoadRifle 106)
   with the click (sound 1147), 0.6 sec, the barrel following its hero. Then the gun at the
   shoulder (ReadyRifle 48) held 1.6 sec, and the aim locks there: it no longer turns. Then the
   shot (AttackRifle 49): a puff of dust at his feet (kit 5930), the bang (spell 8682's cast kit:
   AttackRifle and GunRelease 1148), a tracer (Fire Shot's bolt, 6980, with a pale smoke streak,
   flown at 20 yd/s), the 1 yd-wide line flashed on the deck for the ball's flight (a ground
   burst, drawn with the floor markers off), and a dust burst where it stops (22786, Irritating
   Spores; Rocky Ground Impact's debris filmed as pink boulders that lingered, take 336). The ball strikes each hero on its line as it reaches them, along the barrel to its
   full 32 yd or the first thing in the way. Step out of the line the barrel points along, or
   block facing the gunner.
4. **The captain never idles.** Thrust and the broad clearing cut, all the time, with no freeze for
   a volley. He only won't start a blow that would land within 0.8 sec of a gunner's shot
   (counting a blow held a beat). Run to a gunner (2 to 3 sword hits) with him chasing, or duel
   him and step out of the lines.
5. **A gunner falls.** His gun clatters to the deck (PutDownLargeMetal, 1207), his body stays at
   his post, and his turns go with him: the clock keeps its 3.6 sec beat, so his partner fires
   every 7.2 sec, half the shots. The captain fights straight through the empty turns.
6. **The last gunner falls.** At his next free moment, "I'll do it meself." A real draw: GoblinMale's
   Sheath (89), the gun in his hand 0.3 sec in; the load as the cocking (106 and the click), a
   1.2 sec aim (48), and the shot with its puff, bang, tracer, line and dust. The gun stays
   out 6 sec after a shot, then he puts it away the same way. From now on he has no pause
   between his attacks: that's the escalation.
7. **The harpoon,** only with both gunners down and only at a hero who has stayed 8 yd or more
   away for 2 sec. A harpoon in his hand (item display 5949) held back over his shoulder
   (ReadyThrown 108) the full 1.7 sec, then the throw (AttackThrown 107) along a line, on the
   native net (6533) at 14 yd/s. A fair hit hauls the hero to 3 yd from him, and he goes straight
   into his clearing cut. A sidestep, a dodge or a guard spares them the haul. The gun is for a
   hero who kites him: at least 6 yd away, at most one shot every 10 sec.
8. **Death.** "My ship..." The shared fixes stop his swing as he falls. The gunners still standing
   go with him; the fallen lie where they fell.

A wipe, the leash or a reset takes the crew and their shots away, and he goes home. Home again,
the crew stands at its posts again, whole. A party death retargets the same crew; a solo Vanish
keeps the fight (the crew sees through it).

## What the data couldn't do, and the nearest thing

- **No gun flash or bullet.** The 1.12 data has no muzzle flash, gun smoke or bullet model to
  borrow (`spellfx` and `fxdest` found none). The design's fallback is built: a puff at the
  shooter and a burst where the line ends bracket it, the line is flashed on the deck, and Fire
  Shot's bolt flies as the tracer. The puff is Charge's landing dust at his feet, not at the
  muzzle: a kit plays where its model attaches, and the only hand flashes (Fireball's cast kit 38)
  play a spell-cast arm swing over the gun pose.
- **No pistol.** No one-handed pistol model exists; he draws the short blunderbuss (21071).
- **A shot that crosses the captain staggers him** (optional in the design) is not built.
- **His point is EmotePoint**, the cutlass hand pointed at the gunner, not a raised blade.

## Native data checks

Read with the repo's own tools on 9 Nov 2026 (`animsurvey`, `creaturedisp`, `itemdisplay`,
`itemdisplays`, `spellfx`, `kitanims`, `fxdest`, `soundfind`, `emotes`, `dmprobe`); the output stays
outside the repository.

| What | Id | Found |
| --- | --- | --- |
| Captain | display 7113 | GoblinMale, scale 1.5 |
| Gunner | display 2347 | HumanMale |
| GoblinMale clips | 89, 106, 48, 49, 108, 107 | Sheath 0.5 sec (the weapon moves at 0.3), LoadRifle, ReadyRifle (loops), AttackRifle, ReadyThrown (loops), AttackThrown |
| HumanMale clips | 106, 48, 49 | LoadRifle, ReadyRifle, AttackRifle |
| Gun | item display 21071 | firearm_2h_shotgun_b_01 |
| Harpoon | item display 5949 | misc_2h_harpoon_b_01 |
| The bang | spell 8682 | "Fake Shot": cast kit 167, AttackRifle with sound 1148, no missile |
| Tracer | spell 6980 | "Fire Shot": FireShot_Missile, speed 40 in the data (20 by its `[[missile_look]]`) |
| Harpoon line | spell 6533 | "Net": Net_Missile, speed 40 in the data (14 by its `[[missile_look]]`) |
| End burst | spell 22786 | "Irritating Spores": `dustnova_cast_base` at a point, no caster kit, no rocks (`fxdest`) |
| Puff | kit 5930 | Charge's landing dust (`KIT_FALL_DUST`) |
| Click, bang, clatter | sounds 1147, 1148, 1207 | GunPrecastOneShot, GunRelease, PutDownLargeMetal |
| Point | emote 25 | EmotePoint |

No other ability flies 6980 or 6533, so their looks change nothing else. The flight speeds need a
client with patch 0040; an older client flies them at the data's 40.

The posts were probed on the deck (`dmprobe`, from the captain's home):

| Point | Floor | Walk from home |
| --- | --- | --- |
| Captain (-62, -815) | 41.4 | home |
| West post (-58, -806.5) | 41.7 | 18 steps |
| East post (-58, -824.5) | 41.8 | 20 steps |

The rail at (-59, -805) can't be reached, and (-52, -815) is the raised deck at 51.5. The water
below is at -16.1. Runtime placement still rejects water, a different floor, a blocked post or a
missing walk or path, with a checked spot beside him on the same side for custom test arenas.

## Verification

`crates/htl-survival/src/sim/firing_line_tests.rs` covers each beat: the crew at its posts before
the pull and pulled with him (and a gunner struck first pulling him), the order after his pull
line, the turns 3.6 sec apart and alternating, the captain never idle and no blow landing within
0.8 sec of a shot (three seeds), the load, click, barrel following, shoulder lock and the shot's
clips, puff, bang, tracer, line burst and end burst, the line striking as the ball reaches a hero
and missing a sidestep, another floor or a hero stepping on behind it, a guard blocking it, a
fallen gunner's turns passing unshot while the captain fights through them, the last gunner's
draw, load, 1.2 sec aim and shot with the gun out 6 sec, no pause and the 10 sec gun when alone,
the harpoon waiting for 2 sec at 8 yd, its held pose, spear, throw, haul and clearing cut, a
sidestep or guard sparing the haul, party retarget, Vanish, the wipe putting the crew back at
its posts, death clearing every shot, the floor checks, ranged input on a gunner, knockback, and
the captain-first reward. `a_lone_sword_hero_playing_by_the_tells_clears_the_crew_then_the_captain`
plays a warrior with the starting sword and their own health, reading only what the client is
sent (the crew at its posts, a gun at the shoulder or the harpoon held back, his windups): they
run to each gunner and cut him down, stepping out of each line as it fires and raising the guard
as his blows land, then duel him: 68 to 82 sec, 74 to 200 health left, no shot taken (three
seeds). `a_lone_sword_hero_can_instead_duel_the_captain_stepping_out_of_the_lines` takes the other
choice: it duels him from the start with the crew firing throughout: 60 to 65 sec, 2 to 94 health
left, 3 to 6 of 16 or 17 shots taken. These are the simulation's numbers, not a filmed fight.

**Balance (after take 336).** On film a duel killed him in 29 sec before any gunner fell. His
health is now 1500 (was 520) and a gunner's shot 28 (was 17). The duel is now clearly the harder
way, but not the longer one: alone he attacks without pause, which costs the hero about as much
time as the volleys do, so both ways wear him down at about 25 health a sec. Making the duel
longer as well needs a lever outside health and damage (his guard up while a shot is due, say),
which is a design call.

`tools/visual-check/greenskin-clear.txt` is the unassisted solo clear for the cameraman
(`camera duel Captain Greenskin + Defias Deck Gunner`, markers off): setup before the video, then only the warrior's
own keys (`ActionDash`; the rig's walk keys don't move the hero), pressed by a small player in the
interface that reads the same tells. It breaks the line: the east gunner first (straight ahead of
its start at (-58, -831)), then a 17 yd dash up the deck along x -58 to the west gunner, stopping
about 3 yd short of his post (the cannons stand at the west rail, y -801 to -807: `dmprobe`), then
the captain.
