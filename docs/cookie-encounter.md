# Cookie, then Gorged: dinner interrupted

Corey's revised design (Nov 8, "Deadmines bosses: review and redesign", section 3). The lesson:
**his pot is his weakness: knock it while he's greedy, and step off the line when he serves.**
The code is `crates/htl-survival/src/sim/kitchen.rs`; the numbers are in `content/deadmines.toml`
(`dm_cookie`, `dm_cookie_gorged`).

The first build (Codex, #296 and after) is kept where it worked: the cleaver pair and the ladle,
the one puddle at a time, the scoop's heal budget, cleanup on wipe and reset. This rebuild fixes
what the Nov 8 footage showed: no prop change, nothing in flight, a fog round his own feet, a
scoop whose choice was fake, a full pot after "pot's empty", three faint puffs for a cone, the pot
vanishing on his death, and a Gorged smaller than Cookie who swung as he rose and never belched at
a hero who kept taking the first slap.

## The beats

1. **Pull.** "Mrrgl! Cookie's kitchen!" He takes out his tankard and bangs it on the cauldron twice
   (Attack1H, a mace on metal, 971), 1.6 sec, then the cleaver is back in his hand for the first
   pair. The cauldron (display 216) stands 1.8 yd north of his post all fight, through his death
   and Gorged's, and goes only with a dungeon reset. A wipe leaves it where it is: the next pull
   finds it full again.
2. **Cleaver pair** (0.8 sec, then 0.65 sec, then a 1.2 sec opening) and **ladle sweep** (150
   degrees, 1.2 sec wind-up, a 1.6 sec opening). Unchanged.
3. **Serve It Hot.** He swaps the cleaver for the tankard (metal tankard, item display 2723) and
   holds it up (HoldThrown, held 1.8 sec) as the stew bubbles (Blood Boil's sound, 3135). Where it
   lands is fixed as he begins: where the hero stood, pushed out to at least 4 yd from him and in to
   at most 8, on dry floor he can see. That spot is drawn at once as a ring at its true 2 yd radius
   (an effect, shown with the floor markers off) and redrawn every 0.5 sec until it lands, so a
   broken-off pour stops showing within half a second. On release he throws (AttackThrown) and his
   hand is empty: the mug (Throw Dark Iron Ale's, 14814) flies an arc in 3 legs, up 2.5 yd over
   the middle, 0.32 sec at 4 yd and 0.42 at 8. It lands with a splash (1096) and a tan ring at
   the puddle's edge for 1.2 sec, and leaves a puddle: 12 large steam puffs (Steam02, 431, at 1.5)
   on its edge and its edge drawn as a ring. It deals 20 as it lands and 4 a second while any steam shows: hot 3 sec, then the puffs
   drop away one by one over 1.2 sec, and it's safe when the last is gone. He fights on while it
   cools; the next pour waits for it. Step off the line in the wind-up, then hit him in the
   1.7 sec recovery.
4. **The scoop, after 3, then 4, then 5 exchanges** (5 for every later one), while the pot has stew
   in it. He takes out the tankard and walks to the pot's east side (on the real map
   (-61.4, -898.2)), turns to it side-on, "Mmm... one taste.", and dips the tankard again and
   again (LoadThrown, every second, a slurp each time, Drinking 3373) for 4 sec.
   - **Break it** (posture 120 while he scoops: about 2 sec of a lone hero's light blows, or one
     heavy blow, which counts 2.5 times). He knocks the pot: a stew-coloured ring splashes out
     round it, a splash (1096), its steam and bubbles drop, "MRGLGLGL!", and he reels 5.5 sec in
     the open look (CombatWound held, below). The steam comes back as he recovers: a knock slops the
     stew, it doesn't empty the pot.
   - **Leave it**: a gulp (Eat, 45), he heals 4%, and at once draws in a breath (HoldThrown held
     1.6 sec, MurlocAlert 1109) with the cone drawn on the ground at the 5.4 yd and 65 degrees it
     hits. Then the steam: Frost Breath's white cone on him (kit 2610), a steam burst (4858), a
     white flash of the cone, and 6 steam puffs through the whole cone for a second. A short 1.5
     sec opening follows. Breaking is better (5.5 sec open, no heal, no cone); waiting is a choice.
   - The pot has two helpings: two plumes of steam over its bubbles when full, one after the first
     gulp, nothing after the second ("Mrrgl... pot's empty."). An empty pot is never scooped.
     At most 8% healed in a pull.
   - Other control delays the gulp; only the posture break knocks the pot. A pot he can't reach in
     6 sec is given up: no heal, no teleport, a 1.5 sec opening.
5. **Death.** He falls where he fought, beside the pot. "...burnt." The pot and its steam stay.
6. **Gorged** (the secret ingredient in the killer's bags). The body lies still 2 sec. He gets up
   in a puff of dust (no arcane swirl, no narration), at Cookie's size, walks to the pot, dips into
   it twice with slurps (1.8 sec), gulps it empty (its steam goes), and swells over 1.5 sec, drawn
   a step every 0.1 sec, from Cookie's 1.3 to 1.95. "MORE STEW!", a roar (emote 15, MurlocAggro
   416), then 2 sec before his first blow. Two slaps, then the belch: he rears back (HoldThrown
   held 2.2 sec, MurlocAlert) and swells 10% more as the wind-up runs, the cone drawn on the ground
   at the 7.4 yd and 70 degrees it hits. Then Acid Breath's liquid gush (kit 2373), the burst, a
   stew-coloured flash of the cone, his size back, and a 4 sec cough in the open look.

**Exchanges.** Any blow he lets go counts its string's exchange, once: a pair cut short by a
stagger after its first blow counts; one broken off before any blow lands never does. So a hero
staggered by every first slap still sees the belch within 15 sec.

**Staying in the fight.** He stays on a hero for as long as the engine counts them in his fight
(`in_fight_of`: alive, within his leash, within 10 yd of height of him), and turns to the next one
in it, as every creature does. Take 341 lost Gorged: the kitchen had its own filter, 6 yd of height
from his post, and the cove climbs 8 yd in the 12 east of it, so a hero who dodged up the slope
sent him home mid-fight.

**The open look.** The murloc has no StandWound or kneel. His openings (the knocked pot, the
steam's recovery, the belch's cough) hold CombatWound (clip 9) for their whole length (`HoldPose`),
with MurlocWound (418). Hit him while it's held.

**Gorged's size.** Gorged now uses Cookie's own display (1305) at 1.95. The orange 1773 at 1.15
was filmed smaller than Cookie: 1773 carries its own 2.0 display scale, and drawing the swell on
the same body compares like with like whatever the client does with it. His slaps reach 3.8 yd
(was 3.4) for the bigger body. A client that joins mid-swell draws him at his size then.

## What's built where

| Beat | Code |
| --- | --- |
| Pot kept on death, handed to Gorged, refilled on a fresh pull, gone on reset | `Kitchen::pots`, `kitchen_pot_for`, `remove_kitchen`, `reset_kitchens`, `kitchen_risen` |
| The tankard swap | `kitchen_hand_now`, `kitchen_hand` (the server's `mob_gear_fields`) |
| Pour aimed on the hero, 4 to 8 yd | `pour_point`, `kitchen_pour_mark` |
| The mug's arc | `kitchen_release` (3 legs, `MissileFrom` from the top) |
| Puddle edge, puffs, cooling, damage | `step_puddle` |
| No standing idle while it cools | `step_kitchen` (the old halt is gone) |
| The scoop's dips and its posture | `step_kitchen`, `SCOOP_POSTURE`, `kitchen_posture_share` (`mobs.rs` `posture_limit`, `hurt_mob_as`) |
| Knocked pot vs gulp | `kitchen_broken`, `kitchen_cough` |
| The cones drawn and filled | `kitchen_windup`, `kitchen_release` |
| Gorged's walk, gulp, swell and wait | `kitchen_risen`, `step_rising` (the generic roar in `rise` skips him), `kitchen_scale` (the server's `mob_create`) |
| Exchanges on any released blow | `kitchen_blow_released` (called after every release in `mobs.rs`) |

## Verification

`crates/htl-survival/src/sim/kitchen_tests.rs` covers each beat: the pull's two bangs with the
tankard; any released blow counting once and an unreleased one never (both forms, with and
without network delay); a hero staggered by every first slap still seeing the belch; the rhythm
and fighting on while the puddle cools; scoops after 3, 4 and 5 exchanges; the swap, the held
lift, where the pour lands (on the hero, pushed out to 4 yd), its ring from the first moment, the
arc's 3 legs and 0.3 sec or more in the air, the splash and the edge puffs; the puddle burning just
inside its edge and never just outside, until the last puff drops one by one; a broken-off pour
leaving nothing; the scoop side-on with repeated dips and slurps; two jabs not breaking it, about
2 sec of blows or one heavy blow breaking it; the knock's splash, steam dropping, 5.5 sec reel
and health falling through it; the gulp, heal, cone drawn and filled, the steam's short opening,
the pot emptying after two gulps and never scooped again; the cone hitting inside and missing
outside its drawn edge; control delaying the gulp; waiting out every scoop still winnable; the
blocked pot; the pot staying through death and a wipe and going with a reset; the dungeon wipe;
Gorged lying 2 sec, walking, gulping the pot empty, swelling past Cookie, "MORE STEW!" and waiting
2 sec; the belch's drawn length, pulse, hit and cough, a broken-off belch letting go of its
pulse, and both forms staying on a hero who dodged up a slope like the cove's until they die.

`a_lone_hero_playing_by_the_tells_clears_cookie_and_gorged` plays both forms with a warrior on the
starting weapon and their own health, reading only what the client shows with a 0.3 sec reaction:
the shapes drawn on the ground (step out of them), a wind-up (guard as it lands, one hit back),
the held open look and the dipping tankard (hit him). It never uses a debug command.
Over three seeds it clears both forms without being hit: Cookie in 43 to 45 sec (2 scoops, one
knocked, 2 pours dodged), Gorged in 53 to 56 sec from when he gets up (5 belches dodged), ending on
its full 200 health. It must keep out of the boss's body: inside 0.8 yd every blow lands whatever
its arc, and on Gorged's flank by the pot a hero could be pinned there. That's the simulation, not
the client. Waiting out every scoop instead (no break) still kills him with the starting weapon.

## Filming

`tools/visual-check/cookie-clear.txt` is an unassisted solo clear of both forms, filmed by the
spectator cameraman in the duel shot (`camera duel Cookie|Gorged`), close enough to read the tankard,
the scoop and the knocked pot, with markers off. Setup
before the video: the cove cleared but for Cookie, the secret ingredient in the bags, the warrior
healed 9 yd west of his post. From the video on, only the warrior's own keys play (`.es in`),
pressed by a small player in the interface that reads only what the client shows: a quick wind-up
(`AN` under 1.5 sec: guard it, one hit back), a long one (the pour or a breath: `ActionDash` out
sideways, left and right in turn, since the rig's walk and strafe keys don't move the hero), the
open look (`WP` clip 9: hit him), the scoop's yell (dash in and hit until the pot is knocked), and
otherwise lock on again and press him. Take 341, filmed high, beat Cookie at 63 s and lost Gorged
when he went home mid-fight (fixed above). This version has not been run yet.

The old `cookie-kitchen.txt` and `cookie-gorged.txt` are dev-setup demonstrations (summons, heals,
`.es hurt`) and prove nothing about the fight.

## Not tested in the client

Take 341 (camera high) showed the pull, the pour's orange ring, the knocked pot's reel, the
belch's green gush and the steam cones, too small to judge from that height. Not judged yet: whether the tankard reads in his hand at the camera's distance; whether
the 3-leg mug reads as one arc (each leg after the first flies from an unseen stand-in a tick
late); whether Blood Boil's sound reads as bubbling stew; the Steam02 puffs' size on the puddle's
edge and in the cones; Frost Breath's and Acid Breath's looks on a murloc's head; the HoldThrown
pose as a held lift and a rear-back; CombatWound held as the open look (it may read as repeated
flinches rather than a reel); the swell's 0.1 sec steps; Gorged on Cookie's blue skin (the orange
skin is gone with display 1773).
