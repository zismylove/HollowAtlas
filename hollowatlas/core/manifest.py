from __future__ import annotations

from pathlib import Path
from typing import Any, Dict, Iterable

from .types import AtlasResult, PackedSprite


def build_godot_tpsheet(atlases: Iterable[AtlasResult]) -> Dict[str, Any]:
    textures = []
    for atlas in atlases:
        textures.append(
            {
                "image": Path(atlas.image_path).name,
                "format": "RGBA8888",
                "size": {"w": atlas.width, "h": atlas.height},
                "sprites": _build_sprite_items(atlas.sprites),
            }
        )

    return {
        "textures": textures,
        "meta": {
            "app": "Texture Atlas Packer",
            "version": "0.1.0",
            "target": "Godot TexturePacker Importer",
            "scale": "1",
        },
    }


def _build_sprite_items(sprites: Iterable[PackedSprite]) -> list[Dict[str, Any]]:
    sprite_items = []
    for sprite in sorted(sprites, key=lambda item: item.rel_path.lower()):
        margin_w = max(0, sprite.source_w - sprite.trim_w)
        margin_h = max(0, sprite.source_h - sprite.trim_h)
        sprite_items.append(
            {
                "filename": sprite.rel_path,
                "region": {"x": sprite.x, "y": sprite.y, "w": sprite.w, "h": sprite.h},
                "margin": {
                    "x": sprite.offset_x,
                    "y": sprite.offset_y,
                    "w": margin_w,
                    "h": margin_h,
                },
                "spriteSourceSize": {
                    "x": sprite.offset_x,
                    "y": sprite.offset_y,
                    "w": sprite.trim_w,
                    "h": sprite.trim_h,
                },
                "sourceSize": {"w": sprite.source_w, "h": sprite.source_h},
                "rotated": sprite.rotated,
                "trimmed": sprite.trimmed,
            }
        )
    return sprite_items


def build_texturepacker_json_hash(image_name: str, width: int, height: int, sprites: Iterable[PackedSprite]) -> Dict[str, Any]:
    frames: Dict[str, Dict[str, Any]] = {}
    for sprite in sorted(sprites, key=lambda item: item.rel_path.lower()):
        frames[sprite.rel_path] = {
            "frame": {"x": sprite.x, "y": sprite.y, "w": sprite.w, "h": sprite.h},
            "rotated": sprite.rotated,
            "trimmed": sprite.trimmed,
            "spriteSourceSize": {
                "x": sprite.offset_x,
                "y": sprite.offset_y,
                "w": sprite.trim_w,
                "h": sprite.trim_h,
            },
            "sourceSize": {"w": sprite.source_w, "h": sprite.source_h},
        }

    return {
        "frames": frames,
        "meta": {
            "app": "Texture Atlas Packer",
            "version": "0.1.0",
            "image": image_name,
            "format": "RGBA8888",
            "size": {"w": width, "h": height},
            "scale": "1",
        },
    }
