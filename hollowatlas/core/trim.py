from __future__ import annotations

from dataclasses import dataclass

from PIL import Image


@dataclass(frozen=True)
class TrimResult:
    image: Image.Image
    trim_x: int
    trim_y: int
    trim_width: int
    trim_height: int
    source_width: int
    source_height: int
    trimmed: bool
    fully_transparent: bool = False


def trim_transparent(image: Image.Image, enabled: bool = True) -> TrimResult:
    rgba = image.convert("RGBA")
    source_width, source_height = rgba.size

    if not enabled:
        return TrimResult(
            image=rgba.copy(),
            trim_x=0,
            trim_y=0,
            trim_width=source_width,
            trim_height=source_height,
            source_width=source_width,
            source_height=source_height,
            trimmed=False,
        )

    alpha = rgba.getchannel("A")
    bbox = alpha.getbbox()
    if bbox is None:
        transparent = Image.new("RGBA", (1, 1), (0, 0, 0, 0))
        return TrimResult(
            image=transparent,
            trim_x=0,
            trim_y=0,
            trim_width=1,
            trim_height=1,
            source_width=source_width,
            source_height=source_height,
            trimmed=True,
            fully_transparent=True,
        )

    left, top, right, bottom = bbox
    cropped = rgba.crop(bbox)
    return TrimResult(
        image=cropped,
        trim_x=left,
        trim_y=top,
        trim_width=right - left,
        trim_height=bottom - top,
        source_width=source_width,
        source_height=source_height,
        trimmed=(left, top, right, bottom) != (0, 0, source_width, source_height),
    )
