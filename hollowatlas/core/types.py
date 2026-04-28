from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Any, Dict, List, Optional

from PIL import Image


class SplitMode(str, Enum):
    ALL_IN_ONE = "all_in_one"
    BY_FIRST_LEVEL_FOLDER = "by_first_level_folder"


class OutputFormat(str, Enum):
    GODOT_TPSHEET = "godot_tpsheet"
    JSON_DEBUG = "json_debug"


@dataclass(frozen=True)
class PackConfig:
    max_size: int = 2048
    padding: int = 2
    extrude: int = 1
    trim: bool = True
    allow_rotation: bool = False
    power_of_two: bool = True
    square: bool = True
    split_mode: SplitMode = SplitMode.ALL_IN_ONE
    output_format: OutputFormat = OutputFormat.GODOT_TPSHEET
    debug_json: bool = False

    def normalized(self) -> "PackConfig":
        return PackConfig(
            max_size=max(1, int(self.max_size)),
            padding=max(0, int(self.padding)),
            extrude=max(0, int(self.extrude)),
            trim=bool(self.trim),
            allow_rotation=bool(self.allow_rotation),
            power_of_two=bool(self.power_of_two),
            square=bool(self.square),
            split_mode=SplitMode(self.split_mode),
            output_format=OutputFormat(self.output_format),
            debug_json=bool(self.debug_json),
        )

    def to_dict(self) -> Dict[str, Any]:
        return {
            "max_size": self.max_size,
            "padding": self.padding,
            "extrude": self.extrude,
            "trim": self.trim,
            "allow_rotation": self.allow_rotation,
            "power_of_two": self.power_of_two,
            "square": self.square,
            "split_mode": self.split_mode.value,
            "output_format": self.output_format.value,
            "debug_json": self.debug_json,
        }


@dataclass
class FileTreeNode:
    name: str
    path: str
    type: str
    children: List["FileTreeNode"] = field(default_factory=list)
    image_count: int = 0

    def to_dict(self) -> Dict[str, Any]:
        return {
            "name": self.name,
            "path": self.path,
            "type": self.type,
            "children": [child.to_dict() for child in self.children],
            "imageCount": self.image_count,
        }


@dataclass
class SourceImage:
    id: int
    name: str
    abs_path: str
    rel_path: str
    width: int
    height: int
    file_size: int
    readable: bool = True
    error: Optional[str] = None

    def to_dict(self) -> Dict[str, Any]:
        return {
            "id": self.id,
            "name": self.name,
            "abs_path": self.abs_path,
            "rel_path": self.rel_path,
            "width": self.width,
            "height": self.height,
            "file_size": self.file_size,
            "readable": self.readable,
            "error": self.error,
        }


@dataclass
class ScanResult:
    root: FileTreeNode
    images: List[SourceImage]
    total_images: int
    warnings: List[str] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "root": self.root.to_dict(),
            "images": [image.to_dict() for image in self.images],
            "total_images": self.total_images,
            "warnings": list(self.warnings),
        }


@dataclass
class PreparedSprite:
    id: int
    name: str
    abs_path: str
    rel_path: str
    source_width: int
    source_height: int
    trim_x: int
    trim_y: int
    trim_width: int
    trim_height: int
    image: Image.Image
    padding: int
    extrude: int
    trimmed: bool

    @property
    def packed_width(self) -> int:
        return self.image.width + self.padding * 2

    @property
    def packed_height(self) -> int:
        return self.image.height + self.padding * 2

    @property
    def area(self) -> int:
        return self.packed_width * self.packed_height


@dataclass(frozen=True)
class Rect:
    x: int
    y: int
    w: int
    h: int

    @property
    def right(self) -> int:
        return self.x + self.w

    @property
    def bottom(self) -> int:
        return self.y + self.h

    @property
    def area(self) -> int:
        return self.w * self.h

    def intersects(self, other: "Rect") -> bool:
        return (
            self.x < other.right
            and self.right > other.x
            and self.y < other.bottom
            and self.bottom > other.y
        )

    def contains(self, other: "Rect") -> bool:
        return (
            self.x <= other.x
            and self.y <= other.y
            and self.right >= other.right
            and self.bottom >= other.bottom
        )

    def to_dict(self) -> Dict[str, int]:
        return {"x": self.x, "y": self.y, "w": self.w, "h": self.h}


@dataclass
class Placement:
    sprite: PreparedSprite
    rect: Rect
    rotated: bool = False


@dataclass
class PackedSprite:
    name: str
    rel_path: str
    atlas_index: int
    x: int
    y: int
    w: int
    h: int
    source_w: int
    source_h: int
    offset_x: int
    offset_y: int
    trim_w: int
    trim_h: int
    rotated: bool
    trimmed: bool
    pack_x: int
    pack_y: int
    pack_w: int
    pack_h: int

    def to_dict(self) -> Dict[str, Any]:
        return {
            "name": self.name,
            "rel_path": self.rel_path,
            "atlas_index": self.atlas_index,
            "x": self.x,
            "y": self.y,
            "w": self.w,
            "h": self.h,
            "source_w": self.source_w,
            "source_h": self.source_h,
            "offset_x": self.offset_x,
            "offset_y": self.offset_y,
            "trim_w": self.trim_w,
            "trim_h": self.trim_h,
            "rotated": self.rotated,
            "trimmed": self.trimmed,
            "pack_x": self.pack_x,
            "pack_y": self.pack_y,
            "pack_w": self.pack_w,
            "pack_h": self.pack_h,
        }


@dataclass
class AtlasBuild:
    atlas_index: int
    group_name: str
    width: int
    height: int
    placements: List[Placement]
    usage: float


@dataclass
class AtlasResult:
    image_path: str
    tpsheet_path: str
    debug_json_path: Optional[str]
    width: int
    height: int
    usage: float
    sprites: List[PackedSprite]

    def to_dict(self) -> Dict[str, Any]:
        return {
            "image_path": self.image_path,
            "tpsheet_path": self.tpsheet_path,
            "debug_json_path": self.debug_json_path,
            "width": self.width,
            "height": self.height,
            "usage": self.usage,
            "sprites": [sprite.to_dict() for sprite in self.sprites],
        }


@dataclass
class LogMessage:
    level: str
    message: str

    def to_dict(self) -> Dict[str, str]:
        return {"level": self.level, "message": self.message}


@dataclass
class PackResult:
    atlases: List[AtlasResult]
    total_sprites: int
    total_atlases: int
    logs: List[LogMessage]

    def to_dict(self) -> Dict[str, Any]:
        return {
            "atlases": [atlas.to_dict() for atlas in self.atlases],
            "total_sprites": self.total_sprites,
            "total_atlases": self.total_atlases,
            "logs": [log.to_dict() for log in self.logs],
        }


def as_posix(path: Path) -> str:
    return path.as_posix()
