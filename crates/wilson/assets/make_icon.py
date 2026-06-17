#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Generate `wilson.ico` — Wilson Reborn's own application icon (original flat art,
not copyrighted game assets): a desert island with a palm tree, the theme of Johnny
Castaway. Requires Pillow (`pip install Pillow`). Run it to regenerate `wilson.ico`
next to this script. The icon is drawn at 1024px and downsampled for clean edges."""
import math
import os
from PIL import Image, ImageDraw

S = 1024
img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
d = ImageDraw.Draw(img)


def lerp(a, b, t):
    return tuple(int(a[i] * (1 - t) + b[i] * t) for i in range(3))


# sky gradient
sky_top, sky_hor = (102, 204, 255), (200, 238, 255)
horizon = int(S * 0.54)
for y in range(horizon):
    d.line([(0, y), (S, y)], fill=lerp(sky_top, sky_hor, y / horizon) + (255,))

# sun
sx, sy, sr = S * 0.74, S * 0.16, S * 0.085
d.ellipse([sx - sr, sy - sr, sx + sr, sy + sr], fill=(255, 222, 92, 255))
d.ellipse([sx - sr * 0.6, sy - sr * 0.6, sx + sr * 0.6, sy + sr * 0.6], fill=(255, 238, 150, 255))

# a cloud
for dx, dy, r in [(-1.1, 0.1, 0.55), (0, -0.2, 0.7), (1.1, 0.1, 0.55), (0, 0.25, 0.6)]:
    cx, cy, s = S * 0.26, S * 0.20, S * 0.05
    d.ellipse([cx + dx * s - r * s, cy + dy * s - r * s,
               cx + dx * s + r * s, cy + dy * s + r * s], fill=(255, 255, 255, 235))

# sea bands
bands = [(74, 162, 236), (46, 124, 212), (28, 90, 178), (16, 62, 150)]
bh = S - horizon
for i, c in enumerate(bands):
    d.rectangle([0, horizon + bh * i // len(bands), S, horizon + bh * (i + 1) // len(bands)], fill=c + (255,))

# island
icx, icy, iw, ih = S * 0.5, S * 0.74, S * 0.64, S * 0.205
d.ellipse([icx - iw * 0.62, icy - ih * 0.35, icx + iw * 0.62, icy + ih * 0.95],
          outline=(220, 240, 255, 180), width=int(S * 0.006))
d.ellipse([icx - iw / 2, icy - ih / 2, icx + iw / 2, icy + ih / 2], fill=(232, 202, 128, 255))
d.ellipse([icx - iw * 0.40, icy - ih * 0.55, icx + iw * 0.40, icy + ih * 0.12], fill=(247, 224, 158, 255))

# palm trunk
base_x, base_y = icx + S * 0.03, icy - S * 0.015
top_x, top_y = icx - S * 0.06, S * 0.345
bw_b, bw_t = S * 0.028, S * 0.016
d.polygon([(base_x - bw_b, base_y), (base_x + bw_b, base_y),
           (top_x + bw_t, top_y), (top_x - bw_t, top_y)], fill=(146, 86, 50, 255))
d.polygon([(base_x - bw_b, base_y), (base_x - bw_b + S * 0.012, base_y),
           (top_x - bw_t + S * 0.010, top_y), (top_x - bw_t, top_y)], fill=(120, 66, 38, 255))

# palm fronds
leaf, leaf_dk = (48, 172, 86), (30, 132, 62)
cx, cy = top_x, top_y
for ang_deg, ln, col in [(-168, 0.255, leaf_dk), (-128, 0.285, leaf), (-90, 0.235, leaf),
                         (-52, 0.285, leaf), (-12, 0.255, leaf_dk), (-148, 0.20, leaf_dk),
                         (-32, 0.20, leaf_dk)]:
    a = math.radians(ang_deg)
    ex, ey = cx + math.cos(a) * S * ln, cy + math.sin(a) * S * ln
    pp = a + math.pi / 2
    wx, wy = math.cos(pp) * S * 0.032, math.sin(pp) * S * 0.032
    d.polygon([(cx, cy), (ex + wx, ey + wy), (ex, ey), (ex - wx, ey - wy)], fill=col)
for ddx in (-0.018, 0.012):
    d.ellipse([cx + ddx * S - S * 0.016, cy - S * 0.004, cx + ddx * S + S * 0.016, cy + S * 0.028],
              fill=(92, 56, 32, 255))

icon = img.resize((256, 256), Image.LANCZOS)
out = os.path.join(os.path.dirname(os.path.abspath(__file__)), "wilson.ico")
icon.save(out, sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
print("wrote", out)
