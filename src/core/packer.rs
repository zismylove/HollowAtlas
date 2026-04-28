use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use image::{imageops, ImageReader, Rgba, RgbaImage};
use rayon::prelude::*;

use crate::core::atlas_writer::{write_atlas, write_tpsheet};
use crate::core::extrude::extrude_image;
use crate::core::maxrects::MaxRectsPacker;
use crate::core::scanner::scan_folder;
use crate::core::trim::trim_transparent;
use crate::core::types::{
    AtlasBuild, AtlasResult, LogMessage, OutputFormat, PackConfig, PackResult, Placement,
    PreparedSprite, Rect, SourceImage, SplitMode,
};

pub fn pack_folder(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    config: PackConfig,
) -> Result<PackResult> {
    let plan = build_pack_plan(input_path, config)?;
    let output_path = output_path.as_ref();

    std::fs::create_dir_all(output_path)?;
    let mut results: Vec<AtlasResult> = Vec::new();
    let mut logs = plan.logs;
    let tpsheet_name = shared_tpsheet_name();

    for build in &plan.builds {
        let atlas_name = format!("atlas_{}", build.atlas_index);
        let result = write_atlas(build, output_path, &atlas_name, tpsheet_name, plan.config)?;
        logs.push(LogMessage::new(
            "success",
            format!(
                "Generated {}, usage {:.1}%.",
                Path::new(&result.image_path)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("atlas.png"),
                result.usage * 100.0
            ),
        ));
        results.push(result);
    }

    remove_legacy_tpsheets(output_path, tpsheet_name)?;
    let tpsheet_path = write_tpsheet(output_path, tpsheet_name, &results)?;
    let tpsheet_path_str = tpsheet_path.to_string_lossy().to_string();
    for result in &mut results {
        result.tpsheet_path = tpsheet_path_str.clone();
    }
    logs.push(LogMessage::new(
        "success",
        format!(
            "Generated {}.",
            tpsheet_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("atlas.tpsheet")
        ),
    ));

    logs.push(LogMessage::new("success", "Export complete."));
    Ok(PackResult {
        total_sprites: plan.total_sprites,
        total_atlases: results.len(),
        atlases: results,
        logs,
    })
}

pub fn preview_folder(input_path: impl AsRef<Path>, config: PackConfig) -> Result<PackResult> {
    let plan = build_pack_plan(input_path, config)?;
    let mut results: Vec<AtlasResult> = Vec::with_capacity(plan.builds.len());
    let preview_dir = preview_output_dir();
    let tpsheet_name = shared_tpsheet_name();

    std::fs::create_dir_all(&preview_dir)?;

    for build in &plan.builds {
        let atlas_name = format!("atlas_{}", build.atlas_index);
        results.push(write_atlas(
            build,
            &preview_dir,
            &atlas_name,
            tpsheet_name,
            plan.config,
        )?);
    }

    remove_legacy_tpsheets(&preview_dir, tpsheet_name)?;
    let tpsheet_path = write_tpsheet(&preview_dir, tpsheet_name, &results)?;
    let tpsheet_path_str = tpsheet_path.to_string_lossy().to_string();
    for result in &mut results {
        result.tpsheet_path = tpsheet_path_str.clone();
    }

    let mut logs = plan.logs;
    logs.push(LogMessage::new("success", "Preview ready."));
    Ok(PackResult {
        total_sprites: plan.total_sprites,
        total_atlases: results.len(),
        atlases: results,
        logs,
    })
}

fn preview_output_dir() -> std::path::PathBuf {
    std::env::temp_dir().join("hollowatlas_preview")
}

fn shared_tpsheet_name() -> &'static str {
    "atlas.tpsheet"
}

fn remove_legacy_tpsheets(output_dir: &Path, keep_name: &str) -> Result<()> {
    for entry in fs::read_dir(output_dir)? {
        let entry = entry?;
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        if file_name == keep_name {
            continue;
        }

        if file_name.starts_with("atlas_") && file_name.ends_with(".tpsheet") {
            fs::remove_file(path)?;
        }
    }

    Ok(())
}

struct PackPlan {
    config: PackConfig,
    builds: Vec<AtlasBuild>,
    logs: Vec<LogMessage>,
    total_sprites: usize,
}

fn build_pack_plan(input_path: impl AsRef<Path>, mut config: PackConfig) -> Result<PackPlan> {
    config = config.normalized();
    let input_path = input_path.as_ref();

    let mut logs = Vec::new();
    if config.allow_rotation && config.output_format == OutputFormat::GodotTpSheet {
        logs.push(LogMessage::new(
            "warning",
            "Godot TexturePacker Importer does not restore rotated sprites; rotation disabled for .tpsheet export.",
        ));
        config.allow_rotation = false;
    }
    if config.align_to_grid && config.allow_rotation {
        logs.push(LogMessage::new(
            "warning",
            format!(
                "Grid alignment mode keeps sprites on {}x{} cells; rotation disabled.",
                config.grid_cell_size, config.grid_cell_size
            ),
        ));
        config.allow_rotation = false;
    }

    logs.push(LogMessage::new(
        "info",
        format!("Scan folder: {}", input_path.display()),
    ));
    let scan = scan_folder(input_path)?;
    logs.extend(
        scan.warnings
            .iter()
            .map(|warning| LogMessage::new("warning", warning.clone())),
    );

    let readable: Vec<SourceImage> = scan
        .images
        .iter()
        .filter(|image| image.readable)
        .cloned()
        .collect();

    logs.push(LogMessage::new(
        "info",
        format!("Images found: {}", scan.total_images),
    ));
    logs.push(LogMessage::new(
        "info",
        format!("Readable images: {}", readable.len()),
    ));

    if readable.is_empty() {
        bail!("No readable images were found.");
    }

    logs.push(LogMessage::new(
        "info",
        "Prepare sprites: trim, extrude, padding.",
    ));
    let mut prepared = prepare_sprites(&readable, config, &mut logs)?;
    prepared.sort_by_key(|sprite| std::cmp::Reverse(required_pack_area(sprite, config)));

    if config.align_to_grid {
        logs.push(LogMessage::new(
            "info",
            format!(
                "Grid alignment enabled: sprites snap to {}x{} cells{}.",
                config.grid_cell_size,
                config.grid_cell_size,
                if config.slice_grid_cells {
                    "; transparent grid holes split the image into occupied grid regions"
                } else {
                    "; each source image stays a single sprite"
                }
            ),
        ));
    }

    let groups = split_groups(&prepared, config.split_mode);
    logs.push(LogMessage::new(
        "info",
        format!("Packing groups: {}", groups.len()),
    ));

    let mut builds: Vec<AtlasBuild> = Vec::new();
    let mut atlas_index = 0usize;

    for (group_name, sprites) in groups {
        logs.push(LogMessage::new(
            "info",
            format!("Pack group '{group_name}' with {} sprites.", sprites.len()),
        ));
        let group_builds = build_atlases_for_group(&group_name, &sprites, config, atlas_index)?;
        atlas_index += group_builds.len();
        builds.extend(group_builds);
    }

    Ok(PackPlan {
        config,
        builds,
        logs,
        total_sprites: prepared
            .iter()
            .map(|sprite| output_sprite_count(sprite, config))
            .sum(),
    })
}

pub fn prepare_sprites(
    sources: &[SourceImage],
    config: PackConfig,
    logs: &mut Vec<LogMessage>,
) -> Result<Vec<PreparedSprite>> {
    let prepared: Vec<Result<(Vec<PreparedSprite>, Vec<String>)>> = sources
        .par_iter()
        .map(|source| prepare_source_sprites(source, config))
        .collect();

    let mut sprites = Vec::new();
    for item in prepared {
        match item {
            Ok((mut prepared_sprites, warnings)) => {
                for warning in warnings {
                    logs.push(LogMessage::new("warning", warning));
                }
                sprites.append(&mut prepared_sprites);
            }
            Err(err) => logs.push(LogMessage::new("warning", err.to_string())),
        }
    }

    if config.align_to_grid && config.slice_grid_cells {
        logs.push(LogMessage::new(
            "info",
            format!(
                "Grid slicing kept {} occupied tile cells from {} source images.",
                sprites
                    .iter()
                    .map(|sprite| sprite.grid_slices.len())
                    .sum::<usize>(),
                sources.len()
            ),
        ));
    }

    if sprites.is_empty() {
        bail!("No images could be prepared for packing.");
    }

    Ok(sprites)
}

fn prepare_source_sprites(
    source: &SourceImage,
    config: PackConfig,
) -> Result<(Vec<PreparedSprite>, Vec<String>)> {
    let image = ImageReader::open(&source.abs_path)
        .with_context(|| format!("Failed to open {}", source.rel_path))?
        .decode()
        .with_context(|| format!("Failed to decode {}", source.rel_path))?
        .to_rgba8();

    if !config.align_to_grid {
        let (sprite, warning) = prepare_standard_sprite(source, image, config);
        return Ok((vec![sprite], warning.into_iter().collect()));
    }

    let (aligned_image, was_padded) = pad_image_to_grid(image, config.grid_cell_size);

    if config.slice_grid_cells {
        let (sprites, mut warnings) = prepare_grid_region_sprites(source, &aligned_image, config);
        if was_padded {
            warnings.insert(
                0,
                format!(
                    "{} padded to {}x{} for grid slicing.",
                    source.rel_path,
                    aligned_image.width(),
                    aligned_image.height()
                ),
            );
        }
        return Ok((sprites, warnings));
    }

    let (sprite, mut warnings) = prepare_grid_aligned_sprite(source, &aligned_image, config);
    if was_padded {
        warnings.insert(
            0,
            format!(
                "{} padded to {}x{} for grid alignment.",
                source.rel_path,
                aligned_image.width(),
                aligned_image.height()
            ),
        );
    }

    Ok((vec![sprite], warnings))
}

pub fn prepare_sprite(
    source: &SourceImage,
    config: PackConfig,
) -> Result<(PreparedSprite, Option<String>)> {
    let image = ImageReader::open(&source.abs_path)
        .with_context(|| format!("Failed to open {}", source.rel_path))?
        .decode()
        .with_context(|| format!("Failed to decode {}", source.rel_path))?
        .to_rgba8();

    Ok(prepare_standard_sprite(source, image, config))
}

fn prepare_standard_sprite(
    source: &SourceImage,
    image: RgbaImage,
    config: PackConfig,
) -> (PreparedSprite, Option<String>) {
    let trim = trim_transparent(&image, config.trim);
    let extruded = extrude_image(&trim.image, config.extrude);
    let warning = trim.fully_transparent.then(|| {
        format!(
            "{} is fully transparent; packed as a 1x1 transparent sprite.",
            source.rel_path
        )
    });

    (
        PreparedSprite {
            id: source.id,
            name: source.name.clone(),
            abs_path: source.abs_path.clone(),
            rel_path: source.rel_path.clone(),
            source_width: trim.source_width,
            source_height: trim.source_height,
            trim_x: trim.trim_x,
            trim_y: trim.trim_y,
            trim_width: trim.trim_width,
            trim_height: trim.trim_height,
            image: extruded,
            padding: config.padding,
            extrude: config.extrude,
            trimmed: trim.trimmed,
            grid_slices: Vec::new(),
        },
        warning,
    )
}

fn prepare_grid_aligned_sprite(
    source: &SourceImage,
    image: &RgbaImage,
    config: PackConfig,
) -> (PreparedSprite, Vec<String>) {
    let cell = config.grid_cell_size.max(1);
    let original_width = image.width();
    let original_height = image.height();

    let (crop_x, crop_y, crop_width, crop_height, trimmed, warning) =
        if let Some((min_col, min_row, max_col, max_row)) = occupied_grid_bounds(image, cell) {
            let crop_x = min_col * cell;
            let crop_y = min_row * cell;
            let crop_width = (max_col - min_col) * cell;
            let crop_height = (max_row - min_row) * cell;
            let trimmed = crop_x != 0
                || crop_y != 0
                || crop_width != original_width
                || crop_height != original_height;
            (crop_x, crop_y, crop_width, crop_height, trimmed, None)
        } else {
            (
                0,
                0,
                original_width,
                original_height,
                false,
                Some(format!(
                    "{} is fully transparent; kept original grid footprint.",
                    source.rel_path
                )),
            )
        };

    let cropped = imageops::crop_imm(image, crop_x, crop_y, crop_width, crop_height).to_image();
    let extruded = extrude_image(&cropped, config.extrude);

    (
        PreparedSprite {
            id: source.id,
            name: source.name.clone(),
            abs_path: source.abs_path.clone(),
            rel_path: source.rel_path.clone(),
            source_width: original_width,
            source_height: original_height,
            trim_x: crop_x,
            trim_y: crop_y,
            trim_width: crop_width,
            trim_height: crop_height,
            image: extruded,
            padding: config.padding,
            extrude: config.extrude,
            trimmed,
            grid_slices: Vec::new(),
        },
        warning.into_iter().collect(),
    )
}

fn prepare_grid_region_sprites(
    source: &SourceImage,
    image: &RgbaImage,
    config: PackConfig,
) -> (Vec<PreparedSprite>, Vec<String>) {
    let cell = config.grid_cell_size.max(1);
    let Some((min_col, min_row, max_col, max_row)) = occupied_grid_bounds(image, cell) else {
        return (
            Vec::new(),
            vec![format!(
                "{} is fully transparent; skipped in grid slicing mode.",
                source.rel_path
            )],
        );
    };

    let crop_x = min_col * cell;
    let crop_y = min_row * cell;
    let crop_width = (max_col - min_col) * cell;
    let crop_height = (max_row - min_row) * cell;
    let cropped = imageops::crop_imm(image, crop_x, crop_y, crop_width, crop_height).to_image();
    let extruded = extrude_image(&cropped, config.extrude);

    let mut grid_slices = Vec::new();
    for row in min_row..max_row {
        for column in min_col..max_col {
            if !cell_has_visible_pixels(image, column * cell, row * cell, cell) {
                continue;
            }

            grid_slices.push(crate::core::types::GridSlice {
                x: column - min_col,
                y: row - min_row,
                name: build_grid_cell_label(&source.name, row, column),
                rel_path: build_grid_cell_label(&source.rel_path, row, column),
            });
        }
    }

    let trimmed =
        crop_x != 0 || crop_y != 0 || crop_width != image.width() || crop_height != image.height();

    let sprite = PreparedSprite {
        id: source.id,
        name: source.name.clone(),
        abs_path: source.abs_path.clone(),
        rel_path: source.rel_path.clone(),
        source_width: image.width(),
        source_height: image.height(),
        trim_x: crop_x,
        trim_y: crop_y,
        trim_width: crop_width,
        trim_height: crop_height,
        image: extruded,
        padding: config.padding,
        extrude: config.extrude,
        trimmed,
        grid_slices,
    };

    let warnings = if sprite.grid_slices.is_empty() {
        vec![format!(
            "{} is fully transparent; skipped in grid slicing mode.",
            source.rel_path
        )]
    } else {
        Vec::new()
    };

    (vec![sprite], warnings)
}

fn pad_image_to_grid(image: RgbaImage, cell_size: u32) -> (RgbaImage, bool) {
    let cell = cell_size.max(1);
    let padded_width = align_up(image.width(), cell);
    let padded_height = align_up(image.height(), cell);

    if padded_width == image.width() && padded_height == image.height() {
        return (image, false);
    }

    let mut padded = RgbaImage::from_pixel(padded_width, padded_height, Rgba([0, 0, 0, 0]));
    for y in 0..image.height() {
        for x in 0..image.width() {
            padded.put_pixel(x, y, *image.get_pixel(x, y));
        }
    }

    (padded, true)
}

fn occupied_grid_bounds(image: &RgbaImage, cell_size: u32) -> Option<(u32, u32, u32, u32)> {
    let cell = cell_size.max(1);
    let columns = image.width().div_ceil(cell);
    let rows = image.height().div_ceil(cell);
    let mut min_col = columns;
    let mut min_row = rows;
    let mut max_col = 0;
    let mut max_row = 0;
    let mut found = false;

    for row in 0..rows {
        for column in 0..columns {
            let x = column * cell;
            let y = row * cell;
            if !cell_has_visible_pixels(image, x, y, cell) {
                continue;
            }

            found = true;
            min_col = min_col.min(column);
            min_row = min_row.min(row);
            max_col = max_col.max(column + 1);
            max_row = max_row.max(row + 1);
        }
    }

    found.then_some((min_col, min_row, max_col, max_row))
}

fn cell_has_visible_pixels(image: &RgbaImage, start_x: u32, start_y: u32, cell_size: u32) -> bool {
    let end_x = (start_x + cell_size).min(image.width());
    let end_y = (start_y + cell_size).min(image.height());

    for y in start_y..end_y {
        for x in start_x..end_x {
            if image.get_pixel(x, y).0[3] > 0 {
                return true;
            }
        }
    }

    false
}

fn build_grid_cell_label(label: &str, row: u32, column: u32) -> String {
    match label.rsplit_once('.') {
        Some((base, ext)) => format!("{base}__r{row}_c{column}.{ext}"),
        None => format!("{label}__r{row}_c{column}"),
    }
}

pub fn split_groups(
    sprites: &[PreparedSprite],
    split_mode: SplitMode,
) -> Vec<(String, Vec<PreparedSprite>)> {
    if split_mode == SplitMode::AllInOne {
        return vec![("all".to_string(), sprites.to_vec())];
    }

    let mut groups: BTreeMap<String, Vec<PreparedSprite>> = BTreeMap::new();
    for sprite in sprites {
        let group = sprite
            .rel_path
            .split_once('/')
            .map(|(first, _)| first.to_string())
            .unwrap_or_else(|| "_root".to_string());
        groups.entry(group).or_default().push(sprite.clone());
    }

    groups.into_iter().collect()
}

pub fn build_atlases_for_group(
    group_name: &str,
    sprites: &[PreparedSprite],
    config: PackConfig,
    start_index: usize,
) -> Result<Vec<AtlasBuild>> {
    let mut remaining = sprites.to_vec();
    remaining.sort_by_key(|sprite| std::cmp::Reverse(required_pack_area(sprite, config)));

    let mut builds = Vec::new();
    let mut local_index = 0usize;

    while !remaining.is_empty() {
        ensure_sprites_fit_max_size(&remaining, config)?;

        let (placements, bin_width, bin_height) = if let Some(full) =
            try_pack_all_smallest(&remaining, config)
        {
            remaining.clear();
            full
        } else {
            let (placements, leftover, bin_width, bin_height) =
                pack_partial(&remaining, config.max_size, config.max_size, config);
            if placements.is_empty() {
                let first = &remaining[0];
                bail!(
                    "Unable to pack sprite {} into {}x{}.",
                    first.rel_path,
                    config.max_size,
                    config.max_size
                );
            }
            remaining = leftover;
            remaining.sort_by_key(|sprite| std::cmp::Reverse(required_pack_area(sprite, config)));
            (placements, bin_width, bin_height)
        };

        let (width, height, usage) =
            finalize_atlas_size(&placements, bin_width, bin_height, config);
        builds.push(AtlasBuild {
            atlas_index: start_index + local_index,
            group_name: group_name.to_string(),
            width,
            height,
            placements,
            usage,
        });
        local_index += 1;
    }

    Ok(builds)
}

pub fn ensure_sprites_fit_max_size(sprites: &[PreparedSprite], config: PackConfig) -> Result<()> {
    for sprite in sprites {
        let (required_width, required_height) = required_pack_dimensions(sprite, config);
        let fits_normal = required_width <= config.max_size && required_height <= config.max_size;
        let fits_rotated = config.allow_rotation
            && required_height <= config.max_size
            && required_width <= config.max_size;

        if !(fits_normal || fits_rotated) {
            bail!(
                "Image {} is larger than max atlas size after padding/extrude: {}x{} > {}.",
                sprite.rel_path,
                required_width,
                required_height,
                config.max_size
            );
        }
    }

    Ok(())
}

pub fn try_pack_all_smallest(
    sprites: &[PreparedSprite],
    config: PackConfig,
) -> Option<(Vec<Placement>, u32, u32)> {
    for (width, height) in candidate_bins(config) {
        let (placements, leftover, _, _) = pack_partial(sprites, width, height, config);
        if leftover.is_empty() && !placements.is_empty() {
            return Some((placements, width, height));
        }
    }

    None
}

pub fn candidate_bins(config: PackConfig) -> Vec<(u32, u32)> {
    if config.align_to_grid {
        let cell = config.grid_cell_size.max(1);
        let max_cells = (config.max_size / cell).max(1);
        if !config.power_of_two {
            return (1..=max_cells)
                .map(|cells| (cells.saturating_mul(cell), cells.saturating_mul(cell)))
                .collect();
        }

        let mut sizes = Vec::new();
        let mut cells = 1u32;
        while cells <= max_cells {
            sizes.push((cells.saturating_mul(cell), cells.saturating_mul(cell)));
            cells = cells.saturating_mul(2);
            if cells == 0 {
                break;
            }
        }
        return sizes;
    }

    if !config.power_of_two {
        return vec![(config.max_size, config.max_size)];
    }

    let mut sizes: Vec<u32> = [64, 128, 256, 512, 1024, 2048, 4096, 8192]
        .into_iter()
        .filter(|size| *size <= config.max_size)
        .collect();
    if !sizes.contains(&config.max_size) {
        sizes.push(config.max_size);
    }
    sizes.sort_unstable();
    sizes.into_iter().map(|size| (size, size)).collect()
}

pub fn pack_partial(
    sprites: &[PreparedSprite],
    width: u32,
    height: u32,
    config: PackConfig,
) -> (Vec<Placement>, Vec<PreparedSprite>, u32, u32) {
    if config.align_to_grid && config.slice_grid_cells {
        return pack_partial_grid_shapes(sprites, width, height, config);
    }

    let mut packer = MaxRectsPacker::new(width, height);
    let mut placements = Vec::new();
    let mut leftover = Vec::new();

    for sprite in sprites {
        let (required_width, required_height) = required_pack_dimensions(sprite, config);
        match packer.insert(
            sprite.id,
            required_width,
            required_height,
            config.allow_rotation,
        ) {
            Some(result) => placements.push(Placement {
                sprite: sprite.clone(),
                rect: result.rect,
                rotated: result.rotated,
            }),
            None => leftover.push(sprite.clone()),
        }
    }

    (placements, leftover, width, height)
}

fn pack_partial_grid_shapes(
    sprites: &[PreparedSprite],
    width: u32,
    height: u32,
    config: PackConfig,
) -> (Vec<Placement>, Vec<PreparedSprite>, u32, u32) {
    let cell = config.grid_cell_size.max(1);
    let columns = (width / cell).max(1);
    let rows = (height / cell).max(1);
    let mut occupied = vec![false; (columns * rows) as usize];
    let mut placements = Vec::new();
    let mut leftover = Vec::new();
    let mut used_right = 0u32;
    let mut used_bottom = 0u32;

    for sprite in sprites {
        let (bbox_width, bbox_height) = required_pack_dimensions(sprite, config);
        let bbox_columns = bbox_width.div_ceil(cell).max(1);
        let bbox_rows = bbox_height.div_ceil(cell).max(1);

        let Some((cell_x, cell_y)) = find_grid_shape_position(
            sprite,
            &occupied,
            columns,
            rows,
            bbox_columns,
            bbox_rows,
            used_right,
            used_bottom,
        ) else {
            leftover.push(sprite.clone());
            continue;
        };

        mark_grid_shape_occupied(sprite, &mut occupied, columns, cell_x, cell_y);
        used_right = used_right.max(cell_x + bbox_columns);
        used_bottom = used_bottom.max(cell_y + bbox_rows);
        placements.push(Placement {
            sprite: sprite.clone(),
            rect: Rect {
                x: cell_x * cell,
                y: cell_y * cell,
                w: bbox_columns * cell,
                h: bbox_rows * cell,
            },
            rotated: false,
        });
    }

    (placements, leftover, width, height)
}

fn find_grid_shape_position(
    sprite: &PreparedSprite,
    occupied: &[bool],
    atlas_columns: u32,
    atlas_rows: u32,
    bbox_columns: u32,
    bbox_rows: u32,
    used_right: u32,
    used_bottom: u32,
) -> Option<(u32, u32)> {
    if bbox_columns > atlas_columns || bbox_rows > atlas_rows {
        return None;
    }

    let mut best: Option<(u32, u32, u64, u32, u32)> = None;
    for y in 0..=atlas_rows - bbox_rows {
        for x in 0..=atlas_columns - bbox_columns {
            if !grid_shape_fits(sprite, occupied, atlas_columns, x, y) {
                continue;
            }

            let right = used_right.max(x + bbox_columns);
            let bottom = used_bottom.max(y + bbox_rows);
            let area = right as u64 * bottom as u64;

            let candidate = (x, y, area, bottom, right);
            match best {
                None => best = Some(candidate),
                Some((best_x, best_y, best_area, best_bottom, best_right)) => {
                    if area < best_area
                        || (area == best_area && bottom < best_bottom)
                        || (area == best_area && bottom == best_bottom && right < best_right)
                        || (area == best_area
                            && bottom == best_bottom
                            && right == best_right
                            && (y < best_y || (y == best_y && x < best_x)))
                    {
                        best = Some(candidate);
                    }
                }
            }
        }
    }

    best.map(|(x, y, _, _, _)| (x, y))
}

fn grid_shape_fits(
    sprite: &PreparedSprite,
    occupied: &[bool],
    atlas_columns: u32,
    x: u32,
    y: u32,
) -> bool {
    let slices = if sprite.grid_slices.is_empty() {
        return false;
    } else {
        &sprite.grid_slices
    };

    slices.iter().all(|slice| {
        let atlas_x = x + slice.x;
        let atlas_y = y + slice.y;
        let index = (atlas_y * atlas_columns + atlas_x) as usize;
        !occupied[index]
    })
}

fn mark_grid_shape_occupied(
    sprite: &PreparedSprite,
    occupied: &mut [bool],
    atlas_columns: u32,
    x: u32,
    y: u32,
) {
    for slice in &sprite.grid_slices {
        let atlas_x = x + slice.x;
        let atlas_y = y + slice.y;
        let index = (atlas_y * atlas_columns + atlas_x) as usize;
        occupied[index] = true;
    }
}

pub fn finalize_atlas_size(
    placements: &[Placement],
    bin_width: u32,
    bin_height: u32,
    config: PackConfig,
) -> (u32, u32, f32) {
    let used_width = placements
        .iter()
        .map(|placement| placement.rect.right())
        .max()
        .unwrap_or(1);
    let used_height = placements
        .iter()
        .map(|placement| placement.rect.bottom())
        .max()
        .unwrap_or(1);

    let mut width = if config.align_to_grid {
        let cell = config.grid_cell_size.max(1);
        let cells = used_width.div_ceil(cell).max(1);
        if config.power_of_two {
            next_power_of_two(cells).saturating_mul(cell)
        } else {
            cells.saturating_mul(cell)
        }
    } else if config.power_of_two {
        next_power_of_two(used_width)
    } else {
        used_width
    };
    let mut height = if config.align_to_grid {
        let cell = config.grid_cell_size.max(1);
        let cells = used_height.div_ceil(cell).max(1);
        if config.power_of_two {
            next_power_of_two(cells).saturating_mul(cell)
        } else {
            cells.saturating_mul(cell)
        }
    } else if config.power_of_two {
        next_power_of_two(used_height)
    } else {
        used_height
    };

    if config.square {
        let mut side = width.max(height);
        if config.align_to_grid {
            let cell = config.grid_cell_size.max(1);
            let side_cells = side.div_ceil(cell).max(1);
            side = if config.power_of_two {
                next_power_of_two(side_cells).saturating_mul(cell)
            } else {
                side_cells.saturating_mul(cell)
            };
        } else if config.power_of_two {
            side = next_power_of_two(side);
        }
        width = side;
        height = side;
    }

    width = width.clamp(1, bin_width);
    height = height.clamp(1, bin_height);

    let content_area: u64 = placements
        .iter()
        .map(|placement| {
            if config.align_to_grid
                && config.slice_grid_cells
                && !placement.sprite.grid_slices.is_empty()
            {
                let cell = config.grid_cell_size.max(1) as u64;
                placement.sprite.grid_slices.len() as u64 * cell * cell
            } else {
                placement.sprite.trim_width as u64 * placement.sprite.trim_height as u64
            }
        })
        .sum();
    let usage = if width > 0 && height > 0 {
        content_area as f32 / (width as f32 * height as f32)
    } else {
        0.0
    };

    (width, height, usage)
}

pub fn next_power_of_two(value: u32) -> u32 {
    value.max(1).next_power_of_two()
}

fn required_pack_area(sprite: &PreparedSprite, config: PackConfig) -> u64 {
    if config.align_to_grid && config.slice_grid_cells && !sprite.grid_slices.is_empty() {
        let cell = config.grid_cell_size.max(1) as u64;
        return sprite.grid_slices.len() as u64 * cell * cell;
    }
    let (width, height) = required_pack_dimensions(sprite, config);
    width as u64 * height as u64
}

fn output_sprite_count(sprite: &PreparedSprite, config: PackConfig) -> usize {
    if config.align_to_grid && config.slice_grid_cells && !sprite.grid_slices.is_empty() {
        sprite.grid_slices.len()
    } else {
        1
    }
}

fn required_pack_dimensions(sprite: &PreparedSprite, config: PackConfig) -> (u32, u32) {
    if !config.align_to_grid {
        return (sprite.packed_width(), sprite.packed_height());
    }

    let cell = config.grid_cell_size.max(1);
    (
        align_up(sprite.packed_width(), cell),
        align_up(sprite.packed_height(), cell),
    )
}

fn align_up(value: u32, step: u32) -> u32 {
    let step = step.max(1);
    value.max(1).div_ceil(step).saturating_mul(step)
}

#[allow(dead_code)]
fn _rect_from_placement(placement: &Placement) -> Rect {
    placement.rect
}
