export type SplitMode = "all_in_one" | "by_first_level_folder";
export type OutputFormat = "godot_tpsheet" | "json_debug";

export type PackConfig = {
  max_size: number;
  padding: number;
  extrude: number;
  trim: boolean;
  align_to_grid: boolean;
  grid_cell_size: number;
  slice_grid_cells: boolean;
  allow_rotation: boolean;
  power_of_two: boolean;
  square: boolean;
  split_mode: SplitMode;
  output_format: OutputFormat;
  debug_json: boolean;
};

export type FileTreeNode = {
  name: string;
  path: string;
  type: "directory" | "image";
  children: FileTreeNode[];
  imageCount: number;
};

export type SourceImage = {
  id: number;
  name: string;
  abs_path: string;
  rel_path: string;
  width: number;
  height: number;
  file_size: number;
  readable: boolean;
  error: string | null;
};

export type ScanResult = {
  root: FileTreeNode;
  images: SourceImage[];
  total_images: number;
  warnings: string[];
};

export type PackedSprite = {
  name: string;
  rel_path: string;
  atlas_index: number;
  x: number;
  y: number;
  w: number;
  h: number;
  source_w: number;
  source_h: number;
  offset_x: number;
  offset_y: number;
  trim_w: number;
  trim_h: number;
  rotated: boolean;
  trimmed: boolean;
  pack_x: number;
  pack_y: number;
  pack_w: number;
  pack_h: number;
};

export type AtlasResult = {
  image_path: string;
  tpsheet_path: string;
  debug_json_path: string | null;
  image_data_url?: string | null;
  width: number;
  height: number;
  usage: number;
  sprites: PackedSprite[];
};

export type LogMessage = {
  level: "info" | "warning" | "error" | "success" | string;
  message: string;
};

export type PackResult = {
  atlases: AtlasResult[];
  total_sprites: number;
  total_atlases: number;
  logs: LogMessage[];
};

export type ProjectFile = {
  version: number;
  input_path: string;
  output_path: string;
  config: PackConfig;
  show_bounds: boolean;
};

export type RecentProject = {
  path: string;
  name: string;
  last_opened_at: number;
  exists: boolean;
};
