"""Draws the STARY app icon: a dark rounded square, three arcs of star trails around a four-pointed star.
The icon is symmetric left to right: the arcs are open at the bottom and the ember part is at the top.

    python3 brand/make-icon.py

Writes brand/app-icon.png (1024 px, transparent corners). The icons of the app (src-tauri/icons) are
made from it with `npx tauri icon brand/app-icon.png`. Needs Pillow.
"""
import math
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
SIZE = 1024
SCALE = 4  # drawn four times larger, then reduced, for smooth edges

MIDNIGHT = (7, 16, 31)  # the dark of the icon, a little deeper than the app's background
TRAIL = (34, 54, 84)
EMBER = (216, 164, 143)  # --color-ember of the app
STAR = (221, 227, 236)

BOX = (40, 40, 985, 985)  # the rounded square
CORNER = 210
RADII = (215.0, 310.0, 405.0)  # middle of each arc
WIDTH = 26.0
EMBER_RADIUS = 310.0  # the ember part of the middle arc is a little wider
EMBER_WIDTH = 34.0
GAP = (60.0, 120.0)  # degrees (clockwise from the right), at the bottom, where the arcs are open
EMBER_AT = -90.0  # the middle of the ember part: the top
EMBER_SPAN = 60.0  # degrees on each side of it, on the middle arc
TIP = 175.0  # of the star, from the center
WAIST = 27.5

# The picture is centered by its outer arc: the middle between the top of that arc and its lowest
# corner (its ends are at the open angle) is the middle of the icon, so the star sits a little
# above the center.
_outer = RADII[-1] + WIDTH / 2
_middle = (BOX[1] + BOX[3]) / 2
CENTER = (512.0, _middle + (_outer - _outer * math.sin(math.radians(GAP[0]))) / 2)


def sector(radius: float, start: float, end: float, width: float = WIDTH) -> list[tuple[float, float]]:
    """The outline of a ring segment between two angles (degrees, y down), flat at both ends."""
    inner, outer = radius - width / 2, radius + width / 2
    steps = max(8, round(abs(end - start) * 2))
    angles = [math.radians(start + (end - start) * i / steps) for i in range(steps + 1)]
    cx, cy = CENTER
    along = [(cx + outer * math.cos(a), cy + outer * math.sin(a)) for a in angles]
    back = [(cx + inner * math.cos(a), cy + inner * math.sin(a)) for a in reversed(angles)]
    return along + back


def draw() -> Image.Image:
    big = Image.new('RGBA', (SIZE * SCALE, SIZE * SCALE), (0, 0, 0, 0))
    pen = ImageDraw.Draw(big)

    def scaled(points):
        return [(x * SCALE, y * SCALE) for x, y in points]

    pen.rounded_rectangle([v * SCALE for v in BOX], radius=CORNER * SCALE, fill=MIDNIGHT + (255,))
    for radius in RADII:
        pen.polygon(scaled(sector(radius, GAP[1], GAP[0] + 360)), fill=TRAIL + (255,))
    pen.polygon(
        scaled(sector(EMBER_RADIUS, EMBER_AT - EMBER_SPAN, EMBER_AT + EMBER_SPAN, EMBER_WIDTH)),
        fill=EMBER + (255,),
    )
    cx, cy = CENTER
    star = [
        (cx, cy - TIP), (cx + WAIST, cy - WAIST), (cx + TIP, cy), (cx + WAIST, cy + WAIST),
        (cx, cy + TIP), (cx - WAIST, cy + WAIST), (cx - TIP, cy), (cx - WAIST, cy - WAIST),
    ]
    pen.polygon(scaled(star), fill=STAR + (255,))
    return big.resize((SIZE, SIZE), Image.LANCZOS)


if __name__ == '__main__':
    icon = draw()
    out = ROOT / 'brand/app-icon.png'
    icon.save(out, optimize=True)
    print(out)
    # the app's top bar shows the icon small
    small = ROOT / 'ui/src/assets/app-icon.png'
    icon.resize((128, 128), Image.LANCZOS).save(small, optimize=True)
    print(small)
