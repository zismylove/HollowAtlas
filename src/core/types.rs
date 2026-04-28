use std::fmt;
use std::path::Path;

use clap::ValueEnum;
use image::RgbaImage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
#[value(rename_all = "snake_case")]
pub enum SplitMode {
    #[value(name = "all_in_one")]
    AllInOne,
    #[value(name = "by_first_level_folder")]
    ByFirstLevelFolder,
}

impl Default for SplitMode {
    fn default() -> Self {
        Self::AllInOne
    }
}

impl fmt::Display for SplitMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SplitMode::AllInOne => write!(f, "all_in_one"),
            SplitMode::ByFirstLevelFolder => write!(f, "by_first_level_folder"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
#[value(rename_all = "snake_case")]
pub enum OutputFormat {
    #[serde(rename = "godot_tpsheet", alias = "godot_tp_sheet")]
    #[value(name = "godot_tpsheet")]
    GodotTpSheet,
    #[serde(rename = "json_debug")]
    #[value(name = "json_debug")]
    JsonDebug,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::GodotTpSheet
    }
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputFormat::GodotTpSheet => write!(f, "godot_tpsheet"),
            OutputFormat::JsonDebug => write!(f, "json_debug"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackConfig {
    pub max_size: u32,
    pub padding: u32,
    pub extrude: u32,
    pub trim: bool,
    #[serde(default)]
    pub align_to_grid: bool,
    #[serde(default = "default_grid_cell_size")]
    pub grid_cell_size: u32,
    #[serde(default = "default_slice_grid_cells")]
    pub slice_grid_cells: bool,
    pub allow_rotation: bool,
    pub power_of_two: bool,
    pub square: bool,
    pub split_mode: SplitMode,
    pub output_format: OutputFormat,
    pub debug_json: bool,
}

impl Default for PackConfig {
    fn default() -> Self {
        Self {
            max_size: 2048,
            padding: 2,
            extrude: 1,
            trim: true,
            align_to_grid: false,
            grid_cell_size: default_grid_cell_size(),
            slice_grid_cells: default_slice_grid_cells(),
            allow_rotation: false,
            power_of_two: true,
            square: true,
            split_mode: SplitMode::AllInOne,
            output_format: OutputFormat::GodotTpSheet,
            debug_json: false,
        }
    }
}

impl PackConfig {
    pub fn normalized(self) -> Self {
        let grid_cell_size = self.grid_cell_size.max(1);
        let max_size = if self.align_to_grid {
            let available_cells = (self.max_size / grid_cell_size).max(1);
            let normalized_cells = if self.power_of_two {
                highest_power_of_two_not_exceeding(available_cells)
            } else {
                available_cells
            };
            normalized_cells.saturating_mul(grid_cell_size)
        } else {
            self.max_size.max(1)
        };

        Self {
            max_size,
            padding: self.padding,
            extrude: self.extrude,
            trim: self.trim,
            align_to_grid: self.align_to_grid,
            grid_cell_size,
            slice_grid_cells: self.slice_grid_cells,
            allow_rotation: self.allow_rotation,
            power_of_two: self.power_of_two,
            square: self.square,
            split_mode: self.split_mode,
            output_format: self.output_format,
            debug_json: self.debug_json,
        }
    }
}

fn default_grid_cell_size() -> u32 {
    48
}

fn default_slice_grid_cells() -> bool {
    true
}

fn highest_power_of_two_not_exceeding(value: u32) -> u32 {
    let value = value.max(1);
    1 << (u32::BITS - 1 - value.leading_zeros())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTreeNode {
    pub name: String,
    pub path: String,
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(default)]
    pub children: Vec<FileTreeNode>,
    #[serde(rename = "imageCount")]
    pub image_count: usize,
}

impl FileTreeNode {
    pub fn directory(name: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            node_type: "directory".to_string(),
            children: Vec::new(),
            image_count: 0,
        }
    }

    pub fn image(name: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            node_type: "image".to_string(),
            children: Vec::new(),
            image_count: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceImage {
    pub id: usize,
    pub name: String,
    pub abs_path: String,
    pub rel_path: String,
    pub width: u32,
    pub height: u32,
    pub file_size: u64,
    pub readable: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub root: FileTreeNode,
    pub images: Vec<SourceImage>,
    pub total_images: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GridSlice {
    pub x: u32,
    pub y: u32,
    pub name: String,
    pub rel_path: String,
}

#[derive(Debug, Clone)]
pub struct PreparedSprite {
    pub id: usize,
    pub name: String,
    pub abs_path: String,
    pub rel_path: String,
    pub source_width: u32,
    pub source_height: u32,
    pub trim_x: u32,
    pub trim_y: u32,
    pub trim_width: u32,
    pub trim_height: u32,
    pub image: RgbaImage,
    pub padding: u32,
    pub extrude: u32,
    pub trimmed: bool,
    pub grid_slices: Vec<GridSlice>,
}

impl PreparedSprite {
    pub fn packed_width(&self) -> u32 {
        self.image.width() + self.padding * 2
    }

    pub fn packed_height(&self) -> u32 {
        self.image.height() + self.padding * 2
    }

    pub fn area(&self) -> u64 {
        self.packed_width() as u64 * self.packed_height() as u64
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    pub fn right(self) -> u32 {
        self.x + self.w
    }

    pub fn bottom(self) -> u32 {
        self.y + self.h
    }

    pub fn area(self) -> u64 {
        self.w as u64 * self.h as u64
    }

    pub fn intersects(self, other: Rect) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }

    pub fn contains(self, other: Rect) -> bool {
        self.x <= other.x
            && self.y <= other.y
            && self.right() >= other.right()
            && self.bottom() >= other.bottom()
    }
}

#[derive(Debug, Clone)]
pub struct Placement {
    pub sprite: PreparedSprite,
    pub rect: Rect,
    pub rotated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackedSprite {
    pub name: String,
    pub rel_path: String,
    pub atlas_index: usize,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub source_w: u32,
    pub source_h: u32,
    pub offset_x: u32,
    pub offset_y: u32,
    pub trim_w: u32,
    pub trim_h: u32,
    pub rotated: bool,
    pub trimmed: bool,
    pub pack_x: u32,
    pub pack_y: u32,
    pub pack_w: u32,
    pub pack_h: u32,
}

#[derive(Debug, Clone)]
pub struct AtlasBuild {
    pub atlas_index: usize,
    pub group_name: String,
    pub width: u32,
    pub height: u32,
    pub placements: Vec<Placement>,
    pub usage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasResult {
    pub image_path: String,
    pub tpsheet_path: String,
    pub debug_json_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_data_url: Option<String>,
    pub width: u32,
    pub height: u32,
    pub usage: f32,
    pub sprites: Vec<PackedSprite>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogMessage {
    pub level: String,
    pub message: String,
}

impl LogMessage {
    pub fn new(level: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            level: level.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackResult {
    pub atlases: Vec<AtlasResult>,
    pub total_sprites: usize,
    pub total_atlases: usize,
    pub logs: Vec<LogMessage>,
}

pub fn path_to_posix(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
