from __future__ import annotations

from PIL import Image

try:
    NEAREST = Image.Resampling.NEAREST
except AttributeError:  # pragma: no cover - Pillow before 9.
    NEAREST = Image.NEAREST


def extrude_image(image: Image.Image, amount: int) -> Image.Image:
    amount = max(0, int(amount))
    rgba = image.convert("RGBA")
    if amount == 0:
        return rgba.copy()

    width, height = rgba.size
    out = Image.new("RGBA", (width + amount * 2, height + amount * 2), (0, 0, 0, 0))
    out.paste(rgba, (amount, amount))

    top = rgba.crop((0, 0, width, 1)).resize((width, amount), NEAREST)
    bottom = rgba.crop((0, height - 1, width, height)).resize((width, amount), NEAREST)
    left = rgba.crop((0, 0, 1, height)).resize((amount, height), NEAREST)
    right = rgba.crop((width - 1, 0, width, height)).resize((amount, height), NEAREST)

    out.paste(top, (amount, 0))
    out.paste(bottom, (amount, amount + height))
    out.paste(left, (0, amount))
    out.paste(right, (amount + width, amount))

    top_left = rgba.crop((0, 0, 1, 1)).resize((amount, amount), NEAREST)
    top_right = rgba.crop((width - 1, 0, width, 1)).resize((amount, amount), NEAREST)
    bottom_left = rgba.crop((0, height - 1, 1, height)).resize((amount, amount), NEAREST)
    bottom_right = rgba.crop((width - 1, height - 1, width, height)).resize((amount, amount), NEAREST)

    out.paste(top_left, (0, 0))
    out.paste(top_right, (amount + width, 0))
    out.paste(bottom_left, (0, amount + height))
    out.paste(bottom_right, (amount + width, amount + height))
    return out
