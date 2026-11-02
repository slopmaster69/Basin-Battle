"""Pick the heroes' attack clips from the clip census: one character model's, for every race.

    scripts/clip-census.sh                                  # writes logs/clip-census.md
    python3 tools/clip-census/heroes.py logs/clip-census.md [--dry]

Run from the repository root; it rewrites the `[[moveset]]` tables in `content/survival.toml`
(`--dry` only prints the picks). A hero's own model plays the clips today, so the same blow looks
different on every race (a night elf woman's heavy attack is a leaping flourish, a dwarf's a short
chop). This gives every hero the same, most readable version of each blow, whatever their race.

Readable means: grounded (no flip, no spin, no leap except on the leap attack), a clear stroke that
says what it is (light blows a cut, a sweep or a thrust; heavy blows an overhead or a chop, not
a short jab), and no slow clip in a quick string. Consistent means: each weapon's whole set of
blows (its light string, heavy, running, roll and leap attacks) comes from one model, so a hero's
moves look like one fighter's, and the light string's steps use different takes so they read as a
combo. A model used for another weapon is preferred a little, so every grip feels alike.
"""
import collections, re, sys, tomllib

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
    """How well a take reads as this kind of blow (None: never)."""
    if tags & SHOWY:
        return None
    if "leap" in tags and role != "leap":
        return None
    if role == "light":
        if "slow" in tags:
            return None
        return 2 * bool(fits & {"cut", "flurry"}) + bool(tags & {"sweep", "thrust", "overhead", "chop"}) - 2 * ("short" in tags)
    if role == "heavy":
        return 3 * bool(fits & {"heavy", "wide"}) + 2 * bool(tags & {"overhead", "chop"}) - 3 * ("quick" in tags) - 2 * ("short" in tags)
    if role == "leap":
        return 2 * ("gap closer" in fits) + ("lunge" in tags) + bool(tags & {"overhead", "chop"})
    return 2 * ("cut" in fits) + bool(tags & {"sweep", "overhead", "chop", "upswing", "thrust"}) - 2 * ("short" in tags)  # running, roll


def roles(ms):
    """Each blow of a moveset: (where, index, role, its table)."""
    out = []
    light = ms.get("light", [])
    for i, s in enumerate(light):
        heavy_finish = i == len(light) - 1 and len(light) > 2 and s.get("windup", 1.0) >= 1.1
        out.append(("light", i, "heavy" if heavy_finish else "light", s))
    for k in ("heavy", "running", "roll", "leap"):
        if k in ms:
            out.append((k, 0, "heavy" if k == "heavy" else k, ms[k]))
    return out


def plan(ms, donor, liked):
    """The takes this model gives a moveset, and how readable it is: (score, {(where, i): take}) or None."""
    total, picks, light_takes = 0.0, {}, []
    for where, i, role, s in roles(ms):
        clip = s["clip"]
        takes = census.get(clip, {}).get(donor)
        if clip not in census:
            continue  # not an attack clip the census measures (a bash, a kick): left as it is
        if not takes:
            return None  # this model can't play every blow: not this one
        best = None
        for take, (impact, tags, fits) in takes.items():
            sc = score(role, tags, fits)
            if sc is None:
                continue
            if where == "light" and (clip, take) in light_takes:
                sc -= 4  # a combo's steps read as steps when they differ
            if best is None or sc > best[0]:
                best = (sc, take)
        if best is None:
            return None
        if where == "light":
            light_takes.append((clip, best[1]))
        picks[(where, i)] = best[1]
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
    print(ms["key"], donor, round(sc, 1), *(f"{w}{i}:{t}" for (w, i), t in sorted(picks.items())), sep="\t")
    ranked = sorted(options, reverse=True)[:4]
    print("   next best:", ", ".join(f"{d} {round(s, 1)}" for s, d, _ in ranked[1:]), file=sys.stderr)

if dry:
    sys.exit(0)

# Rewrite each moveset's blows in place: `from` and the take on each table the census covered.
out, key, where = [], None, None
light_i = 0
for line in text.split("\n"):
    if line.startswith("[[moveset]]"):
        key, where = None, None
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
            take = picks[(where, i)]
            line = re.sub(r'from = "[^"]*", ', "", line)
            line = re.sub(r"take = \d+, ", "", line)
            line = re.sub(r"\{ clip = (\d+), ", lambda mt: '{ from = "%s", clip = %s, take = %d, ' % (donor, mt.group(1), take), line, count=1)
            line = re.sub(r"\{ clip = (\d+) \}", lambda mt: '{ from = "%s", clip = %s, take = %d }' % (donor, mt.group(1), take), line, count=1)
        if where == "light":
            light_i += 1
    if line.startswith("]"):
        where = None
    out.append(line)
open("content/survival.toml", "w").write("\n".join(out))
