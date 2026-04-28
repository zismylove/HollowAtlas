from __future__ import annotations

from dataclasses import dataclass
from typing import List, Optional, Sequence, Tuple

from .types import Rect


@dataclass(frozen=True)
class InsertResult:
    index: int
    rect: Rect
    rotated: bool


class MaxRectsPacker:
    def __init__(self, width: int, height: int):
        if width <= 0 or height <= 0:
            raise ValueError("Atlas dimensions must be positive.")
        self.width = int(width)
        self.height = int(height)
        self.free_rects: List[Rect] = [Rect(0, 0, self.width, self.height)]
        self.used_rects: List[Rect] = []

    def insert(self, index: int, width: int, height: int, allow_rotation: bool = False) -> Optional[InsertResult]:
        best = self._find_best_position(width, height, allow_rotation)
        if best is None:
            return None

        rect, rotated = best
        self._place(rect)
        return InsertResult(index=index, rect=rect, rotated=rotated)

    def _find_best_position(self, width: int, height: int, allow_rotation: bool) -> Optional[Tuple[Rect, bool]]:
        best_rect: Optional[Rect] = None
        best_rotated = False
        best_area_score: Optional[int] = None
        best_short_side: Optional[int] = None

        for free in self.free_rects:
            candidates = [(width, height, False)]
            if allow_rotation and width != height:
                candidates.append((height, width, True))

            for candidate_w, candidate_h, rotated in candidates:
                if candidate_w > free.w or candidate_h > free.h:
                    continue

                leftover_area = free.area - candidate_w * candidate_h
                short_side = min(free.w - candidate_w, free.h - candidate_h)
                if (
                    best_area_score is None
                    or leftover_area < best_area_score
                    or (leftover_area == best_area_score and short_side < (best_short_side or 0))
                ):
                    best_rect = Rect(free.x, free.y, candidate_w, candidate_h)
                    best_rotated = rotated
                    best_area_score = leftover_area
                    best_short_side = short_side

        if best_rect is None:
            return None
        return best_rect, best_rotated

    def _place(self, rect: Rect) -> None:
        next_free: List[Rect] = []
        for free in self.free_rects:
            if not free.intersects(rect):
                next_free.append(free)
                continue
            next_free.extend(split_free_rect(free, rect))

        self.free_rects = prune_free_rects(next_free)
        self.used_rects.append(rect)


def split_free_rect(free: Rect, used: Rect) -> List[Rect]:
    if not free.intersects(used):
        return [free]

    result: List[Rect] = []

    if used.x < free.right and used.right > free.x:
        if used.y > free.y and used.y < free.bottom:
            result.append(Rect(free.x, free.y, free.w, used.y - free.y))
        if used.bottom < free.bottom:
            result.append(Rect(free.x, used.bottom, free.w, free.bottom - used.bottom))

    if used.y < free.bottom and used.bottom > free.y:
        if used.x > free.x and used.x < free.right:
            result.append(Rect(free.x, free.y, used.x - free.x, free.h))
        if used.right < free.right:
            result.append(Rect(used.right, free.y, free.right - used.right, free.h))

    return [rect for rect in result if rect.w > 0 and rect.h > 0]


def prune_free_rects(rects: Sequence[Rect]) -> List[Rect]:
    pruned: List[Rect] = []
    for i, rect in enumerate(rects):
        contained = False
        for j, other in enumerate(rects):
            if i != j and other.contains(rect):
                contained = True
                break
        if not contained and rect not in pruned:
            pruned.append(rect)
    return pruned
