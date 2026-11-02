"""Give each move of every foe its own animation, from the clips its model really has.

    scripts/clip-census.sh                                       # logs/clip-census.md
    python3 tools/clip-census/displays.py > logs/displays.txt
    scripts/clip-census.sh --displays logs/displays.txt          # logs/creature-census.md
    python3 tools/clip-census/foes.py logs/creature-census.md logs/clip-census.md [--dry]

Run from the repository root; it rewrites the patterns in `content/*.toml` (`--dry` only prints).

A creature's model has only so many attack clips: a wolf three (two bites and a pounce), a boar
two, a murloc eight. The patterns named clips the model often lacks (`17` on a beast, which only
has `16`), so the client fell back to the same bite for every move, and a boss's string, its
held blow and its lunge looked alike. This picks, for every melee step, a clip and take the model
has (or its `from`/`clips_from` donor has), so that:

- each pattern opens on an animation none of the foe's other patterns opens on, while the model
  has one to spare (a model with fewer clips than patterns repeats the least it can); those
  opening on a heavy blow or a lunge choose first, so the dangerous moves get the clearest clips;
- a heavy step plays the clip with the biggest wind-up (the head or weapon drawn farthest back:
  the census's Tell), a special if the model has one; a lunge one that carries the body
  (`lunge`, `leap`); a quick step a quick one; one with no tell to read (`small`) only if
  nothing else fits;
- a string's steps don't play the same take twice running.

A cast (`ranged = true`) plays its spell's own cast; a foe with more than one casting pattern
gives its second and third their own clip where its model has one (54, both hands thrown up; 55,
a roar; 53, a pointed cast), so a bolt, a volley and a curse look different. A melee step that
already names a cast clip (an arcane explosion's 54) is left as it is. A step with `from` keeps
its donor and picks among the donor's takes of the same grip's clips.
"""
import collections, re, sys, tomllib, glob

ATTACKS = {16, 17, 18, 19, 57, 58, 59, 85, 87, 88, 95, 118, 130, 176, 177}
SPECIALS = {118, 57, 58, 59, 87, 95, 130, 176, 177}
CASTS = (54, 55, 53)
# A weapon-holder keeps to its grip's clips (a staff isn't swung unarmed).
GRIP = {"2h": {18, 58}, "staff": {18, 58, 57}, "polearm": {19, 58}, "dagger": {85, 88, 57}, "1h": {17, 57, 59}, "bow": {17, 57}}

dry = "--dry" in sys.argv
args = [a for a in sys.argv[1:] if not a.startswith("--")]

# The creature census: each display's clips, measured.
own = collections.defaultdict(list)  # display -> [(clip, take, lands, tell, tags)]
for line in open(args[0]):
    if not line.startswith("| ") or not line[2].isdigit():
        continue
    c = [x.strip() for x in line.strip().strip("|").split("|")]
    if len(c) < 12:
        continue
    lands = float(c[5].split()[0]) if c[5] else None
    own[int(c[0])].append((int(c[2].split()[0]), int(c[3]), lands, float(c[10] or 0), {t.strip() for t in c[11].split(",") if t.strip()}))
# A model's displays all measure alike: one display measured stands for any other of its model.
model_of = {}
for line in open(args[0]):
    m = re.match(r"## Display (\d+): (\S+)", line)
    if m:
        model_of[int(m.group(1))] = m.group(2).lower()
by_model = {}
for d, rows in own.items():
    by_model.setdefault(model_of.get(d), rows)

# The character census: each donor's takes of each clip.
donors = collections.defaultdict(list)  # donor -> [(clip, take, lands, tags | fits)]
for line in open(args[1]):
    c = [x.strip() for x in line.strip().strip("|").split("|")]
    if len(c) < 13 or not c[0][:1].isdigit():
        continue
    donors[c[1]].append((int(c[0].split()[0]), int(c[2]), float(c[4].split()[0]), {t.strip() for t in (c[11] + "," + c[12]).split(",") if t.strip()}))


def role(s, mob):
    a = mob.get("attack", {})
    if s.get("lunge", 0) > 0:
        return "lunge"
    if s.get("heavy") or s.get("windup", a.get("windup", 0.5)) >= 0.85:
        return "heavy"
    return "light"


def menu(mob, s, display):
    """(clip, take, score-by-role) the step may play."""
    donor = s.get("from") or mob.get("clips_from")
    out = []
    if donor:
        clip = s.get("clip")
        fam = {clip} if clip not in (None, 16, 17, 18, 19, 85) else {clip or 17, {16: 118, 17: 57, 18: 58, 19: 58, 85: 88}.get(clip or 17, 57)}
        for c, t, lands, tags in donors.get(donor, []):
            if c not in ATTACKS:
                continue
            # Another grip's clip only when the foe's own run out (a kick, a shove, a bash).
            off = 0.0 if c in fam else -2.5
            sc = {"heavy": 2 * bool(tags & {"overhead", "chop", "heavy", "boss special"}) - ("quick" in tags),
                  "lunge": 2 * ("gap closer" in tags) + ("lunge" in tags),
                  "light": bool(tags & {"cut", "flurry"}) - ("slow" in tags)}
            if tags & {"flip", "spin", "leap"} and not mob.get("boss"):
                continue
            out.append((c, t, {k: v + off for k, v in sc.items()}))
        return out
    rows = own.get(display) or by_model.get(model_of.get(display)) or []
    grip = GRIP.get(mob.get("grip", ""))
    for c, t, lands, tell, tags in rows:
        if lands is None or c not in ATTACKS:
            continue
        # Another grip's clip only when its own run out (a kick, a shove, a bash).
        base = 4 * tell - 1.5 * ("small" in tags) - (2.5 if grip and c not in grip else 0.0)
        sc = {"heavy": base + 2 * (c in SPECIALS or "slow" in tags) + ("rear" in tags) - ("quick" in tags),
              "lunge": base + 3 * ("lunge" in tags) + 3 * ("leap" in tags),
              "light": base + ("quick" in tags) - ("slow" in tags) - 3 * ("leap" in tags)}
        out.append((c, t, sc))
    return out


def casts_of(mob, display):
    if mob.get("clips_from"):
        return list(CASTS)
    rows = own.get(display) or by_model.get(model_of.get(display)) or []
    have = {c for c, *_ in rows}
    return [c for c in CASTS if c in have]


report, left = [], []
for path in sorted(glob.glob("content/*.toml")):
    text = open(path).read()
    data = tomllib.loads(text)
    mobs = {m["key"]: m for m in data.get("mob", [])}
    # Plan each foe: the clip and take of every step of every pattern, by pattern name.
    plans = {}
    for key, mob in mobs.items():
        pats = mob.get("pattern", [])
        if not pats or mob.get("critter"):
            continue
        display = mob.get("display")
        firsts = collections.Counter()  # (from, clip, take) a pattern opens on
        plan = {}
        # Taken first: the cast-clip melee steps already set (they keep theirs).
        for p in pats:
            st = p.get("steps", [])
            if st and not st[0].get("ranged") and st[0].get("clip") in CASTS:
                firsts[(st[0].get("from") or mob.get("clips_from"), st[0]["clip"], st[0].get("take", 1))] += 1
        # The melee patterns choosing in turn: those opening on a heavy blow or a lunge first (the
        # dangerous ones get the most readable clip), then the most used.
        def opens_light(p):
            st = p.get("steps", [])
            return not st or role(st[0], mob) == "light"
        order = sorted(range(len(pats)), key=lambda i: (opens_light(pats[i]), -pats[i].get("weight", 1.0)))
        for i in order:
            p = pats[i]
            st = p.get("steps", [])
            if not st or st[0].get("ranged") or st[0].get("clip") in CASTS:
                continue
            picks, prev = [], None
            for n, s in enumerate(st):
                if s.get("ranged") or s.get("clip") in CASTS:
                    picks.append(None)
                    continue
                opts = menu(mob, s, display)
                if not opts:
                    picks.append(None)
                    continue
                r = role(s, mob)
                who = s.get("from") or mob.get("clips_from")
                best = None
                for c, t, sc in opts:
                    v = sc[r]
                    if n == 0:
                        v -= 10 * firsts[(who, c, t)]
                    if prev == (c, t):
                        v -= 3
                    if best is None or v > best[0]:
                        best = (v, c, t)
                _, c, t = best
                if n == 0:
                    if firsts[(who, c, t)]:
                        left.append((path, key, p["name"], c, t))
                    firsts[(who, c, t)] += 1
                picks.append((c, t))
                prev = (c, t)
            plan[p["name"]] = picks
            report.append((path, key, p["name"], " ".join(f"{x[0]}/{x[1]}" if x else "-" for x in picks)))
        # The casting patterns: the first plays its spell's cast; the next take clips of their own.
        free = [c for c in casts_of(mob, display) if not any(k[1] == c for k in firsts)]
        casting = [p for p in pats if p.get("steps") and p["steps"][0].get("ranged") and "clip" not in p["steps"][0]]
        for p in casting[1:]:
            if not free:
                break
            c = free.pop(0)
            plan[p["name"]] = [(c, 0)] + [None] * (len(p["steps"]) - 1)
            report.append((path, key, p["name"], f"cast {c}"))
        plans[key] = plan
    # Rewrite: the steps lines of each pattern of each planned foe.
    out, key, pname = [], None, None
    for line in text.split("\n"):
        if re.match(r"^\[\[mob\]\]\s*$", line):
            key, pname = None, None
        m = re.match(r'^key = "([^"]+)"', line)
        if m and key is None and m.group(1) in mobs:
            key = m.group(1)
        m = re.match(r'^\s+name = "([^"]+)"', line)
        if m and key in plans:
            pname = m.group(1)
        sm = re.match(r"^(\s+steps = )(\[.*\])\s*$", line)
        if sm and key in plans and pname in plans[key]:
            picks = plans[key][pname]
            idx = [0]

            def rep(mt):
                i = idx[0]
                idx[0] += 1
                body = mt.group(0)
                pick = picks[i] if i < len(picks) else None
                if pick is None:
                    return body
                c, t = pick
                body = re.sub(r"clip = \d+, ", "", body)
                body = re.sub(r"take = \d+, ", "", body)
                fm = re.match(r'\{ (from = "[^"]*", )?', body)
                head = fm.group(0)
                return head + "clip = %d, take = %d, " % (c, t) + body[len(head):]

            line = sm.group(1) + re.sub(r"\{ [^{}]*\}", rep, sm.group(2))
        out.append(line)
    if not dry:
        open(path, "w").write("\n".join(out))

for r in report:
    print(*r, sep="\t")
print(f"{len(report)} patterns planned; {len(left)} still open on another's animation (the model has too few):", file=sys.stderr)
for x in left:
    print("   ", *x, file=sys.stderr)
