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
COVERS = IMAGES / 'covers'

SCREEN_WIDTH = 1280
SCREEN_HEIGHT = 720

NOTE_CELL_WIDTH = 64
NOTE_CELL_HEIGHT = 24
NOTE_STATE_COUNT = 6
NOTE_ROW_COUNT = 7
NOTE_MINE_ROW = 6
NOTE_STATE_NORMAL = 0
NOTE_STATE_LN_START = 1
NOTE_STATE_LN_BODY = 2
NOTE_STATE_LN_END = 3
NOTE_STATE_LN_ACTIVE = 4
NOTE_STATE_PROCESSED = 5

DIGIT_CELL_COUNT = 11
DIGIT_BLANK_CELL = 10
DIGIT_SMALL_SIZE = (14, 20)
DIGIT_MEDIUM_SIZE = (20, 28)
DIGIT_LARGE_SIZE = (32, 44)
SEGMENT_HEIGHTS_PER_CELL = 6
FLOAT_COLUMN_COUNT = 11
FLOAT_POINT_CELL = 10
FLOAT_ROW_COUNT = 2
FLOAT_NEGATIVE_ROW = 1

UI_SIZE = 256
UI_LAMP_SIZE = 16
UI_LAMP_COUNT = 10
UI_GAUGE_NODE_WIDTH = 32
UI_GAUGE_NODE_HEIGHT = 16

COVER_WIDTH = 320
COVER_HEIGHT = 480

FRAME_BORDER = 2
FRAME_CORNER = 20
SEPARATOR_THICKNESS = 2

SEGMENT_MAP = {
    0: 'ABCDEF',
    1: 'BC',
    2: 'ABGED',
    3: 'ABGCD',
    4: 'FGBC',
    5: 'AFGCD',
    6: 'AFGEDC',
    7: 'ABC',
    8: 'ABCDEFG',
    9: 'ABCDFG',
}


def color(value: str, alpha: int = 255) -> tuple[int, int, int, int]:
    hex_value = value.removeprefix('#')
    return int(hex_value[0:2], 16), int(hex_value[2:4], 16), int(hex_value[4:6], 16), alpha


def mix(first: tuple[int, int, int, int], second: tuple[int, int, int, int], ratio: float) -> tuple[int, int, int, int]:
    return tuple(round(first[index] * (1 - ratio) + second[index] * ratio) for index in range(4))


def scale(rgba: tuple[int, int, int, int], factor: float, alpha: int | None = None) -> tuple[int, int, int, int]:
    return (
        min(255, max(0, round(rgba[0] * factor))),
        min(255, max(0, round(rgba[1] * factor))),
        min(255, max(0, round(rgba[2] * factor))),
        rgba[3] if alpha is None else alpha,
    )


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

    def rounded(self, x: int, y: int, width: int, height: int, rgba: tuple[int, int, int, int], radius: int = 1) -> None:
        for row in range(height):
            inset = 0
            if row < radius:
                inset = radius - row
            elif row >= height - radius:
                inset = radius - (height - 1 - row)
            span = width - inset * 2
            if span <= 0:
                continue
            self.rect(x + inset, y + row, span, 1, rgba)

    def ring(self, center_x: int, center_y: int, outer: int, inner: int, rgba: tuple[int, int, int, int]) -> None:
        for py in range(center_y - outer, center_y + outer + 1):
            delta = py - center_y
            if delta * delta > outer * outer:
                continue
            span_outer = int((outer * outer - delta * delta) ** 0.5)
            if delta * delta < inner * inner:
                span_inner = int((inner * inner - delta * delta) ** 0.5)
                self.rect(center_x - span_outer, py, span_outer - span_inner, 1, rgba)
                self.rect(center_x + span_inner + 1, py, span_outer - span_inner, 1, rgba)
                continue
            self.rect(center_x - span_outer, py, span_outer * 2 + 1, 1, rgba)

    def blit(self, source: 'Canvas', x: int, y: int) -> None:
        for row in range(source.height):
            start = row * source.width * 4
            target = ((y + row) * self.width + x) * 4
            self.pixels[target : target + source.width * 4] = source.pixels[start : start + source.width * 4]

    def mirrored(self) -> 'Canvas':
        flipped = Canvas(self.width, self.height)
        for row in range(self.height):
            start = row * self.width * 4
            source_row = self.pixels[start : start + self.width * 4]
            columns = [source_row[column * 4 : column * 4 + 4] for column in range(self.width)]
            flipped.pixels[start : start + self.width * 4] = b''.join(reversed(columns))
        return flipped

    def save(self, path: Path) -> None:
        raw = b''.join(b'\x00' + self.pixels[row * self.width * 4 : (row + 1) * self.width * 4] for row in range(self.height))
        chunk = lambda kind, data: struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data) & 0xFFFFFFFF)
        path.write_bytes(
            b'\x89PNG\r\n\x1a\n'
            + chunk(b'IHDR', struct.pack('>IIBBBBB', self.width, self.height, 8, 6, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(raw, 9))
            + chunk(b'IEND', b'')
        )


def vertical_gradient(
    canvas: Canvas,
    x: int,
    y: int,
    width: int,
    height: int,
    top: tuple[int, int, int, int],
    bottom: tuple[int, int, int, int],
) -> None:
    for row in range(height):
        ratio = row / max(height - 1, 1)
        canvas.rect(x, y + row, width, 1, mix(top, bottom, ratio))


def bloom(canvas: Canvas, x: int, y: int, radius: int, rgba: tuple[int, int, int, int]) -> None:
    radius_squared = radius * radius
    for py in range(max(y - radius, 0), min(y + radius + 1, canvas.height)):
        for px in range(max(x - radius, 0), min(x + radius + 1, canvas.width)):
            distance_squared = (px - x) * (px - x) + (py - y) * (py - y)
            if distance_squared > radius_squared:
                continue
            intensity = 1 - distance_squared / radius_squared
            canvas.point(px, py, (*rgba[:3], round(rgba[3] * intensity * intensity)))


def sparks(canvas: Canvas, palette: dict[str, str], zones: tuple[tuple[int, int, int, int], ...], count: int, phase: int) -> None:
    shades = (color(palette['cyan'], 148), color(palette['violet'], 132), color(palette['rose'], 116))
    for index in range(count):
        zone_x, zone_y, zone_width, zone_height = zones[index % len(zones)]
        x = zone_x + (index * 29 + phase * 13 + 7) % max(zone_width, 1)
        y = zone_y + (index * 83 + phase * 37 + 11) % max(zone_height, 1)
        size = 1 + index % 3
        shade = shades[(index + phase) % len(shades)]
        canvas.rect(x, y, size, size, shade)
        if index % 9 == 0:
            canvas.line(max(x - 4, zone_x), y, min(x + 4, zone_x + zone_width - 1), y, shade)
            canvas.line(x, max(y - 4, zone_y), x, min(y + 4, zone_y + zone_height - 1), shade)


def skyline(canvas: Canvas, palette: dict[str, str], x: int, y: int, width: int, height: int, phase: int) -> None:
    silhouette = color(palette['ink'], 196)
    edge = color(palette['metal'], 120)
    lit = color(palette['cyan'], 96)
    warm = color(palette['amber'], 78)
    cursor = x
    index = 0
    while cursor < x + width:
        block_width = min(16 + (index * 13 + phase * 7) % 30, x + width - cursor)
        block_height = 22 + (index * 29 + phase * 11) % max(height - 18, 20)
        block_top = y + height - block_height
        canvas.rect(cursor, block_top, block_width, block_height, silhouette)
        canvas.rect(cursor, block_top, block_width, 1, edge)
        for row in range(6, block_height - 6, 11):
            for column in range(3, block_width - 4, 8):
                if (index + row + column + phase) % 5 == 0:
                    canvas.rect(cursor + column, block_top + row, 3, 4, warm if (index + row) % 3 == 0 else lit)
        cursor += block_width + 4
        index += 1


def draw_digit(canvas: Canvas, x: int, y: int, width: int, height: int, segments: str, rgba: tuple[int, int, int, int]) -> None:
    pad_x = max(1, round(width * 0.13))
    pad_y = max(1, round(height * 0.08))
    inner_width = width - pad_x * 2
    inner_height = height - pad_y * 2
    thickness = max(2, min(round(height * 0.14), inner_height // SEGMENT_HEIGHTS_PER_CELL))
    left = x + pad_x
    top = y + pad_y
    middle = top + (inner_height - thickness) // 2
    bottom = top + inner_height - thickness
    right = left + inner_width - thickness
    horizontal = inner_width - thickness
    bar_left = left + thickness // 2
    upper = middle - top - thickness
    lower = bottom - middle - thickness
    if 'A' in segments:
        canvas.rounded(bar_left, top, horizontal, thickness, rgba)
    if 'G' in segments:
        canvas.rounded(bar_left, middle, horizontal, thickness, rgba)
    if 'D' in segments:
        canvas.rounded(bar_left, bottom, horizontal, thickness, rgba)
    joined = 'G' not in segments
    for column, upper_segment, lower_segment in ((left, 'F', 'E'), (right, 'B', 'C')):
        has_upper = upper_segment in segments
        has_lower = lower_segment in segments
        if joined and has_upper and has_lower:
            canvas.rect(column, top, thickness, inner_height, rgba)
            continue
        if has_upper:
            canvas.rounded(column, top + thickness, thickness, upper, rgba)
        if has_lower:
            canvas.rounded(column, middle + thickness, thickness, lower, rgba)


def draw_point(canvas: Canvas, x: int, y: int, width: int, height: int, rgba: tuple[int, int, int, int]) -> None:
    thickness = max(2, round(height * 0.14))
    pad_y = max(1, round(height * 0.08))
    canvas.rounded(x + (width - thickness) // 2, y + height - pad_y - thickness, thickness, thickness, rgba, 1)


def digit_sheet(cell_width: int, cell_height: int) -> Canvas:
    canvas = Canvas(cell_width * DIGIT_CELL_COUNT, cell_height)
    ink = (255, 255, 255, 255)
    for value in range(10):
        draw_digit(canvas, value * cell_width, 0, cell_width, cell_height, SEGMENT_MAP[value], ink)
    return canvas


def float_sheet(cell_width: int, cell_height: int, palette: dict[str, str]) -> Canvas:
    canvas = Canvas(cell_width * FLOAT_COLUMN_COUNT, cell_height * FLOAT_ROW_COUNT)
    inks = ((255, 255, 255, 255), color(palette['rose']))
    for row in range(FLOAT_ROW_COUNT):
        ink = inks[FLOAT_NEGATIVE_ROW] if row == FLOAT_NEGATIVE_ROW else inks[0]
        for value in range(10):
            draw_digit(canvas, value * cell_width, row * cell_height, cell_width, cell_height, SEGMENT_MAP[value], ink)
        draw_point(canvas, FLOAT_POINT_CELL * cell_width, row * cell_height, cell_width, cell_height, ink)
    return canvas


def note_row_colors(palette: dict[str, str]) -> tuple[tuple[int, int, int, int], ...]:
    return (
        color(palette['white']),
        color(palette['blue']),
        color(palette['rose']),
        color(palette['amber']),
        color(palette['cyan']),
        mix(color(palette['rose']), color(palette['violet']), 0.35),
        color(palette['amber']),
    )


def note_cell(canvas: Canvas, x: int, y: int, base: tuple[int, int, int, int], state: int, mine: bool, palette: dict[str, str]) -> None:
    width = NOTE_CELL_WIDTH
    height = NOTE_CELL_HEIGHT
    ink = color(palette['ink'])
    if mine:
        canvas.rounded(x + 1, y + 2, width - 2, height - 4, (*scale(base, 0.85)[:3], 236), 2)
        canvas.rect(x + 1, y + 2, width - 2, 1, (*base[:3], 255))
        for offset in range(-height, width, 8):
            canvas.line(x + offset, y + height - 3, x + offset + height, y + 2, (*ink[:3], 190))
        canvas.line(x + width // 2 - 6, y + 6, x + width // 2 + 6, y + height - 7, (*ink[:3], 235), 3)
        canvas.line(x + width // 2 + 6, y + 6, x + width // 2 - 6, y + height - 7, (*ink[:3], 235), 3)
        return
    if state == NOTE_STATE_PROCESSED:
        canvas.rounded(x + 2, y + 4, width - 4, height - 8, (*base[:3], 88), 2)
        canvas.rect(x + 2, y + 4, width - 4, 1, (*base[:3], 150))
        canvas.rect(x + 2, y + height - 5, width - 4, 1, (*base[:3], 150))
        return
    if state in (NOTE_STATE_LN_BODY, NOTE_STATE_LN_ACTIVE):
        active = state == NOTE_STATE_LN_ACTIVE
        fill_alpha = 232 if active else 176
        canvas.rect(x + 1, y, width - 2, height, (*scale(base, 0.34)[:3], fill_alpha))
        for stripe in range(1, width - 2, 10):
            canvas.rect(x + stripe, y, 5, height, (*base[:3], 208 if active else 140))
        canvas.rect(x + 1, y, 2, height, (*base[:3], 236))
        canvas.rect(x + width - 3, y, 2, height, (*base[:3], 236))
        if active:
            canvas.rect(x + 1, y + height // 2 - 2, width - 2, 4, (*color(palette['white'])[:3], 224))
        return
    canvas.rounded(x + 1, y + 2, width - 2, height - 4, scale(base, 0.55), 2)
    vertical_gradient(canvas, x + 2, y + 4, width - 4, height - 8, scale(base, 1.18), scale(base, 0.72))
    canvas.rect(x + 3, y + 4, width - 6, 2, (*color(palette['white'])[:3], 196))
    canvas.rect(x + 3, y + height - 6, width - 6, 1, (*ink[:3], 168))
    if state == NOTE_STATE_LN_START:
        canvas.rect(x + 1, y + height - 7, width - 2, 7, (*base[:3], 198))
    if state == NOTE_STATE_LN_END:
        canvas.rect(x + 1, y, width - 2, 7, (*base[:3], 198))


def notes_sheet(palette: dict[str, str]) -> Canvas:
    canvas = Canvas(NOTE_CELL_WIDTH * NOTE_STATE_COUNT, NOTE_CELL_HEIGHT * NOTE_ROW_COUNT)
    rows = note_row_colors(palette)
    for row in range(NOTE_ROW_COUNT):
        for state in range(NOTE_STATE_COUNT):
            note_cell(canvas, state * NOTE_CELL_WIDTH, row * NOTE_CELL_HEIGHT, rows[row], state, row == NOTE_MINE_ROW, palette)
    return canvas


def lamp_colors(palette: dict[str, str]) -> tuple[tuple[int, int, int, int], ...]:
    return (
        color(palette['metal']),
        color(palette['violetDeep']),
        color(palette['violet']),
        color(palette['cyanDeep']),
        color(palette['blue']),
        color(palette['rose']),
        color(palette['amber']),
        color(palette['cyan']),
        color(palette['white']),
        mix(color(palette['amber']), color(palette['white']), 0.5),
    )


def ui_sheet(palette: dict[str, str]) -> Canvas:
    canvas = Canvas(UI_SIZE, UI_SIZE)
    ink = color(palette['ink'])
    night = color(palette['night'])
    panel = color(palette['panel'])
    metal = color(palette['metal'])
    metal_light = color(palette['metalLight'])
    violet = color(palette['violet'])
    violet_deep = color(palette['violetDeep'])
    cyan = color(palette['cyan'])
    cyan_deep = color(palette['cyanDeep'])
    white = color(palette['white'])
    rose = color(palette['rose'])

    canvas.rect(0, 0, 64, 64, (*night[:3], 224))
    canvas.rect(0, 0, 64, 1, (*metal_light[:3], 150))
    canvas.rect(0, 63, 64, 1, (*ink[:3], 190))

    vertical_gradient(canvas, 64, 0, 64, 64, scale(panel, 1.25), panel)
    canvas.rect(64, 0, 64, 1, (*metal_light[:3], 180))
    canvas.rect(64, 63, 64, 1, (*ink[:3], 200))

    vertical_gradient(canvas, 128, 0, 64, 32, scale(metal, 1.2), scale(metal, 0.72))
    canvas.rect(128, 0, 64, 1, (*metal_light[:3], 220))
    vertical_gradient(canvas, 192, 0, 64, 32, violet, violet_deep)
    canvas.rect(192, 0, 64, 1, (*white[:3], 180))

    vertical_gradient(canvas, 0, 64, 64, 32, scale(panel, 1.35), scale(panel, 0.85))
    canvas.rect(0, 64, 64, 1, (*metal[:3], 220))
    canvas.rect(0, 95, 64, 1, (*ink[:3], 210))
    vertical_gradient(canvas, 64, 64, 64, 32, scale(cyan_deep, 1.35), cyan_deep)
    canvas.rect(64, 64, 64, 1, (*cyan[:3], 240))
    canvas.rect(64, 95, 64, 1, (*ink[:3], 200))

    canvas.rounded(128, 64, 32, 32, (*violet_deep[:3], 236), 3)
    canvas.rect(128, 64, 32, 1, (*violet[:3], 230))

    for index, lamp in enumerate(lamp_colors(palette)):
        x = index * UI_LAMP_SIZE
        canvas.rounded(x, 96, UI_LAMP_SIZE, UI_LAMP_SIZE, lamp, 2)
        canvas.rect(x + 2, 98, UI_LAMP_SIZE - 4, 2, (*white[:3], 110))

    vertical_gradient(canvas, 0, 112, 64, 8, (*cyan[:3], 90), (*cyan[:3], 255))
    canvas.rect(0, 116, 64, 2, (*white[:3], 220))
    vertical_gradient(canvas, 64, 112, 64, 8, (*metal[:3], 70), (*metal_light[:3], 210))

    vertical_gradient(canvas, 0, 120, 64, 64, (*violet[:3], 0), (*violet[:3], 214))
    canvas.rect(0, 180, 64, 4, (*white[:3], 150))

    canvas.ring(88, 144, 23, 15, (*metal[:3], 228))
    canvas.ring(88, 144, 15, 11, (*cyan[:3], 210))
    canvas.ring(88, 144, 6, 0, (*metal_light[:3], 236))

    vertical_gradient(canvas, 112, 120, 24, 40, scale(metal_light, 0.62), scale(metal, 0.72))
    canvas.rect(112, 120, 24, 2, (*metal_light[:3], 180))
    vertical_gradient(canvas, 136, 120, 24, 40, white, scale(cyan, 0.92))
    canvas.rect(136, 120, 24, 2, (*white[:3], 255))
    vertical_gradient(canvas, 160, 120, 20, 40, scale(panel, 1.1), scale(ink, 1.3))
    canvas.rect(160, 120, 20, 2, (*metal[:3], 190))
    vertical_gradient(canvas, 180, 120, 20, 40, scale(color(palette['blue']), 1.15), scale(color(palette['blue']), 0.7))
    canvas.rect(180, 120, 20, 2, (*white[:3], 220))

    gauge_nodes = (
        (0, scale(cyan, 1.0), 255),
        (1, scale(rose, 1.0), 255),
        (2, scale(cyan_deep, 0.65), 120),
        (3, scale(violet_deep, 0.85), 120),
    )
    for index, tone, alpha in gauge_nodes:
        x = index * UI_GAUGE_NODE_WIDTH
        canvas.rounded(x + 1, 185, UI_GAUGE_NODE_WIDTH - 2, UI_GAUGE_NODE_HEIGHT - 2, (*tone[:3], alpha), 2)
        canvas.rect(x + 3, 187, UI_GAUGE_NODE_WIDTH - 6, 2, (*white[:3], 120 if alpha == 255 else 50))

    vertical_gradient(canvas, 128, 184, 64, 64, violet_deep, scale(panel, 1.1))
    canvas.rect(128, 184, 4, 64, (*cyan[:3], 240))
    canvas.rect(128, 184, 64, 1, (*metal_light[:3], 170))
    vertical_gradient(canvas, 192, 184, 64, 64, scale(panel, 1.05), night)
    canvas.rect(192, 184, 4, 64, (*metal[:3], 220))
    canvas.rect(192, 184, 64, 1, (*metal[:3], 140))

    for x in range(128):
        ratio = x / 127
        canvas.rect(x, 200, 1, 8, mix((*violet[:3], 200), (*cyan[:3], 200), ratio))
    return canvas


def cover_solid(palette: dict[str, str]) -> Canvas:
    canvas = Canvas(COVER_WIDTH, COVER_HEIGHT)
    vertical_gradient(canvas, 0, 0, COVER_WIDTH, COVER_HEIGHT, color(palette['ink']), color(palette['night']))
    canvas.rect(0, COVER_HEIGHT - 4, COVER_WIDTH, 4, color(palette['cyan'], 210))
    return canvas


def cover_gradient(palette: dict[str, str]) -> Canvas:
    canvas = Canvas(COVER_WIDTH, COVER_HEIGHT)
    vertical_gradient(canvas, 0, 0, COVER_WIDTH, COVER_HEIGHT, color(palette['night'], 255), color(palette['night'], 0))
    canvas.rect(0, 0, COVER_WIDTH, 3, color(palette['violet'], 210))
    return canvas


def covers_sheet(palette: dict[str, str], solid: Canvas, gradient_cover: Canvas) -> Canvas:
    canvas = Canvas(COVER_WIDTH * 2, COVER_HEIGHT)
    canvas.blit(solid, 0, 0)
    canvas.blit(gradient_cover, COVER_WIDTH, 0)
    return canvas


def backdrop(palette: dict[str, str], top: tuple[int, int, int, int], bottom: tuple[int, int, int, int]) -> Canvas:
    canvas = Canvas(SCREEN_WIDTH, SCREEN_HEIGHT)
    vertical_gradient(canvas, 0, 0, SCREEN_WIDTH, SCREEN_HEIGHT, top, bottom)
    return canvas


def select_background(palette: dict[str, str]) -> Canvas:
    canvas = backdrop(palette, color(palette['night']), color(palette['ink']))
    bloom(canvas, 260, 600, 260, color(palette['violet'], 26))
    bloom(canvas, 1020, 120, 250, color(palette['cyan'], 22))
    skyline(canvas, palette, 24, 500, 640, 156, 3)
    sparks(canvas, palette, ((0, 44, 24, 632), (682, 44, 16, 612), (1258, 44, 22, 632)), 92, 5)
    return canvas


def decide_background(palette: dict[str, str]) -> Canvas:
    canvas = backdrop(palette, color(palette['night']), color(palette['ink']))
    bloom(canvas, 980, 200, 260, color(palette['cyan'], 28))
    bloom(canvas, 840, 560, 220, color(palette['violet'], 24))
    skyline(canvas, palette, 700, 250, 560, 228, 11)
    sparks(canvas, palette, ((708, 30, 560, 210), (0, 40, 74, 640), (1230, 540, 46, 160)), 84, 7)
    return canvas


def play_background(palette: dict[str, str]) -> Canvas:
    canvas = backdrop(palette, color(palette['night']), color(palette['ink']))
    bloom(canvas, 200, 700, 240, color(palette['violet'], 24))
    bloom(canvas, 1150, 40, 220, color(palette['cyan'], 20))
    skyline(canvas, palette, 250, 640, 104, 78, 13)
    zones = ((0, 12, 12, 690), (1000, 12, 6, 690), (1274, 12, 6, 690), (250, 636, 104, 82), (604, 694, 396, 24))
    sparks(canvas, palette, zones, 78, 9)
    return canvas


def result_background(palette: dict[str, str], accent_key: str, base_key: str, phase: int) -> Canvas:
    canvas = backdrop(palette, color(palette[base_key]), color(palette['ink']))
    bloom(canvas, 910, 240, 250, color(palette[accent_key], 30))
    bloom(canvas, 910, 640, 200, color(palette['violet'], 22))
    skyline(canvas, palette, 824, 552, 182, 140, phase)
    sparks(canvas, palette, ((0, 44, 16, 650), (402, 44, 36, 650), (824, 44, 182, 500), (1274, 44, 6, 650)), 86, phase)
    return canvas


def outline(canvas: Canvas, rect: tuple[int, int, int, int], primary: tuple[int, int, int, int], accent: tuple[int, int, int, int]) -> None:
    x, y, width, height = rect
    left = x - FRAME_BORDER
    top = y - FRAME_BORDER
    outer_width = width + FRAME_BORDER * 2
    outer_height = height + FRAME_BORDER * 2
    canvas.rect(left, top, outer_width, FRAME_BORDER, primary)
    canvas.rect(left, y + height, outer_width, FRAME_BORDER, primary)
    canvas.rect(left, top, FRAME_BORDER, outer_height, primary)
    canvas.rect(x + width, top, FRAME_BORDER, outer_height, primary)
    canvas.rect(left, top, FRAME_CORNER, FRAME_BORDER, accent)
    canvas.rect(x + width + FRAME_BORDER - FRAME_CORNER, top, FRAME_CORNER, FRAME_BORDER, accent)
    canvas.rect(left, y + height, FRAME_CORNER, FRAME_BORDER, accent)
    canvas.rect(x + width + FRAME_BORDER - FRAME_CORNER, y + height, FRAME_CORNER, FRAME_BORDER, accent)
    canvas.rect(left, top, FRAME_BORDER, FRAME_CORNER, accent)
    canvas.rect(left, y + height + FRAME_BORDER - FRAME_CORNER, FRAME_BORDER, FRAME_CORNER, accent)
    canvas.rect(x + width, top, FRAME_BORDER, FRAME_CORNER, accent)
    canvas.rect(x + width, y + height + FRAME_BORDER - FRAME_CORNER, FRAME_BORDER, FRAME_CORNER, accent)


def clear_areas(canvas: Canvas, areas: tuple[tuple[int, int, int, int], ...]) -> None:
    for x, y, width, height in areas:
        x_start = max(x, 0)
        y_start = max(y, 0)
        x_end = min(x + width, canvas.width)
        y_end = min(y + height, canvas.height)
        if x_start >= x_end or y_start >= y_end:
            continue
        blank = bytes(4 * (x_end - x_start))
        for py in range(y_start, y_end):
            start = (py * canvas.width + x_start) * 4
            canvas.pixels[start : start + len(blank)] = blank


def frame_canvas(
    palette: dict[str, str],
    rects: tuple[tuple[int, int, int, int], ...],
    separators: tuple[tuple[int, int, int, int], ...],
    protected: tuple[tuple[int, int, int, int], ...],
) -> Canvas:
    canvas = Canvas(SCREEN_WIDTH, SCREEN_HEIGHT)
    primary = color(palette['metal'], 214)
    accent_a = color(palette['cyan'], 230)
    accent_b = color(palette['violet'], 230)
    for index, rect in enumerate(rects):
        outline(canvas, rect, primary, accent_a if index % 2 == 0 else accent_b)
    for x, y, width, height in separators:
        canvas.rect(x, y, width, height, color(palette['metal'], 180))
    clear_areas(canvas, protected)
    return canvas


def frame_single_play(palette: dict[str, str]) -> Canvas:
    rects = (
        (34, 0, 320, 630),
        (368, 8, 632, 100),
        (368, 112, 632, 474),
        (368, 596, 632, 116),
        (1008, 8, 264, 704),
    )
    separators = (
        (34, 503, 320, SEPARATOR_THICKNESS),
        (34, 567, 320, SEPARATOR_THICKNESS),
        (34, 597, 320, SEPARATOR_THICKNESS),
        (609, 596, SEPARATOR_THICKNESS, 116),
        (809, 596, SEPARATOR_THICKNESS, 116),
    )
    protected = (
        (34, 0, 320, 500),
        (34, 508, 320, 56),
        (34, 572, 320, 22),
        (34, 602, 320, 28),
        (368, 8, 632, 100),
        (368, 112, 632, 474),
        (368, 596, 232, 116),
        (620, 610, 180, 80),
        (820, 610, 180, 80),
        (1008, 8, 264, 704),
    )
    return frame_canvas(palette, rects, separators, protected)


def frame_single_play_near(palette: dict[str, str]) -> Canvas:
    rects = (
        (34, 0, 320, 630),
        (368, 8, 264, 704),
        (640, 8, 600, 100),
        (640, 112, 600, 450),
        (640, 596, 600, 116),
    )
    separators = (
        (34, 503, 320, SEPARATOR_THICKNESS),
        (34, 567, 320, SEPARATOR_THICKNESS),
        (34, 597, 320, SEPARATOR_THICKNESS),
        (872, 596, SEPARATOR_THICKNESS, 116),
        (1050, 596, SEPARATOR_THICKNESS, 116),
    )
    protected = (
        (34, 0, 320, 500),
        (34, 508, 320, 56),
        (34, 572, 320, 22),
        (34, 602, 320, 28),
        (368, 8, 264, 704),
        (640, 8, 600, 100),
        (640, 112, 600, 450),
        (640, 596, 232, 116),
        (874, 610, 176, 80),
        (1052, 610, 188, 80),
    )
    return frame_canvas(palette, rects, separators, protected)


def frame_dual_play(palette: dict[str, str]) -> Canvas:
    rects = (
        (14, 8, 232, 704),
        (300, 0, 320, 564),
        (674, 0, 320, 564),
        (460, 572, 360, 22),
        (300, 602, 320, 28),
        (674, 602, 320, 28),
        (1008, 8, 264, 704),
    )
    separators = (
        (14, 164, 232, SEPARATOR_THICKNESS),
        (14, 346, 232, SEPARATOR_THICKNESS),
        (14, 536, 232, SEPARATOR_THICKNESS),
        (300, 505, 320, SEPARATOR_THICKNESS),
        (674, 505, 320, SEPARATOR_THICKNESS),
    )
    protected = (
        (14, 8, 232, 150),
        (14, 168, 232, 174),
        (14, 350, 232, 174),
        (14, 540, 232, 170),
        (300, 0, 320, 500),
        (300, 508, 320, 56),
        (674, 0, 320, 500),
        (674, 508, 320, 56),
        (460, 572, 360, 22),
        (300, 602, 320, 28),
        (674, 602, 320, 28),
        (1008, 8, 264, 704),
    )
    return frame_canvas(palette, rects, separators, protected)


def frame_select(palette: dict[str, str]) -> Canvas:
    rects = (
        (0, 0, 1280, 40),
        (24, 52, 640, 150),
        (24, 212, 640, 124),
        (24, 352, 336, 130),
        (380, 352, 284, 130),
        (700, 60, 556, 600),
        (0, 660, 1280, 60),
    )
    separators = ((684, 44, SEPARATOR_THICKNESS, 612),)
    protected = rects
    return frame_canvas(palette, rects, separators, protected)


def frame_result(palette: dict[str, str]) -> Canvas:
    rects = (
        (0, 0, 1280, 40),
        (16, 48, 384, 648),
        (440, 56, 380, 640),
        (1008, 8, 264, 704),
    )
    separators = (
        (16, 251, 384, SEPARATOR_THICKNESS),
        (16, 507, 384, SEPARATOR_THICKNESS),
        (440, 243, 380, SEPARATOR_THICKNESS),
        (440, 423, 380, SEPARATOR_THICKNESS),
        (440, 558, 380, SEPARATOR_THICKNESS),
    )
    protected = (
        (0, 0, 1280, 40),
        (16, 48, 384, 200),
        (16, 256, 384, 250),
        (16, 512, 384, 200),
        (440, 80, 380, 150),
        (440, 260, 380, 150),
        (440, 440, 380, 110),
        (440, 560, 380, 60),
        (440, 630, 380, 40),
        (1008, 8, 264, 704),
        (0, 700, 1280, 20),
    )
    return frame_canvas(palette, rects, separators, protected)


def main() -> None:
    palette = json.loads((ROOT / 'palette.json').read_text())
    IMAGES.mkdir(parents=True, exist_ok=True)
    COVERS.mkdir(parents=True, exist_ok=True)
    solid = cover_solid(palette)
    gradient_cover = cover_gradient(palette)
    single_play_frame = frame_single_play(palette)
    near_play_frame = frame_single_play_near(palette)
    images = {
        'notes.png': notes_sheet(palette),
        'digits-s.png': digit_sheet(*DIGIT_SMALL_SIZE),
        'digits-m.png': digit_sheet(*DIGIT_MEDIUM_SIZE),
        'digits-l.png': digit_sheet(*DIGIT_LARGE_SIZE),
        'digits-f.png': float_sheet(*DIGIT_MEDIUM_SIZE, palette),
        'ui.png': ui_sheet(palette),
        'covers.png': covers_sheet(palette, solid, gradient_cover),
        'select-bg.png': select_background(palette),
        'decide-bg.png': decide_background(palette),
        'play-bg.png': play_background(palette),
        'result-bg-aaa.png': result_background(palette, 'amber', 'night', 19),
        'result-bg-aa.png': result_background(palette, 'cyan', 'night', 23),
        'result-bg-a.png': result_background(palette, 'blue', 'night', 29),
        'result-bg-clear.png': result_background(palette, 'violet', 'night', 31),
        'result-bg-failed.png': result_background(palette, 'rose', 'violetDeep', 37),
        'frame-sp.png': single_play_frame,
        'frame-sp-2p.png': single_play_frame.mirrored(),
        'frame-sp-near.png': near_play_frame,
        'frame-sp-2p-near.png': near_play_frame.mirrored(),
        'frame-dp.png': frame_dual_play(palette),
        'frame-select.png': frame_select(palette),
        'frame-result.png': frame_result(palette),
    }
    for name, canvas in images.items():
        canvas.save(IMAGES / name)
    solid.save(COVERS / 'Solid.png')
    gradient_cover.save(COVERS / 'Gradient.png')


if __name__ == '__main__':
    main()
