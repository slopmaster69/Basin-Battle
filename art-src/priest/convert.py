import sys, struct, numpy as np
from PIL import Image
src, dst = sys.argv[1], sys.argv[2]
specs = {  # name: (size, crop to content)
 "P1-salvation-ring": ("salvation_ring", 512, False), "P2-salvation-rim": ("salvation_rim", 512, False),
 "P3-salvation-sigil": ("salvation_sigil", 256, True), "P4-holy-light-stack": ("holy_light", 128, True),
 "P5-grace-mote": ("grace_mote", 64, True), "P6-grace-halo": ("grace_halo", 256, False),
 "P7-shield-perfect": ("shield_perfect", 256, True), "P8-flay-gauge": ("flay_gauge", 512, False),
 "P9-flay-taut": ("flay_taut", 512, False), "P10-flay-snap": ("flay_snap", 256, True),
 "P11-penance-bolt": ("penance_bolt", 128, True), "P12-kindled": ("kindled", 256, True), "P13-fade": ("fade", 256, True),
}
import os
for stem, (name, size, crop) in specs.items():
    im = Image.open(os.path.join(src, stem + ".png")).convert("RGB")
    rgb = np.asarray(im).astype(np.float64) / 255.0
    if crop:
        lit = rgb.max(axis=2) > 0.04
        ys, xs = np.where(lit)
        cx, cy = (xs.min() + xs.max()) / 2, (ys.min() + ys.max()) / 2
        half = max(xs.max() - xs.min(), ys.max() - ys.min()) / 2 * 1.08 + 4
        box = (int(cx - half), int(cy - half), int(cx + half), int(cy + half))
        im = im.crop(box)  # pads with black outside the image
    im = im.resize((size, size), Image.LANCZOS)
    rgb = np.asarray(im).astype(np.float64) / 255.0
    a = np.clip((rgb.max(axis=2) - 0.03) / 0.97, 0.0, 1.0)
    col = np.where(a[..., None] > 1e-3, np.clip(rgb / np.maximum(a[..., None], 1e-3), 0, 1), 0)
    out = np.clip(np.concatenate([col, a[..., None]], axis=2) * 255 + 0.5, 0, 255).astype(np.uint8)[:, :, [2, 1, 0, 3]]
    with open(os.path.join(dst, "priest_" + name + ".tga"), "wb") as f:
        f.write(struct.pack("<BBBHHBHHHHBB", 0, 0, 2, 0, 0, 0, 0, 0, size, size, 32, 0x28))
        f.write(out.tobytes())
    print("priest_" + name + ".tga", size)
