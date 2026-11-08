# Captain Greenskin: break the firing line

Greenskin remains a short, 520 base-health elite. His pull introduces exactly 2 hostile,
85 base-health deck gunners. They are ordinary damageable creatures on reachable deck, with no
shield, range-only requirement, replacement wave or health threshold. They load for 0.6 sec,
hold a native shouldered gun pose, and release after a 2.2 sec warning. Aim freezes at the
start. A 3 yd sidestep leaves the 0.85 yd shot impact. Blocking also works.

One encounter clock separates their starts by at least 3.6 sec. The captain waits through the
opening volley, including projectile flight. He then has a 5 sec melee window while they reload.
The next volley waits for his current attack and recovery to finish. A gunner death removes that
turn. It never accelerates the remaining gunner or adds a third shot.

The captain has a narrow, committed 3 yd thrust and a 220 degree clearing cut. Both leave a
real recovery. Killing both gunners, at any captain health, unlocks a 2.4 sec gun draw/shot and a
1.7 sec harpoon preparation. The first escalation is always the gun draw. The harpoon pulls up
to 8 yd only after its fair hit resolves: a sidestep, dodge, block or counter can prevent it.
The pull uses the existing collision-limited movement and never snaps through a wall or drop.

The crew shares the captain's encounter and recognizes a living, veiled foe. A party death
retargets without replacing the crew. Wipe, return home, rest/reset, debug removal and captain
death remove crew, held aim, missiles and pending damage. The gunners do not belong to the
captain's lasting pack and carry no plunder/gear. Killing the captain first cleans up survivors
before ordinary elite-pack clear and rewards, exactly once. Knockbacks and pulls still move
gunners normally; their stationary firing AI does not swallow that motion.

## Native data checks

Read-only watcher queries on 8 Nov 2026 checked the installed 1.12 data. Raw answers and map
images remain outside the repository in `Documents/visual-check/out/20261108-greenskin-*.queries.txt`.

- Captain display 7113 is `Character/Goblin/Male/GoblinMale`; gunner display 2347 is HumanMale.
- GoblinMale has native 85 Attack1HPierce, 57 Special1H, 106 LoadRifle, 48 ReadyRifle,
  49 AttackRifle, 108 ReadyThrown and 107 AttackThrown. HumanMale has the gun clips as well.
- Item display 21071 resolves to the installed short shotgun/blunderbuss model.
- Spell 8682 uses LoadRifle and AttackRifle; sounds 1147/1148 are native gun loading/release.
- Spell 6533 uses a native rope/net projectile, already used by Reverse Grapple. The harpoon
  release explicitly uses native AttackThrown 107.
- No one-handed pistol item or spear/harpoon missile was found. The current native asset
  substitutions are a short blunderbuss for the pistol and the rope/net for the harpoon.
  Exact muzzle appearance is unverified; no extra smoke particle or generic floor disk is added.

The actual deck query measured these supports and paths from the authored captain home:

| Point | Measured support | Path |
| --- | --- | --- |
| Captain `[-62,-815,41.5]` | 41.4 | home |
| Gunner `[-58,-813,41.5]` | 41.8 | 5 steps |
| Gunner `[-65,-813,41.5]` | 41.1 | 4 steps |
| Recording hero `[-70,-815,41.5]` | 40.6 | 9 steps |
| Melee approach `[-65,-815,41.5]` | 41.1 | 4 steps |

These are dry ship supports, with the water at -16.1 below them. Nearby coordinates requested
at z40 can select that water floor. Keep the explicit 41.5 setup height. Runtime placement
rejects water, a different floor, a blocked post or a missing walk/path, with a checked nearby
fallback for custom test arenas. No structural props or collision flags are involved.

## Validation and recording

`firing_line_tests.rs` exercises actual scheduled shots and captain attacks, frozen aim and a
successful sidestep, both crew-death gates, no health escalation or replacements, party retarget,
Vanish, wipe/reset/death cleanup, RTT-fair harpoon pull, normal knockback motion, floor rejection,
ordinary sword solo/duo clears, charged projectile inputs, and captain-first elite rewards.
The sword runs use inflated hero HP as controlled pressure fixtures, not balance evidence.

`tools/visual-check/greenskin-firing-line.txt` is an isolated normal-spawn demonstration with
ambient foes cleared and markers off. Debug healing and crew/captain damage are explicitly
labelled assistance. Hero `[-70,-815,41.5]` faces +x before the spawn command, placing the
captain 8 yd ahead at the authored home. Preserve yaw0: the camera controls actual hero facing.
Start with pitch-35/distance22 and a short framing probe. If the ship scenery collides with the
camera, the capture worker can override pitch/distance locally while preserving yaw0.

The sequence shows the opening, sidesteps, captain reload window, one then two gunner deaths,
the captain alone and cleanup. It is a controlled mechanic demonstration, not a complete
unassisted showcase. Real-client native pose, weapon attachment, sound, rope/net appearance,
marks-off readability and fight balance still require the filmed pass. No client patch or
add-on version change is included.
