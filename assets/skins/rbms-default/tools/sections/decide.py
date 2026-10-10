from __future__ import annotations

from sprites import Sprite, digit_strip

LEVEL_DIGIT_SIZE = 144


def build() -> list[Sprite]:
    return [digit_strip(f'num{LEVEL_DIGIT_SIZE}', LEVEL_DIGIT_SIZE)]
