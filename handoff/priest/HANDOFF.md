# The Priest: handoff to the desktop chat

From the cloud chat (Cloud POV), 5 Nov 2026. Corey asked the desktop chat to take the Priest from here: **film it, make its art, and design the rest of the class.** Everything this chat knows is in this folder, because the desktop can't see the claude.ai Project:

- `HANDOFF.md` (this file): what's built, where, how to play it, what's open.
- `DESIGN.md`: the design as Corey shaped it in conversation (draft 2 of the Priest kit), with his rulings.
- `../../art-src/priest/REQUEST.md`: the art request for Corey's ChatGPT chat.

**This branch (`claude/priest-art`) is for coordination only.** Commit the art here. Don't merge it to main as it is.

## 1. What's built

Three stacked draft PRs on top of the cloud chat's Mage stack (`corios/caster-counters`, #179 → #193). Merge them in order, after the Mage stack:

| PR | Branch | What | Tests |
| --- | --- | --- | --- |
| #206 | `corios/priest-core` | The staffs (a right click that heals or drains, never a guard), Power Word: Shield parrying for a friend, Grace | `tests_priest_core.rs`, 14 |
| #207 | `corios/priest-shadow` | Mind Flay's tension and snap, Mind Sear, Mind Blast's snap, Shadow Word: Death, Fade, Void Shift, Dispersion, Vampiric Touch | `tests_priest_shadow.rs`, 18 |
| #208 | `corios/priest-light` | Holy Word: Salvation (the group check), Penance's flick, Holy Fire's Kindle, Holy Nova and Leap of Faith saving a friend; Void Rift moves to Shadow | `tests_priest_light.rs`, 16 |

- **State:** all three are green on CI and merge cleanly in order. The top branch (`corios/priest-light`) has everything: htl-survival 669 tests, every server test, the add-on check clean, `check-pr` clean. The one warning: `ui.rs` changed without the add-on, but only the stat-line text did.
- **Nothing has been seen in the client.** Every look is a first pick, listed below.
- **The Mage lines need reconciling.** The desktop's own Mage work (#199 `claude/mage-playtest`, #200, #192, #194, with `Check.lua` and the `art/check/` textures) is a separate line from the cloud's Mage stack (#179–#193). The Priest sits on the cloud stack. Before the Priest can go to main, the two Mage lines need reconciling, or the Priest needs merging onto whichever one lands. The Priest's add-on change is one line: the guard's `Say` now carries the aim, so right-click spells go along the reticle. That bumps the add-on `VERSION` and the server's `ADDON_NEWEST` to 22, which may clash with #199's numbers.

## 2. The Priest as built, key by key

**The staffs** (on the rack beside the Mage's; `WeaponDef.alt` is the right click as a spell, and a weapon with one has no guard at all):

| Staff | Click | Hold | Right click |
| --- | --- | --- | --- |
| **Acolyte's Staff** (display 472) | Smite, 9 (look 585) | Heal: a 2 s cast, 15% of max health, on the ally under the reticle, else yourself; slowed, a heavy blow breaks it (2060) | Flash Heal bolt: passes foes, heals the first ally 3.5%, about 1.5 a sec (6685 flying, 2061 landing) |
| **Shadowcaller's Staff** (display 5415, shared with the Mage's staffs for now) | Shadow bolt, 7, hangs Pain 2/s for 4 s (589) | Shadow Burst, charged 14–26, perfect on the flare | Drain (held): a cord within 15 yd, 4/s, healing you as much (1120) |

**Grace** (`[grace]` in the content, `sim/priest.rs`):
- **Rises:** a level, up to III, on every perfect of a Priest spell on the hero's own keys. It's central (`step_grace` scans the events), so new Priest spells feed it just by pushing `Event::Perfect`. It also rises when a friend lands the Priest's Salvation beat.
- **Falls:** a level when a blow hurts the Priest. It fades 10 s after the last rise.
- **Spent:** by the holy staff's Heal as the cast begins. I: 1.4 s cast. II: 0.8 s, and overheal becomes a shield for 8 s. III: instant, twice the overheal, and a column of light (24171).
- **Looks:** Renew 139, Fear Ward 6346, Inner Fire 588, one more per level.

| Key | Light (choice 1) | Shadow (choice 2) |
| --- | --- | --- |
| **Q** | **Penance**: 3 bolts at 0, 0.5 and 1 s, each along the reticle as it fires; the first ally is healed 12, the first foe hurt 12. **Perfect:** all three land, on a friend and a foe. *EX* **Holy Fire**: a pillar, 20 damage in 2.5 yd, a burn of 3/s for 6 s; **Kindle**: each of your projectiles on the burning foe adds +1/s, up to +5 | **Mind Flay**: a cord, 4/s up close to 14/s at 26–30 yd, snaps past 30 (a flinch), the foe slowed 30%. **Perfect:** a second taut. Q again while taut: the rest, twice over, 45 at most. *EX* **Mind Sear**: half of each blow on foes within 5 yd |
| **E** | **Holy Nova**: 8 yd, 14 to foes, heals friends 14. **Perfect:** cast as a blow lands on a friend inside it, and the blow is caught whole. *EX* **Psychic Scream** (unchanged) | **Mind Blast**: 22 on the first foe along the reticle. **Perfect:** pressed while the Flay is taut, 44 on the flayed foe at once. *EX* **Shadow Word: Death**: 15, or 45 under 20% health, and 10 back at you if it doesn't kill |
| **1** | **Leap of Faith**: pulls the first ally or foe to you. **Perfect:** a friend pulled out as a blow lands, and it misses. *EX* **Mind Grip** (unchanged) | **Fade**: a 6 yd glide, untouchable 0.5 s. **Perfect** (Blink's slip check): foes within 8 yd of where you stood slowed 50% for 2 s, you 40% faster for 1.5 s, an afterimage look. *EX* **Void Shift**: swap places, 20 yd |
| **2** | **Power Word: Shield**: absorbs 30 for 6 s on the ally under the reticle (yourself if none), 2 charges of 24 s. **Perfect:** it lands on a friend within the window before a blow (or while the blow waits on their round trip), and the blow is **parried for them**: the striker reels, 15 Valor. Never on yourself. *EX* **Power Word: Barrier** (the dome, unchanged) | **Dispersion**: 3 s, 70% less damage, 5% back a sec, no attacking. **Perfect:** entered as a blow lands, it passes through and the striker is slowed. *EX* **Vampiric Touch**: a grip for 8 s, the Priest's own damage on it heals her 20% |
| **3** | **Holy Word: Salvation** (100 Valor, 40 s): a 10 yd circle everyone sees; its rim closes 3 times (2, 1.5, 1 s); everyone inside who acts on a beat (on their own screen) gets a stack of Holy Light (+10% damage, up to 3, 10 s); the third beat's pillar is 30 + 15 a lander (110 at most) and heals each lander 20 | **Void Rift** (unchanged, moved here from Light) |

**Where they're learned:**
- **The Altar of the Abbey** (shrine): Penance, Holy Nova, Mind Flay, Mind Blast.
- **The Abbey Reliquary:** Leap of Faith, Power Word: Shield, Fade, Dispersion.
- **The trainer of ultimates:** Holy Word: Salvation and Void Rift.
- **Saves:** heroes keep their keys. A Barrier saved on key 2 loads as the Shield, with the Barrier its EX. A Mind Blast saved on Q loads as Penance; anywhere else it stays Mind Blast (`RENAMED_ON` in `save.rs`).

**Playing it in a dev build:** `.es spells` puts every ability on the board, and `.es slot <key> <ability>` puts one on a key. The staffs are on the rack.

## 3. Readings the builders took (in the PRs)

- **Shield charges:** 24 s each, not 12. Charges come back one at a time, so 2 × 12 would allow a Shield every 6 s and break the board's 12-sec rule for key 2.
- **Heal amounts:** the staff's heals are shares of max health, as every other heal is. Penance, Holy Nova and the pillar use flat numbers (`mend`), as the draft has them.
- **Grace:** spent when the Heal begins, so a broken cast loses it. "30% faster" means 30% less cast time.
- **The old Shield:** it was 45 with a 20% haste; now it's 30 with no haste.
- **Drain's Valor:** it pays 0.5 Valor a tick, or a held cord would fill the bar.
- **Salvation:**
  - "Acts" is attack, heavy, guard, dodge, bash, kick or any ability key, not moving.
  - A press in the 0.3 s before a beat's window spends that beat, so mashing doesn't land every beat.
  - Landers get 10 Valor. The Priest's own per-beat perfect gives Grace, not Valor, since she just spent the bar.
  - The closing rim is the friendly ring redrawn 6 times a beat (no add-on change).
- **Mind Flay:**
  - Any other key during the cord lets it go and fires. Before, a key during a channel waited.
  - It gives Valor a second, as a beam does.
- **Fade's slow** centres on where she stood.
- **Dispersion** keeps its 24 s cooldown even when it catches a blow.

## 4. Open: not built, or yours to design

Corey said the desktop chat will design the rest of the class. Here's what's still open, roughly in order of how much it matters.

1. **The art** (`art-src/priest/REQUEST.md`). Salvation's ring and closing rim most of all: the redrawn friendly ring is the weakest look in the kit. After that: Grace's motes and halo, the Shield's perfect, Mind Flay's tension gauge and snap, Penance's bolts, Kindle, Fade. They plug into `Check.lua`'s way of drawing (your `art/check/` pipeline).
2. **Checks for the spells left as they were:** Psychic Scream, Mind Grip and Power Word: Barrier (Light's EX spells), and **Void Rift** (Shadow's ultimate). Corey wants a skill check on every spell, and the Mage's ultimates also backfire. A tension idea would suit Void Rift (Shadow is tension); the draft also parked **Shadowfiend** as a shadow ultimate on a leash: stronger the farther it fights, snapping back past 20 yd.
3. **Fade's afterimage** is a look only. A real decoy that creatures swing at for a moment would make the perfect read better.
4. **Staffs:**
   - Neither school bonus is built (the Mage's staffs don't have one yet either).
   - The rare staffs from the draft aren't built: **Benediction** (every third Flash Heal bounces to a second ally) and **Anathema** (Pain spreads when its host dies).
   - The Shadowcaller's Staff borrows the Mage's display.
5. **Lag limits:**
   - A blow on a friend waits on *the friend's* round trip, not the Priest's. A Priest with a longer round trip than the friend can press Shield, Holy Nova or Leap on time on her screen and still be late.
   - Blink's and Fade's slip works for creatures' blows, not for a rival hero's swing that finds no one.
6. **The quadrant boss** (Corey's idea: the group splits to parry blows from four sides) isn't built. The Priest's answer is a perfect Shield on the friend covering a side, which counts as that friend's parry. That already works, because the Shield's perfect is a parry.
7. **Holy Light's** +10% a stack is the draught of might's mechanism. Check it reads on the body (Holy Strength 20007).

## 5. What to film

Most of it needs two heroes (one to shield, heal, pull or flick to) and a wolf or grunt to bite them.

- **Staffs:** the healing bolt reading as friendly; the right click never blocking; Drain's cord.
- **Shield and Grace:** three perfect Shields on a friend as the wolf bites them (it reels each time), then an instant Heal at Grace III. **Watch whether "just as it lands on them" reads from the Priest's view.**
- **Shadow:** kite a wolf at the edge of Mind Flay (the cord tightens and whitens past 26 yd, snaps past 30), cash it in with Mind Blast. Fade through a bite. Dispersion through a blow. **Watch whether the tension reads without words.**
- **Light:** Salvation with two or three heroes, one missing a beat. Penance flicked between a friend and a foe. Holy Fire fanned by Smite. Holy Nova and Leap of Faith saving a friend from a bite. **Watch whether the closing rim is readable.**
