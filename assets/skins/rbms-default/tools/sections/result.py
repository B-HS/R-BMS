from __future__ import annotations

from sprites import Sprite, block

BAR_WIDTH = 512
BAR_HEIGHT = 8


def build() -> list[Sprite]:
    return [block('bar', BAR_WIDTH, BAR_HEIGHT)]
