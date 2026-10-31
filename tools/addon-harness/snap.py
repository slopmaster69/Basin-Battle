#!/usr/bin/env python3
"""Draw a snapshot of an add-on's screen (the harness's `%SNAP` render list) to a PNG.

    snap.py <list.json> <out.png> [art folder]

A stand-in for the game's own drawing, to look at a layout: the add-on's own art is drawn as it
is (its .tga files), the game's icons as labelled tiles (no game files are read), and text in
a near font. The backdrop is a plain dark field. Colours, alpha, texture coordinates and
justification follow the list.
"""
import json
import os
import sys
import zlib

from PIL import Image, ImageDraw, ImageFont

FONTS = {
    "MORPHEUS": "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Bold.ttf",
    "FRIZQT": "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "ARIALN": "/usr/share/fonts/truetype/dejavu/DejaVuSansCondensed.ttf",
}
FALLBACK_FONT = "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"


def font_for(name, px):
    path = FALLBACK_FONT
    for k, v in FONTS.items():
        if k in name.upper():
            path = v
    return ImageFont.truetype(path, max(6, int(round(px))))


def main():
    src, out = sys.argv[1], sys.argv[2]
    art = sys.argv[3] if len(sys.argv) > 3 else os.path.join(os.path.dirname(__file__), "../../addon/ElwynnSurvival/art")
    quads = [json.loads(l) for l in open(src) if l.strip()]
    # The screen: the full-screen red edge's rect when it's there (it covers UIParent exactly).
    full = [q for q in quads if q["k"] == "screen"] or [q for q in quads if q.get("path") and q["path"].endswith("vignette")]
    quads = [q for q in quads if q["k"] != "screen"]
    if full:
        ui_w, ui_h = full[0]["r"][2] - full[0]["r"][0], full[0]["r"][3] - full[0]["r"][1]
    else:
        ui_w = max(q["r"][2] for q in quads)
        ui_h = max(q["r"][3] for q in quads)
    W, H = 1600, int(round(1600 * ui_h / ui_w))
    k = W / ui_w
    img = Image.new("RGBA", (W, H), (0, 0, 0, 255))
    # A dusky field, darker at the top: something like a forest floor at dusk.
    bg = ImageDraw.Draw(img)
    for y in range(H):
        t = y / H
        bg.line([(0, y), (W, y)], fill=(int(28 + 30 * t), int(38 + 34 * t), int(30 + 18 * t), 255))
    cache = {}

    def texture(path):
        if path in cache:
            return cache[path]
        name = path.replace("\\", "/").split("/")[-1]
        im = None
        if "AddOns" in path:
            f = os.path.join(art, name + ".tga")
            if os.path.exists(f):
                im = Image.open(f).convert("RGBA")
        cache[path] = im
        return im

    for q in quads:
        l, b, r, t = q["r"]
        x0, x1 = l * k, r * k
        y0, y1 = (ui_h - t) * k, (ui_h - b) * k
        w, h = int(round(x1 - x0)), int(round(y1 - y0))
        if w <= 0 or h <= 0 or x1 < 0 or y1 < 0 or x0 > W or y0 > H:
            continue
        a = q.get("a", 1.0)
        if q["k"] == "tex":
            c = q.get("color") or [1, 1, 1, 1]
            path = q.get("path")
            if path:
                im = texture(path)
                if im is None:
                    # A game icon: a tile in a colour of its name, with its first letters.
                    name = path.replace("\\", "/").split("/")[-1]
                    hue = zlib.crc32(name.encode()) & 0xFFFFFF
                    rgb = ((hue >> 16) & 255, (hue >> 8) & 255, hue & 255)
                    rgb = tuple(int(60 + v * 0.6) for v in rgb)
                    tile = Image.new("RGBA", (w, h), rgb + (255,))
                    d = ImageDraw.Draw(tile)
                    label = "".join(p[0] for p in name.split("_")[1:3]) or name[:2]
                    fnt = font_for("ARIALN", max(6, h * 0.35))
                    d.text((w / 2, h / 2), label, font=fnt, fill=(255, 255, 255, 200), anchor="mm")
                    im2 = tile
                else:
                    tc = q.get("tc")
                    src_im = im
                    if tc:
                        iw, ih = im.size
                        L, R, T, B = tc
                        box = (int(min(L, R) * iw), int(min(T, B) * ih), max(int(max(L, R) * iw), int(min(L, R) * iw) + 1), max(int(max(T, B) * ih), int(min(T, B) * ih) + 1))
                        src_im = im.crop(box)
                    im2 = src_im.resize((w, h), Image.BILINEAR)
                if q.get("desat"):
                    g = im2.convert("LA").convert("RGBA")
                    im2 = g
                px = im2.split()
                rr = px[0].point(lambda v: v * c[0])
                gg = px[1].point(lambda v: v * c[1])
                bb = px[2].point(lambda v: v * c[2])
                aa = px[3].point(lambda v: v * c[3] * a)
                im2 = Image.merge("RGBA", (rr, gg, bb, aa))
            else:
                im2 = Image.new("RGBA", (w, h), (int(c[0] * 255), int(c[1] * 255), int(c[2] * 255), int(c[3] * a * 255)))
            img.alpha_composite(im2, (int(round(x0)), int(round(y0))))
        else:
            c = q.get("color") or [1, 0.82, 0, 1]
            fnt = font_for(q.get("font", ""), q.get("h", 12) * q.get("scale", 1) * k * 0.92)
            layer = Image.new("RGBA", img.size, (0, 0, 0, 0))
            d = ImageDraw.Draw(layer)
            text = q["text"]
            # Strip colour codes, keep the words.
            import re
            text = re.sub(r"\|c[0-9a-fA-F]{8}", "", text).replace("|r", "")
            j = q.get("j", "Center")
            anchor = {"Left": "lm", "Right": "rm"}.get(j, "mm")
            x = x0 if j == "Left" else x1 if j == "Right" else (x0 + x1) / 2
            y = (y0 + y1) / 2
            fill = (int(c[0] * 255), int(c[1] * 255), int(c[2] * 255), int(c[3] * a * 255))
            stroke = 1 if q.get("outline", "None") != "None" else 0
            d.text((x + 1, y + 1), text, font=fnt, fill=(0, 0, 0, int(200 * a)), anchor=anchor)
            d.text((x, y), text, font=fnt, fill=fill, anchor=anchor, stroke_width=stroke, stroke_fill=(0, 0, 0, int(255 * a)))
            img.alpha_composite(layer)
    img.convert("RGB").save(out)


if __name__ == "__main__":
    main()
