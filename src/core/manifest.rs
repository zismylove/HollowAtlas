use std::path::Path;

use serde_json::{json, Value};

use crate::core::types::{AtlasResult, PackedSprite};

pub fn build_godot_tpsheet(atlases: &[AtlasResult]) -> Value {
    let texture_items: Vec<Value> = atlases.iter().map(build_texture_item).collect();

    json!({
        "textures": texture_items,
        "meta": {
            "app": "Texture Atlas Packer",
            "version": "0.1.0",
            "target": "Godot TexturePacker Importer",
            "scale": "1"
        }
    })
}

fn build_texture_item(atlas: &AtlasResult) -> Value {
    let image_name = Path::new(&atlas.image_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("atlas.png");
    let sprite_items = build_sprite_items(&atlas.sprites);

    json!({
        "image": image_name,
        "format": "RGBA8888",
        "size": { "w": atlas.width, "h": atlas.height },
        "sprites": sprite_items
    })
}

fn build_sprite_items(sprites: &[PackedSprite]) -> Vec<Value> {
    let mut sorted = sprites.to_vec();
    sorted.sort_by_key(|sprite| sprite.rel_path.to_ascii_lowercase());

    sorted
        .iter()
        .map(|sprite| {
            let margin_w = sprite.source_w.saturating_sub(sprite.trim_w);
            let margin_h = sprite.source_h.saturating_sub(sprite.trim_h);
            json!({
                "filename": sprite.rel_path,
                "region": { "x": sprite.x, "y": sprite.y, "w": sprite.w, "h": sprite.h },
                "margin": {
                    "x": sprite.offset_x,
                    "y": sprite.offset_y,
                    "w": margin_w,
                    "h": margin_h
                },
                "spriteSourceSize": {
                    "x": sprite.offset_x,
                    "y": sprite.offset_y,
                    "w": sprite.trim_w,
                    "h": sprite.trim_h
                },
                "sourceSize": { "w": sprite.source_w, "h": sprite.source_h },
                "rotated": sprite.rotated,
                "trimmed": sprite.trimmed
            })
        })
        .collect()
}

pub fn build_texturepacker_json_hash(
    image_name: &str,
    width: u32,
    height: u32,
    sprites: &[PackedSprite],
) -> Value {
    let mut sorted = sprites.to_vec();
    sorted.sort_by_key(|sprite| sprite.rel_path.to_ascii_lowercase());

    let mut frames = serde_json::Map::new();
    for sprite in sorted {
        frames.insert(
            sprite.rel_path.clone(),
            json!({
                "frame": { "x": sprite.x, "y": sprite.y, "w": sprite.w, "h": sprite.h },
                "rotated": sprite.rotated,
                "trimmed": sprite.trimmed,
                "spriteSourceSize": {
                    "x": sprite.offset_x,
                    "y": sprite.offset_y,
                    "w": sprite.trim_w,
                    "h": sprite.trim_h
                },
                "sourceSize": { "w": sprite.source_w, "h": sprite.source_h }
            }),
        );
    }

    json!({
        "frames": frames,
        "meta": {
            "app": "Texture Atlas Packer",
            "version": "0.1.0",
            "image": image_name,
            "format": "RGBA8888",
            "size": { "w": width, "h": height },
            "scale": "1"
        }
    })
}
