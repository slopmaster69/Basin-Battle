#!/usr/bin/env python3
"""A spell lab run on one sheet: its screenshots in time order, each labelled with its time from
the first step and with what the server sent in the frame's interval (docs/spell-lab.md).

    sheet.py <shots> <trace.txt> <out.png> [--every MS] [--lead S] [--cols N] [--crop F]
             [--from S] [--to S]

<shots> is a folder of the run's screenshots (only those, as the client named them:
WoWScrnShot_MMDDYY_HHMMSS.png, then _2, _3 ... within a second), or a list file of their
paths. --every and --lead default to the trace's header. --crop keeps that share of each frame
round its middle (the hero stands there in the over-the-shoulder view), 0.6 by default.
--from and --to keep only the frames in that window (seconds from the first step).
"""
import argparse
import os
import re
import sys

from PIL import Image, ImageDraw, ImageFont

FONT = "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"
BOLD = "/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf"
NAME = re.compile(r"WoWScrnShot_(\d{6})_(\d{6})(?:_(\d+))?\.(png|jpg|tga)$", re.I)


def order_key(path):
    m = NAME.search(os.path.basename(path))
    if not m:
        return (os.path.basename(path), 0)
    date, time, n, _ = m.groups()
    # MMDDYY to YYMMDD so the date sorts.
    return (date[4:6] + date[0:4] + time, int(n or 1))


def shots_of(src):
    if os.path.isdir(src):
        files = [os.path.join(src, f) for f in os.listdir(src) if NAME.search(f)]
    else:
        files = [l.strip() for l in open(src) if l.strip()]
    return sorted(files, key=order_key)


def read_trace(path):
    head, rows = [], []
    for line in open(path, encoding="utf-8", errors="replace"):
        line = line.rstrip("\n")
        if line.startswith("#"):
            head.append(line[1:].strip())
            continue
        m = re.match(r"([+-]\d+\.\d+) (.*)", line)
        if m:
            rows.append((float(m.group(1)), m.group(2)))
    return head, rows


def short(text, width):
    """An event's line made readable at a glance: the variant and its telling fields."""
    text = re.sub(r"\b(pos|from|to|at|path): \[[^\]]*\]", "", text)
    text = re.sub(r"\[\[.*?\]\]", "[..]", text)
    text = re.sub(r"\s+", " ", text).replace(" ,", ",").replace("{ ", "{").replace(" }", "}")
    return text if len(text) <= width else text[: width - 1] + "~"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("shots")
    ap.add_argument("trace")
    ap.add_argument("out")
    ap.add_argument("--every", type=float)
    ap.add_argument("--lead", type=float)
    ap.add_argument("--cols", type=int, default=4)
    ap.add_argument("--crop", type=float, default=0.6)
    ap.add_argument("--width", type=int, default=560)
    ap.add_argument("--from", dest="t0", type=float, default=-9e9)
    ap.add_argument("--to", dest="t1", type=float, default=9e9)
    a = ap.parse_args()

    head, rows = read_trace(a.trace)
    every, lead = a.every, a.lead
    m = re.search(r"every (\d+) ms from -([\d.]+) s", " ".join(head))
    if m:
        every = every or float(m.group(1))
        lead = lead if lead is not None else float(m.group(2))
    every = (every or 80.0) / 1000.0
    lead = lead if lead is not None else 0.25
    files = shots_of(a.shots)
    if not files:
        sys.exit(f"no screenshots in {a.shots}")
    frames = [(i * every - lead, f) for i, f in enumerate(files)]
    frames = [(t, f) for t, f in frames if a.t0 <= t <= a.t1]

    font = ImageFont.truetype(FONT, 13)
    bold = ImageFont.truetype(BOLD, 16)
    W = a.width
    first = Image.open(frames[0][1])
    fw, fh = first.size
    cw, ch = int(fw * a.crop), int(fh * a.crop)
    box = ((fw - cw) // 2, (fh - ch) // 2 + int(fh * 0.05), (fw + cw) // 2, (fh + ch) // 2 + int(fh * 0.05))
    H = int(W * ch / cw)
    LINES = 6
    cell_h = H + 22 + LINES * 15 + 8
    cols = max(1, a.cols)
    rows_n = (len(frames) + cols - 1) // cols
    top = 30 + 16 * min(len(head), 3)
    sheet = Image.new("RGB", (cols * (W + 8) + 8, top + rows_n * cell_h + 8), (18, 18, 20))
    d = ImageDraw.Draw(sheet)
    for k, h in enumerate(head[:3]):
        d.text((10, 8 + 16 * k), short(h, 200), font=font, fill=(220, 220, 200))
    for n, (t, f) in enumerate(frames):
        x = 8 + (n % cols) * (W + 8)
        y = top + (n // cols) * cell_h
        im = Image.open(f).convert("RGB")
        if im.size != (fw, fh):
            im = im.resize((fw, fh))
        sheet.paste(im.crop(box).resize((W, H)), (x, y))
        d.text((x + 4, y + H + 2), f"{t:+.2f} s", font=bold, fill=(255, 210, 120))
        d.text((x + 90, y + H + 4), os.path.basename(f)[-14:], font=font, fill=(120, 120, 120))
        # What the server sent from this frame's time to the next's.
        said = [r for r in rows if t - every < r[0] <= t]
        for k, (_, text) in enumerate(said[:LINES]):
            colour = (140, 220, 255) if text.startswith("STEP") else (200, 200, 200)
            d.text((x + 4, y + H + 22 + 15 * k), short(text, W // 8), font=font, fill=colour)
        if len(said) > LINES:
            d.text((x + W - 70, y + H + 4), f"+{len(said) - LINES} more", font=font, fill=(255, 120, 120))
    sheet.save(a.out)
    print(f"{a.out}: {len(frames)} frames, {len(rows)} trace lines")


if __name__ == "__main__":
    main()
