from __future__ import annotations

from sprites import Sprite, block, digit_strip, signed_digit_strip

DIGIT_SIZES = (18, 24, 32, 44, 64, 96)
SIGNED_DIGIT_SIZES = (24, 32)
BLOCK_SIDE = 8


def build() -> list[Sprite]:
    sprites = [digit_strip(f'num{size}', size) for size in DIGIT_SIZES]
    sprites += [signed_digit_strip(f'snum{size}', size) for size in SIGNED_DIGIT_SIZES]
    sprites.append(block('block', BLOCK_SIDE, BLOCK_SIDE))
    return sprites
