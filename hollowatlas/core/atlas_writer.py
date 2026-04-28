from __future__ import annotations

import json
from pathlib import Path
from typing import List, Optional

from PIL import Image

from .manifest import build_godot_tpsheet
from .types import AtlasBuild, AtlasResult, PackConfig, PackedSprite, Placement

TRANSPARENT_RGBA = (0, 0, 0, 0)


def write_atlas(
    build: AtlasBuild,
    output_dir: str | Path,
    atlas_name: str,
    tpsheet_name: str,
    config: PackConfig,
) -> AtlasResult:
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)

    png_path = output_path / f"{atlas_name}.png"
    tpsheet_path = output_path / tpsheet_name
    debug_path: Optional[Path] = output_path / f"{atlas_name}.debug.json" if config.debug_json else None

    atlas_image = Image.new("RGBA", (build.width, build.height), TRANSPARENT_RGBA)  # type: ignore[arg-type]
    packed_sprites: List[PackedSprite] = []

    for placement in build.placements:
        sprite = placement.sprite
        draw_image = _image_for_placement(placement)
        paste_x = placement.rect.x + config.padding
        paste_y = placement.rect.y + config.padding
        atlas_image.alpha_composite(draw_image.convert("RGBA"), (paste_x, paste_y))

        frame_x = paste_x + config.extrude
        frame_y = paste_y + config.extrude
        frame_w = sprite.trim_height if placement.rotated else sprite.trim_width
        frame_h = sprite.trim_width if placement.rotated else sprite.trim_height

        packed_sprites.append(
            PackedSprite(
                name=sprite.name,
                rel_path=sprite.rel_path,
                atlas_index=build.atlas_index,
                x=frame_x,
                y=frame_y,
                w=frame_w,
                h=frame_h,
                source_w=sprite.source_width,
                source_h=sprite.source_height,
                offset_x=sprite.trim_x,
                offset_y=sprite.trim_y,
                trim_w=sprite.trim_width,
                trim_h=sprite.trim_height,
                rotated=placement.rotated,
                trimmed=sprite.trimmed,
                pack_x=placement.rect.x,
                pack_y=placement.rect.y,
                pack_w=placement.rect.w,
                pack_h=placement.rect.h,
            )
        )

    atlas_image.save(png_path)

    if debug_path is not None:
        debug = {
            "config": config.to_dict(),
            "atlas": {
                "name": atlas_name,
                "image": png_path.name,
                "tpsheet": tpsheet_name,
                "width": build.width,
                "height": build.height,
                "usage": build.usage,
                "group": build.group_name,
            },
            "sprites": [sprite.to_dict() for sprite in packed_sprites],
        }
        debug_path.write_text(json.dumps(debug, ensure_ascii=False, indent=2), encoding="utf-8")

    return AtlasResult(
        image_path=str(png_path),
        tpsheet_path=str(tpsheet_path),
        debug_json_path=str(debug_path) if debug_path else None,
        width=build.width,
        height=build.height,
        usage=build.usage,
        sprites=packed_sprites,
    )


def write_tpsheet(output_dir: str | Path, tpsheet_name: str, atlases: list[AtlasResult]) -> Path:
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)

    tpsheet_path = output_path / tpsheet_name
    manifest = build_godot_tpsheet(atlases)
    tpsheet_path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8")
    return tpsheet_path


def _image_for_placement(placement: Placement) -> Image.Image:
    if not placement.rotated:
        return placement.sprite.image
    return placement.sprite.image.transpose(Image.Transpose.ROTATE_90)
