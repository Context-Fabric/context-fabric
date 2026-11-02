"""Draw the program icons from the Context-Fabric mark (frontend/public/favicon.svg).

The mark consists of rectangles only, so it is drawn directly with Pillow: gold
on a dark rounded tile, in the colours of the interface. Writes icons/ next to
this script: cfabric-browser.ico (Windows), cfabric-browser.icns (macOS) and
cfabric-browser.png (Linux, the window).

    python packaging/make_icons.py      # needs Pillow
"""

import re
from pathlib import Path

from PIL import Image, ImageDraw

HERE = Path(__file__).parent
SVG = HERE.parent / "frontend" / "public" / "favicon.svg"
OUT = HERE / "icons"

SIZE = 1024
TILE = "#1c1b19"  # --ink
RADIUS = 0.22  # corner radius, as part of the size
MARK_SCALE = 0.68  # the mark's longest side, as part of the size


def rectangles(svg: str) -> tuple[tuple[float, float], list[tuple[float, ...]]]:
    """The viewBox size and the rectangles (x0, y0, x1, y1) of the mark."""
    box = re.search(r'viewBox="0 0 ([\d.]+) ([\d.]+)"', svg)
    assert box, "no viewBox"
    rects = []
    for part in re.findall(r"M ([^Z]+) Z", svg):
        numbers = [float(n) for n in re.findall(r"[\d.]+", part)]
        xs, ys = numbers[0::2], numbers[1::2]
        rects.append((min(xs), min(ys), max(xs), max(ys)))
    return (float(box[1]), float(box[2])), rects


def draw() -> Image.Image:
    svg = SVG.read_text(encoding="utf-8")
    (width, height), rects = rectangles(svg)
    gold = re.search(r'fill="(#[0-9A-Fa-f]{6})"', svg)
    assert gold, "no fill colour"

    image = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    pen = ImageDraw.Draw(image)
    pen.rounded_rectangle((0, 0, SIZE - 1, SIZE - 1), round(SIZE * RADIUS), TILE)
    scale = SIZE * MARK_SCALE / max(width, height)
    dx = (SIZE - width * scale) / 2
    dy = (SIZE - height * scale) / 2
    for x0, y0, x1, y1 in rects:
        box = (
            dx + x0 * scale,
            dy + y0 * scale,
            dx + x1 * scale - 1,
            dy + y1 * scale - 1,
        )
        pen.rectangle(box, gold[1])
    return image


def main() -> None:
    OUT.mkdir(exist_ok=True)
    image = draw()
    image.resize((512, 512), Image.LANCZOS).save(OUT / "cfabric-browser.png")
    image.save(
        OUT / "cfabric-browser.ico",
        sizes=[
            (16, 16),
            (24, 24),
            (32, 32),
            (48, 48),
            (64, 64),
            (128, 128),
            (256, 256),
        ],
    )
    image.save(OUT / "cfabric-browser.icns")
    for f in sorted(OUT.iterdir()):
        print(f"{f.name}: {f.stat().st_size} bytes")


if __name__ == "__main__":
    main()
