"""Set the dungeons' weapons by tier: a harder dungeon's weapon hits harder than an easier one's.

    python3 tools/gear-tiers.py [--dry]

Run from the repository root; it rewrites `damage = [min, max]` of each dungeon's own weapons
(those its foes carry as `gear` or `trophy`) in `content/<dungeon>.toml` (`--dry` only prints).

Before this a weapon's numbers didn't follow its dungeon's tier: the Deadmines' Smite's Mighty
Hammer (tier 1) out-hit every tier-3 two-hander, and a tier-3 greatsword matched the camp's own.
A weapon's power is its damage a second over its swing (`(min + max) / 2` over wind-up, active
moment and recovery). The weapons the camp's rack has (no dungeon's, nobody's trophy) set each
grip's measure, one-handed and two-handed apart; a dungeon's weapons of a tier are set so that,
taken together, they're `TIER_STEP` more a tier past the camp's (the first tier already a step
up), each keeping half its own lead or lag over the others of its tier, so a fast blade stays
quicker and lighter than a maul and a boss's weapon can still be its tier's best. Only the
damage changes: speed, reach, how many a swing strikes, perks and feel stay the weapon's own.
"""
import collections, glob, re, sys, tomllib

TIER_STEP = 0.15  # each tier this much more than the camp's rack (tier 1: 15%, tier 3: 45%)
KEEP = 0.5  # how much of a weapon's own lead or lag over its tier's others it keeps

dry = "--dry" in sys.argv
weapons, owner, tier, trophies = {}, {}, {}, set()
for path in sorted(glob.glob("content/*.toml")):
    t = tomllib.load(open(path, "rb"))
    for w in t.get("weapon", []):
        weapons[w["key"]] = (w, path)
    d = t.get("dungeon")
    for m in t.get("mob", []):
        trophies.update(m.get("trophy", {}).keys())
        if d is not None:
            for g in m.get("gear", []) + list(m.get("trophy", {}).keys()):
                owner.setdefault(g, path)
    if d is not None:
        tier[path] = d.get("tier", 1)


def power(w):
    return (w["damage"][0] + w["damage"][1]) / 2 / (w["windup"] + w["active"] + w["recovery"])


# Each grip's measure: the camp's rack.
camp = collections.defaultdict(list)
for k, (w, path) in weapons.items():
    if k not in owner and k not in trophies:
        camp[bool(w.get("two_handed"))].append(power(w))
measure = {g: sum(v) / len(v) for g, v in camp.items()}

# Each dungeon tier's weapons, by grip.
groups = collections.defaultdict(list)
for k, (w, path) in weapons.items():
    if k in owner:
        groups[(tier[owner[k]], bool(w.get("two_handed")))].append(k)

new = {}
for (t, two), keys in sorted(groups.items()):
    mean = sum(power(weapons[k][0]) for k in keys) / len(keys)
    target = measure[two] * (1 + TIER_STEP * t)
    for k in keys:
        w = weapons[k][0]
        p = target * (power(w) / mean) ** KEEP
        f = p / power(w)
        lo, hi = round(w["damage"][0] * f), round(w["damage"][1] * f)
        new[k] = (lo, max(hi, lo + 1))
        print(f"tier {t} {'2h' if two else '1h'} {k:24} {w['damage']} -> [{new[k][0]}, {new[k][1]}]  ({power(w):.1f} -> {p:.1f} a second; camp {measure[two]:.1f})")

if dry:
    sys.exit(0)
for path in sorted(set(weapons[k][1] for k in new)):
    out, key, in_weapon = [], None, False
    for line in open(path).read().split("\n"):
        if line.startswith("["):
            in_weapon = line.strip() == "[[weapon]]"
            key = None
        m = re.match(r'^key = "([^"]+)"', line)
        if m and in_weapon:
            key = m.group(1)
        if in_weapon and key in new and re.match(r"^damage = \[\d+, \d+\]", line):
            line = re.sub(r"^damage = \[\d+, \d+\]", "damage = [%d, %d]" % new[key], line)
        out.append(line)
    open(path, "w").write("\n".join(out))
