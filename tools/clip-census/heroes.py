"""Pick the heroes' attack clips from the clip census: one character model's, for every race.

    scripts/clip-census.sh                                  # writes logs/clip-census.md
    python3 tools/clip-census/heroes.py logs/clip-census.md [--dry]

Run from the repository root; it rewrites the `[[moveset]]` tables in `content/survival.toml`
(`--dry` only prints the picks). A hero's own model plays the clips today, so the same blow looks
different on every race (a night elf woman's heavy attack is a leaping flourish, a dwarf's a short
chop). This gives every hero the same, most readable version of each blow, whatever their race.

Readable means: grounded (no flip, no spin, no leap except on the leap attack), a clear stroke that
says what it is (light blows a cut, a sweep or a thrust; heavy blows an overhead or a chop, not
a short jab), and timing that fits: each clip is warped so its impact lands on the blow, so a take
that would need a big speed-up costs (a heavy may hold its coil). A heavy blow may play its grip's
ordinary attack where a model's special is a spin or a leap: an overhead held a beat reads heavy. Consistent means: each weapon's whole set of
blows (its light string, heavy, running, roll and leap attacks) comes from one model, so a hero's
moves look like one fighter's, and the light string's steps use different takes so they read as a
combo. A model used for another weapon is preferred a little, so every grip feels alike.
"""
import collections, math, re, sys, tomllib

census = collections.defaultdict(dict)  # clip -> donor -> {take: (impact, tags, fits)}
for line in open(sys.argv[1]):
    cells = [c.strip() for c in line.strip().strip("|").split("|")]
    if len(cells) < 13 or not cells[0][:1].isdigit():
        continue  # the header, its rule, anything else
    clip, donor, take, lands = int(cells[0].split()[0]), cells[1], int(cells[2]), float(cells[4].split()[0])
    if donor.startswith("Goblin"):
        continue  # not every install has the goblin models, and nothing preloads them
    census[clip].setdefault(donor, {})[take] = (lands, {t.strip() for t in cells[11].split(",")}, {f.strip() for f in cells[12].split(",")})
dry = "--dry" in sys.argv
SHOWY = {"flip", "spin"}


def score(role, tags, fits):
    """How well a take reads as this kind of blow (None: never). A heavy blow is an overhead or a
    chop above all (a sweep will do, a thrust or a jab won't); a light one a clear cut, sweep or
    thrust; the leap a blow brought down from the air or a lunge."""
    if tags & SHOWY:
        return None
    if "leap" in tags and role != "leap":
        return None
    short = "short" in tags
    if role == "light":
        # (A slow take is only as bad as the speed-up it needs: `timing` weighs that.)
        return 2 * bool(fits & {"cut", "flurry"}) + bool(tags & {"sweep", "thrust", "overhead", "chop", "upswing"}) - 3 * short
    if role == "heavy":
        # (An overhead's wind-up, the weapon raised over the head first, is the plainest warning.)
        return 4 * bool(tags & {"overhead", "chop"}) + ("overhead" in tags) + ("sweep" in tags) + 2 * ("heavy" in fits) - 3 * ("quick" in tags) - 3 * short
    if role == "leap":
        return 3 * bool(tags & {"overhead", "chop"}) + ("overhead" in tags) + 2 * ("gap closer" in fits) + ("lunge" in tags) - 3 * short
    return 2 * ("cut" in fits) + bool(tags & {"sweep", "overhead", "chop", "upswing", "thrust"}) - 3 * short  # running, roll


# The clips a blow may play, by the clip its table names: its own first, then its grip's others. A
# heavy blow may take its grip's ordinary attack when that model's special is a spin or a leap (an
# overhead swing held a beat reads as heavy); a one-hander's special may be a one-handed swing.
FAMILY = {57: (57, 17), 58: (58, 18, 19), 17: (17,), 18: (18,), 19: (19,), 85: (85, 88), 88: (88, 85)}
# When a blow lands with each moveset's weapons (their wind-up, on average, and half their
# active moment), before a step's own share: the timing a clip is warped to fit.
WEAPON_IMPACT = {"one_hand": 0.31 + 0.06, "dagger": 0.18 + 0.04, "two_hand": 0.57 + 0.08, "polearm": 0.39 + 0.06}
HEAVY_WINDUP = 1.55


def roles(ms):
    """Each blow of a moveset: (where, index, role, its table, its wind-up share)."""
    out = []
    light = ms.get("light", [])
    for i, s in enumerate(light):
        heavy_finish = i == len(light) - 1 and len(light) > 2 and s.get("windup", 1.0) >= 1.1
        out.append(("light", i, "heavy" if heavy_finish else "light", s, s.get("windup", 1.0)))
    for k in ("heavy", "running", "roll", "leap"):
        if k in ms:
            out.append((k, 0, "heavy" if k == "heavy" else k, ms[k], ms[k].get("windup", HEAVY_WINDUP if k == "heavy" else 1.0)))
    return out


def timing(role, clip_impact, step_impact):
    """What warping a clip to land on the blow costs: a heavy may hold its coil (slowed), only a
    speed-up costs it; anything else costs either way, more the further."""
    rate = clip_impact / max(step_impact, 0.05)
    if rate > 3.0 or rate < 0.35:
        return None  # past what the client will bend a clip
    if role == "heavy":
        return -3 * max(0.0, math.log(rate))
    return -2 * abs(math.log(rate))


def plan(ms, donor, liked):
    """The (clip, take) this model gives each blow of a moveset, and how readable it all is:
    (score, {(where, i): (clip, take)}) or None when it can't play every blow well."""
    total, picks, light_takes, used = 0.0, {}, [], set()
    base = WEAPON_IMPACT.get(ms["key"], 0.4)
    for where, i, role, s, share in roles(ms):
        own = s["clip"]
        if own not in census:
            continue  # not an attack clip the census measures (a bash, a kick): left as it is
        best = None
        for clip in FAMILY.get(own, (own,)):
            for take, (impact, tags, fits) in census.get(clip, {}).get(donor, {}).items():
                sc = score(role, tags, fits)
                t = timing(role, impact, base * share)
                if sc is None or t is None:
                    continue
                sc += t - 0.5 * (clip != own)  # its own clip, all else alike
                if where == "light" and (clip, take) in light_takes:
                    sc -= 4  # a combo's steps read as steps when they differ
                if (clip, take) in used:
                    sc -= 1  # a heavy that isn't also the leap and the running blow, where it can
                if best is None or sc > best[0]:
                    best = (sc, clip, take)
        if best is None:
            return None
        if where == "light":
            light_takes.append((best[1], best[2]))
        picks[(where, i)] = (best[1], best[2])
        used.add((best[1], best[2]))
        total += best[0]
    return total + 0.5 * (donor in liked), picks


text = open("content/survival.toml").read()
movesets = tomllib.loads(text).get("moveset", [])
donors = sorted({d for c in census.values() for d in c})
liked, chosen = set(), {}
for ms in movesets:
    options = [(p[0], d, p[1]) for d in donors if (p := plan(ms, d, liked))]
    if not options:
        print(f"{ms['key']}: no model can play all of it; left as it is", file=sys.stderr)
        continue
    sc, donor, picks = max(options)
    chosen[ms["key"]] = (donor, picks)
    liked.add(donor)
    print(ms["key"], donor, round(sc, 1), *(f"{w}{i}:{c}/{t}" for (w, i), (c, t) in sorted(picks.items())), sep="\t")
    ranked = sorted(options, reverse=True)[:4]
    print("   next best:", ", ".join(f"{d} {round(s, 1)}" for s, d, _ in ranked[1:]), file=sys.stderr)

if dry:
    sys.exit(0)

# Rewrite each moveset's blows in place: `from` and the take on each table the census covered.
out, key, where = [], None, None
light_i = 0
for line in text.split("\n"):
    if line.startswith("["):
        key, where = (None, None) if line.startswith("[[moveset]]") else ("", None)  # (any other table: none of ours)
    m = re.match(r'^key = "([^"]+)"', line)
    if m and key is None:
        key = m.group(1)
    w = re.match(r"^(light|heavy|running|roll|leap) = ", line)
    if w:
        where, light_i = w.group(1), 0
    if key in chosen and re.search(r"\{ clip = \d+", line) and where:
        donor, picks = chosen[key]
        i = light_i if where == "light" else 0
        if (where, i) in picks:
            clip, take = picks[(where, i)]
            line = re.sub(r'from = "[^"]*", ', "", line)
            line = re.sub(r"take = \d+, ", "", line)
            line = re.sub(r"\{ clip = \d+, ", '{ from = "%s", clip = %d, take = %d, ' % (donor, clip, take), line, count=1)
            line = re.sub(r"\{ clip = \d+ \}", '{ from = "%s", clip = %d, take = %d }' % (donor, clip, take), line, count=1)
        if where == "light":
            light_i += 1
    if line.startswith("]") or (where and where != "light"):
        where = None  # the light string ends at its `]`; the others are one line each
    out.append(line)
open("content/survival.toml", "w").write("\n".join(out))
