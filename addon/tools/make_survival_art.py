#!/usr/bin/env python3
"""Draw the Elwynn Survival add-on's own textures (white shapes the add-on tints) as 32-bit TGAs.

    python3 addon/tools/make_survival_art.py

Writes into addon/ElwynnSurvival/art/: the aiming reticle and its dot, the hit marker, the
equipment slots' frame and well, the vitals' frame, the screen-edge vignette (the hit flash), a
soft band for the death screen and the kill feed, a diamond, and the windows' panel, frame, row
band and divider. The older pieces there (bar,
banner, button, card, circle, ring) come from make_art.py. Every side is a power of two no larger
than 512, as the 1.12 client requires. Needs numpy and Pillow.
"""

import math
import os
import struct

import numpy as np
from PIL import Image, ImageFilter

OUT = os.path.join(os.path.dirname(__file__), "..", "ElwynnSurvival", "art")
SS = 4  # supersampling for smooth edges


def save_tga(name, rgba):
    """rgba: float array (h, w, 4) in 0..1."""
    h, w, _ = rgba.shape
    assert w & (w - 1) == 0 and h & (h - 1) == 0 and max(w, h) <= 512, name
    a = np.clip(rgba * 255.0 + 0.5, 0, 255).astype(np.uint8)
    bgra = a[:, :, [2, 1, 0, 3]]
    header = struct.pack("<BBBHHBHHHHBB", 0, 0, 2, 0, 0, 0, 0, 0, w, h, 32, 0x28)
    with open(os.path.join(OUT, name + ".tga"), "wb") as f:
        f.write(header)
        f.write(bgra.tobytes())


def white(alpha, shade=None):
    h, w = alpha.shape
    out = np.ones((h, w, 4), dtype=np.float64)
    if shade is not None:
        for c in range(3):
            out[:, :, c] = shade
    out[:, :, 3] = np.clip(alpha, 0, 1)
    return out


def grid(w, h, s=SS):
    ys, xs = np.mgrid[0 : h * s, 0 : w * s]
    return (xs + 0.5) / s, (ys + 0.5) / s


def down(a, s=SS):
    h, w = a.shape
    return a.reshape(h // s, s, w // s, s).mean(axis=(1, 3))


def blur(alpha, radius):
    img = Image.fromarray(np.clip(alpha * 255, 0, 255).astype(np.uint8), "L")
    return np.asarray(img.filter(ImageFilter.GaussianBlur(radius)), dtype=np.float64) / 255.0


def rounded_rect_sdf(w, h, inset, r):
    px, py = grid(w, h)
    cx, cy = w / 2, h / 2
    hx, hy = w / 2 - inset - r, h / 2 - inset - r
    qx = np.abs(px - cx) - hx
    qy = np.abs(py - cy) - hy
    return np.hypot(np.maximum(qx, 0), np.maximum(qy, 0)) + np.minimum(np.maximum(qx, qy), 0) - r


def reticle():
    """A thin ring broken at the four quarters, with short inward ticks: the bow's sight."""
    n = 128
    px, py = grid(n, n)
    dx, dy = px - n / 2, py - n / 2
    r = np.hypot(dx, dy)
    ring = (np.abs(r - 54) <= 2.2).astype(np.float64)
    ang = np.degrees(np.arctan2(dy, dx)) % 90
    gaps = (np.minimum(ang, 90 - ang) < 9).astype(np.float64)
    ring *= 1 - gaps
    ticks = ((np.abs(dx) <= 1.6) & (np.abs(np.abs(dy) - 47) <= 7)) | ((np.abs(dy) <= 1.6) & (np.abs(np.abs(dx) - 47) <= 7))
    a = np.maximum(ring, ticks.astype(np.float64))
    a = down(a)
    save_tga("reticle", white(np.maximum(a, blur(a, 1.5) * 0.5)))


def dot():
    n = 32
    px, py = grid(n, n)
    r = np.hypot(px - n / 2, py - n / 2)
    a = down((r <= 3.2).astype(np.float64))
    save_tga("dot", white(np.maximum(a, blur(a, 2.0) * 0.6)))


def hitmark():
    """Four short diagonal strokes around an empty middle: a blow landed."""
    n = 64
    px, py = grid(n, n)
    dx, dy = px - n / 2, py - n / 2
    u = (dx + dy) / math.sqrt(2)
    v = (dx - dy) / math.sqrt(2)
    arm = lambda a, b: (np.abs(a) <= 1.7) & (np.abs(b) >= 9) & (np.abs(b) <= 22)
    a = down((arm(u, v) | arm(v, u)).astype(np.float64))
    save_tga("hitmark", white(np.maximum(a, blur(a, 1.2) * 0.6)))


def slot():
    """An equipment slot: a bevelled frame (tinted gold) and, apart, its dark well."""
    n = 64
    outer = rounded_rect_sdf(n, n, 2, 7)
    frame = down(((outer <= 0) & (outer >= -3.2)).astype(np.float64))
    # A brighter top-left edge, darker bottom-right: a little bevel.
    px, py = np.mgrid[0:n, 0:n]
    shade = 0.75 + 0.25 * (1 - (px + py) / (2 * n))
    save_tga("slot", white(frame, shade))
    well = down((outer <= -2).astype(np.float64))
    # Darker toward the rim: a recessed well.
    inner = down(np.clip(-outer / 14.0, 0, 1))
    rgba = white(well * (0.55 + 0.45 * (1 - inner)))
    save_tga("slotwell", rgba)


def frame():
    """A long thin bar frame: a rounded outline, 256 by 32."""
    w, h = 256, 32
    sdf = rounded_rect_sdf(w, h, 1, 6)
    a = down(((sdf <= 0) & (sdf >= -2.2)).astype(np.float64))
    save_tga("barframe", white(a))


def vignette():
    """Clear in the middle, rising toward the edges: the hit flash and the low-health pulse."""
    n = 256
    ys, xs = np.mgrid[0:n, 0:n]
    dx = (xs + 0.5) / n * 2 - 1
    dy = (ys + 0.5) / n * 2 - 1
    # Only the outer edge: by the distance to the nearest side, not to the middle, so a wide
    # screen's middle stays clear.
    d = np.minimum(1 - np.abs(dx), 1 - np.abs(dy))
    a = np.clip(1 - d / 0.32, 0, 1) ** 1.7 * 0.82
    save_tga("vignette", white(a))


def band():
    """A soft horizontal band, fading at both ends and the edges: the death screen's backdrop and
    the kill feed's rows."""
    w, h = 512, 64
    ys, xs = np.mgrid[0:h, 0:w]
    u = (xs + 0.5) / w
    v = (ys + 0.5) / h
    horiz = np.clip(np.minimum(u, 1 - u) / 0.28, 0, 1)
    vert = np.clip(np.minimum(v, 1 - v) / 0.35, 0, 1)
    a = (horiz * vert) ** 1.2
    save_tga("band", white(a))


def diamond():
    n = 32
    px, py = grid(n, n)
    a = down(((np.abs(px - n / 2) + np.abs(py - n / 2)) <= 13).astype(np.float64))
    save_tga("diamond", white(a))


def panel():
    """A window: its body (dark, a little lighter under the title, darker to the edges) and,
    apart, its gilded frame (an outer line and a fine inner one) to lay over it; both fill the
    canvas, stretched to the window."""
    n = 256
    ys, xs = np.mgrid[0:n, 0:n]
    u = (xs + 0.5) / n
    v = (ys + 0.5) / n
    edge = np.minimum(np.minimum(u, 1 - u), np.minimum(v, 1 - v))
    shade = 0.55 + 0.45 * np.clip(edge / 0.2, 0, 1)
    shade = shade * (0.85 + 0.15 * np.exp(-((v - 0.06) / 0.12) ** 2))
    save_tga("panel", white(np.ones((n, n)), shade))
    outer = rounded_rect_sdf(n, n, 1.5, 6)
    line = ((outer <= 0) & (outer >= -1.8)) | ((outer <= -5) & (outer >= -5.8))
    save_tga("panelframe", white(down(line.astype(np.float64))))


def rowband():
    """A list row's backdrop: soft at the ends, for the hovered or chosen row."""
    w, h = 512, 64
    ys, xs = np.mgrid[0:h, 0:w]
    u = (xs + 0.5) / w
    v = (ys + 0.5) / h
    horiz = np.clip(np.minimum(u / 0.04, (1 - u) / 0.25), 0, 1)
    vert = np.clip(np.minimum(v, 1 - v) / 0.12, 0, 1)
    save_tga("rowband", white(horiz * vert))


def divider():
    """A thin rule, fading at both ends."""
    w, h = 256, 8
    ys, xs = np.mgrid[0:h, 0:w]
    u = (xs + 0.5) / w
    v = (ys + 0.5) / h
    a = np.clip(1 - np.abs(u * 2 - 1) ** 3, 0, 1) * np.exp(-((v - 0.5) / 0.14) ** 2)
    save_tga("divider", white(a))


def dock():
    """The bottom dock's backing: a dark rounded plate, deeper toward the bottom, its edges soft;
    256 by 64, stretched under the vitals and the ability keys."""
    w, h = 256, 64
    sdf = rounded_rect_sdf(w, h, 2, 14)
    inside = down(np.clip(-sdf / 6.0, 0, 1))
    ys, _ = np.mgrid[0:h, 0:w]
    v = (ys + 0.5) / h
    save_tga("dock", white(inside * (0.75 + 0.25 * v)))


def hairline():
    """A fine rounded outline (a bar's or a key's rim), 256 by 16: crisp at any length."""
    w, h = 256, 16
    sdf = rounded_rect_sdf(w, h, 0.5, 3.5)
    a = down(((sdf <= 0) & (sdf >= -1.1)).astype(np.float64))
    save_tga("hairline", white(a))


def keycap():
    """A key's little plate: a rounded fill, a touch lighter at its top, 32 by 16."""
    w, h = 32, 16
    sdf = rounded_rect_sdf(w, h, 0.5, 4)
    a = down((sdf <= 0).astype(np.float64))
    ys, _ = np.mgrid[0:h, 0:w]
    shade = 0.8 + 0.2 * (1 - (ys + 0.5) / h)
    save_tga("keycap", white(a, shade))


def square():
    """A rounded square, filled, 64 by 64: a card's or a slot's plate, a cooldown's shade."""
    n = 64
    sdf = rounded_rect_sdf(n, n, 1, 8)
    save_tga("square", white(down((sdf <= 0).astype(np.float64))))


def squareline():
    """The same square's rim alone, fine, 64 by 64."""
    n = 64
    sdf = rounded_rect_sdf(n, n, 1, 8)
    save_tga("squareline", white(down(((sdf <= 0) & (sdf >= -1.6)).astype(np.float64))))


def main():
    os.makedirs(OUT, exist_ok=True)
    dock()
    hairline()
    keycap()
    square()
    squareline()
    panel()
    rowband()
    divider()
    reticle()
    dot()
    hitmark()
    slot()
    frame()
    vignette()
    band()
    diamond()
    print("wrote", sorted(f for f in os.listdir(OUT) if f.endswith(".tga")))


if __name__ == "__main__":
    main()
