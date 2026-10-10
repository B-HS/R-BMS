from __future__ import annotations

import math
import re
from dataclasses import dataclass, field
from functools import cache
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
FONT_FILE = ROOT / 'font' / 'Inter-Regular.ttf'
THEME_FILE = ROOT / 'lua' / 'theme.lua'
THEME_GROUP = r'^theme\.{group} = \{{\n(.*?)^\}}'
THEME_COLOR = re.compile(r'^\s*\[?(\w+)\]? = \{ (\d+), (\d+), (\d+) \},$', re.MULTILINE)

Rgb = tuple[int, int, int]

GLYPH_PAD = 2
OPAQUE = 255
WHITE = (255, 255, 255)
DIGITS = '0123456789'
DIGIT_COUNT = 10
DIGIT_SIDE_RATIO = 0.04
MIN_DIGIT_SIDE = 1
SIGNED_COLUMNS = 12
SIGNED_ROWS = 2
SIGNED_MARK_COLUMN = 11
SIGN_MARKS = ('+', '-')
INK_CANVAS_EMS = 3
INK_ORIGIN_EMS = (1, 2)


@dataclass
class Sprite:
    name: str
    image: Image.Image
    divx: int = 1
    divy: int = 1
    meta: dict[str, int | list[int]] = field(default_factory=dict)


@cache
def font_at(size: int) -> ImageFont.FreeTypeFont:
    return ImageFont.truetype(str(FONT_FILE), size)


@cache
def palette(group: str) -> dict[str, Rgb]:
    block = re.search(THEME_GROUP.format(group=re.escape(group)), THEME_FILE.read_text(encoding='utf-8'), re.MULTILINE | re.DOTALL)
    colors = {name: (int(red), int(green), int(blue)) for name, red, green, blue in THEME_COLOR.findall(block.group(1) if block else '')}
    if not colors:
        raise SystemExit(f'{THEME_FILE.name} has no colors under theme.{group}')
    return colors


def tinted(mask: Image.Image, color: tuple[int, int, int] = WHITE) -> Image.Image:
    image = Image.new('RGBA', mask.size, (*color, 0))
    image.putalpha(mask)
    return image


def glyph_rows(size: int, glyphs: str) -> tuple[int, int]:
    font = font_at(size)
    boxes = [font.getbbox(glyph, anchor='ls') for glyph in glyphs]
    return min(box[1] for box in boxes), max(box[3] for box in boxes)


@cache
def ink_columns(size: int, glyph: str) -> tuple[int, int]:
    origin_x, origin_y = (size * ems for ems in INK_ORIGIN_EMS)
    canvas = Image.new('L', (size * INK_CANVAS_EMS, size * INK_CANVAS_EMS), 0)
    ImageDraw.Draw(canvas).text((origin_x, origin_y), glyph, font=font_at(size), fill=OPAQUE, anchor='ls')
    left, _, right, _ = canvas.getbbox()
    return left - origin_x, right - origin_x


def ink_width(size: int, glyph: str) -> int:
    left, right = ink_columns(size, glyph)
    return right - left


def digit_cell(size: int, glyphs: str) -> tuple[int, int, int]:
    top, bottom = glyph_rows(size, glyphs)
    side = max(MIN_DIGIT_SIDE, round(size * DIGIT_SIDE_RATIO))
    width = max(ink_width(size, glyph) for glyph in glyphs) + side * 2
    return width, bottom - top + GLYPH_PAD * 2, GLYPH_PAD - top


def draw_digit(draw: ImageDraw.ImageDraw, size: int, glyph: str, cell_x: int, cell_width: int, baseline: int) -> None:
    left, _ = ink_columns(size, glyph)
    x = cell_x + (cell_width - ink_width(size, glyph)) // 2 - left
    draw.text((x, baseline), glyph, font=font_at(size), fill=OPAQUE, anchor='ls')


def digit_meta(size: int) -> dict[str, int | list[int]]:
    return {'size': size, 'pad': GLYPH_PAD, 'ink': [ink_width(size, glyph) for glyph in DIGITS]}


def digit_strip(name: str, size: int) -> Sprite:
    width, height, baseline = digit_cell(size, DIGITS)
    mask = Image.new('L', (width * DIGIT_COUNT, height), 0)
    draw = ImageDraw.Draw(mask)
    for column, glyph in enumerate(DIGITS):
        draw_digit(draw, size, glyph, column * width, width, baseline)
    return Sprite(name, tinted(mask), DIGIT_COUNT, 1, digit_meta(size))


def signed_digit_strip(name: str, size: int) -> Sprite:
    width, height, baseline = digit_cell(size, DIGITS + ''.join(SIGN_MARKS))
    mask = Image.new('L', (width * SIGNED_COLUMNS, height * SIGNED_ROWS), 0)
    draw = ImageDraw.Draw(mask)
    for row, mark in enumerate(SIGN_MARKS):
        for column, glyph in (*enumerate(DIGITS), (SIGNED_MARK_COLUMN, mark)):
            draw_digit(draw, size, glyph, column * width, width, row * height + baseline)
    return Sprite(name, tinted(mask), SIGNED_COLUMNS, SIGNED_ROWS, digit_meta(size))


def word(name: str, text: str, size: int, color: tuple[int, int, int] = WHITE) -> Sprite:
    font = font_at(size)
    top, bottom = glyph_rows(size, text)
    width = math.ceil(font.getlength(text)) + GLYPH_PAD * 2
    mask = Image.new('L', (width, bottom - top + GLYPH_PAD * 2), 0)
    ImageDraw.Draw(mask).text((GLYPH_PAD, GLYPH_PAD - top), text, font=font, fill=OPAQUE, anchor='ls')
    return Sprite(name, tinted(mask, color), 1, 1, {'size': size, 'pad': GLYPH_PAD})


def block(name: str, width: int, height: int, color: tuple[int, int, int] = WHITE) -> Sprite:
    return Sprite(name, Image.new('RGBA', (width, height), (*color, OPAQUE)))
