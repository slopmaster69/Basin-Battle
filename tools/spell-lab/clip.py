#!/usr/bin/env python3
"""A spell lab run as a video to watch and share: its screenshots in time order, each stamped with
its time from the first press, at real speed or slowed (docs/spell-lab.md).

    clip.py <shots> <out.mp4|out.gif> [--every MS] [--lead S] [--speed X] [--crop F] [--width W]
            [--label TEXT]

<shots> is a folder of the run's screenshots (as sheet.py takes them). --speed 0.25 plays it at a
quarter speed. --crop keeps that share of each frame round its middle (1 keeps it all). A .gif is
made for chats that won't play a video.
"""
import argparse
import os
import subprocess
import sys
import tempfile

from PIL import Image, ImageDraw, ImageFont

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from sheet import BOLD, shots_of  # noqa: E402


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("shots")
    ap.add_argument("out")
    ap.add_argument("--every", type=float, default=80.0)
    ap.add_argument("--lead", type=float, default=0.25)
    ap.add_argument("--speed", type=float, default=1.0)
    ap.add_argument("--crop", type=float, default=0.7)
    ap.add_argument("--width", type=int, default=1280)
    ap.add_argument("--label", default="")
    a = ap.parse_args()
    files = shots_of(a.shots)
    if not files:
        sys.exit(f"no screenshots in {a.shots}")
    every = a.every / 1000.0
    font = ImageFont.truetype(BOLD, 28)
    with tempfile.TemporaryDirectory() as tmp:
        for i, f in enumerate(files):
            im = Image.open(f).convert("RGB")
            w, h = im.size
            cw, ch = int(w * a.crop), int(h * a.crop)
            box = ((w - cw) // 2, (h - ch) // 2 + int(h * 0.05 * (a.crop < 1)), (w + cw) // 2, (h + ch) // 2 + int(h * 0.05 * (a.crop < 1)))
            im = im.crop(box)
            H = int(a.width * ch / cw) // 2 * 2
            im = im.resize((a.width, H))
            d = ImageDraw.Draw(im)
            t = i * every - a.lead
            text = f"{t:+.2f} s" + (f"   {a.label}" if a.label else "") + (f"   x{a.speed:g}" if a.speed != 1 else "")
            d.rectangle((0, 0, a.width, 44), fill=(0, 0, 0))
            d.text((12, 6), text, font=font, fill=(255, 210, 120))
            im.save(os.path.join(tmp, f"f{i:04d}.png"))
        fps = a.speed / every
        src = ["-framerate", f"{fps:.4f}", "-i", os.path.join(tmp, "f%04d.png")]
        if a.out.endswith(".gif"):
            vf = f"fps={max(fps, 1):.3f},scale={min(a.width, 800)}:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse"
            cmd = ["ffmpeg", "-y", "-loglevel", "error", *src, "-vf", vf, a.out]
        else:
            # Each frame held as long as it lasted, at a steady 30 fps any player plays.
            cmd = ["ffmpeg", "-y", "-loglevel", "error", *src, "-vf", "fps=30", "-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "20", a.out]
        subprocess.run(cmd, check=True)
    print(f"{a.out}: {len(files)} frames, {len(files) * every / a.speed:.1f} s")


if __name__ == "__main__":
    main()
