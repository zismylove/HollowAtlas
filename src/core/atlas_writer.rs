use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use image::{imageops, RgbaImage};
use serde_json::json;

use crate::core::manifest::build_godot_tpsheet;
use crate::core::types::{AtlasBuild, AtlasResult, PackConfig, PackedSprite, Placement};

pub fn write_atlas(
    build: &AtlasBuild,
    output_dir: impl AsRef<Path>,
    atlas_name: &str,
    tpsheet_name: &str,
    config: PackConfig,
) -> Result<AtlasResult> {
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir)?;

    let png_path = output_dir.join(format!("{atlas_name}.png"));
    let tpsheet_path = output_dir.join(tpsheet_name);
    let debug_path = config
        .debug_json
        .then(|| output_dir.join(format!("{atlas_name}.debug.json")));

    let (atlas_image, packed_sprites) = compose_atlas(build, config);

    atlas_image.save(&png_path)?;

    if let Some(debug_path) = &debug_path {
        let debug = json!({
            "config": config,
            "atlas": {
                "name": atlas_name,
                "image": png_path.file_name().and_then(|name| name.to_str()).unwrap_or("atlas.png"),
                "tpsheet": tpsheet_name,
                "width": build.width,
                "height": build.height,
                "usage": build.usage,
                "group": build.group_name
            },
            "sprites": packed_sprites
        });
        fs::write(debug_path, serde_json::to_string_pretty(&debug)?)?;
    }

    Ok(AtlasResult {
        image_path: path_string(&png_path),
        tpsheet_path: path_string(&tpsheet_path),
        debug_json_path: debug_path.as_ref().map(|path| path_string(path)),
        image_data_url: None,
        width: build.width,
        height: build.height,
        usage: build.usage,
        sprites: packed_sprites,
    })
}

pub fn write_tpsheet(
    output_dir: impl AsRef<Path>,
    tpsheet_name: &str,
    atlases: &[AtlasResult],
) -> Result<PathBuf> {
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir)?;

    let tpsheet_path = output_dir.join(tpsheet_name);
    let manifest = build_godot_tpsheet(atlases);
    fs::write(&tpsheet_path, serde_json::to_string_pretty(&manifest)?)?;
    Ok(tpsheet_path)
}

fn compose_atlas(build: &AtlasBuild, config: PackConfig) -> (RgbaImage, Vec<PackedSprite>) {
    let mut atlas_image = RgbaImage::new(build.width, build.height);
    let mut packed_sprites = Vec::with_capacity(build.placements.len());

    for placement in &build.placements {
        let sprite = &placement.sprite;
        let draw_image = image_for_placement(placement);
        let paste_x = placement.rect.x + config.padding;
        let paste_y = placement.rect.y + config.padding;
        copy_rgba(&draw_image, &mut atlas_image, paste_x, paste_y);

        if config.align_to_grid && config.slice_grid_cells && !sprite.grid_slices.is_empty() {
            let cell = config.grid_cell_size.max(1);
            let frame_origin_x = paste_x + config.extrude;
            let frame_origin_y = paste_y + config.extrude;

            for slice in &sprite.grid_slices {
                packed_sprites.push(PackedSprite {
                    name: slice.name.clone(),
                    rel_path: slice.rel_path.clone(),
                    atlas_index: build.atlas_index,
                    x: frame_origin_x + slice.x * cell,
                    y: frame_origin_y + slice.y * cell,
                    w: cell,
                    h: cell,
                    source_w: cell,
                    source_h: cell,
                    offset_x: 0,
                    offset_y: 0,
                    trim_w: cell,
                    trim_h: cell,
                    rotated: false,
                    trimmed: false,
                    pack_x: placement.rect.x + slice.x * cell,
                    pack_y: placement.rect.y + slice.y * cell,
                    pack_w: cell,
                    pack_h: cell,
                });
            }
            continue;
        }

        let frame_x = paste_x + config.extrude;
        let frame_y = paste_y + config.extrude;
        let frame_w = if placement.rotated {
            sprite.trim_height
        } else {
            sprite.trim_width
        };
        let frame_h = if placement.rotated {
            sprite.trim_width
        } else {
            sprite.trim_height
        };

        packed_sprites.push(PackedSprite {
            name: sprite.name.clone(),
            rel_path: sprite.rel_path.clone(),
            atlas_index: build.atlas_index,
            x: frame_x,
            y: frame_y,
            w: frame_w,
            h: frame_h,
            source_w: sprite.source_width,
            source_h: sprite.source_height,
            offset_x: sprite.trim_x,
            offset_y: sprite.trim_y,
            trim_w: sprite.trim_width,
            trim_h: sprite.trim_height,
            rotated: placement.rotated,
            trimmed: sprite.trimmed,
            pack_x: placement.rect.x,
            pack_y: placement.rect.y,
            pack_w: placement.rect.w,
            pack_h: placement.rect.h,
        });
    }

    (atlas_image, packed_sprites)
}

fn image_for_placement(placement: &Placement) -> RgbaImage {
    if placement.rotated {
        imageops::rotate90(&placement.sprite.image)
    } else {
        placement.sprite.image.clone()
    }
}

fn copy_rgba(source: &RgbaImage, target: &mut RgbaImage, x: u32, y: u32) {
    for source_y in 0..source.height() {
        for source_x in 0..source.width() {
            let target_x = x + source_x;
            let target_y = y + source_y;
            if target_x < target.width() && target_y < target.height() {
                let pixel = *source.get_pixel(source_x, source_y);
                if pixel.0[3] > 0 {
                    target.put_pixel(target_x, target_y, pixel);
                }
            }
        }
    }
}

fn path_string(path: &PathBuf) -> String {
    path.to_string_lossy().to_string()
}
