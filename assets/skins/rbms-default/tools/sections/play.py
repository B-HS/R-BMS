from __future__ import annotations

from PIL import Image, ImageDraw

from sprites import OPAQUE, WHITE, Rgb, Sprite, block, digit_strip, guarded, palette, themes, tinted, word

NOTE = palette('note')
TONE = palette('tone')
JUDGE = palette('judge')
JUDGE_WORD = palette('judge_word')
LAMP = palette('lamp')

NOTE_SHADE: Rgb = NOTE['shade']
DRAIN: Rgb = TONE['rose']
PURE_NOTES = 'pure'

NOTE_COLORS: dict[str, Rgb] = {
    'white': NOTE['white'],
    'cyan': NOTE['cyan'],
    'gray': NOTE['gray'],
    'silver': NOTE['silver'],
    'red': TONE['red'],
    'yellow': TONE['yellow'],
    'green': TONE['green'],
    'blue': TONE['blue'],
}
NOTE_SIZE = (4, 8)
NOTE_FOOT_ROWS = 1
NOTE_FOOT_SHARE = 0.55
BODY_SIZE = (8, 4)
BODY_COLUMNS = (1, 7)
BODY_IDLE_SHARE = 0.4
BODY_HELD_SHARE = 0.8
RAIL_COLUMNS = (3, 5)
RAIL_SIDE_SHARE = 0.3
MINE_SIZE = (4, 8)
MINE_EDGE_ROWS = 2
MINE_FILL_ALPHA = 130

GAUGE_COLORS: tuple[Rgb, ...] = (LAMP['assist'], LAMP['easy'], LAMP['normal'], LAMP['hard'], LAMP['exhard'], TONE['red'])
GAUGE_NODE_SIZE = (4, 8)
GAUGE_GAP_COLUMNS = 1
GAUGE_BELOW_BORDER_SHARE = 0.55
GAUGE_AHEAD_SHARE = 0.25
GAUGE_TIP_WHITE_SHARE = 0.5

JUDGE_WORD_SIZE = 64
JUDGE_WORDS: tuple[tuple[str, str, Rgb], ...] = (
    ('pgreat', 'GREAT', JUDGE_WORD['pgreat']),
    ('great', 'GREAT', JUDGE_WORD['great']),
    ('good', 'GOOD', JUDGE['good']),
    ('bad', 'BAD', JUDGE['bad']),
    ('poor', 'POOR', JUDGE['poor']),
)
COMBO_JUDGES = 3

BOMB_FRAMES = 8
BOMB_SIDE = 128
BOMB_OVERSAMPLE = 4
BOMB_RADII = (16.0, 46.0)
BOMB_WIDTHS = (10.0, 2.0)
BOMB_ALPHAS = (255, 70)

BEAM_SIZE = (8, 64)
BEAM_ALPHA = 118
PULSE_ALPHA = 71
BEAM_CURVE = 2
BEAM_GUARD = 1

BAR_LENGTH = 256
BAR_THICKNESS = 4


def mixed(color: Rgb, toward: Rgb, share: float) -> Rgb:
    return tuple(round(toward[channel] + (color[channel] - toward[channel]) * share) for channel in range(len(color)))


def shaded(color: Rgb, share: float, toward: Rgb = NOTE_SHADE) -> Rgb:
    return mixed(color, toward, share)


def filled(size: tuple[int, int], color: Rgb, alpha: int = OPAQUE) -> Image.Image:
    return Image.new('RGBA', size, (*color, alpha))


def gray(color: Rgb) -> Rgb:
    level = round(sum(color) / len(color))
    return (level,) * len(color)


def note(name: str, color: Rgb, toward: Rgb) -> Sprite:
    width, height = NOTE_SIZE
    image = filled(NOTE_SIZE, color)
    image.paste(filled((width, NOTE_FOOT_ROWS), shaded(color, NOTE_FOOT_SHARE, toward)), (0, height - NOTE_FOOT_ROWS))
    return Sprite(name, image)


def body(name: str, side: Rgb, middle: Rgb | None = None) -> Sprite:
    _, height = BODY_SIZE
    left, right = BODY_COLUMNS
    image = Image.new('RGBA', BODY_SIZE, (*WHITE, 0))
    image.paste(filled((right - left, height), side), (left, 0))
    if middle is not None:
        inner_left, inner_right = RAIL_COLUMNS
        image.paste(filled((inner_right - inner_left, height), middle), (inner_left, 0))
    return Sprite(name, image)


def mine(name: str) -> Sprite:
    width, height = MINE_SIZE
    image = filled(MINE_SIZE, DRAIN, MINE_FILL_ALPHA)
    edge = filled((width, MINE_EDGE_ROWS), DRAIN)
    image.paste(edge, (0, 0))
    image.paste(edge, (0, height - MINE_EDGE_ROWS))
    return Sprite(name, image)


def note_family(key: str, color: Rgb, toward: Rgb) -> list[Sprite]:
    return [
        note(f'note_{key}', color, toward),
        body(f'long_{key}', shaded(color, BODY_IDLE_SHARE, toward)),
        body(f'held_{key}', shaded(color, BODY_HELD_SHARE, toward)),
        body(f'hell_{key}', shaded(color, RAIL_SIDE_SHARE, toward), shaded(color, BODY_HELD_SHARE, toward)),
        body(f'hell_held_{key}', shaded(color, BODY_IDLE_SHARE, toward), mixed(color, WHITE, GAUGE_TIP_WHITE_SHARE)),
    ]


def notes() -> list[Sprite]:
    sprites = [mine('mine'), body('hell_drain', shaded(DRAIN, RAIL_SIDE_SHARE), DRAIN)]
    for key, color in NOTE_COLORS.items():
        sprites += note_family(key, color, NOTE_SHADE)
    sprites += note_family(PURE_NOTES, WHITE, gray(NOTE_SHADE))
    return sprites


def gauge_node(name: str, color: Rgb) -> Sprite:
    width, height = GAUGE_NODE_SIZE
    image = Image.new('RGBA', GAUGE_NODE_SIZE, (*WHITE, 0))
    image.paste(filled((width - GAUGE_GAP_COLUMNS, height), color), (0, 0))
    return Sprite(name, image)


def gauge_states(color: Rgb, surface: dict[str, Rgb]) -> tuple[Rgb, ...]:
    return (
        color,
        shaded(color, GAUGE_BELOW_BORDER_SHARE, surface['bg']),
        shaded(color, GAUGE_AHEAD_SHARE, surface['bg']),
        surface['line'],
        mixed(color, WHITE, GAUGE_TIP_WHITE_SHARE),
        color,
    )


def gauge() -> list[Sprite]:
    return [
        gauge_node(f'gauge_{kind}_{state}{suffix}', shade)
        for suffix, surface in themes().items()
        for kind, color in enumerate(GAUGE_COLORS)
        for state, shade in enumerate(gauge_states(color, surface))
    ]


def between(ends: tuple[float, float], share: float) -> float:
    return ends[0] + (ends[1] - ends[0]) * share


def bomb(name: str) -> Sprite:
    side = BOMB_SIDE * BOMB_OVERSAMPLE
    middle = side / 2
    mask = Image.new('L', (BOMB_SIDE * BOMB_FRAMES, BOMB_SIDE), 0)
    for frame in range(BOMB_FRAMES):
        share = frame / (BOMB_FRAMES - 1)
        radius = between(BOMB_RADII, share) * BOMB_OVERSAMPLE
        width = round(between(BOMB_WIDTHS, share) * BOMB_OVERSAMPLE)
        ring = Image.new('L', (side, side), 0)
        bounds = (middle - radius, middle - radius, middle + radius, middle + radius)
        ImageDraw.Draw(ring).ellipse(bounds, outline=round(between(BOMB_ALPHAS, share)), width=width)
        mask.paste(ring.resize((BOMB_SIDE, BOMB_SIDE), Image.Resampling.BOX), (frame * BOMB_SIDE, 0))
    return Sprite(name, tinted(mask), BOMB_FRAMES, 1)


def beam(name: str, alpha: int) -> Sprite:
    width, height = BEAM_SIZE
    mask = Image.new('L', BEAM_SIZE, 0)
    for row in range(height):
        share = (row + 1) / height
        mask.paste(round(alpha * share**BEAM_CURVE), (0, row, width, row + 1))
    return guarded(Sprite(name, tinted(mask)), BEAM_GUARD)


def judge_word(name: str, text: str, color: Rgb) -> Sprite:
    sprite = word(name, text, JUDGE_WORD_SIZE, color)
    left, _, right, _ = sprite.image.getchannel('A').getbbox()
    return Sprite(name, sprite.image.crop((left, 0, right, sprite.image.height)), meta=sprite.meta)


def build() -> list[Sprite]:
    sprites = notes() + gauge()
    sprites += [judge_word(f'judge_{name}', text, color) for name, text, color in JUDGE_WORDS]
    sprites += [digit_strip(f'combo_{name}', JUDGE_WORD_SIZE, color) for name, _, color in JUDGE_WORDS[:COMBO_JUDGES]]
    sprites += [bomb('bomb'), beam('beam', BEAM_ALPHA), beam('pulse', PULSE_ALPHA)]
    sprites += [block('bar_h', BAR_LENGTH, BAR_THICKNESS), block('bar_v', BAR_THICKNESS, BAR_LENGTH)]
    return sprites
