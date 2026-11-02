#!/usr/bin/env python3
"""Write a readable catalog of a class's level-up cards from its content file.

    python tools/card_catalog.py content/classes/warrior.toml > docs/warrior-cards.md

Every skill with its deck of upgrades, the class talents, the combinations, the weapon paths
and their path cards, straight from the game's own data (Python 3.11 or newer: it reads TOML itself).
"""
import re
import sys
import tomllib

RARITY = {"common": "Common", "uncommon": "Uncommon", "rare": "Rare", "epic": "Epic", "legendary": "Legendary"}


def text_at(t, rank):
    vals = t.get("values", [])

    def rep(m):
        spec = m.group(1)
        d = re.match(r"\d+", spec)
        unit = spec[d.end():]
        v = vals[int(d.group())] * rank
        if unit == "%":
            return f"{round(v * 100, 1):g}%"
        if unit == "s":
            return f"{v:g} sec"
        if unit == "yd":
            return f"{v:g} yd"
        return f"{v:g}"

    return re.sub(r"\{([^}]*)\}", rep, t["text"])


def cell(s):
    return s.replace("|", "/").replace("\n", " ")


def main(path):
    k = tomllib.load(open(path, "rb"))
    spells = k["spell"]
    by_key = {s["key"]: s for s in spells}
    trees = {t["key"]: t["name"] for t in k["tree"]}
    up = k.get("upgrades", {})
    per = up.get("per_spell", 4)
    power = up.get("power", 0.1)
    cost = up.get("cost", 0)
    talents = k.get("talent", [])
    paths = [t for t in talents if "path" in t]
    path_cards = [t for t in talents if "path_only" in t]
    combos = [t for t in talents if len(t.get("requires", [])) >= 2]
    klass = [t for t in talents if t not in paths and t not in combos and t not in path_cards]
    path_names = {t["path"]: t.get("path_name", t["path"].capitalize()) for t in paths}
    label = k.get("path_label", "Weapon Path")
    labels = label.lower() + ("s" if not label.endswith("s") else "")
    name = path.replace("\\", "/").split("/")[-1].replace(".toml", "").capitalize()
    perks = sum(len(s.get("perk", [])) for s in spells)
    out = []
    w = out.append
    w(f"# {name}: every level-up card")
    w("")
    w(f"_Generated from `{path}` by `tools/card_catalog.py`. Edit the content file, not this page._")
    w("")
    w(f"- **{len(spells)} skills**, up to 6 on your bar at once.")
    w(f"- **{perks} skill upgrades** (about {perks // max(1, len(spells))} per skill). A skill takes **{per}** in a run, so no run sees a whole deck.")
    extra = f" and costs {round(cost * 100):g}% less rage" if cost else ""
    w(f"- Every upgrade also makes its skill hit {round(power * 100):g}% harder{extra}, on top of what the card says.")
    w(f"- **{len(klass)} class talents** (offered whatever skills you hold; some come back for more ranks).")
    w(f"- **{len(combos)} combinations** (offered only while you hold both skills).")
    w(f"- **{len(paths)} {labels}** (one a run, at random, as a Legendary card). It teaches nothing by itself: it opens its own skills to the skill cards (on keys of their own, so a full bar never shuts them out) and its own **{len(path_cards)} path cards**.")
    w("- **Rarity means something**: Common cards are numbers, Uncommon change how you play, Rare light a key, reset or cast, and each skill's **Epic** capstone comes only after 5 of its upgrades (a path's after 4 of its cards). Skills themselves are not rolled for rarity.")
    w("")
    w(f"How to read the upgrade tables: **Rank-up xN** can be taken N times. **Choice: x** means only one upgrade with that choice can be taken for the skill. **After: Y** is offered once Y is taken. **Lights up a key** makes a skill's next cast ready at once and empowered (its key glows until you press it). What each skill is for: [{name.lower()}-roles.md]({name.lower()}-roles.md).")
    w("")
    w("## Skills and their upgrades")
    w("")
    for s in spells:
        bits = [trees.get(s["tree"], s["tree"]), s["kind"]]
        if s.get("cooldown"):
            bits.append(f"{s['cooldown']:g} sec cooldown")
        if s.get("cost"):
            bits.append(f"{s['cost']:g} rage")
        if s.get("core"):
            bits.append("can be your first skill")
        if s.get("path"):
            bits.append(f"{path_names[s['path']]} path only")
        w(f"### {s['name']}")
        w("")
        w("_" + " · ".join(bits) + "_")
        if s.get("note"):
            w("")
            w(s["note"])
        w("")
        w("| Upgrade | Rarity | Kind | What it does |")
        w("|---|---|---|---|")
        names = {p["key"]: p["name"] for p in s.get("perk", [])}
        for p in s.get("perk", []):
            kind = []
            if p.get("repeat", 1) > 1:
                kind.append(f"Rank-up x{p['repeat']}")
            if "fork" in p:
                kind.append(f"Choice: {p['fork']}")
            if "after" in p:
                kind.append(f"After: {names.get(p['after'], p['after'])}")
            if p.get("needs"):
                kind.append(f"Capstone: after {p['needs']} upgrades")
            if "trigger" in p:
                trig = p["trigger"] if isinstance(p["trigger"], list) else [p["trigger"]]
                if any(t.get("buff", {}).get("next") for t in trig):
                    kind.append("Lights up a key")
                else:
                    kind.append("Triggered")
            w(f"| {cell(p['name'])} | {RARITY.get(p.get('rarity', 'common'))} | {', '.join(kind) or '-'} | {cell(p['text'])} |")
        w("")
    w("## Class talents")
    w("")
    w("| Talent | Rarity | Ranks | Rank 1 | Top rank |")
    w("|---|---|---|---|---|")
    for t in klass:
        r = t.get("ranks", 1)
        top = text_at(t, r) if r > 1 else "-"
        needs = f" (needs {by_key[t['requires'][0]]['name']})" if t.get("requires") else ""
        w(f"| {cell(t['name'])}{needs} | {RARITY.get(t.get('rarity', 'common'))} | {r} | {cell(text_at(t, 1))} | {cell(top)} |")
    w("")
    w("## Combinations")
    w("")
    w("| Combination | Rarity | Skills | What it does |")
    w("|---|---|---|---|")
    for t in sorted(combos, key=lambda t: [by_key[r]["name"] for r in t["requires"]]):
        skills = " + ".join(by_key[r]["name"] for r in t["requires"])
        w(f"| {cell(t['name'])} | {RARITY.get(t.get('rarity', 'common'))} | {skills} | {cell(text_at(t, 1))} |")
    w("")
    w(f"## {label}s and path cards")
    w("")
    for t in paths:
        w(f"### {t['name']}")
        w("")
        w(text_at(t, 1))
        w("")
        w("| Path card | Rarity | Kind | What it does |")
        w("|---|---|---|---|")
        for c in [c for c in path_cards if c["path_only"] == t["path"]]:
            kind = []
            r = c.get("ranks", 1)
            if r > 1:
                kind.append(f"Rank-up x{r}")
            if "fork" in c:
                kind.append("Choice")
            if c.get("needs"):
                kind.append(f"Capstone: after {c['needs']} path cards")
            top = f" (rank {r}: {text_at(c, r)})" if r > 1 else ""
            w(f"| {cell(c['name'])} | {RARITY.get(c.get('rarity', 'common'))} | {', '.join(kind) or '-'} | {cell(text_at(c, 1) + top)} |")
        w("")
    # Which skills each skill combines with.
    w("## Which skills combine")
    w("")
    w("| Skill | Combines with |")
    w("|---|---|")
    for s in spells:
        partners = []
        for t in combos:
            if s["key"] in t["requires"]:
                partners += [f"{by_key[r]['name']} ({t['name']})" for r in t["requires"] if r != s["key"]]
        w(f"| {s['name']} | {', '.join(partners) or '-'} |")
    w("")
    sys.stdout.write("\n".join(out))


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "content/classes/warrior.toml")
