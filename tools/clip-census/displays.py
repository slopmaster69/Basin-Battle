"""Every mob's display id in content/*.toml, one a line with its key and name, for the creature census.

    python3 tools/clip-census/displays.py > logs/displays.txt
    scripts/clip-census.sh --displays logs/displays.txt          # writes logs/creature-census.md

Run from anywhere; it reads the repository's content/ folder. A display used by several mobs is
listed once, with every mob after it; the census reads only the number and skips the `#` comment.
"""
import collections, pathlib, tomllib

content = pathlib.Path(__file__).resolve().parents[2] / "content"
mobs = collections.defaultdict(list)  # display -> ["file:key name", ...]
for path in sorted(content.glob("*.toml")):
    with open(path, "rb") as f:
        data = tomllib.load(f)
    for mob in data.get("mob", []):
        display = mob.get("display")
        if isinstance(display, int):
            mobs[display].append(f"{path.stem}:{mob.get('key', '?')} {mob.get('name', '')}".strip())
for display, who in sorted(mobs.items()):
    print(f"{display}  # {'; '.join(who)}")
