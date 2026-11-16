# Playtest checklist

Everything merged since 14 Nov 2026 with the `needs playtest` label: nobody has seen it in the
real client yet. Work down the list in play and tick what reads right. For what doesn't, open an
issue; take the label off the pull request once it's fine (AGENTS.md). Each line is what to do,
what you should see, and the pull request. The newest round (Elwynn stages, weapons, EX abilities)
comes first in each section. **Risk** marks what the pull request itself flagged as weak or unproven.

PR links: `https://github.com/slopmaster69/Basin-Battle/pull/N`.

Sections: [Camp and Elwynn](#camp-and-elwynn) |
[Elwynn sites and bosses](#elwynn-sites-and-bosses) |
[Class abilities](#class-abilities-the-ex-reworks) |
[Weapons](#weapons-by-family) |
[Dungeons](#dungeons) |
[Basin Battle mode](#basin-battle-mode) |
[Camera and visuals](#camera-and-visuals) |
[Networking feel](#networking-feel)

## Camp and Elwynn

- [ ] Press (F) beside Hale, Tobin, Garrick Stone, Wren, Maren, Marten, Wyle, a sentry and a footman: a page opens with what they say, headed "Talk to" ([#400](https://github.com/slopmaster69/Basin-Battle/pull/400)). **Risk:** only the use key reaches a person, not right-click; the page itself has not been seen in the client.
- [ ] Talk to the same person before and after a raid and after a kill: the words change with the camp ([#400](https://github.com/slopmaster69/Basin-Battle/pull/400)).
- [ ] Take the add-on to v69 (older add-on against this server): a hero on an older add-on is told how to update ([#400](https://github.com/slopmaster69/Basin-Battle/pull/400)).
- [ ] Build a barracks and an infirmary, then beat the fourth raid: the fifth raid, "The Brotherhood's Price", comes along the Goldshire road ([#425](https://github.com/slopmaster69/Basin-Battle/pull/425)).
- [ ] Beat the fifth raid: it returns with one more of each kind and "(2)" in the title, and Brannick stays one ([#425](https://github.com/slopmaster69/Basin-Battle/pull/425)).
- [ ] Kill Brannick: a stonemason from Stormwind arrives in camp and talks ([#425](https://github.com/slopmaster69/Basin-Battle/pull/425)).
- [ ] Open the journal: the new goals sit in order before "Every corner" (messenger, drop, ford, Paymaster, crossing, Fenros, Stitches, Brotherhood's Price), and an older save keeps the goals it finished ([#409](https://github.com/slopmaster69/Basin-Battle/pull/409), [#414](https://github.com/slopmaster69/Basin-Battle/pull/414), [#417](https://github.com/slopmaster69/Basin-Battle/pull/417), [#425](https://github.com/slopmaster69/Basin-Battle/pull/425)).
- [ ] Finish every site: "Every corner" completes at 36 of 36 ([#409](https://github.com/slopmaster69/Basin-Battle/pull/409), [#414](https://github.com/slopmaster69/Basin-Battle/pull/414)).
- [ ] Load a save made before the weapon removals with an Apprentice's Staff, Squallshaper's Staff, Woodcutter's Axe or Engineer's Wrench anywhere (hand, off hand, bag, storage): it comes back as an Ember Staff, Goldshire Halberd, Footman's Axe or Militia Mace ([#398](https://github.com/slopmaster69/Basin-Battle/pull/398)).
- [ ] Read the tooltips on perks, spins and hangs: the durations and stat lines match what the ability does ([#359](https://github.com/slopmaster69/Basin-Battle/pull/359)).
- [ ] Have two heroes in the Deadmines, one in a fight, and the other tries a lamp rest: refused with "Not while a friend is in a fight." and nothing resets ([#357](https://github.com/slopmaster69/Basin-Battle/pull/357)).

## Elwynn sites and bosses

### Smugglers' Ford and Paymaster Voss (Stage 2)

- [ ] Walk the Goldshire road to Jerod's Landing: a Defias Messenger walks it there and back ([#409](https://github.com/slopmaster69/Basin-Battle/pull/409)).
- [ ] Try the Messenger's Drop at the dock before killing him: it stays shut. Kill him, then open it: a map leading to the Smugglers' Ford ([#409](https://github.com/slopmaster69/Basin-Battle/pull/409)).
- [ ] Open the Paymaster's Strongbox at the ford: Voss and 2 hirelings wake on the opener, and wait again when it fills ([#409](https://github.com/slopmaster69/Basin-Battle/pull/409)).
- [ ] Fight Voss while a hireling lives: he circles off and won't come in. Kill the hirelings or go in after him ([#409](https://github.com/slopmaster69/Basin-Battle/pull/409)).
- [ ] Take Voss to 35%: he says "Hold them. You're paid till sundown.", calls 2 thugs, runs to his boat, and fights again once a hero is within 10 yd ([#409](https://github.com/slopmaster69/Basin-Battle/pull/409)).
- [ ] Parry the first blow of one of Voss's flurries: the rest of the string never comes, and he doesn't reel (a boss) ([#428](https://github.com/slopmaster69/Basin-Battle/pull/428)). **Risk:** `parry_ends`, new in this round.
- [ ] Kill Voss, then talk to Marshal Wyle: he says where the orders came from and links the Deadmines page ([#409](https://github.com/slopmaster69/Basin-Battle/pull/409)).

### Nightbane Crossing and Fenros (Stage 3)

- [ ] Walk to the river's south bank: Nightbane Prowlers, Shadow Weavers and Bleak Worgs hold the crossing. Kill all 6 and the crossing becomes a camp ([#414](https://github.com/slopmaster69/Basin-Battle/pull/414)).
- [ ] After finding the crossing, find Watcher Dodds on its north side ([#414](https://github.com/slopmaster69/Basin-Battle/pull/414)). **Risk:** he wears the Westbrook sentry's look, a stand-in.
- [ ] Find Fenros between the crossing and the woods west of Eastvale: quick claw strings and a leap from range ([#414](https://github.com/slopmaster69/Basin-Battle/pull/414)).
- [ ] Parry the first claw of Fenros's first form: the string stops and he reels ([#414](https://github.com/slopmaster69/Basin-Battle/pull/414), [#428](https://github.com/slopmaster69/Basin-Battle/pull/428)).
- [ ] Kill the first form: he lies still, and 4 sec later rises as Fenros, Risen ("Not... yet."), slower and heavier. Try to parry him: you can only block ([#414](https://github.com/slopmaster69/Basin-Battle/pull/414)).
- [ ] Kill Fenros, Risen: the fall is told once, and Dodds says "from a keep your book should name" and links the Shadowfang Keep page ([#414](https://github.com/slopmaster69/Basin-Battle/pull/414)).

### Stitches on the Goldshire road (Stage 4)

- [ ] Walk the cobbled road north-west of Goldshire, between the bridge and the crossroads: Stitches walks it, never near the camp ([#417](https://github.com/slopmaster69/Basin-Battle/pull/417)).
- [ ] Parry his cleaver chop: a block only, no parry and no stagger. His swing all round glows red when you crowd him: dodge it ([#417](https://github.com/slopmaster69/Basin-Battle/pull/417)).
- [ ] Stand 6 yd or more off: he lurches at you, and throws a cleaver from afar ([#417](https://github.com/slopmaster69/Basin-Battle/pull/417)). **Risk:** the cleaver's missile look and his clips were never seen on the golem.
- [ ] Hit him below 50% health: his stitches split, and each heavy blow he lands leaves bile (8 damage a tick for 8 sec, within 3 yd). Walk off it and it stops ([#417](https://github.com/slopmaster69/Basin-Battle/pull/417)). **Risk:** the bile uses the fire look (flames); there is no green ground look. Does that read as bile?
- [ ] Kill him: the fall is told to everyone as a rare's, and he comes back somewhere else later ([#417](https://github.com/slopmaster69/Basin-Battle/pull/417)).

### Brannick the Mason (Stage 5)

- [ ] Sprint round Brannick inside one of his wind-ups: you end up at his back, and a blow there lands 1.5 times as hard ([#425](https://github.com/slopmaster69/Basin-Battle/pull/425)).
- [ ] Hug him: a stomp all round, 3.5 yd or less, that you can't block. At arm's length he swings the maul (a glowing overhead or a wide sweep) instead ([#425](https://github.com/slopmaster69/Basin-Battle/pull/425)).
- [ ] Take him to 50%: he says "Pitch on the stones, lads." and each heavy blow leaves burning pitch where it lands ([#425](https://github.com/slopmaster69/Basin-Battle/pull/425)). **Risk:** the pitch uses the fire look.
- [ ] Fight the raid's footpads, lookouts and conjurers at the camp's wall: the counts match "5 thugs, 3 footpads, 2 lookouts, 2 conjurers" ([#425](https://github.com/slopmaster69/Basin-Battle/pull/425)).

### Rares and champions, all

- [ ] Goldtooth, Morgan the Collector and Paymaster Voss: parry the first blow of a flurry and the string ends with no reel and no riposte ([#428](https://github.com/slopmaster69/Basin-Battle/pull/428)). **Risk:** the parry-ends strings on all three; a boss doesn't reel.
- [ ] Any "too big to parry" creature: a timed guard only blocks ([#428](https://github.com/slopmaster69/Basin-Battle/pull/428)).
- [ ] Any boss blow that no guard stops: Danger flashes on your screen as it winds up, with no floor marker needed ([#428](https://github.com/slopmaster69/Basin-Battle/pull/428)).

## Class abilities (the EX reworks)

Hold an ability's key a quarter second (or Ctrl with it) to cast its EX for Valor.

### Priest

- [ ] Penance's EX, Divine Star (25 Valor): a star goes 24 yd along the reticle and back, hurting each foe and healing each friend it passes, each way. Press Penance again to turn it early. Catch it coming home for 5 Valor and the perfect ([#366](https://github.com/slopmaster69/Basin-Battle/pull/366)). **Risk:** the catch has no timing check.
- [ ] Psychic Scream's EX, Void Tendrils (25 Valor): every foe within 8 yd is rooted 3 sec (heroes 1.5) by a tendril of 50 health. Hit a tendril until it breaks ([#366](https://github.com/slopmaster69/Basin-Battle/pull/366)).
- [ ] Fade's EX, Dominate Mind (25 Valor): a bolt takes the first creature for 6 sec to fight for you. It strikes the nearest creature within 14 yd, follows you when none is near, and shrugs off bosses ([#366](https://github.com/slopmaster69/Basin-Battle/pull/366)). **Risk:** if spell look 605 has no state look on the client, the glow is missing and nothing else.
- [ ] Dominate Mind on a hero (another player): they walk toward you and can't act for 1.5 sec ([#366](https://github.com/slopmaster69/Basin-Battle/pull/366)).
- [ ] Holy Nova's EX, Guardian Spirit (25 Valor): the ally nearest your reticle (or you) for 6 sec. A killing blow leaves them at 1 health and heals 60%, once ([#373](https://github.com/slopmaster69/Basin-Battle/pull/373)). **Risk:** spell 47788 isn't in the 1.12 client, so the look is Flash Heal plus Holy Strength's glow.
- [ ] Angelic Feather's EX, Leap of Faith (25 Valor): pulls the ally nearest your reticle, up to 30 yd, to your side. With no ally it is refused and the Valor comes back ([#373](https://github.com/slopmaster69/Basin-Battle/pull/373)).
- [ ] Power Word: Shield's EX, Pain Suppression (25 Valor): the ally under the reticle (30 yd) or you takes 40% less damage and can't be staggered for 4 sec ([#377](https://github.com/slopmaster69/Basin-Battle/pull/377)).
- [ ] Vampiric Embrace (Shadow's key 2, 15 sec cooldown): for 6 sec half of the shadow you deal heals the lowest-health ally within 12 yd, or you. A perfect Mind Blast inside it adds 2 sec ([#377](https://github.com/slopmaster69/Basin-Battle/pull/377)).
- [ ] Vampiric Embrace's EX, Psychic Ward (25 Valor): ward the ally under the reticle or you for 6 sec. The first foe to strike them takes 20 damage and a panic of 3 sec (heroes 1.5). The blow still lands ([#377](https://github.com/slopmaster69/Basin-Battle/pull/377)).
- [ ] Load a save with Shadowguard on key 2: it comes back as Vampiric Embrace ([#377](https://github.com/slopmaster69/Basin-Battle/pull/377)).

### Mage

- [ ] Fireball's EX, Ice Lance (25 Valor): a 30 yd frost bolt, 12 damage unfrozen, 1.5 times that on a frozen foe, with the freeze ended ([#391](https://github.com/slopmaster69/Basin-Battle/pull/391)).
- [ ] Ice Lance on one frozen foe in a cluster: every other frozen foe within 5 yd shatters too (once each, no chaining), whoever froze it ([#391](https://github.com/slopmaster69/Basin-Battle/pull/391)). **Risk:** Ice Lance's spread gives Valor per foe, so check how fast a cluster fills the bar.
- [ ] Cast Frost Nova, then press the Fireball key: your next Fireball is a free Ice Lance for 5 sec, and the key glows ([#391](https://github.com/slopmaster69/Basin-Battle/pull/391)). **Risk:** Frost Nova frees Ice Lance, so freeze-then-lance is now a loop.
- [ ] Flamestrike's EX, Blast Wave: slot Flamestrike, hold its key, and an instant ring of fire goes round you: 6 yd, 12 damage, a 6 yd knockback and a 50% slow for 3 sec ([#379](https://github.com/slopmaster69/Basin-Battle/pull/379)). **Risk:** Blast Wave only shows with Flamestrike slotted (it is benched); on the default board you can't reach it.

### Hunter

- [ ] Bank Shot's EX, Scatter Shot (25 Valor): a shot up to 10 yd that does no damage and disorients one foe 4 sec (heroes 1.5). It wanders, can't act, and any damage ends it ([#378](https://github.com/slopmaster69/Basin-Battle/pull/378)). **Risk:** the "Disoriented" banner word is the only add-on change, and `check-addon.sh` didn't run.
- [ ] Fire Aimed Shot, Multi-Shot and Bank Shot after the Split Arrow and Piercing Shot names went: all three still fly and land as before ([#378](https://github.com/slopmaster69/Basin-Battle/pull/378)).

### Warlock

- [ ] Seed of Corruption's EX, Soul Swap: put two or more of your rots on a foe, aim at it, and each other foe within 10 yd of it gets them with the time they had left. With no rot or nobody near, nothing is spent ([#378](https://github.com/slopmaster69/Basin-Battle/pull/378)).
- [ ] Shadowfury's EX, Howl of Terror: every foe within 10 yd flees 4 sec (heroes 1.5). Damage frees them early, but not in the first 1 sec ([#378](https://github.com/slopmaster69/Basin-Battle/pull/378)).

### Warrior and Rogue

- [ ] Cleave's EX, Piercing Howl: every foe within 10 yd is slowed 60% for 6 sec (heroes 3), with no damage. A boss holds its ground ([#378](https://github.com/slopmaster69/Basin-Battle/pull/378)).
- [ ] Fan of Knives' EX, Kidney Shot: after a 0.25 sec windup, a stab for 40% weapon damage and a 3 sec stun (heroes 1.5). A Marked foe is stunned 1 sec longer and the Mark is used up ([#378](https://github.com/slopmaster69/Basin-Battle/pull/378)).

### Shaman and Druid

- [ ] Thunderstorm's EX, Capacitor Totem: a totem at the reticle (up to 20 yd). After a 2 sec fuse it stuns every enemy within 8 yd for 3 sec (heroes 1.5) and falls. Destroy it first (20 health) and it does nothing ([#379](https://github.com/slopmaster69/Basin-Battle/pull/379)).
- [ ] Feral Leap's EX, Prowl: stealth for up to 10 sec. The next attack out of it stuns its target 2 sec (heroes 1) and deals 20 more damage. Damage on you ends it with no strike ([#379](https://github.com/slopmaster69/Basin-Battle/pull/379)). **Risk:** no model swap exists, so the stealth shows by its spell look (5215).

## Weapons, by family

Each weapon's tooltip sentence is built from its numbers, so check the sentence against what you see.

### All families

- [ ] Swing a 1H axe, a 1H mace, a 2H axe and a maul: each has its own moveset (wide hacking cleaves; overhead slams, each slower and heavier; big diagonal cleaves; one slow overhead slam) ([#402](https://github.com/slopmaster69/Basin-Battle/pull/402)). **Risk:** only chosen stock clips, never watched in the client.
- [ ] Start a fresh hero with the Footman's Axe: it still swings at the grip's old pace ([#402](https://github.com/slopmaster69/Basin-Battle/pull/402)).

### Swords

- [ ] Silver-Edged Longsword on a worgen or an undead foe: 50% more damage, and it stays down once slain ([#405](https://github.com/slopmaster69/Basin-Battle/pull/405), [#407](https://github.com/slopmaster69/Basin-Battle/pull/407)).
- [ ] Cruel Barb, heavy: the foe is hooked and pulled up to 6 yd to you ([#405](https://github.com/slopmaster69/Basin-Battle/pull/405)).
- [ ] Pirate's Cutlass, heavy: a 5 yd boarding leap at the foe ([#405](https://github.com/slopmaster69/Basin-Battle/pull/405)). **Risk:** the leap's look needs a playtest.
- [ ] Sneed's Machete, light hits: each chops every foe in a 5 yd line, and it has no backstab ([#405](https://github.com/slopmaster69/Basin-Battle/pull/405)).
- [ ] Cursed Felblade, last blow of the light chain: you step behind the foe ([#405](https://github.com/slopmaster69/Basin-Battle/pull/405)). **Risk:** the fel step's look needs a playtest.

### One-hand axes and polearms

- [ ] Bearded Axe, heavy, against a shield-bearer: they can't block for 4 sec ([#407](https://github.com/slopmaster69/Basin-Battle/pull/407)).
- [ ] Bloodlust Edge: each kill speeds your swing 8%, up to 4 times, fading 6 sec after the last ([#407](https://github.com/slopmaster69/Basin-Battle/pull/407)).
- [ ] Defias Miner's Pick on a golem, a rockling or the Shredder: 50% more damage, and stances break quicker ([#407](https://github.com/slopmaster69/Basin-Battle/pull/407)).
- [ ] The Horseman's Scythe: a 200 degree sweep of up to 5 foes, and each foe struck heals you 1% of its health ([#407](https://github.com/slopmaster69/Basin-Battle/pull/407)).
- [ ] Loksey's Boar Spear: its thrust slows the foe 40% for 2.5 sec ([#407](https://github.com/slopmaster69/Basin-Battle/pull/407)).

### Maces

- [ ] Hogger's Cudgel, heavy: the foe is knocked 5 yd back ([#412](https://github.com/slopmaster69/Basin-Battle/pull/412)).
- [ ] Cookie's Tenderizer: with a friend, hit a foe and the friend's next hit deals 25% more, once. Your own next hit gets no bonus ([#412](https://github.com/slopmaster69/Basin-Battle/pull/412), [#416](https://github.com/slopmaster69/Basin-Battle/pull/416)).
- [ ] The Smelter's Spanner (Gilnid's Wrench), heavy: stuns a mechanical, non-champion foe for 4 sec ([#412](https://github.com/slopmaster69/Basin-Battle/pull/412)). **Risk:** Gilnid's Wrench has no live target; the Shredder is a champion and shrugs it off, so you can't see it work yet.
- [ ] Hand of Righteousness: each hit heals the most wounded hero within 15 yd (you too) 3% of their health ([#412](https://github.com/slopmaster69/Basin-Battle/pull/412)).
- [ ] Baron's Scepter: a cursed foe that dies within 12 sec bursts in shadow on foes within 5 yd ([#412](https://github.com/slopmaster69/Basin-Battle/pull/412)).
- [ ] Ironspine's Fist: each hit gives a 5 point shield for 8 sec, stacking to 30 ([#412](https://github.com/slopmaster69/Basin-Battle/pull/412), [#416](https://github.com/slopmaster69/Basin-Battle/pull/416)).

### Daggers

- [ ] Hypnotic Blade: a hit makes the foe drop you for 4 sec. Keep stabbing and it is not lulled twice, so it hits back ([#413](https://github.com/slopmaster69/Basin-Battle/pull/413), [#416](https://github.com/slopmaster69/Basin-Battle/pull/416)).
- [ ] Meteor Shard, a blow from behind: a fire burst for 50% on foes within 4 yd, and it breaks a frozen foe's ice ([#413](https://github.com/slopmaster69/Basin-Battle/pull/413)).
- [ ] Bazil's Shiv: each hit adds a poison stack (3 a sec, up to 5, 6 sec), beside a Fireball's burn ([#413](https://github.com/slopmaster69/Basin-Battle/pull/413), [#416](https://github.com/slopmaster69/Basin-Battle/pull/416)).
- [ ] Thornspike: a hit interrupts a casting foe and staggers it 1.5 sec (not a champion) ([#413](https://github.com/slopmaster69/Basin-Battle/pull/413)).
- [ ] Cobrahn's Fang, a blow from behind: the foe sleeps 3 sec and damage wakes it. A woken foe can't be put to sleep again for 6 sec ([#413](https://github.com/slopmaster69/Basin-Battle/pull/413), [#416](https://github.com/slopmaster69/Basin-Battle/pull/416)).

### Two-handers

- [ ] Fallen Champion's Greatsword, heavy: heroes within 8 yd (you too) take 15% less damage for 5 sec ([#416](https://github.com/slopmaster69/Basin-Battle/pull/416)).
- [ ] Thornroot Maul, heavy: the foes hit are rooted 2.5 sec ([#416](https://github.com/slopmaster69/Basin-Battle/pull/416)).
- [ ] Mograine's Might, heavy: a holy ring goes out 6 yd for 0.8 of the blow ([#416](https://github.com/slopmaster69/Basin-Battle/pull/416)).
- [ ] Smite's Mighty Hammer: the 3rd hit in a row on one foe, within 3 sec each, staggers it 1.5 sec ([#416](https://github.com/slopmaster69/Basin-Battle/pull/416)).
- [ ] Rhahk'Zor's Maul, heavy: it marks the spot, and 1 sec later rocks fall for 0.6 of the blow within 2.5 yd ([#416](https://github.com/slopmaster69/Basin-Battle/pull/416)).
- [ ] Targorr's Cleaver: the lower your health, the harder it hits, up to 80% more ([#416](https://github.com/slopmaster69/Basin-Battle/pull/416)).
- [ ] Corpsemaker: its hits cow the foe: 25% less damage and slower wind-ups for 4 sec ([#416](https://github.com/slopmaster69/Basin-Battle/pull/416)).
- [ ] Guillotine Axe, heavy, on a stunned, asleep, rooted, frozen or staggered foe: triple damage ([#416](https://github.com/slopmaster69/Basin-Battle/pull/416)).

## Dungeons

### Deadmines

- [ ] Deadmines shops and drops: the Squallshaper, Woodcutter, Engineer and Craftsman still fight as ordinary foes with their other drops, and the four removed weapons never drop ([#398](https://github.com/slopmaster69/Basin-Battle/pull/398)).
- [ ] Rhahk'Zor at 26%: he wears a bright white-blue shell, each refused blow sparks off it and shows "Block" instead of a number, and the shell goes when the last face stands ([#355](https://github.com/slopmaster69/Basin-Battle/pull/355)).
- [ ] Rhahk'Zor from 25%: flames at both hands for as long as the maul burns. Lodged maul (1.2 sec after a slam, 3 sec after the pound's last step): the fire goes out of him ([#355](https://github.com/slopmaster69/Basin-Battle/pull/355)). **Risk:** the maul itself doesn't burn, and a lodged maul isn't shown stuck.
- [ ] Rhahk'Zor's collapse: his overhead blow at a wall brings the mine down, not a cast. The shake and the rock volley read as the roof coming in ([#365](https://github.com/slopmaster69/Basin-Battle/pull/365)). **Risk:** same clip as his slam; the pale rock can look bluish under the lamps.
- [ ] Rhahk'Zor's collapse, section by section: the route's lanterns go dark one section at a time, a wreck stands behind each face, the ship is gone ([#369](https://github.com/slopmaster69/Basin-Battle/pull/369), [#370](https://github.com/slopmaster69/Basin-Battle/pull/370)). **Risk:** that the lanterns' lights really go dark at the pivot distances; the marker's reload after the kill.
- [ ] Rhahk'Zor's fourth face stands: dust rolls along it, he roars, turns red and grows to 1.15 times his size until the end ([#355](https://github.com/slopmaster69/Basin-Battle/pull/355)).
- [ ] With floor markers off (the default), watch Rhahk'Zor's falling rock: dust trickles from the roof over the spot for the whole 1.3 sec warning, then the rock lands ([#392](https://github.com/slopmaster69/Basin-Battle/pull/392)).
- [ ] Leave a Rhahk'Zor fight (the last hero closes the game) while the mine is coming down, then come back: the mine is put back whole, lamps lit, Rhahk'Zor at his post ([#399](https://github.com/slopmaster69/Basin-Battle/pull/399)).
- [ ] Gilnid's valve, at "Useless. Every one of you.": a band on the floor marks the fire's channel (pale blue: it will burn; bright with a whistle: 1.2 sec; amber flames: burning) ([#356](https://github.com/slopmaster69/Basin-Battle/pull/356)). **Risk:** the whistle was never heard; the mark fades under steam; the valve has no moving part.
- [ ] Gilnid's valve as a race: reach him and stagger him while his hands are on it (one heavy blow, or 120 posture): no fire, "No! You've jammed the valve!", he reels 4 sec ([#356](https://github.com/slopmaster69/Basin-Battle/pull/356)). **Risk:** a real pull was never played; only the scripted warrior won.
- [ ] Gilnid with the markers off: the spill, the sticky floor and the valve lane read without rings ([#393](https://github.com/slopmaster69/Basin-Battle/pull/393)).
- [ ] Gilnid's second engineer: shoot or strike the one lit pole to bring him down, with no timer. Aim the reticle at the pole (ranged) ([#371](https://github.com/slopmaster69/Basin-Battle/pull/371)).
- [ ] Leave Gilnid's fight with the last hero: the whole fight is back at once for whoever comes next ([#406](https://github.com/slopmaster69/Basin-Battle/pull/406)).
- [ ] Sneed's Shredder stall: it opens with a backfire (sound 971), lasts 4.5 sec, runs longer after a crash into a wall, and ends on a 1 sec count (the catch's roar, the smoke stopping) ([#367](https://github.com/slopmaster69/Basin-Battle/pull/367)).
- [ ] The Shredder's saw, stamp and rake look different from one another, and Sneed's opening can be hit ([#374](https://github.com/slopmaster69/Basin-Battle/pull/374)).
- [ ] The broken Shredder crashes into its log pile, Sneed crawls out of the wreck, and a rigged log dropped on him stops his repair ([#364](https://github.com/slopmaster69/Basin-Battle/pull/364)). **Risk:** the repair left alone to the end, a hero under the log, and the cry as the log lands were never seen.
- [ ] The Shredder with the markers off: the log's drop and the tie-off read without rings ([#396](https://github.com/slopmaster69/Basin-Battle/pull/396)).
- [ ] Leave the Shredder fight with the last hero: the whole fight is back at once ([#404](https://github.com/slopmaster69/Basin-Battle/pull/404)).
- [ ] Cookie: the pour and the breath read with the markers off ([#395](https://github.com/slopmaster69/Basin-Battle/pull/395)). Kill him, or wipe, during the 5.5 sec reel after the pot is knocked: the pot steams again ([#361](https://github.com/slopmaster69/Basin-Battle/pull/361)).
- [ ] Leave Cookie's fight with the last hero: the whole kitchen is back ([#426](https://github.com/slopmaster69/Basin-Battle/pull/426)).
- [ ] Captain Greenskin: the harpoon comes after every third blow, and the last gunner left is the dangerous one ([#390](https://github.com/slopmaster69/Basin-Battle/pull/390)). **Risk:** his step back is straight away from the hero, not along the rail.
- [ ] Greenskin's firing line reads with the markers off ([#394](https://github.com/slopmaster69/Basin-Battle/pull/394)); leaving with the last hero puts the whole fight back ([#415](https://github.com/slopmaster69/Basin-Battle/pull/415)).
- [ ] Mr. Smite changes weapon: the rack lights, he faces you, and can't fall before the new weapon is in hand ([#389](https://github.com/slopmaster69/Basin-Battle/pull/389)). Leaving with the last hero puts the fight back ([#411](https://github.com/slopmaster69/Basin-Battle/pull/411)).
- [ ] Edwin VanCleef on the deck: his Blackguards ring the fight until they leave at 30%. Do 8 guards read as a closed ring, is the shove fair, how does the hop look ([#372](https://github.com/slopmaster69/Basin-Battle/pull/372))?
- [ ] VanCleef's Invitation round with Unbowed: red blades, and standing still is met with the thrust ([#388](https://github.com/slopmaster69/Basin-Battle/pull/388)). Leaving with the last hero puts the fight back ([#420](https://github.com/slopmaster69/Basin-Battle/pull/420)).
- [ ] A small lobbed splash from other enemies reads with the markers off ([#397](https://github.com/slopmaster69/Basin-Battle/pull/397), [#427](https://github.com/slopmaster69/Basin-Battle/pull/427)).

### Shadowfang Keep

- [ ] Enter Shadowfang Keep (both lamps and the worldport). If you fall through the floor: grep the host log for "fell through the world". The line gives the hero's name, where they were, how many seconds after arriving, and where they were put back ([#403](https://github.com/slopmaster69/Basin-Battle/pull/403)).
- [ ] If it happens, also check the client's terminal for `terrain: streaming map Shadowfang`, `worldport: ack sent at settle release` and any `settle: TIMED OUT` ([#403](https://github.com/slopmaster69/Basin-Battle/pull/403)).
- [ ] Talk to Watcher Dodds after Fenros: the link opens the Shadowfang Keep lore page ([#414](https://github.com/slopmaster69/Basin-Battle/pull/414)).

## Basin Battle mode

- [ ] Pick a Hunter in Basin Battle with the Splitting card: a ranged missile also hits up to 2 other enemies within 6 yd of its target for the card's share (50% at rank 1) ([#385](https://github.com/slopmaster69/Basin-Battle/pull/385)).
- [ ] Take Echo (and the two other `echo` cards): abilities fire twice more often, the card's chance added to a spell's own echo ([#386](https://github.com/slopmaster69/Basin-Battle/pull/386)). **Risk:** Echo cards get stronger, so check nothing runs away.
- [ ] Run a Basin Battle round for a hero with Pyroblast, Ice Barrier and the other Basin Battle cards: they still work, unchanged by the Elwynn Survival EX reworks ([#378](https://github.com/slopmaster69/Basin-Battle/pull/378), [#379](https://github.com/slopmaster69/Basin-Battle/pull/379), [#391](https://github.com/slopmaster69/Basin-Battle/pull/391)).

## Camera and visuals

- [ ] Watch the cameraman. After the hero is hidden from a shot for 0.3 sec it cuts within about 1 sec. A shot that still has the hero holds 5 sec ([#354](https://github.com/slopmaster69/Basin-Battle/pull/354), client patch 0093). **Risk:** the client's own tests don't link on the laptop; patch 0094 fixed 16 failures ([#363](https://github.com/slopmaster69/Basin-Battle/pull/363)).
- [ ] The last 3 sec of Rhahk'Zor's collapse: the camera follows the hero down the tunnel, not the fallen rock ([#354](https://github.com/slopmaster69/Basin-Battle/pull/354)).
- [ ] Gilnid's plank: the camera cuts about 4 times in 21 sec, not every 3 sec ([#354](https://github.com/slopmaster69/Basin-Battle/pull/354)).
- [ ] Switch floor markers on, then off: Rhahk'Zor's rings, the surge band and the fire's edge show only with markers on ([#392](https://github.com/slopmaster69/Basin-Battle/pull/392)). Use add-on v68 or later; an older add-on draws them as before.
- [ ] Every boss blow shows an animation or a cast as it winds up, with the markers off ([#428](https://github.com/slopmaster69/Basin-Battle/pull/428)).
- [ ] The Dominated, Disoriented and crowd-control banners read (the add-on's crowd-control line) ([#366](https://github.com/slopmaster69/Basin-Battle/pull/366), [#378](https://github.com/slopmaster69/Basin-Battle/pull/378)).
- [ ] Look at the new EX effects' stock looks: Divine Star (Holy Wrath's bolt 2812), the dominated glow (605), Ice Lance (Frostbolt 116), Blast Wave (11113), Prowl (5215) ([#366](https://github.com/slopmaster69/Basin-Battle/pull/366), [#379](https://github.com/slopmaster69/Basin-Battle/pull/379), [#391](https://github.com/slopmaster69/Basin-Battle/pull/391)).

## Networking feel

- [ ] Walk in a circle round a waiting creature (a boar, a raider) in the Elwynn wave, close in. Its turning may look steppy: it should turn in small snaps of about 6 degrees at least 4 times a second, not smoothly ([#418](https://github.com/slopmaster69/Basin-Battle/pull/418)). **Risk:** turning may look steppy. The 1.12 client has no facing interpolation for a standing creature; the throttle trades smoothness for 28% less traffic.
- [ ] Wait out a creature's wind-up while it faces you: the facing at the moment of the attack is exact, not a few degrees off ([#418](https://github.com/slopmaster69/Basin-Battle/pull/418)).
- [ ] Play a raid with many raiders and several heroes: no new lag or rubber-banding in the Elwynn wave, and the Deadmines bosses (2 to 5% less traffic) feel as before ([#418](https://github.com/slopmaster69/Basin-Battle/pull/418)).
- [ ] With 2 heroes, leave and rejoin mid-fight: nothing resets under the hero who stays ([#399](https://github.com/slopmaster69/Basin-Battle/pull/399), [#357](https://github.com/slopmaster69/Basin-Battle/pull/357)).
