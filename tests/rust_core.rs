use std::fs;
use std::path::Path;

use hollowatlas::core::extrude::extrude_image;
use hollowatlas::core::maxrects::MaxRectsPacker;
use hollowatlas::core::packer::{pack_folder, preview_folder};
use hollowatlas::core::trim::trim_transparent;
use hollowatlas::core::types::{OutputFormat, PackConfig, SplitMode};
use image::{Rgba, RgbaImage};

#[test]
fn trim_transparent_tracks_source_offsets() {
    let mut image = RgbaImage::from_pixel(6, 5, Rgba([0, 0, 0, 0]));
    image.put_pixel(2, 1, Rgba([255, 0, 0, 255]));
    image.put_pixel(4, 3, Rgba([0, 255, 0, 255]));

    let result = trim_transparent(&image, true);

    assert_eq!(result.trim_x, 2);
    assert_eq!(result.trim_y, 1);
    assert_eq!(result.trim_width, 3);
    assert_eq!(result.trim_height, 3);
    assert_eq!(result.image.dimensions(), (3, 3));
    assert!(result.trimmed);
}

#[test]
fn fully_transparent_image_becomes_one_pixel_sprite() {
    let image = RgbaImage::from_pixel(4, 4, Rgba([0, 0, 0, 0]));
    let result = trim_transparent(&image, true);

    assert_eq!(result.image.dimensions(), (1, 1));
    assert!(result.fully_transparent);
    assert!(result.trimmed);
}

#[test]
fn extrude_replicates_edge_pixels() {
    let mut image = RgbaImage::from_pixel(2, 2, Rgba([0, 0, 0, 0]));
    image.put_pixel(0, 0, Rgba([10, 0, 0, 255]));
    image.put_pixel(1, 0, Rgba([20, 0, 0, 255]));
    image.put_pixel(0, 1, Rgba([30, 0, 0, 255]));
    image.put_pixel(1, 1, Rgba([40, 0, 0, 255]));

    let out = extrude_image(&image, 1);

    assert_eq!(out.dimensions(), (4, 4));
    assert_eq!(*out.get_pixel(0, 0), Rgba([10, 0, 0, 255]));
    assert_eq!(*out.get_pixel(3, 0), Rgba([20, 0, 0, 255]));
    assert_eq!(*out.get_pixel(0, 3), Rgba([30, 0, 0, 255]));
    assert_eq!(*out.get_pixel(3, 3), Rgba([40, 0, 0, 255]));
}

#[test]
fn maxrects_places_without_overlap() {
    let mut packer = MaxRectsPacker::new(64, 64);
    let placed = vec![
        packer.insert(0, 32, 32, false).unwrap(),
        packer.insert(1, 16, 16, false).unwrap(),
        packer.insert(2, 20, 12, false).unwrap(),
        packer.insert(3, 8, 40, false).unwrap(),
    ];

    for (index, item) in placed.iter().enumerate() {
        assert!(item.rect.right() <= 64);
        assert!(item.rect.bottom() <= 64);
        for other in placed.iter().skip(index + 1) {
            assert!(!item.rect.intersects(other.rect));
        }
    }
}

#[test]
fn pack_folder_writes_png_and_tpsheet() {
    let base = Path::new("build/rust_tests/pack_folder");
    let input_dir = base.join("input");
    let output_dir = base.join("output");
    fs::create_dir_all(input_dir.join("characters")).unwrap();
    fs::create_dir_all(&output_dir).unwrap();

    let mut hero = RgbaImage::from_pixel(16, 16, Rgba([0, 0, 0, 0]));
    for x in 4..12 {
        for y in 3..13 {
            hero.put_pixel(x, y, Rgba([255, 0, 0, 255]));
        }
    }
    hero.save(input_dir.join("characters/hero.png")).unwrap();
    RgbaImage::from_pixel(8, 8, Rgba([0, 0, 255, 255]))
        .save(input_dir.join("icon.png"))
        .unwrap();

    let result = pack_folder(
        &input_dir,
        &output_dir,
        PackConfig {
            max_size: 128,
            padding: 2,
            extrude: 1,
            trim: true,
            debug_json: true,
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(result.total_sprites, 2);
    assert_eq!(result.total_atlases, 1);
    let atlas = &result.atlases[0];
    assert!(Path::new(&atlas.image_path).exists());
    assert!(Path::new(&atlas.tpsheet_path).exists());
    assert_eq!(
        Path::new(&atlas.tpsheet_path).file_name().unwrap(),
        "atlas.tpsheet"
    );
    assert!(atlas
        .debug_json_path
        .as_ref()
        .map(Path::new)
        .unwrap()
        .exists());

    let manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&atlas.tpsheet_path).unwrap()).unwrap();
    let textures = manifest["textures"].as_array().unwrap();
    assert_eq!(textures.len(), 1);
    let texture = &textures[0];
    assert_eq!(texture["image"], "atlas_0.png");
    assert_eq!(texture["format"], "RGBA8888");
    let sprites = texture["sprites"].as_array().unwrap();
    let hero_frame = sprites
        .iter()
        .find(|sprite| sprite["filename"] == "characters/hero.png")
        .unwrap();
    assert_eq!(hero_frame["trimmed"], true);
    assert_eq!(hero_frame["spriteSourceSize"]["x"], 4);
    assert_eq!(hero_frame["spriteSourceSize"]["y"], 3);
    assert_eq!(
        hero_frame["sourceSize"],
        serde_json::json!({"w": 16, "h": 16})
    );
    assert_eq!(
        hero_frame["margin"],
        serde_json::json!({"x": 4, "y": 3, "w": 8, "h": 6})
    );
}

#[test]
fn pack_folder_splits_multiple_atlases_when_needed() {
    let base = Path::new("build/rust_tests/multi_atlas");
    let input_dir = base.join("input");
    let output_dir = base.join("output");
    fs::create_dir_all(&input_dir).unwrap();
    fs::create_dir_all(&output_dir).unwrap();

    for index in 0..5 {
        RgbaImage::from_pixel(32, 32, Rgba([index * 20, 0, 200, 255]))
            .save(input_dir.join(format!("sprite_{index}.png")))
            .unwrap();
    }

    let result = pack_folder(
        &input_dir,
        &output_dir,
        PackConfig {
            max_size: 64,
            padding: 0,
            extrude: 0,
            trim: false,
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(result.total_sprites, 5);
    assert_eq!(result.total_atlases, 2);
    assert_eq!(
        result.atlases[0].tpsheet_path,
        result.atlases[1].tpsheet_path
    );
    let manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&result.atlases[0].tpsheet_path).unwrap())
            .unwrap();
    assert_eq!(manifest["textures"].as_array().unwrap().len(), 2);
}

#[test]
fn pack_folder_by_first_level_folder_creates_grouped_atlases() {
    let base = Path::new("build/rust_tests/by_first_level");
    let input_dir = base.join("input");
    let output_dir = base.join("output");
    fs::create_dir_all(input_dir.join("characters")).unwrap();
    fs::create_dir_all(input_dir.join("ui")).unwrap();
    fs::create_dir_all(&output_dir).unwrap();

    RgbaImage::from_pixel(8, 8, Rgba([255, 0, 0, 255]))
        .save(input_dir.join("characters/hero.png"))
        .unwrap();
    RgbaImage::from_pixel(8, 8, Rgba([0, 255, 0, 255]))
        .save(input_dir.join("ui/button.png"))
        .unwrap();

    let result = pack_folder(
        &input_dir,
        &output_dir,
        PackConfig {
            max_size: 64,
            padding: 0,
            extrude: 0,
            trim: false,
            split_mode: SplitMode::ByFirstLevelFolder,
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(result.total_sprites, 2);
    assert_eq!(result.total_atlases, 2);
    assert_eq!(
        result
            .atlases
            .iter()
            .map(|atlas| atlas.sprites.len())
            .collect::<Vec<_>>(),
        vec![1, 1]
    );
    assert_eq!(
        result.atlases[0].tpsheet_path,
        result.atlases[1].tpsheet_path
    );
}

#[test]
fn godot_tpsheet_disables_rotation_for_importer_safety() {
    let base = Path::new("build/rust_tests/rotation_guard");
    let input_dir = base.join("input");
    let output_dir = base.join("output");
    fs::create_dir_all(&input_dir).unwrap();
    fs::create_dir_all(&output_dir).unwrap();

    RgbaImage::from_pixel(16, 32, Rgba([255, 0, 255, 255]))
        .save(input_dir.join("tall.png"))
        .unwrap();

    let result = pack_folder(
        &input_dir,
        &output_dir,
        PackConfig {
            max_size: 64,
            padding: 0,
            extrude: 0,
            trim: false,
            allow_rotation: true,
            ..Default::default()
        },
    )
    .unwrap();

    assert!(result
        .logs
        .iter()
        .any(|log| log.message.contains("rotation disabled")));
    assert!(!result.atlases[0].sprites[0].rotated);
}

#[test]
fn output_format_serde_accepts_ui_and_legacy_values() {
    let parsed: OutputFormat = serde_json::from_str("\"godot_tpsheet\"").unwrap();
    assert_eq!(parsed, OutputFormat::GodotTpSheet);

    let legacy: OutputFormat = serde_json::from_str("\"godot_tp_sheet\"").unwrap();
    assert_eq!(legacy, OutputFormat::GodotTpSheet);

    assert_eq!(
        serde_json::to_string(&OutputFormat::GodotTpSheet).unwrap(),
        "\"godot_tpsheet\""
    );
}

#[test]
fn grid_alignment_snaps_tiles_to_cell_boundaries() {
    let base = Path::new("build/rust_tests/grid_alignment");
    let input_dir = base.join("input");
    let output_dir = base.join("output");
    fs::create_dir_all(&input_dir).unwrap();
    fs::create_dir_all(&output_dir).unwrap();

    for index in 0..3 {
        RgbaImage::from_pixel(48, 48, Rgba([index as u8 * 40, 120, 200, 255]))
            .save(input_dir.join(format!("tile_{index}.png")))
            .unwrap();
    }

    let result = pack_folder(
        &input_dir,
        &output_dir,
        PackConfig {
            max_size: 1024,
            padding: 0,
            extrude: 0,
            trim: false,
            align_to_grid: true,
            grid_cell_size: 48,
            allow_rotation: true,
            power_of_two: true,
            square: false,
            ..Default::default()
        },
    )
    .unwrap();

    assert!(result
        .logs
        .iter()
        .any(|log| log.message.contains("Grid alignment enabled")));
    assert!(result
        .logs
        .iter()
        .any(|log| log.message.contains("rotation disabled")));
    assert_eq!(result.total_atlases, 1);
    let atlas = &result.atlases[0];
    assert_eq!(atlas.width % 48, 0);
    assert_eq!(atlas.height % 48, 0);

    for sprite in &atlas.sprites {
        assert_eq!(sprite.pack_x % 48, 0);
        assert_eq!(sprite.pack_y % 48, 0);
        assert_eq!(sprite.pack_w % 48, 0);
        assert_eq!(sprite.pack_h % 48, 0);
    }
}

#[test]
fn grid_cell_slicing_reuses_internal_empty_cells() {
    let base = Path::new("build/rust_tests/grid_cell_slicing");
    let input_dir = base.join("input");
    let output_dir = base.join("output");
    fs::create_dir_all(&input_dir).unwrap();
    fs::create_dir_all(&output_dir).unwrap();

    let mut image = RgbaImage::from_pixel(96, 96, Rgba([0, 0, 0, 0]));
    for y in 0..48 {
        for x in 0..48 {
            image.put_pixel(x, y, Rgba([255, 0, 0, 255]));
            image.put_pixel(x + 48, y, Rgba([0, 255, 0, 255]));
            image.put_pixel(x, y + 48, Rgba([0, 0, 255, 255]));
        }
    }
    image.save(input_dir.join("shape.png")).unwrap();

    let result = pack_folder(
        &input_dir,
        &output_dir,
        PackConfig {
            max_size: 1024,
            padding: 0,
            extrude: 0,
            trim: false,
            align_to_grid: true,
            grid_cell_size: 48,
            slice_grid_cells: true,
            power_of_two: true,
            square: false,
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(result.total_sprites, 3);
    let atlas = &result.atlases[0];
    assert_eq!(atlas.sprites.len(), 3);
    assert!(atlas
        .sprites
        .iter()
        .all(|sprite| sprite.source_w == 48 && sprite.source_h == 48));
    assert!(atlas
        .sprites
        .iter()
        .all(|sprite| sprite.rel_path.contains("__r")));
}

#[test]
fn grid_cell_slicing_keeps_full_rectangles_merged() {
    let base = Path::new("build/rust_tests/grid_cell_slicing_full_rect");
    let input_dir = base.join("input");
    let output_dir = base.join("output");
    fs::create_dir_all(&input_dir).unwrap();
    fs::create_dir_all(&output_dir).unwrap();

    let image = RgbaImage::from_pixel(96, 96, Rgba([120, 220, 80, 255]));
    image.save(input_dir.join("full_block.png")).unwrap();

    let result = pack_folder(
        &input_dir,
        &output_dir,
        PackConfig {
            max_size: 1024,
            padding: 0,
            extrude: 0,
            trim: false,
            align_to_grid: true,
            grid_cell_size: 48,
            slice_grid_cells: true,
            power_of_two: true,
            square: false,
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(result.total_sprites, 4);
    assert_eq!(result.atlases[0].sprites.len(), 4);
    assert!(result.atlases[0]
        .sprites
        .iter()
        .all(|sprite| sprite.source_w == 48 && sprite.source_h == 48));
}

#[test]
fn grid_outer_cell_trim_keeps_whole_sprite_mode() {
    let base = Path::new("build/rust_tests/grid_outer_trim");
    let input_dir = base.join("input");
    let output_dir = base.join("output");
    fs::create_dir_all(&input_dir).unwrap();
    fs::create_dir_all(&output_dir).unwrap();

    let mut image = RgbaImage::from_pixel(144, 48, Rgba([0, 0, 0, 0]));
    for y in 0..48 {
        for x in 48..96 {
            image.put_pixel(x, y, Rgba([255, 200, 0, 255]));
        }
    }
    image.save(input_dir.join("strip.png")).unwrap();

    let result = pack_folder(
        &input_dir,
        &output_dir,
        PackConfig {
            max_size: 1024,
            padding: 0,
            extrude: 0,
            trim: false,
            align_to_grid: true,
            grid_cell_size: 48,
            slice_grid_cells: false,
            power_of_two: true,
            square: false,
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(result.total_sprites, 1);
    let sprite = &result.atlases[0].sprites[0];
    assert_eq!(sprite.source_w, 144);
    assert_eq!(sprite.source_h, 48);
    assert_eq!(sprite.offset_x, 48);
    assert_eq!(sprite.trim_w, 48);
    assert_eq!(sprite.trim_h, 48);
}

#[test]
fn preview_folder_writes_temp_preview_image() {
    let base = Path::new("build/rust_tests/preview_folder");
    let input_dir = base.join("input");
    fs::create_dir_all(&input_dir).unwrap();

    RgbaImage::from_pixel(8, 8, Rgba([255, 128, 0, 255]))
        .save(input_dir.join("icon.png"))
        .unwrap();

    let result = preview_folder(
        &input_dir,
        PackConfig {
            max_size: 64,
            padding: 0,
            extrude: 0,
            trim: false,
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(result.total_atlases, 1);
    assert!(Path::new(&result.atlases[0].image_path).exists());
    assert!(result.atlases[0].image_data_url.is_none());
    assert!(Path::new(&result.atlases[0].tpsheet_path).exists());
}
