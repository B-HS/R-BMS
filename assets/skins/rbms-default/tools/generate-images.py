# /// script
# requires-python = ">=3.11"
# dependencies = ["pillow==12.3.0"]
# ///
from __future__ import annotations

import io
import sys
from dataclasses import dataclass
from pathlib import Path

sys.dont_write_bytecode = True

from PIL import Image

from sections import common, decide, play, result, select
from sprites import ROOT, WHITE, Sprite

IMAGE = ROOT / 'image'
ATLAS = ROOT / 'lua' / 'atlas'
SCRIPT_NAME = 'tools/generate-images.py'

MAX_SHEET_SIDE = 4096
SHEET_WIDTHS = (128, 256, 512, 1024, 2048, 4096)
SPRITE_GAP = 2
CHECK_FLAG = '--check'

SECTIONS = {
    'common': common.build,
    'decide': decide.build,
    'select': select.build,
    'play': play.build,
    'result': result.build,
}


@dataclass
class Placed:
    sprite: Sprite
    x: int
    y: int


def shelves(sprites: list[Sprite], sheet_width: int) -> tuple[list[Placed], int]:
    placed: list[Placed] = []
    x = y = shelf_height = 0
    for sprite in sprites:
        width, height = sprite.image.size
        if x > 0 and x + width > sheet_width:
            x, y, shelf_height = 0, y + shelf_height + SPRITE_GAP, 0
        placed.append(Placed(sprite, x, y))
        x += width + SPRITE_GAP
        shelf_height = max(shelf_height, height)
    return placed, y + shelf_height


def packed(sprites: list[Sprite]) -> tuple[list[Placed], int, int]:
    ordered = sorted(sprites, key=lambda sprite: (-sprite.image.height, -sprite.image.width, sprite.name))
    widest = max(sprite.image.width for sprite in ordered)
    for sheet_width in SHEET_WIDTHS:
        if sheet_width < widest:
            continue
        placed, height = shelves(ordered, sheet_width)
        if height <= min(sheet_width, MAX_SHEET_SIDE):
            return placed, sheet_width, height
    placed, height = shelves(ordered, MAX_SHEET_SIDE)
    if widest > MAX_SHEET_SIDE or height > MAX_SHEET_SIDE:
        raise SystemExit(f'the sprites do not fit a {MAX_SHEET_SIDE}x{MAX_SHEET_SIDE} sheet')
    return placed, MAX_SHEET_SIDE, height


def sheet_bytes(placed: list[Placed], width: int, height: int) -> bytes:
    sheet = Image.new('RGBA', (width, height), (*WHITE, 0))
    for entry in placed:
        sheet.paste(entry.sprite.image, (entry.x, entry.y))
    buffer = io.BytesIO()
    sheet.save(buffer, format='PNG', optimize=True)
    return buffer.getvalue()


def lua_value(value: int | list[int]) -> str:
    if isinstance(value, list):
        return '{ ' + ', '.join(str(item) for item in value) + ' }'
    return str(value)


def atlas_text(section: str, placed: list[Placed], width: int, height: int) -> str:
    lines = [
        'return {',
        f'    generated_by = "{SCRIPT_NAME}",',
        f'    path = "image/{section}.png",',
        f'    width = {width},',
        f'    height = {height},',
        '    sprites = {',
    ]
    for entry in sorted(placed, key=lambda entry: entry.sprite.name):
        sprite = entry.sprite
        fields = {'x': entry.x, 'y': entry.y, 'w': sprite.image.width, 'h': sprite.image.height, 'divx': sprite.divx, 'divy': sprite.divy} | sprite.meta
        body = ', '.join(f'{key} = {lua_value(value)}' for key, value in fields.items())
        lines.append(f'        {sprite.name} = {{ {body} }},')
    lines += ['    },', '}', '']
    return '\n'.join(lines)


def outputs(section: str) -> dict[Path, bytes]:
    sprites = SECTIONS[section]()
    if not sprites:
        return {}
    placed, width, height = packed(sprites)
    return {
        IMAGE / f'{section}.png': sheet_bytes(placed, width, height),
        ATLAS / f'{section}.lua': atlas_text(section, placed, width, height).encode('utf-8'),
    }


def main(arguments: list[str]) -> int:
    check = CHECK_FLAG in arguments
    wanted = [argument for argument in arguments if argument != CHECK_FLAG] or list(SECTIONS)
    unknown = [section for section in wanted if section not in SECTIONS]
    if unknown:
        raise SystemExit(f'unknown section: {", ".join(unknown)} (known: {", ".join(SECTIONS)})')
    stale = 0
    for section in wanted:
        files = outputs(section)
        if not files:
            print(f'{section}: nothing to generate')
            continue
        for path, data in files.items():
            relative = path.relative_to(ROOT).as_posix()
            if check:
                matches = path.is_file() and path.read_bytes() == data
                stale += 0 if matches else 1
                print(f'{section}: {relative} {"matches" if matches else "differs"}')
                continue
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
            print(f'{section}: {relative} {len(data)} bytes')
    return 1 if stale else 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
