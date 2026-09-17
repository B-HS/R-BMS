# /// script
# requires-python = ">=3.11"
# ///
from __future__ import annotations

import json
import struct
import zlib
from pathlib import Path


CANVAS_WIDTH = 1280
CANVAS_HEIGHT = 720
ROOT = Path(__file__).resolve().parent.parent
IMAGES = ROOT / 'images'


def color(value: str, alpha: int = 255) -> tuple[int, int, int, int]:
    hex_value = value.removeprefix('#')
    return int(hex_value[0:2], 16), int(hex_value[2:4], 16), int(hex_value[4:6], 16), alpha


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
        x_start = max(x, 0)
        y_start = max(y, 0)
        x_end = min(x + width, self.width)
        y_end = min(y + height, self.height)
        if x_start >= x_end or y_start >= y_end:
            return
        if rgba[3] == 255:
            row = bytes(rgba) * (x_end - x_start)
            for py in range(y_start, y_end):
                start = (py * self.width + x_start) * 4
                self.pixels[start : start + len(row)] = row
            return
        for py in range(y_start, y_end):
            for px in range(x_start, x_end):
                self.point(px, py, rgba)

    def line(self, x0: int, y0: int, x1: int, y1: int, rgba: tuple[int, int, int, int], thickness: int = 1) -> None:
        distance = max(abs(x1 - x0), abs(y1 - y0), 1)
        for step in range(distance + 1):
            x = round(x0 + (x1 - x0) * step / distance)
            y = round(y0 + (y1 - y0) * step / distance)
            self.rect(x - thickness // 2, y - thickness // 2, thickness, thickness, rgba)

    def save(self, path: Path) -> None:
        raw = b''.join(b'\x00' + self.pixels[row * self.width * 4 : (row + 1) * self.width * 4] for row in range(self.height))
        chunk = lambda kind, data: struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data) & 0xFFFFFFFF)
        path.write_bytes(
            b'\x89PNG\r\n\x1a\n'
            + chunk(b'IHDR', struct.pack('>IIBBBBB', self.width, self.height, 8, 6, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(raw, 9))
            + chunk(b'IEND', b'')
        )


def gradient(canvas: Canvas, start: tuple[int, int, int, int], end: tuple[int, int, int, int]) -> None:
    for y in range(canvas.height):
        ratio = y / max(canvas.height - 1, 1)
        rgba = tuple(round(start[index] * (1 - ratio) + end[index] * ratio) for index in range(4))
        canvas.rect(0, y, canvas.width, 1, rgba)


def segmented_line(canvas: Canvas, y: int, palette: dict[str, str], offset: int = 0) -> None:
    cyan = color(palette['cyan'], 218)
    violet = color(palette['violet'], 218)
    for x in range(34 + offset, CANVAS_WIDTH - 34, 52):
        canvas.rect(x, y, 28, 3, cyan if (x // 52) % 2 else violet)


def bloom(canvas: Canvas, x: int, y: int, radius: int, rgba: tuple[int, int, int, int]) -> None:
    radius_squared = radius * radius
    for py in range(y - radius, y + radius + 1):
        for px in range(x - radius, x + radius + 1):
            distance_squared = (px - x) * (px - x) + (py - y) * (py - y)
            if distance_squared > radius_squared:
                continue
            intensity = 1 - distance_squared / radius_squared
            canvas.point(px, py, (*rgba[:3], round(rgba[3] * intensity * intensity)))


def star_cluster(
    canvas: Canvas,
    palette: dict[str, str],
    x: int,
    y: int,
    width: int,
    height: int,
    count: int,
    phase: int,
) -> None:
    shades = (color(palette['cyan'], 150), color(palette['violet'], 135), color(palette['rose'], 118))
    for index in range(count):
        px = x + (index * 47 + phase * 19 + 17) % width
        py = y + (index * 71 + phase * 31 + 11) % height
        size = 1 + (index + phase) % 3
        shade = shades[(index + phase) % len(shades)]
        canvas.rect(px, py, size, size, shade)
        if index % 17 == 0:
            canvas.line(px - 4, py, px + 4, py, shade)
            canvas.line(px, py - 4, px, py + 4, shade)


def rail(canvas: Canvas, x: int, y: int, width: int, palette: dict[str, str], reverse: bool = False) -> None:
    metal = color(palette['metal'], 210)
    highlight = color(palette['metalLight'], 196)
    accent = color(palette['violet'] if reverse else palette['cyan'], 192)
    canvas.rect(x, y, width, 8, metal)
    canvas.line(x, y, x + width - 1, y, highlight)
    canvas.line(x, y + 7, x + width - 1, y + 7, color(palette['ink'], 214))
    for offset in range(10, width - 5, 34):
        canvas.rect(x + offset, y + 2, 16, 2, accent)
        canvas.rect(x + offset + 18, y + 2, 3, 2, highlight)


def corner(canvas: Canvas, x: int, y: int, dx: int, dy: int, primary: tuple[int, int, int, int], secondary: tuple[int, int, int, int]) -> None:
    canvas.line(x, y, x + dx * 80, y, primary, 3)
    canvas.line(x, y, x, y + dy * 58, primary, 3)
    canvas.line(x + dx * 16, y + dy * 10, x + dx * 58, y + dy * 10, secondary, 1)
    canvas.line(x + dx * 10, y + dy * 16, x + dx * 10, y + dy * 44, secondary, 1)


def safe_sparks(canvas: Canvas, palette: dict[str, str], gutters: tuple[tuple[int, int], ...]) -> None:
    cyan = color(palette['cyan'], 145)
    violet = color(palette['violet'], 126)
    for index in range(92):
        left, right = gutters[index % len(gutters)]
        width = right - left
        x = left + (index * 29 + 11) % max(width, 1)
        y = 22 + (index * 83 + 37) % 676
        size = 1 + index % 3
        shade = cyan if index % 2 else violet
        canvas.rect(x, y, size, size, shade)
        if index % 11 == 0:
            canvas.line(x - 3, y, x + 3, y, shade)
            canvas.line(x, y - 3, x, y + 3, shade)


def select_detail_glow(canvas: Canvas, palette: dict[str, str]) -> None:
    for y in range(56, 674):
        vertical = (y - 56) / 618
        for x in range(24, 614):
            horizontal = (x - 24) / 590
            bloom_x = (x - 228) / 280
            bloom_y = (y - 238) / 290
            bloom_amount = max(0.0, 1.0 - bloom_x * bloom_x - bloom_y * bloom_y)
            shade = (
                round(23 + 42 * (1 - horizontal) + 69 * bloom_amount),
                round(14 + 11 * (1 - vertical) + 10 * bloom_amount),
                round(48 + 47 * (1 - horizontal) + 76 * bloom_amount),
                255,
            )
            index = (y * canvas.width + x) * 4
            canvas.pixels[index : index + 4] = bytes(shade)
    bloom(canvas, 454, 170, 168, color(palette['cyan'], 45))
    bloom(canvas, 132, 534, 196, color(palette['rose'], 37))
    bloom(canvas, 514, 524, 128, color(palette['violet'], 45))
    cyan = color(palette['cyan'], 160)
    violet = color(palette['violet'], 180)
    rose = color(palette['rose'], 145)
    for index in range(164):
        x = 34 + (index * 79 + 23) % 560
        y = 74 + (index * 47 + 17) % 574
        size = 1 + index % 4
        shade = cyan if index % 3 == 0 else violet if index % 3 == 1 else rose
        canvas.rect(x, y, size, size, shade)
        if index % 9 == 0:
            canvas.line(x - 5, y, x + 5, y, shade)
            canvas.line(x, y - 5, x, y + 5, shade)
    star_cluster(canvas, palette, 56, 86, 524, 542, 96, 3)


def common_atlas(palette: dict[str, str]) -> Canvas:
    canvas = Canvas(256, 128)
    canvas.rect(0, 0, 256, 128, color(palette['ink']))
    for x in range(256):
        ratio = x / 255
        cyan = color(palette['cyan'])
        violet = color(palette['violet'])
        shade = tuple(round(violet[index] * (1 - ratio) + cyan[index] * ratio) for index in range(3)) + (255,)
        canvas.rect(x, 0, 1, 28, shade)
    canvas.rect(0, 42, 96, 62, color(palette['panel']))
    canvas.line(0, 42, 95, 42, color(palette['metalLight']), 2)
    canvas.line(0, 103, 95, 103, color(palette['violet']), 2)
    for index in range(7):
        canvas.rect(110 + index * 19, 46, 13, 52, color(palette['cyan'] if index % 2 else palette['violet']))
    canvas.rect(110, 106, 136, 8, color(palette['amber']))
    return canvas


def select_background(palette: dict[str, str]) -> Canvas:
    canvas = Canvas(CANVAS_WIDTH, CANVAS_HEIGHT)
    gradient(canvas, color(palette['night']), color(palette['ink']))
    canvas.rect(0, 0, CANVAS_WIDTH, 56, color(palette['violetDeep'], 92))
    canvas.rect(0, 674, CANVAS_WIDTH, 46, color(palette['cyanDeep'], 82))
    canvas.rect(0, 0, 18, CANVAS_HEIGHT, color(palette['violetDeep'], 120))
    canvas.rect(1262, 0, 18, CANVAS_HEIGHT, color(palette['cyanDeep'], 120))
    select_detail_glow(canvas, palette)
    canvas.rect(614, 76, 14, 598, color(palette['ink'], 114))
    safe_sparks(canvas, palette, ((18, 24), (614, 628), (1256, 1262), (24, 46), (1234, 1256)))
    star_cluster(canvas, palette, 630, 84, 606, 574, 56, 7)
    return canvas


def select_foreground(palette: dict[str, str]) -> Canvas:
    canvas = Canvas(CANVAS_WIDTH, CANVAS_HEIGHT)
    cyan = color(palette['cyan'], 226)
    violet = color(palette['violet'], 226)
    metal = color(palette['metalLight'], 178)
    corner(canvas, 10, 10, 1, 1, violet, cyan)
    corner(canvas, 1270, 10, -1, 1, cyan, violet)
    corner(canvas, 10, 710, 1, -1, cyan, violet)
    corner(canvas, 1270, 710, -1, -1, violet, cyan)
    segmented_line(canvas, 42, palette)
    segmented_line(canvas, 706, palette, 26)
    rail(canvas, 102, 50, 416, palette, True)
    rail(canvas, 760, 50, 416, palette)
    rail(canvas, 86, 662, 432, palette)
    rail(canvas, 760, 662, 432, palette, True)
    canvas.line(24, 64, 614, 64, metal, 2)
    canvas.line(628, 64, 1256, 64, metal, 2)
    canvas.line(621, 80, 621, 670, color(palette['cyan'], 120), 1)
    canvas.line(10, 70, 10, 650, color(palette['violet'], 145), 2)
    canvas.line(1270, 70, 1270, 650, color(palette['cyan'], 145), 2)
    return canvas


def decide_backdrop(palette: dict[str, str]) -> Canvas:
    canvas = Canvas(CANVAS_WIDTH, CANVAS_HEIGHT)
    gradient(canvas, color(palette['night']), color(palette['ink']))
    cyan = color(palette['cyan'], 185)
    violet = color(palette['violet'], 160)
    bloom(canvas, 1088, 182, 270, color(palette['cyan'], 34))
    bloom(canvas, 936, 554, 238, color(palette['violet'], 30))
    for index in range(32):
        x = 540 + (index * 61) % 690
        y = 28 + (index * 97) % 510
        height = 40 + (index % 5) * 28
        canvas.rect(x, y, 18 + (index % 3) * 10, height, color(palette['metal'], 112))
        canvas.rect(x + 4, y + 6, 3, height - 12, cyan if index % 2 else violet)
    for index in range(10):
        start_x = 584 + index * 58
        start_y = 74 + (index * 43) % 224
        canvas.line(start_x, start_y, start_x + 280, start_y + 138, color(palette['cyan'] if index % 2 else palette['violet'], 44), 1)
    star_cluster(canvas, palette, 584, 42, 634, 346, 74, 11)
    canvas.rect(40, 108, 510, 286, color(palette['ink'], 178))
    canvas.line(40, 108, 550, 108, cyan, 3)
    canvas.line(40, 394, 550, 394, violet, 3)
    segmented_line(canvas, 438, palette)
    rail(canvas, 604, 580, 548, palette)
    return canvas


def play_background(palette: dict[str, str], dual: bool) -> Canvas:
    canvas = Canvas(CANVAS_WIDTH, CANVAS_HEIGHT)
    gradient(canvas, color(palette['night']), color(palette['ink']))
    canvas.rect(0, 0, CANVAS_WIDTH, 40, color(palette['violetDeep'], 95))
    canvas.rect(0, 660, CANVAS_WIDTH, 60, color(palette['cyanDeep'], 78))
    canvas.rect(0, 0, 24, CANVAS_HEIGHT, color(palette['violetDeep'], 112))
    canvas.rect(1256, 0, 24, CANVAS_HEIGHT, color(palette['cyanDeep'], 112))
    if dual:
        canvas.rect(404, 48, 18, 578, color(palette['ink'], 82))
        canvas.rect(808, 48, 18, 578, color(palette['ink'], 82))
        gutters = ((24, 38), (402, 424), (806, 828), (1242, 1256))
        bloom(canvas, 434, 142, 128, color(palette['violet'], 28))
        bloom(canvas, 836, 504, 118, color(palette['cyan'], 25))
        star_cluster(canvas, palette, 424, 62, 32, 554, 32, 13)
        star_cluster(canvas, palette, 828, 62, 22, 554, 22, 17)
    else:
        canvas.rect(448, 48, 20, 578, color(palette['ink'], 82))
        canvas.rect(912, 116, 20, 330, color(palette['ink'], 72))
        gutters = ((24, 42), (450, 472), (912, 936), (1232, 1256))
        bloom(canvas, 458, 522, 118, color(palette['violet'], 25))
        bloom(canvas, 922, 84, 98, color(palette['cyan'], 20))
        star_cluster(canvas, palette, 452, 62, 16, 554, 28, 13)
        star_cluster(canvas, palette, 914, 466, 14, 166, 16, 17)
    safe_sparks(canvas, palette, gutters)
    return canvas


def play_foreground(palette: dict[str, str], dual: bool) -> Canvas:
    canvas = Canvas(CANVAS_WIDTH, CANVAS_HEIGHT)
    cyan = color(palette['cyan'], 226)
    violet = color(palette['violet'], 226)
    metal = color(palette['metalLight'], 176)
    corner(canvas, 10, 10, 1, 1, violet, cyan)
    corner(canvas, 1270, 10, -1, 1, cyan, violet)
    corner(canvas, 10, 710, 1, -1, cyan, violet)
    corner(canvas, 1270, 710, -1, -1, violet, cyan)
    segmented_line(canvas, 34, palette)
    segmented_line(canvas, 686, palette, 26)
    rail(canvas, 94, 660, 248 if not dual else 210, palette, True)
    rail(canvas, 694 if not dual else 562, 660, 204, palette)
    rail(canvas, 980, 660, 176, palette, True)
    canvas.line(28, 52, 438 if not dual else 386, 52, metal, 2)
    canvas.line(28, 650, 438 if not dual else 386, 650, metal, 2)
    canvas.line(946 if not dual else 852, 52, 1252, 52, metal, 2)
    canvas.line(946 if not dual else 852, 650, 1252, 650, metal, 2)
    canvas.line(12, 72, 12, 648, color(palette['violet'], 150), 2)
    canvas.line(1268, 72, 1268, 648, color(palette['cyan'], 150), 2)
    return canvas


def result_background(palette: dict[str, str]) -> Canvas:
    canvas = Canvas(CANVAS_WIDTH, CANVAS_HEIGHT)
    gradient(canvas, color(palette['night']), color(palette['ink']))
    canvas.rect(0, 0, CANVAS_WIDTH, 48, color(palette['violetDeep'], 90))
    canvas.rect(0, 662, CANVAS_WIDTH, 58, color(palette['cyanDeep'], 76))
    bloom(canvas, 100, 324, 292, color(palette['cyan'], 28))
    bloom(canvas, 1188, 264, 264, color(palette['violet'], 24))
    bloom(canvas, 636, 646, 338, color(palette['cyan'], 18))
    for index in range(12):
        y = 484 + index * 12
        canvas.line(20, y, 410, y + 24, color(palette['cyan'] if index % 2 else palette['violet'], 20), 1)
        canvas.line(874, y + 24, 1242, y, color(palette['violet'] if index % 2 else palette['cyan'], 18), 1)
    safe_sparks(canvas, palette, ((0, 20), (420, 440), (976, 994), (1260, 1280)))
    star_cluster(canvas, palette, 24, 72, 372, 438, 66, 19)
    star_cluster(canvas, palette, 1006, 72, 232, 438, 48, 23)
    return canvas


def result_foreground(palette: dict[str, str]) -> Canvas:
    canvas = Canvas(CANVAS_WIDTH, CANVAS_HEIGHT)
    cyan = color(palette['cyan'], 224)
    violet = color(palette['violet'], 224)
    corner(canvas, 10, 10, 1, 1, violet, cyan)
    corner(canvas, 1270, 10, -1, 1, cyan, violet)
    corner(canvas, 10, 710, 1, -1, cyan, violet)
    corner(canvas, 1270, 710, -1, -1, violet, cyan)
    segmented_line(canvas, 38, palette)
    segmented_line(canvas, 686, palette, 20)
    rail(canvas, 112, 50, 336, palette, True)
    rail(canvas, 832, 50, 336, palette)
    rail(canvas, 102, 700, 330, palette)
    rail(canvas, 842, 700, 330, palette, True)
    canvas.line(18, 64, 18, 640, color(palette['violet'], 140), 2)
    canvas.line(1262, 64, 1262, 640, color(palette['cyan'], 140), 2)
    return canvas


def main() -> None:
    palette = json.loads((ROOT / 'palette.json').read_text())
    IMAGES.mkdir(exist_ok=True)
    images = {
        'common-atlas.png': common_atlas(palette),
        'select-background.png': select_background(palette),
        'select-foreground.png': select_foreground(palette),
        'decide-backdrop.png': decide_backdrop(palette),
        'play-background.png': play_background(palette, False),
        'play-foreground.png': play_foreground(palette, False),
        'play-dual-background.png': play_background(palette, True),
        'play-dual-foreground.png': play_foreground(palette, True),
        'result-background.png': result_background(palette),
        'result-foreground.png': result_foreground(palette),
    }
    for name, canvas in images.items():
        canvas.save(IMAGES / name)


if __name__ == '__main__':
    main()
