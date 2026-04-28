from PIL import Image

from hollowatlas.core.extrude import extrude_image
from hollowatlas.core.trim import trim_transparent


def test_trim_transparent_tracks_source_offsets():
    image = Image.new("RGBA", (6, 5), (0, 0, 0, 0))
    image.putpixel((2, 1), (255, 0, 0, 255))
    image.putpixel((4, 3), (0, 255, 0, 255))

    result = trim_transparent(image, enabled=True)

    assert result.trim_x == 2
    assert result.trim_y == 1
    assert result.trim_width == 3
    assert result.trim_height == 3
    assert result.image.size == (3, 3)
    assert result.source_width == 6
    assert result.source_height == 5
    assert result.trimmed is True


def test_fully_transparent_image_becomes_one_pixel_sprite():
    image = Image.new("RGBA", (4, 4), (0, 0, 0, 0))

    result = trim_transparent(image, enabled=True)

    assert result.image.size == (1, 1)
    assert result.fully_transparent is True
    assert result.trimmed is True


def test_extrude_replicates_edge_pixels():
    image = Image.new("RGBA", (2, 2), (0, 0, 0, 0))
    image.putpixel((0, 0), (10, 0, 0, 255))
    image.putpixel((1, 0), (20, 0, 0, 255))
    image.putpixel((0, 1), (30, 0, 0, 255))
    image.putpixel((1, 1), (40, 0, 0, 255))

    out = extrude_image(image, 1)

    assert out.size == (4, 4)
    assert out.getpixel((0, 0)) == (10, 0, 0, 255)
    assert out.getpixel((3, 0)) == (20, 0, 0, 255)
    assert out.getpixel((0, 3)) == (30, 0, 0, 255)
    assert out.getpixel((3, 3)) == (40, 0, 0, 255)
    assert out.getpixel((1, 1)) == (10, 0, 0, 255)
