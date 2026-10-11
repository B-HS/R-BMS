from __future__ import annotations

import math

from PIL import Image, ImageDraw

from sprites import OPAQUE, WHITE, Sprite, block, font_at, glyph_rows, palette, tinted, word

BAR_SIZE = (728, 64)
BAR_FILL = (92, 115, 160)
BAR_FILL_ALPHA = 26
BAR_FAINT_ALPHA = 13
BAR_TAG_SIZE = 18
BAR_TAG_COLOR = palette('color')['muted']
BAR_TAG_LEFT = 20
BARS = (
    ('bar_song', '', BAR_FILL_ALPHA),
    ('bar_folder', 'FOLDER', BAR_FILL_ALPHA),
    ('bar_table', 'TABLE', BAR_FILL_ALPHA),
    ('bar_course', 'COURSE', BAR_FILL_ALPHA),
    ('bar_missing', 'MISSING', BAR_FAINT_ALPHA),
    ('bar_command', 'COMMAND', BAR_FILL_ALPHA),
    ('bar_search', 'SEARCH', BAR_FILL_ALPHA),
)

LABEL_SIZE = 18
LABELS = (
    ('label_ln', 'LN'),
    ('label_random', 'RANDOM'),
    ('label_mine', 'MINE'),
    ('label_cn', 'CN'),
    ('label_hcn', 'HCN'),
)

LAMP_CELL = 8
LAMP = palette('lamp')
LAMP_COLORS = tuple(LAMP.values())
LAMP_FRAMES: dict[str, tuple[str | None, ...]] = {
    'failed': ('failed', None),
    'exhard': ('normal', 'failed'),
    'fullcombo': ('normal', 'fullcombo', 'hard'),
    'perfect': ('normal', 'assist', 'hard'),
    'max': ('normal', 'assist', 'hard'),
}

METER_SIZE = (512, 4)

SUPERSAMPLE = 4
ICON_SIDE = 32
STAR_SIDE = 24
FULL_TURN = math.tau

GEAR_TEETH = 8
GEAR_TOOTH_REACH = 0.47
GEAR_TOOTH_HALF_WIDTH = 0.09
GEAR_BODY_RADIUS = 0.36
GEAR_HOLE_RADIUS = 0.15

SPEAKER_BOX = (0.10, 0.37, 0.30, 0.63)
SPEAKER_CONE = ((0.30, 0.37), (0.52, 0.16), (0.52, 0.84), (0.30, 0.63))
SPEAKER_WAVES = ((0.20, 0.07), (0.34, 0.07))
SPEAKER_WAVE_CENTER = (0.52, 0.5)
SPEAKER_WAVE_SPAN = (-50, 50)

STAR_POINTS = 5
STAR_OUTER_RADIUS = 0.48
STAR_INNER_RADIUS = 0.20
STAR_OUTLINE_SCALE = 0.62
STAR_CENTER = (0.5, 0.53)
QUARTER_TURN_BACK = -FULL_TURN / 4


def bar(name: str, tag: str, alpha: int) -> Sprite:
    height = BAR_SIZE[1]
    image = Image.new('RGBA', BAR_SIZE, (*BAR_FILL, alpha))
    if tag:
        font = font_at(BAR_TAG_SIZE)
        top, bottom = glyph_rows(BAR_TAG_SIZE, tag)
        mask = Image.new('L', BAR_SIZE, 0)
        baseline = (height - (bottom - top)) // 2 - top
        ImageDraw.Draw(mask).text((BAR_TAG_LEFT, baseline), tag, font=font, fill=OPAQUE, anchor='ls')
        image.paste(Image.new('RGBA', BAR_SIZE, (*BAR_TAG_COLOR, OPAQUE)), (0, 0), mask)
    return Sprite(name, image)


def lamp_strip(name: str) -> Sprite:
    image = Image.new('RGBA', (LAMP_CELL * len(LAMP_COLORS), LAMP_CELL), (*WHITE, 0))
    for column, color in enumerate(LAMP_COLORS):
        image.paste((*color, OPAQUE), (column * LAMP_CELL, 0, (column + 1) * LAMP_CELL, LAMP_CELL))
    return Sprite(name, image, len(LAMP_COLORS), 1)


def lamp_frames(name: str, frames: tuple[str | None, ...]) -> Sprite:
    image = Image.new('RGBA', (LAMP_CELL * len(frames), LAMP_CELL), (*WHITE, 0))
    for column, lamp in enumerate(frames):
        if lamp is not None:
            image.paste((*LAMP[lamp], OPAQUE), (column * LAMP_CELL, 0, (column + 1) * LAMP_CELL, LAMP_CELL))
    return Sprite(name, image, len(frames), 1)


def smooth(side: int, paint) -> Image.Image:
    large = side * SUPERSAMPLE
    mask = Image.new('L', (large, large), 0)
    paint(ImageDraw.Draw(mask), large)
    return tinted(mask.resize((side, side), Image.Resampling.LANCZOS))


def circle(draw: ImageDraw.ImageDraw, side: int, radius: float, fill: int) -> None:
    middle = side / 2
    reach = side * radius
    draw.ellipse((middle - reach, middle - reach, middle + reach, middle + reach), fill=fill)


def paint_gear(draw: ImageDraw.ImageDraw, side: int) -> None:
    middle = side / 2
    reach = side * GEAR_TOOTH_REACH
    half = side * GEAR_TOOTH_HALF_WIDTH
    for tooth in range(GEAR_TEETH):
        angle = FULL_TURN * tooth / GEAR_TEETH
        along = (math.cos(angle), math.sin(angle))
        across = (-along[1], along[0])
        corners = [(middle + along[0] * reach * far + across[0] * half * turn, middle + along[1] * reach * far + across[1] * half * turn) for far, turn in ((0, -1), (1, -1), (1, 1), (0, 1))]
        draw.polygon(corners, fill=OPAQUE)
    circle(draw, side, GEAR_BODY_RADIUS, OPAQUE)
    circle(draw, side, GEAR_HOLE_RADIUS, 0)


def paint_speaker(draw: ImageDraw.ImageDraw, side: int) -> None:
    draw.rectangle(tuple(side * edge for edge in SPEAKER_BOX), fill=OPAQUE)
    draw.polygon([(side * x, side * y) for x, y in SPEAKER_CONE], fill=OPAQUE)
    center_x, center_y = (side * part for part in SPEAKER_WAVE_CENTER)
    for radius, thickness in SPEAKER_WAVES:
        reach = side * radius
        draw.arc((center_x - reach, center_y - reach, center_x + reach, center_y + reach), *SPEAKER_WAVE_SPAN, fill=OPAQUE, width=round(side * thickness))


def star_corners(side: int, scale: float) -> list[tuple[float, float]]:
    center_x, center_y = (side * part for part in STAR_CENTER)
    corners = []
    for corner in range(STAR_POINTS * 2):
        radius = side * scale * (STAR_OUTER_RADIUS if corner % 2 == 0 else STAR_INNER_RADIUS)
        angle = QUARTER_TURN_BACK + FULL_TURN * corner / (STAR_POINTS * 2)
        corners.append((center_x + math.cos(angle) * radius, center_y + math.sin(angle) * radius))
    return corners


def paint_star(draw: ImageDraw.ImageDraw, side: int) -> None:
    draw.polygon(star_corners(side, 1), fill=OPAQUE)


def paint_star_outline(draw: ImageDraw.ImageDraw, side: int) -> None:
    paint_star(draw, side)
    draw.polygon(star_corners(side, STAR_OUTLINE_SCALE), fill=0)


def build() -> list[Sprite]:
    sprites = [bar(name, tag, alpha) for name, tag, alpha in BARS]
    sprites += [word(name, text, LABEL_SIZE) for name, text in LABELS]
    sprites.append(lamp_strip('lamp_strip'))
    sprites += [lamp_frames(f'lamp_{name}', frames) for name, frames in LAMP_FRAMES.items()]
    sprites.append(block('meter', *METER_SIZE))
    sprites.append(Sprite('icon_settings', smooth(ICON_SIDE, paint_gear)))
    sprites.append(Sprite('icon_volume', smooth(ICON_SIDE, paint_speaker)))
    sprites.append(Sprite('star_on', smooth(STAR_SIDE, paint_star)))
    sprites.append(Sprite('star_off', smooth(STAR_SIDE, paint_star_outline)))
    return sprites
