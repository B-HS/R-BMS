# /// script
# requires-python = ">=3.11"
# dependencies = ["pillow==12.3.0"]
# ///
from __future__ import annotations

import io
import sys
from pathlib import Path

sys.dont_write_bytecode = True

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
REPOSITORY = ROOT.parent.parent
WEB_APP = REPOSITORY / 'web' / 'src' / 'app'
CHECK_FLAG = '--check'

CANVAS = 1024
SUPERSAMPLE = 4
BODY_INSET = 100
BODY_RADIUS = 185
BORDER_WIDTH = 6

BACKGROUND = (14, 14, 16, 255)
BORDER = (58, 58, 64, 255)
FIELD = (24, 24, 28, 255)
LANE_LINE = (52, 52, 58, 255)
JUDGE_LINE = (245, 245, 245, 255)
WHITE_NOTE = (240, 240, 240, 255)
BLUE_NOTE = (150, 150, 158, 255)
SCRATCH_NOTE = (200, 200, 206, 255)

FIELD_TOP = 238
FIELD_BOTTOM = 786
FIELD_LEFT = 188
FIELD_RIGHT = 836
SCRATCH_SHARE = 1.6
LANE_COUNT = 7
NOTE_HEIGHT = 40
JUDGE_HEIGHT = 18
NOTE_GAP = 8

NOTES = (
    (0, 722),
    (1, 600),
    (2, 676),
    (3, 506),
    (4, 640),
    (5, 420),
    (6, 560),
    (7, 340),
    (3, 290),
    (6, 716),
)

PNG_SIZES = (16, 32, 48, 64, 128, 256, 512, 1024)
ICO_SIZES = (16, 24, 32, 48, 64, 128, 256)
RUNTIME_SIZE = 256
FAVICON_SIZES = (16, 32, 48)
WEB_ICON_SIZE = 512
APPLE_ICON_SIZE = 180
DISCORD_ICON_SIZE = 1024


def scaled(value: int) -> int:
    return value * SUPERSAMPLE


def lane_bounds() -> list[tuple[int, int]]:
    width = FIELD_RIGHT - FIELD_LEFT
    unit = width / (LANE_COUNT + SCRATCH_SHARE)
    scratch_right = FIELD_LEFT + round(unit * SCRATCH_SHARE)
    bounds = [(FIELD_LEFT, scratch_right)]
    for index in range(LANE_COUNT):
        left = scratch_right + round(unit * index)
        right = scratch_right + round(unit * (index + 1))
        bounds.append((left, right))
    return bounds


def note_colour(lane: int) -> tuple[int, int, int, int]:
    if lane == 0:
        return SCRATCH_NOTE
    return BLUE_NOTE if lane % 2 == 0 else WHITE_NOTE


def master(full_bleed: bool = False) -> Image.Image:
    size = scaled(CANVAS)
    image = Image.new('RGBA', (size, size), BACKGROUND if full_bleed else (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    if not full_bleed:
        body = (scaled(BODY_INSET), scaled(BODY_INSET), scaled(CANVAS - BODY_INSET), scaled(CANVAS - BODY_INSET))
        draw.rounded_rectangle(body, radius=scaled(BODY_RADIUS), fill=BACKGROUND, outline=BORDER, width=scaled(BORDER_WIDTH))
    draw.rectangle((scaled(FIELD_LEFT), scaled(FIELD_TOP), scaled(FIELD_RIGHT), scaled(FIELD_BOTTOM)), fill=FIELD)
    lanes = lane_bounds()
    for left, _ in lanes[1:]:
        draw.rectangle((scaled(left) - SUPERSAMPLE, scaled(FIELD_TOP), scaled(left) + SUPERSAMPLE, scaled(FIELD_BOTTOM)), fill=LANE_LINE)
    for lane, top in NOTES:
        left, right = lanes[lane]
        rect = (scaled(left + NOTE_GAP), scaled(top), scaled(right - NOTE_GAP), scaled(top + NOTE_HEIGHT))
        draw.rectangle(rect, fill=note_colour(lane))
    judge_top = FIELD_BOTTOM - JUDGE_HEIGHT
    draw.rectangle((scaled(FIELD_LEFT), scaled(judge_top), scaled(FIELD_RIGHT), scaled(FIELD_BOTTOM)), fill=JUDGE_LINE)
    return image.resize((CANVAS, CANVAS), Image.Resampling.LANCZOS)


def png_bytes(image: Image.Image) -> bytes:
    buffer = io.BytesIO()
    image.save(buffer, format='PNG', optimize=True)
    return buffer.getvalue()


def outputs() -> dict[str | Path, bytes]:
    icon = master()
    sized = {size: icon if size == CANVAS else icon.resize((size, size), Image.Resampling.LANCZOS) for size in PNG_SIZES}
    files = {f'png/icon-{size}.png': png_bytes(image) for size, image in sized.items()}
    icns = io.BytesIO()
    icon.save(icns, format='ICNS', append_images=[sized[size] for size in PNG_SIZES if size != CANVAS])
    files['R-BMS.icns'] = icns.getvalue()
    ico = io.BytesIO()
    sized[RUNTIME_SIZE].save(ico, format='ICO', sizes=[(size, size) for size in ICO_SIZES])
    files['rbms.ico'] = ico.getvalue()
    favicon = io.BytesIO()
    sized[RUNTIME_SIZE].save(favicon, format='ICO', sizes=[(size, size) for size in FAVICON_SIZES])
    files[WEB_APP / 'favicon.ico'] = favicon.getvalue()
    files[WEB_APP / 'icon.png'] = png_bytes(sized[WEB_ICON_SIZE])
    apple = master(full_bleed=True).convert('RGB').resize((APPLE_ICON_SIZE, APPLE_ICON_SIZE), Image.Resampling.LANCZOS)
    files[WEB_APP / 'apple-icon.png'] = png_bytes(apple)
    discord = master(full_bleed=True).convert('RGB').resize((DISCORD_ICON_SIZE, DISCORD_ICON_SIZE), Image.Resampling.LANCZOS)
    files[f'png/discord-{DISCORD_ICON_SIZE}.png'] = png_bytes(discord)
    return files


def main() -> int:
    check = CHECK_FLAG in sys.argv[1:]
    stale = []
    for name, data in outputs().items():
        path = ROOT / name
        if check:
            if not path.exists() or path.read_bytes() != data:
                stale.append(str(path.relative_to(REPOSITORY)))
            continue
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        print(f'wrote {path.relative_to(REPOSITORY)} ({len(data)} bytes)')
    if stale:
        print('out of date: ' + ', '.join(stale) + ' (run uv run assets/icon/tools/generate-icon.py)', file=sys.stderr)
        return 1
    if check:
        print('icon files match')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
