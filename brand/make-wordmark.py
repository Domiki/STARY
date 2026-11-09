"""Draws the STARY wordmark: the letters in SUIT (font/font.otf) with the A taken from brand/logo-A.png.

    python3 brand/make-wordmark.py

Writes brand/logo-wordmark.png (transparent, for dark backgrounds) and the copies the site, the
manual and the app use. Needs Pillow.
"""
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
SIZE = 320  # pixels of one em
SPACE = round(0.38 * SIZE)  # spacing between the letters
DROP = round(0.1 * SIZE)  # the star sits a little below the baseline
MOON = (224, 228, 233, 255)  # --color-moon of the app
COPIES = {
    'site/assets/img/logo-wordmark.png': None,
    'docs/assets/img/logo-wordmark.png': None,
    'ui/src/assets/logo-wordmark.png': 100,  # height in pixels; the app shows it small
}


def draw() -> Image.Image:
    star = Image.open(ROOT / 'brand/logo-A.png').convert('RGBA')
    star = star.crop(star.getchannel('A').getbbox())
    height = round(0.95 * SIZE)
    star = star.resize((round(star.width * height / star.height), height), Image.LANCZOS)
    font = ImageFont.truetype(str(ROOT / 'font/font.otf'), SIZE)
    ascent, descent = font.getmetrics()
    pad = round(0.12 * SIZE)
    parts = 'STARY'
    width = sum(star.width if p == 'A' else font.getlength(p) for p in parts) + SPACE * (len(parts) - 1)
    canvas = Image.new('RGBA', (round(width) + 2 * pad, ascent + descent + 4 * pad), (0, 0, 0, 0))
    pen = ImageDraw.Draw(canvas)
    baseline = 2 * pad + ascent
    x = pad
    for part in parts:
        if part == 'A':
            canvas.alpha_composite(star, (round(x), baseline + DROP - star.height))
            x += star.width + SPACE
        else:
            pen.text((x, baseline), part, font=font, fill=MOON, anchor='ls')
            x += font.getlength(part) + SPACE
    canvas = canvas.crop(canvas.getchannel('A').getbbox())
    margin = round(canvas.height * 0.1)
    out = Image.new('RGBA', (canvas.width + 2 * margin, canvas.height + 2 * margin), (0, 0, 0, 0))
    out.alpha_composite(canvas, (margin, margin))
    return out


def header(logo: Image.Image) -> Image.Image:
    """The app icon and the wordmark side by side, as one image: the manual's header shows it as its
    logo, so that the browser centers it without any positioning of ours."""
    height = 160
    icon = Image.open(ROOT / 'src-tauri/icons/icon.png').convert('RGBA')
    icon = icon.crop(icon.getchannel('A').getbbox())
    icon = icon.resize((round(icon.width * height / icon.height), height), Image.LANCZOS)
    mark_height = round(height * 0.68)
    mark = logo.resize((round(logo.width * mark_height / logo.height), mark_height), Image.LANCZOS)
    gap = round(height * 0.1)
    out = Image.new('RGBA', (icon.width + gap + mark.width, height), (0, 0, 0, 0))
    out.alpha_composite(icon, (0, 0))
    out.alpha_composite(mark, (icon.width + gap, (height - mark_height) // 2))
    return out


def banner(logo: Image.Image) -> Image.Image:
    """The icon, the wordmark and the motto on the dark of the app: the picture at the top of the README
    (the wordmark has light letters, which cannot be read on the light page of GitHub)."""
    width, height = 1600, 440
    out = Image.new('RGBA', (width, height), (11, 17, 27, 255))  # --color-midnight
    mark_height = 150
    mark = logo.resize((round(logo.width * mark_height / logo.height), mark_height), Image.LANCZOS)
    icon = Image.open(ROOT / 'brand/app-icon.png').convert('RGBA')
    icon = icon.crop(icon.getchannel('A').getbbox()).resize((150, 150), Image.LANCZOS)
    gap = 36
    row = icon.width + gap + mark.width
    left = (width - row) // 2
    top = 100
    out.alpha_composite(icon, (left, top))
    out.alpha_composite(mark, (left + icon.width + gap, top))
    font = ImageFont.truetype(str(ROOT / 'font/font.otf'), 40)
    motto = 'Make your own starry sky story.'
    pen = ImageDraw.Draw(out)
    pen.text((width / 2, 330), motto, font=font, fill=(154, 165, 180, 255), anchor='mm')  # --color-mist
    return out


def main() -> None:
    logo = draw()
    banner(logo).convert('RGB').save(ROOT / 'brand/banner.png', optimize=True)
    header(logo).save(ROOT / 'docs/assets/img/logo-header.png', optimize=True)
    master = ROOT / 'brand/logo-wordmark.png'
    logo.save(master, optimize=True)
    for name, height in COPIES.items():
        target = ROOT / name
        target.parent.mkdir(parents=True, exist_ok=True)
        copy = logo
        if height:
            copy = logo.resize((round(logo.width * height / logo.height), height), Image.LANCZOS)
        copy.save(target, optimize=True)
        print(name, copy.size)


if __name__ == '__main__':
    main()
