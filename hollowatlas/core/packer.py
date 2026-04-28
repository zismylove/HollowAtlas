from __future__ import annotations

from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from typing import Dict, Iterable, List, Sequence, Tuple

from PIL import Image

from .atlas_writer import write_atlas, write_tpsheet
from .extrude import extrude_image
from .maxrects import MaxRectsPacker
from .scanner import scan_folder
from .trim import trim_transparent
from .types import (
    AtlasBuild,
    LogMessage,
    OutputFormat,
    PackConfig,
    PackResult,
    Placement,
    PreparedSprite,
    Rect,
    SourceImage,
    SplitMode,
)


def pack_folder(input_path: str | Path, output_path: str | Path, config: PackConfig | None = None) -> PackResult:
    config = (config or PackConfig()).normalized()
    input_root = Path(input_path).expanduser().resolve()
    output_root = Path(output_path).expanduser().resolve()

    logs: List[LogMessage] = []
    if config.allow_rotation and config.output_format == OutputFormat.GODOT_TPSHEET:
        logs.append(
            LogMessage(
                "warning",
                "Godot TexturePacker Importer does not restore rotated sprites; rotation disabled for .tpsheet export.",
            )
        )
        config = PackConfig(
            max_size=config.max_size,
            padding=config.padding,
            extrude=config.extrude,
            trim=config.trim,
            allow_rotation=False,
            power_of_two=config.power_of_two,
            square=config.square,
            split_mode=config.split_mode,
            output_format=config.output_format,
            debug_json=config.debug_json,
        )

    logs.append(LogMessage("info", f"Scan folder: {input_root}"))
    scan_result = scan_folder(input_root)
    logs.extend(LogMessage("warning", warning) for warning in scan_result.warnings)

    readable = [image for image in scan_result.images if image.readable]
    logs.append(LogMessage("info", f"Images found: {scan_result.total_images}"))
    logs.append(LogMessage("info", f"Readable images: {len(readable)}"))
    if not readable:
        raise ValueError("No readable images were found.")

    logs.append(LogMessage("info", "Prepare sprites: trim, extrude, padding."))
    prepared = prepare_sprites(readable, config, logs)
    prepared.sort(key=lambda sprite: sprite.area, reverse=True)

    groups = split_groups(prepared, config.split_mode)
    logs.append(LogMessage("info", f"Packing groups: {len(groups)}"))

    output_root.mkdir(parents=True, exist_ok=True)
    all_results = []
    atlas_index = 0
    tpsheet_name = shared_tpsheet_name()
    for group_name, sprites in groups:
        logs.append(LogMessage("info", f"Pack group '{group_name}' with {len(sprites)} sprites."))
        builds = build_atlases_for_group(group_name, sprites, config, start_index=atlas_index)
        for build in builds:
            atlas_name = f"atlas_{build.atlas_index}"
            result = write_atlas(build, output_root, atlas_name, tpsheet_name, config)
            all_results.append(result)
            logs.append(
                LogMessage(
                    "success",
                    f"Generated {Path(result.image_path).name}, usage {result.usage * 100:.1f}%.",
                )
            )
        atlas_index += len(builds)

    remove_legacy_tpsheets(output_root, tpsheet_name)
    tpsheet_path = write_tpsheet(output_root, tpsheet_name, all_results)
    logs.append(LogMessage("success", f"Generated {tpsheet_path.name}."))

    logs.append(LogMessage("success", "Export complete."))
    return PackResult(
        atlases=all_results,
        total_sprites=len(prepared),
        total_atlases=len(all_results),
        logs=logs,
    )


def shared_tpsheet_name() -> str:
    return "atlas.tpsheet"


def remove_legacy_tpsheets(output_root: Path, keep_name: str) -> None:
    for path in output_root.glob("atlas_*.tpsheet"):
        if path.name != keep_name and path.is_file():
            path.unlink()


def prepare_sprites(sources: Sequence[SourceImage], config: PackConfig, logs: List[LogMessage]) -> List[PreparedSprite]:
    workers = min(32, max(1, len(sources)))
    prepared: List[PreparedSprite] = []

    with ThreadPoolExecutor(max_workers=workers) as executor:
        futures = {executor.submit(prepare_sprite, source, config): source for source in sources}
        for future in as_completed(futures):
            source = futures[future]
            try:
                sprite, warning = future.result()
            except Exception as exc:
                logs.append(LogMessage("warning", f"Failed to prepare {source.rel_path}: {exc}"))
                continue
            if warning:
                logs.append(LogMessage("warning", warning))
            prepared.append(sprite)

    if not prepared:
        raise ValueError("No images could be prepared for packing.")
    return prepared


def prepare_sprite(source: SourceImage, config: PackConfig) -> Tuple[PreparedSprite, str | None]:
    with Image.open(source.abs_path) as image:
        trim = trim_transparent(image, config.trim)

    extruded = extrude_image(trim.image, config.extrude)
    warning = None
    if trim.fully_transparent:
        warning = f"{source.rel_path} is fully transparent; packed as a 1x1 transparent sprite."

    return (
        PreparedSprite(
            id=source.id,
            name=source.name,
            abs_path=source.abs_path,
            rel_path=source.rel_path,
            source_width=trim.source_width,
            source_height=trim.source_height,
            trim_x=trim.trim_x,
            trim_y=trim.trim_y,
            trim_width=trim.trim_width,
            trim_height=trim.trim_height,
            image=extruded,
            padding=config.padding,
            extrude=config.extrude,
            trimmed=trim.trimmed,
        ),
        warning,
    )


def split_groups(sprites: Sequence[PreparedSprite], split_mode: SplitMode) -> List[Tuple[str, List[PreparedSprite]]]:
    if split_mode == SplitMode.ALL_IN_ONE:
        return [("all", list(sprites))]

    groups: Dict[str, List[PreparedSprite]] = defaultdict(list)
    for sprite in sprites:
        first = sprite.rel_path.split("/", 1)[0]
        group = first if "/" in sprite.rel_path else "_root"
        groups[group].append(sprite)
    return sorted(groups.items(), key=lambda item: item[0].lower())


def build_atlases_for_group(
    group_name: str,
    sprites: Sequence[PreparedSprite],
    config: PackConfig,
    start_index: int = 0,
) -> List[AtlasBuild]:
    remaining = sorted(sprites, key=lambda sprite: sprite.area, reverse=True)
    builds: List[AtlasBuild] = []
    local_index = 0

    while remaining:
        ensure_sprites_fit_max_size(remaining, config)
        full = try_pack_all_smallest(remaining, config)
        if full is not None:
            placements, bin_width, bin_height = full
            remaining = []
        else:
            placements, leftover, bin_width, bin_height = pack_partial(remaining, config.max_size, config.max_size, config)
            if not placements:
                first = remaining[0]
                raise ValueError(
                    f"Unable to pack sprite {first.rel_path} into {config.max_size}x{config.max_size}."
                )
            remaining = sorted(leftover, key=lambda sprite: sprite.area, reverse=True)

        width, height, usage = finalize_atlas_size(placements, bin_width, bin_height, config)
        builds.append(
            AtlasBuild(
                atlas_index=start_index + local_index,
                group_name=group_name,
                width=width,
                height=height,
                placements=placements,
                usage=usage,
            )
        )
        local_index += 1

    return builds


def ensure_sprites_fit_max_size(sprites: Iterable[PreparedSprite], config: PackConfig) -> None:
    for sprite in sprites:
        fits_normal = sprite.packed_width <= config.max_size and sprite.packed_height <= config.max_size
        fits_rotated = (
            config.allow_rotation
            and sprite.packed_height <= config.max_size
            and sprite.packed_width <= config.max_size
        )
        if not (fits_normal or fits_rotated):
            raise ValueError(
                f"Image {sprite.rel_path} is larger than max atlas size after padding/extrude: "
                f"{sprite.packed_width}x{sprite.packed_height} > {config.max_size}."
            )


def try_pack_all_smallest(
    sprites: Sequence[PreparedSprite],
    config: PackConfig,
) -> Tuple[List[Placement], int, int] | None:
    for width, height in candidate_bins(config):
        placements, leftover, _, _ = pack_partial(sprites, width, height, config)
        if not leftover and placements:
            return placements, width, height
    return None


def candidate_bins(config: PackConfig) -> List[Tuple[int, int]]:
    if not config.power_of_two:
        return [(config.max_size, config.max_size)]

    sizes = [64, 128, 256, 512, 1024, 2048, 4096, 8192]
    candidates = [size for size in sizes if size <= config.max_size]
    if config.max_size not in candidates:
        candidates.append(config.max_size)
    candidates.sort()

    if config.square:
        return [(size, size) for size in candidates]
    return [(size, size) for size in candidates]


def pack_partial(
    sprites: Sequence[PreparedSprite],
    width: int,
    height: int,
    config: PackConfig,
) -> Tuple[List[Placement], List[PreparedSprite], int, int]:
    packer = MaxRectsPacker(width, height)
    placements: List[Placement] = []
    leftover: List[PreparedSprite] = []

    for sprite in sprites:
        result = packer.insert(sprite.id, sprite.packed_width, sprite.packed_height, config.allow_rotation)
        if result is None:
            leftover.append(sprite)
            continue
        placements.append(Placement(sprite=sprite, rect=result.rect, rotated=result.rotated))

    return placements, leftover, width, height


def finalize_atlas_size(
    placements: Sequence[Placement],
    bin_width: int,
    bin_height: int,
    config: PackConfig,
) -> Tuple[int, int, float]:
    used_width = max((placement.rect.right for placement in placements), default=1)
    used_height = max((placement.rect.bottom for placement in placements), default=1)

    if config.power_of_two:
        width = next_power_of_two(used_width)
        height = next_power_of_two(used_height)
    else:
        width = used_width
        height = used_height

    if config.square:
        side = max(width, height)
        if config.power_of_two:
            side = next_power_of_two(side)
        width = height = side

    width = min(max(1, width), bin_width)
    height = min(max(1, height), bin_height)
    content_area = sum(placement.sprite.trim_width * placement.sprite.trim_height for placement in placements)
    usage = content_area / float(width * height) if width and height else 0.0
    return width, height, usage


def next_power_of_two(value: int) -> int:
    value = max(1, int(value))
    return 1 << (value - 1).bit_length()
