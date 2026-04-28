import json
from pathlib import Path

from PIL import Image

from hollowatlas.core.packer import pack_folder
from hollowatlas.core.types import PackConfig, SplitMode


def test_pack_folder_writes_png_and_tpsheet():
    base = Path("build/test_runtime/pack_folder")
    input_dir = base / "input"
    output_dir = base / "output"
    (input_dir / "characters").mkdir(parents=True, exist_ok=True)
    output_dir.mkdir(parents=True, exist_ok=True)

    hero = Image.new("RGBA", (16, 16), (0, 0, 0, 0))
    for x in range(4, 12):
        for y in range(3, 13):
            hero.putpixel((x, y), (255, 0, 0, 255))
    hero.save(input_dir / "characters" / "hero.png")

    icon = Image.new("RGBA", (8, 8), (0, 0, 255, 255))
    icon.save(input_dir / "icon.png")

    result = pack_folder(
        input_dir,
        output_dir,
        PackConfig(max_size=128, padding=2, extrude=1, trim=True, debug_json=True),
    )

    assert result.total_sprites == 2
    assert result.total_atlases == 1
    atlas = result.atlases[0]
    assert Path(atlas.image_path).exists()
    assert Path(atlas.tpsheet_path).exists()
    assert Path(atlas.tpsheet_path).name == "atlas.tpsheet"
    assert Path(atlas.debug_json_path).exists()

    manifest = json.loads(Path(atlas.tpsheet_path).read_text(encoding="utf-8"))
    assert len(manifest["textures"]) == 1
    texture = manifest["textures"][0]
    assert texture["image"] == "atlas_0.png"
    assert texture["format"] == "RGBA8888"
    sprites = {sprite["filename"]: sprite for sprite in texture["sprites"]}
    assert "characters/hero.png" in sprites
    hero_frame = sprites["characters/hero.png"]
    assert hero_frame["trimmed"] is True
    assert hero_frame["spriteSourceSize"]["x"] == 4
    assert hero_frame["spriteSourceSize"]["y"] == 3
    assert hero_frame["sourceSize"] == {"w": 16, "h": 16}
    assert hero_frame["margin"] == {"x": 4, "y": 3, "w": 8, "h": 6}


def test_pack_folder_splits_multiple_atlases_when_needed():
    base = Path("build/test_runtime/multi_atlas")
    input_dir = base / "input"
    output_dir = base / "output"
    input_dir.mkdir(parents=True, exist_ok=True)
    output_dir.mkdir(parents=True, exist_ok=True)

    for index in range(5):
        image = Image.new("RGBA", (32, 32), (index * 20, 0, 200, 255))
        image.save(input_dir / f"sprite_{index}.png")

    result = pack_folder(
        input_dir,
        output_dir,
        PackConfig(max_size=64, padding=0, extrude=0, trim=False),
    )

    assert result.total_sprites == 5
    assert result.total_atlases == 2
    assert Path(result.atlases[0].image_path).name == "atlas_0.png"
    assert Path(result.atlases[1].image_path).name == "atlas_1.png"
    assert Path(result.atlases[0].tpsheet_path) == Path(result.atlases[1].tpsheet_path)
    manifest = json.loads(Path(result.atlases[0].tpsheet_path).read_text(encoding="utf-8"))
    assert len(manifest["textures"]) == 2


def test_pack_folder_by_first_level_folder_creates_grouped_atlases():
    base = Path("build/test_runtime/by_first_level")
    input_dir = base / "input"
    output_dir = base / "output"
    (input_dir / "characters").mkdir(parents=True, exist_ok=True)
    (input_dir / "ui").mkdir(parents=True, exist_ok=True)
    output_dir.mkdir(parents=True, exist_ok=True)

    Image.new("RGBA", (8, 8), (255, 0, 0, 255)).save(input_dir / "characters" / "hero.png")
    Image.new("RGBA", (8, 8), (0, 255, 0, 255)).save(input_dir / "ui" / "button.png")

    result = pack_folder(
        input_dir,
        output_dir,
        PackConfig(max_size=64, padding=0, extrude=0, trim=False, split_mode=SplitMode.BY_FIRST_LEVEL_FOLDER),
    )

    assert result.total_sprites == 2
    assert result.total_atlases == 2
    assert [len(atlas.sprites) for atlas in result.atlases] == [1, 1]
    assert Path(result.atlases[0].tpsheet_path) == Path(result.atlases[1].tpsheet_path)


def test_godot_tpsheet_disables_rotation_for_importer_safety():
    base = Path("build/test_runtime/rotation_guard")
    input_dir = base / "input"
    output_dir = base / "output"
    input_dir.mkdir(parents=True, exist_ok=True)
    output_dir.mkdir(parents=True, exist_ok=True)

    Image.new("RGBA", (16, 32), (255, 0, 255, 255)).save(input_dir / "tall.png")

    result = pack_folder(
        input_dir,
        output_dir,
        PackConfig(max_size=64, padding=0, extrude=0, trim=False, allow_rotation=True),
    )

    assert any("rotation disabled" in log.message for log in result.logs)
    assert result.atlases[0].sprites[0].rotated is False
