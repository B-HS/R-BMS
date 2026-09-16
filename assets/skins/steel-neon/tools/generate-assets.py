# /// script
# requires-python = ">=3.11"
# ///
from __future__ import annotations

import json
import struct
import zlib
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
IMAGES = ROOT / 'images'


def color(value: str, alpha: int = 255) -> tuple[int, int, int, int]:
    value = value.removeprefix('#')
    return int(value[0:2], 16), int(value[2:4], 16), int(value[4:6], 16), alpha


def blend(base: tuple[int, int, int, int], top: tuple[int, int, int, int]) -> tuple[int, int, int, int]:
    base_alpha = base[3] / 255
    top_alpha = top[3] / 255
    output_alpha = top_alpha + base_alpha * (1 - top_alpha)
    if output_alpha == 0:
        return 0, 0, 0, 0
    output_rgb = tuple(
        round((top[index] * top_alpha + base[index] * base_alpha * (1 - top_alpha)) / output_alpha) for index in range(3)
    )
    return *output_rgb, round(output_alpha * 255)


class Canvas:
    def __init__(self, width: int, height: int) -> None:
        self.width = width
        self.height = height
        self.pixels = bytearray(width * height * 4)

    def point(self, x: int, y: int, rgba: tuple[int, int, int, int]) -> None:
        if not 0 <= x < self.width or not 0 <= y < self.height:
            return
        index = (y * self.width + x) * 4
        existing = tuple(self.pixels[index : index + 4])
        self.pixels[index : index + 4] = bytes(blend(existing, rgba))

    def rect(self, x: int, y: int, width: int, height: int, rgba: tuple[int, int, int, int]) -> None:
        for py in range(max(y, 0), min(y + height, self.height)):
            for px in range(max(x, 0), min(x + width, self.width)):
                self.point(px, py, rgba)

    def line(self, x0: int, y0: int, x1: int, y1: int, rgba: tuple[int, int, int, int], thickness: int = 1) -> None:
        distance = max(abs(x1 - x0), abs(y1 - y0), 1)
        for step in range(distance + 1):
            x = round(x0 + (x1 - x0) * step / distance)
            y = round(y0 + (y1 - y0) * step / distance)
            self.rect(x - thickness // 2, y - thickness // 2, thickness, thickness, rgba)

    def frame(self, x: int, y: int, width: int, height: int, edge: tuple[int, int, int, int], inner: tuple[int, int, int, int], thickness: int = 2) -> None:
        self.rect(x, y, width, thickness, edge)
        self.rect(x, y + height - thickness, width, thickness, edge)
        self.rect(x, y, thickness, height, edge)
        self.rect(x + width - thickness, y, thickness, height, edge)
        self.rect(x + thickness, y + thickness, width - thickness * 2, height - thickness * 2, inner)

    def save(self, path: Path) -> None:
        raw = b''.join(b'\x00' + self.pixels[row * self.width * 4 : (row + 1) * self.width * 4] for row in range(self.height))
        chunk = lambda kind, data: struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data) & 0xFFFFFFFF)
        path.write_bytes(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', self.width, self.height, 8, 6, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(raw, 9)) + chunk(b'IEND', b''))


def corner(canvas: Canvas, x: int, y: int, direction_x: int, direction_y: int, primary: tuple[int, int, int, int], secondary: tuple[int, int, int, int]) -> None:
    canvas.line(x, y, x + direction_x * 76, y, primary, 3)
    canvas.line(x, y, x, y + direction_y * 54, primary, 3)
    canvas.line(x + direction_x * 18, y + direction_y * 8, x + direction_x * 56, y + direction_y * 8, secondary, 1)
    canvas.line(x + direction_x * 8, y + direction_y * 18, x + direction_x * 8, y + direction_y * 42, secondary, 1)


def stars(canvas: Canvas, palette: dict[str, str]) -> None:
    cyan = color(palette['cyan'], 130)
    violet = color(palette['violet'], 115)
    for index in range(168):
        x = (index * 137 + 47) % canvas.width
        y = (index * 71 + 31) % canvas.height
        size = 1 + index % 3
        canvas.rect(x, y, size, size, cyan if index % 2 else violet)
        if index % 7 == 0:
            canvas.line(x - size * 2, y, x + size * 2, y, cyan, 1)
            canvas.line(x, y - size * 2, x, y + size * 2, cyan, 1)


def common(palette: dict[str, str]) -> Canvas:
    canvas = Canvas(256, 128)
    ink = color(palette['ink'])
    cyan = color(palette['cyan'])
    violet = color(palette['violet'])
    metal = color(palette['metal'])
    canvas.rect(0, 0, 256, 128, ink)
    for column in range(128):
        ratio = column / 127
        shade = tuple(round(cyan[index] * ratio + violet[index] * (1 - ratio)) if index < 3 else 255 for index in range(4))
        canvas.rect(column, 0, 1, 32, shade)
    canvas.frame(0, 40, 96, 64, metal, color(palette['ink']), 3)
    for index in range(7):
        canvas.rect(108 + index * 20, 48, 14, 48, cyan if index % 2 else violet)
    canvas.rect(108, 104, 140, 8, color(palette['amber']))
    return canvas


def select_overlay(palette: dict[str, str]) -> Canvas:
    canvas = Canvas(1280, 720)
    cyan = color(palette['cyan'], 190)
    violet = color(palette['violet'], 185)
    metal = color(palette['metalLight'], 118)
    shadow = color(palette['ink'], 142)
    stars(canvas, palette)
    canvas.rect(0, 0, 1280, 38, shadow)
    canvas.rect(0, 672, 1280, 48, shadow)
    canvas.rect(0, 0, 18, 720, color(palette['metal'], 150))
    canvas.rect(1262, 0, 18, 720, color(palette['metal'], 150))
    corner(canvas, 10, 10, 1, 1, violet, cyan)
    corner(canvas, 1270, 10, -1, 1, cyan, violet)
    corner(canvas, 10, 710, 1, -1, cyan, violet)
    corner(canvas, 1270, 710, -1, -1, violet, cyan)
    canvas.line(28, 34, 612, 34, cyan, 2)
    canvas.line(632, 34, 1252, 34, violet, 2)
    canvas.line(28, 672, 1252, 672, metal, 2)
    canvas.frame(18, 42, 604, 616, metal, (0, 0, 0, 0), 2)
    canvas.frame(626, 42, 636, 616, metal, (0, 0, 0, 0), 2)
    canvas.frame(24, 48, 592, 604, color(palette['violet'], 88), (0, 0, 0, 0), 1)
    canvas.frame(632, 48, 624, 604, color(palette['cyan'], 88), (0, 0, 0, 0), 1)
    for x in range(44, 1250, 42):
        canvas.rect(x, 28, 20, 3, cyan if x % 84 else violet)
        canvas.rect(x, 680, 20, 3, violet if x % 84 else cyan)
    for y in range(76, 652, 44):
        canvas.rect(22, y, 8, 20, violet)
        canvas.rect(1250, y, 8, 20, cyan)
    canvas.line(32, 54, 96, 118, violet, 2)
    canvas.line(1184, 54, 1248, 118, cyan, 2)
    canvas.line(32, 646, 96, 582, cyan, 2)
    canvas.line(1184, 646, 1248, 582, violet, 2)
    return canvas


def decide_backdrop(palette: dict[str, str]) -> Canvas:
    canvas = Canvas(1280, 720)
    ink = color(palette['ink'])
    space = color(palette['space'])
    cyan = color(palette['cyan'])
    amber = color(palette['amber'])
    canvas.rect(0, 0, 1280, 720, ink)
    for y in range(720):
        ratio = y / 719
        tint = tuple(round(space[index] * (1 - ratio) + 225 * ratio) if index < 3 else 255 for index in range(4))
        canvas.rect(0, y, 1280, 1, tint)
    for index in range(22):
        x = 420 + (index * 79) % 830
        y = 32 + (index * 113) % 470
        h = 56 + index % 5 * 22
        canvas.rect(x, y, 22 + index % 3 * 14, h, color(palette['metal'], 135))
        canvas.rect(x + 4, y + 8, 3, h - 16, cyan)
    stars(canvas, palette)
    canvas.frame(48, 120, 520, 250, color(palette['metalLight']), (0, 0, 0, 90), 3)
    canvas.line(48, 394, 1230, 394, cyan, 2)
    canvas.line(48, 420, 1230, 420, amber, 3)
    return canvas


def play_overlay(palette: dict[str, str], bga: tuple[int, int, int, int], score_x: int | None) -> Canvas:
    canvas = Canvas(1280, 720)
    bga_x, bga_y, bga_width, bga_height = bga
    cyan = color(palette['cyan'], 185)
    violet = color(palette['violet'], 185)
    metal = color(palette['metalLight'], 142)
    shadow = color(palette['ink'], 135)
    canvas.rect(0, 0, 1280, 42, shadow)
    canvas.rect(0, 660, 1280, 60, shadow)
    canvas.rect(0, 0, 22, 720, color(palette['metal'], 148))
    canvas.rect(1258, 0, 22, 720, color(palette['metal'], 148))
    corner(canvas, 8, 8, 1, 1, metal, cyan)
    corner(canvas, 1272, 8, -1, 1, metal, violet)
    corner(canvas, 8, 712, 1, -1, metal, violet)
    corner(canvas, 1272, 712, -1, -1, metal, cyan)
    canvas.line(16, 34, 1264, 34, metal, 2)
    canvas.line(16, 680, 1264, 680, metal, 2)
    canvas.frame(bga_x - 14, bga_y - 16, bga_width + 28, bga_height + 36, cyan, (0, 0, 0, 0), 3)
    canvas.frame(bga_x - 22, bga_y - 24, bga_width + 44, bga_height + 52, color(palette['metalLight'], 170), (0, 0, 0, 0), 2)
    canvas.frame(bga_x - 8, bga_y - 10, bga_width + 16, bga_height + 24, color(palette['violet'], 100), (0, 0, 0, 0), 1)
    canvas.line(bga_x - 14, bga_y - 30, bga_x + bga_width // 2, bga_y - 30, violet, 2)
    if score_x is not None:
        canvas.frame(score_x - 8, 42, 194, 600, color(palette['metalLight'], 142), (0, 0, 0, 0), 2)
        canvas.frame(score_x - 2, 48, 182, 588, color(palette['cyan'], 88), (0, 0, 0, 0), 1)
    canvas.line(48, 646, 450, 646, cyan, 2)
    canvas.line(830, 646, 1232, 646, violet, 2)
    for x in range(44, 1260, 38):
        canvas.rect(x, 26, 18, 3, cyan if x % 76 else violet)
        canvas.rect(x, 687, 18, 3, violet if x % 76 else cyan)
    for y in range(68, 648, 48):
        canvas.rect(14, y, 10, 22, cyan)
        canvas.rect(1256, y, 10, 22, violet)
    for y in range(bga_y + 8, bga_y + bga_height - 12, 28):
        canvas.rect(bga_x - 32, y, 6, 14, cyan)
        canvas.rect(bga_x + bga_width + 26, y, 6, 14, violet)
    canvas.line(34, 58, 146, 58, violet, 2)
    canvas.line(34, 62, 104, 62, cyan, 1)
    canvas.line(1134, 58, 1246, 58, cyan, 2)
    canvas.line(1176, 62, 1246, 62, violet, 1)
    return canvas


def result_overlay(palette: dict[str, str]) -> Canvas:
    canvas = Canvas(1280, 720)
    blue = color(palette['spaceLight'], 148)
    cyan = color(palette['cyan'], 180)
    violet = color(palette['violet'], 155)
    metal = color(palette['metalLight'], 132)
    for y in range(720):
        canvas.rect(0, y, 1280, 1, tuple(round(blue[index] * (1 - y / 1100)) if index < 3 else 72 for index in range(4)))
    stars(canvas, palette)
    canvas.rect(0, 0, 1280, 38, color(palette['ink'], 132))
    canvas.rect(0, 664, 1280, 56, color(palette['ink'], 132))
    canvas.frame(16, 50, 406, 590, metal, (0, 0, 0, 0), 2)
    canvas.frame(440, 50, 536, 590, metal, (0, 0, 0, 0), 2)
    canvas.frame(994, 50, 270, 590, metal, (0, 0, 0, 0), 2)
    canvas.line(28, 36, 1252, 36, cyan, 2)
    canvas.line(28, 664, 1252, 664, violet, 2)
    for x in range(46, 1248, 42):
        canvas.rect(x, 28, 20, 3, cyan if x % 84 else violet)
        canvas.rect(x, 673, 20, 3, violet if x % 84 else cyan)
    return canvas


def main() -> None:
    palette = json.loads((ROOT / 'palette.json').read_text())
    IMAGES.mkdir(exist_ok=True)
    for name, canvas in {
        'common-atlas.png': common(palette),
        'select-overlay.png': select_overlay(palette),
        'decide-backdrop.png': decide_backdrop(palette),
        'play-overlay.png': play_overlay(palette, (500, 150, 420, 315), 936),
        'play-dual-overlay.png': play_overlay(palette, (920, 342, 320, 240), None),
        'result-overlay.png': result_overlay(palette),
    }.items():
        canvas.save(IMAGES / name)


if __name__ == '__main__':
    main()
