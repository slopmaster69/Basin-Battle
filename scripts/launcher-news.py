#!/usr/bin/env python3
"""The dispatches on Play's War Table: the latest changes to each game, from this repository's
history, written as the launcher's news.json. The release workflow runs it on the commit it
builds (with enough history checked out):

    python scripts/launcher-news.py "<folder>/launcher/ui/news.json"

Each squash-merged pull request is one commit whose summary line says what changed for the
player (AGENTS.md), so that line is the dispatch: before a colon is its headline, after it the
sentence under the headline. A change that touches only one game's code is that game's; a
change to both, or to the client both run on, is shared and shows on both. Changes to docs,
scripts and the download itself aren't news.
"""
import json
import re
import subprocess
import sys

ELWYNN = re.compile(r"survival|elwynn|siege|deadmines|stockade|shadowfang|ragefire|razorfen|wailing|cathedral|graveyard|library", re.I)
BASIN = re.compile(r"addon/BasinBattle|htl-server/src/game/|htl-sim/|content/(cards|powers|game|sim)\.toml", re.I)
GAME = re.compile(r"^(crates|content|addon|patches)/")
MONTHS = "January February March April May June July August September October November December".split()


def commits(n):
    out = subprocess.run(["git", "log", f"-{n}", "--no-merges", "--date=short", "--name-only", "--format=\x1e%ad\x1f%s"],
                         capture_output=True, text=True, check=True).stdout
    for block in out.split("\x1e")[1:]:
        head, _, files = block.partition("\n")
        date, subject = head.split("\x1f", 1)
        yield date, subject, [f for f in files.splitlines() if f.strip()]


def dispatch(date, subject, shared):
    subject = re.sub(r"(\s*\(#\d+\))+\s*$", "", subject).strip()
    subject = re.sub(r"\s*\(client patch \d+\)", "", subject)
    title, _, text = subject.partition(": ")
    if text:
        text = text[0].upper() + text[1:]
        if not text.endswith((".", "!", "?")):
            text += "."
    y, m, d = (int(x) for x in date.split("-"))
    return {"title": title, "text": text, "date": f"{MONTHS[m - 1]} {d}", "short": f"{MONTHS[m - 1][:3]} {d}", "shared": shared}


def main():
    news = {"elwynn": [], "basin": []}
    for date, subject, files in commits(80):
        game = [f for f in files if GAME.match(f)]
        if not game:
            continue
        e, b = any(ELWYNN.search(f) for f in game), any(BASIN.search(f) for f in game)
        to = ["elwynn"] if e and not b else ["basin"] if b and not e else ["elwynn", "basin"]
        for g in to:
            if len(news[g]) < 3:
                news[g].append(dispatch(date, subject, len(to) == 2))
        if all(len(v) >= 3 for v in news.values()):
            break
    text = json.dumps(news, indent=1, ensure_ascii=False)
    if len(sys.argv) > 1:
        with open(sys.argv[1], "w", encoding="utf-8") as f:
            f.write(text + "\n")
    else:
        print(text)


if __name__ == "__main__":
    main()
