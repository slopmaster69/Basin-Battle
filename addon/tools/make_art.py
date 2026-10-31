#!/usr/bin/env python3
"""Draw the Basin Battle add-on's textures (white shapes the add-on tints) as 32-bit TGAs.

    python3 addon/tools/make_art.py

Writes into addon/BasinBattle/art/. Every side is a power of two no larger than 512, as the 1.12
client requires. Needs numpy and Pillow (only for the blur).
"""

import math
import os
import struct

import numpy as np
from PIL import Image, ImageFilter

OUT = os.path.join(os.path.dirname(__file__), "..", "BasinBattle", "art")
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


def white(alpha):
    h, w = alpha.shape
    out = np.ones((h, w, 4), dtype=np.float64)
    out[:, :, 3] = np.clip(alpha, 0, 1)
    return out


def downsample(a, s=SS):
    h, w = a.shape
    return a.reshape(h // s, s, w // s, s).mean(axis=(1, 3))


def rounded_rect_sdf(w, h, x0, y0, x1, y1, r, s=SS):
    """Signed distance (in output pixels, negative inside) to a rounded rectangle, supersampled."""
    ys, xs = np.mgrid[0 : h * s, 0 : w * s]
    px = (xs + 0.5) / s
    py = (ys + 0.5) / s
    cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
    hx, hy = (x1 - x0) / 2 - r, (y1 - y0) / 2 - r
    qx = np.abs(px - cx) - hx
    qy = np.abs(py - cy) - hy
    outside = np.hypot(np.maximum(qx, 0), np.maximum(qy, 0))
    inside = np.minimum(np.maximum(qx, qy), 0)
    return outside + inside - r


def fill(sdf):
    return downsample((sdf <= 0).astype(np.float64))


def stroke(sdf, inner, outer):
    """Coverage of the band inner <= sdf <= outer."""
    return downsample(((sdf >= inner) & (sdf <= outer)).astype(np.float64))


def blur(alpha, radius):
    img = Image.fromarray(np.clip(alpha * 255, 0, 255).astype(np.uint8), "L")
    img = img.filter(ImageFilter.GaussianBlur(radius))
    return np.asarray(img).astype(np.float64) / 255.0


# The card's canvas: 256x512, the card itself at (16,16)-(240,368); the margin holds the glow.
CW, CH = 256, 512
CARD = (16, 16, 240, 368)
CARD_R = 14


def card_textures():
    sdf = rounded_rect_sdf(CW, CH, *CARD, CARD_R)
    body = fill(sdf)
    # The body: a dark vertical gradient, a touch lighter at the top.
    ys = np.linspace(0, 1, CH)[:, None] * np.ones((1, CW))
    t = np.clip((ys * CH - CARD[1]) / (CARD[3] - CARD[1]), 0, 1)
    rgb = (0.115 - 0.06 * t)[:, :, None] * np.array([1.0, 0.97, 0.92])[None, None, :]
    out = np.zeros((CH, CW, 4))
    out[:, :, :3] = rgb
    out[:, :, 3] = body * 0.96
    save_tga("card", out)
    # The rarity wash: strongest at the top, gone by the middle.
    wash = body * np.clip(1.0 - t / 0.62, 0, 1) ** 1.6
    save_tga("cardwash", white(wash * 0.9))
    # The border: a crisp outer line and a faint inner one.
    border = stroke(sdf, -2.2, 0.0) + 0.28 * stroke(sdf, -6.2, -5.2)
    save_tga("cardborder", white(np.clip(border, 0, 1)))
    # The glow outside the border.
    ring = stroke(sdf, -1.0, 3.0)
    glow = blur(ring, 7) * 1.6 + blur(ring, 2.5) * 0.6
    glow *= np.clip(sdf_to_out(sdf) / 1.0, 0, 1) * 0.0 + 1.0
    save_tga("cardglow", white(np.clip(glow, 0, 1)))
    # A white sheet for the pick flash.
    save_tga("cardflash", white(body))


def sdf_to_out(sdf):
    return downsample(sdf)


def rays():
    n = 512
    ys, xs = np.mgrid[0:n, 0:n]
    dx = (xs + 0.5 - n / 2) / (n / 2)
    dy = (ys + 0.5 - n / 2) / (n / 2)
    r = np.hypot(dx, dy)
    a = np.arctan2(dy, dx)
    count = 16
    wedge = (np.cos(a * count) * 0.5 + 0.5) ** 3.0
    wedge2 = (np.cos(a * count * 0.5 + 0.7) * 0.5 + 0.5) ** 6.0
    # Gone well inside the edge: the add-on spins it by its texture coordinates, which show only
    # the square inscribed in the circle of radius 0.5.
    falloff = np.clip(1.0 - r / 0.68, 0, 1) ** 1.4
    alpha = (wedge * 0.75 + wedge2 * 0.35) * falloff
    alpha += np.exp(-(r / 0.22) ** 2) * 0.55
    save_tga("rays", white(np.clip(alpha, 0, 1)))


def glow():
    n = 128
    ys, xs = np.mgrid[0:n, 0:n]
    r = np.hypot(xs + 0.5 - n / 2, ys + 0.5 - n / 2) / (n / 2)
    save_tga("glow", white(np.clip(np.exp(-(r / 0.45) ** 2) - 0.02, 0, 1)))


def vignette():
    n = 256
    ys, xs = np.mgrid[0:n, 0:n]
    r = np.hypot(xs + 0.5 - n / 2, ys + 0.5 - n / 2) / (n / 2)
    save_tga("vignette", white(np.clip((r - 0.45) / 0.6, 0, 1) ** 1.5))


def pill():
    w, h = 128, 32
    sdf = rounded_rect_sdf(w, h, 1, 1, w - 1, h - 1, (h - 2) / 2)
    save_tga("pill", white(fill(sdf)))
    save_tga("pillborder", white(stroke(sdf, -1.6, 0.0)))


def circle():
    n = 64
    ys, xs = np.mgrid[0 : n * SS, 0 : n * SS]
    r = np.hypot((xs + 0.5) / SS - n / 2, (ys + 0.5) / SS - n / 2)
    save_tga("circle", white(downsample((r <= n / 2 - 1).astype(np.float64))))
    save_tga("ring", white(downsample(((r <= n / 2 - 1) & (r >= n / 2 - 4)).astype(np.float64))))


def button():
    w, h = 256, 64
    sdf = rounded_rect_sdf(w, h, 2, 2, w - 2, h - 2, 9)
    save_tga("button", white(fill(sdf)))
    save_tga("buttonborder", white(stroke(sdf, -2.0, 0.0)))
    # A soft top highlight.
    ys = np.linspace(0, 1, h)[:, None] * np.ones((1, w))
    save_tga("buttonshine", white(fill(sdf) * np.clip(1 - ys / 0.55, 0, 1) * 0.6))


def line():
    w, h = 256, 8
    xs = np.linspace(-1, 1, w)[None, :] * np.ones((h, 1))
    ys = np.linspace(-1, 1, h)[:, None] * np.ones((1, w))
    a = np.clip(1 - np.abs(xs) ** 2, 0, 1) * np.clip(1 - np.abs(ys) * 1.6, 0, 1)
    save_tga("line", white(a))


def banner():
    w, h = 512, 128
    xs = np.linspace(-1, 1, w)[None, :] * np.ones((h, 1))
    ys = np.linspace(-1, 1, h)[:, None] * np.ones((1, w))
    a = np.clip(1 - np.abs(xs) ** 3, 0, 1) * np.clip(1 - np.abs(ys) ** 4, 0, 1)
    out = np.zeros((h, w, 4))
    out[:, :, 3] = a * 0.82
    save_tga("banner", out)
    # The gilded edge lines above and below.
    edge = np.clip(1 - np.abs(xs) ** 2, 0, 1) * (np.exp(-((np.abs(ys) - 0.93) / 0.03) ** 2))
    save_tga("banneredge", white(np.clip(edge, 0, 1)))


def iconframe():
    n = 128
    sdf = rounded_rect_sdf(n, n, 4, 4, n - 4, n - 4, 10)
    frame = stroke(sdf, -5.0, 0.0)
    shade = fill(sdf) * np.clip((sdf_to_out(sdf) + 16) / 16, 0, 1) * 0.5
    a = np.clip(frame + shade, 0, 1)
    # The frame's ring is white (tinted by rarity); the inner shade is black.
    out = np.zeros((n, n, 4))
    out[:, :, :3] = frame[:, :, None]
    out[:, :, 3] = a
    save_tga("iconframe", out)
    save_tga("iconmask", white(fill(sdf)))


def bar():
    w, h = 256, 32
    ys = np.linspace(0, 1, h)[:, None] * np.ones((1, w))
    a = np.ones((h, w))
    out = np.ones((h, w, 4))
    shade = 0.75 + 0.25 * np.cos((ys - 0.3) * math.pi)
    out[:, :, :3] = shade[:, :, None]
    out[:, :, 3] = a
    save_tga("bar", out)


def main():
    os.makedirs(OUT, exist_ok=True)
    card_textures()
    rays()
    glow()
    vignette()
    pill()
    circle()
    button()
    line()
    banner()
    iconframe()
    bar()
    print("art written to", os.path.normpath(OUT))


if __name__ == "__main__":
    main()
