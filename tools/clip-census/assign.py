"""Pick every humanoid foe's borrowed clips from the clip census.

    scripts/clip-census.sh                                  # writes logs/clip-census.md
    python3 tools/clip-census/assign.py logs/clip-census.md [--dry]

Run from the repository root; it rewrites `content/*.toml` (`--dry` only prints the picks).

Each melee step keeps its clip and gets the donor and take whose version of that clip suits its
job: a flurry's quick strokes, a finisher's cut, a heavy's overhead or spin, a lunge's gap closer.
Flips, leaps and spins are for bosses' heavies and gap closers (a regular foe may spin on a heavy,
leap on a lunge). Timing counts: the clip is warped so its impact lands on the blow, so a take whose
impact is far from the step's is penalised (a heavy may hold its coil, so only a speed-up costs).
Variety counts: a foe reuses a donor only when nothing else fits, a boss never repeats a take.
"""
import collections, glob, math, re, sys, tomllib

GRIP_CLIP = {"2h": 18, "staff": 18, "polearm": 19, "dagger": 85, "1h": 17}

census = collections.defaultdict(dict)  # clip -> donor -> [(take, impact, tags, fits)]
for line in open(sys.argv[1]):
    cells = [c.strip() for c in line.strip().strip("|").split("|")]
    if len(cells) < 13 or not cells[0][:1].isdigit():
        continue  # the header, its rule, anything else
    clip, donor, take, lands = int(cells[0].split()[0]), cells[1], int(cells[2]), float(cells[4].split()[0])
    if donor.startswith("Goblin"):
        continue  # not every install has the goblin models, and nothing preloads them
    census[clip].setdefault(donor, []).append(
        (take, lands, {t.strip() for t in cells[11].split(",")}, {f.strip() for f in cells[12].split(",")}))
dry = "--dry" in sys.argv

use = collections.Counter()  # (role, donor) across all foes, to spread the picks


def role_of(s, i, n, windup):
    if s.get("lunge", 0) > 0:
        return "gap"
    if s.get("heavy") or (s.get("heavy") is None and windup >= 0.85):
        return "heavy"
    if n > 1 and i < n - 1 and windup <= 0.45:
        return "flurry"
    return "cut"


def candidates(clip, role, boss, step_impact, arc):
    out = []
    for donor, takes in census.get(clip, {}).items():
        for take, impact, tags, fits in takes:
            showy = role in ("heavy", "gap")
            if "flip" in tags and not (boss and showy):
                continue
            if "leap" in tags and not (role == "gap" or (boss and showy)):
                continue
            # A spin reads as a blow all round: a regular foe's only on one that is (a wide arc); a
            # boss's signature blows may turn.
            if "spin" in tags and not ((boss and showy) or (role == "heavy" and arc >= 150)):
                continue
            if role == "flurry" and "slow" in tags:
                continue
            score = 0.0
            if role == "flurry":
                score += 3 * ("flurry" in fits) + ("quick" in tags) + ("thrust" in tags or "sweep" in tags) * 0.5
            elif role == "cut":
                score += 2 * ("cut" in fits) + ("sweep" in tags or "overhead" in tags or "chop" in tags or "upswing" in tags)
            elif role == "heavy":
                score += 3 * ("heavy" in fits or "boss special" in fits) + 2 * ("overhead" in tags or "chop" in tags or "wide" in fits)
                if boss:
                    score += ("spin" in tags) + ("leap" in tags) + ("flip" in tags)
            else:  # gap
                score += 3 * ("gap closer" in fits) + ("lunge" in tags) + ("thrust" in tags) * 0.5
                if boss:
                    score += ("leap" in tags) + ("flip" in tags) * 0.5
            rate = impact / max(step_impact, 0.05)
            if role == "heavy":
                score -= 4 * max(0.0, math.log(1 / rate))  # sped up to land: costs; held coil: free
            else:
                score -= 6 * abs(math.log(rate))
            out.append((score, donor, take, impact, tags))
    return out


def pick(cands, role, mine, mine_takes, clip):
    best = None
    for score, donor, take, impact, tags in cands:
        s = score - 2.5 * (donor in mine) - 6 * ((donor, (clip, take)) in mine_takes) - 0.15 * use[(role, donor)]
        s -= 1.5 * use[("take", donor, clip, take)]  # one boss's signature isn't every boss's
        if best is None or s > best[0]:
            best = (s, donor, take, impact, tags)
    return best


report = []
for path in sorted(glob.glob("content/*.toml")):
    text = open(path).read()
    data = tomllib.loads(text)
    mobs = {m["key"]: m for m in data.get("mob", [])}
    lines = text.split("\n")
    key = None
    out = []
    mine, mine_takes = set(), set()
    for i, line in enumerate(lines):
        if re.match(r"^\[\[(dungeon\.)?mob\]\]\s*$", line):
            key = None
            mine, mine_takes = set(), set()
        m = re.match(r'^key = "([^"]+)"', line)
        if m and key is None and m.group(1) in mobs:
            key = m.group(1)
        mob = mobs.get(key) if key else None
        if not mob or "clips_from" not in mob:
            out.append(line)
            continue
        boss = bool(mob.get("boss"))
        attack = mob.get("attack", {})
        base_clip = GRIP_CLIP.get(mob.get("grip", ""), 16 if mob.get("weapon", 0) == 0 else 17)
        if re.match(r'^clips_from = "', line):
            w = attack.get("windup", 0.5) + attack.get("active", 0.1) * 0.5
            # The basic swing plays take 0 or 1 at random: a donor good at both.
            best = None
            for donor, takes in census[base_clip].items():
                t = {tk: (imp, tags, fits) for tk, imp, tags, fits in takes}
                if 0 not in t:
                    continue
                t.setdefault(1, t[0])  # a donor with one take plays it for both
                s = sum(2 * ("cut" in t[k][2]) - 4 * abs(math.log(t[k][0] / w)) - 3 * bool(t[k][1] & {"spin", "leap", "flip"}) for k in (0, 1))
                s -= 0.3 * use[("basic", donor)]
                if best is None or s > best[0]:
                    best = (s, donor)
            use[("basic", best[1])] += 1
            report.append((path, key, "basic", base_clip, best[1], "0/1"))
            out.append('clips_from = "%s"' % best[1])
            continue
        sm = re.match(r"^(\s+steps = )(\[.*\])\s*$", line)
        if sm:
            steps = tomllib.loads("x = " + sm.group(2))["x"]
            n = len(steps)
            idx = [0]

            def rep(mt):
                s = steps[idx[0]]
                si = idx[0]
                idx[0] += 1
                body = mt.group(0)
                if s.get("ranged"):
                    return body
                clip = s.get("clip", base_clip)
                if clip not in census:
                    return body  # a cast or a roar: no impact keyframe to measure
                windup = s.get("windup", attack.get("windup", 0.5))
                impact = windup + attack.get("active", 0.1) * 0.5
                role = role_of(s, si, n, windup)
                best = pick(candidates(clip, role, boss, impact, s.get("arc", attack.get("arc", 90.0))), role, mine, mine_takes, clip)
                if best is None:
                    return body
                _, donor, take, cimp, tags = best
                mine.add(donor)
                mine_takes.add((donor, (clip, take)))
                use[(role, donor)] += 1
                use[("take", donor, clip, take)] += 1
                report.append((path, key, role + (" (boss)" if boss else ""), clip, donor, take, round(impact, 2), cimp, ",".join(sorted(tags))))
                body = re.sub(r'from = "[^"]*", ', "", body)
                body = re.sub(r"take = \d+, ", "", body)
                body = re.sub(r"(clip = \d+, )", r"\1take = %d, " % take, body)
                if "clip = " not in body:
                    body = "{ clip = %d, take = %d, " % (clip, take) + body[2:]
                return '{ from = "%s", ' % donor + body[2:]

            line = sm.group(1) + re.sub(r"\{ [^{}]*\}", rep, sm.group(2))
        out.append(line)
    if not dry:
        open(path, "w").write("\n".join(out))

for r in report:
    print(*r, sep="\t")
print("donors:", collections.Counter(r[4] for r in report).most_common(), file=sys.stderr)
