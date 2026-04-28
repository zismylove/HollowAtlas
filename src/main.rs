use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use hollowatlas::core::packer::pack_folder;
use hollowatlas::core::scanner::scan_folder;
use hollowatlas::core::types::{OutputFormat, PackConfig, SplitMode};

#[derive(Debug, Parser)]
#[command(name = "hollowatlas")]
#[command(about = "Texture atlas packer with tileset-friendly grid workflows")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Scan {
        input: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Pack {
        input: PathBuf,
        output: PathBuf,
        #[arg(long, default_value_t = 2048, value_parser = parse_positive_u32)]
        max_size: u32,
        #[arg(long, default_value_t = 2, value_parser = parse_padding)]
        padding: u32,
        #[arg(long, default_value_t = 1, value_parser = parse_extrude)]
        extrude: u32,
        #[arg(long)]
        no_trim: bool,
        #[arg(long)]
        align_to_grid: bool,
        #[arg(long, default_value_t = 48, value_parser = parse_positive_u32)]
        grid_cell_size: u32,
        #[arg(long)]
        no_slice_grid_cells: bool,
        #[arg(long)]
        allow_rotation: bool,
        #[arg(long)]
        no_power_of_two: bool,
        #[arg(long)]
        no_square: bool,
        #[arg(long, default_value_t = SplitMode::AllInOne)]
        split_mode: SplitMode,
        #[arg(long, default_value_t = OutputFormat::GodotTpSheet)]
        output_format: OutputFormat,
        #[arg(long)]
        debug_json: bool,
        #[arg(long)]
        json: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Scan { input, json } => {
            let result = scan_folder(&input)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("Input: {}", input.display());
                println!("Images: {}", result.total_images);
                for warning in result.warnings {
                    eprintln!("warning: {warning}");
                }
            }
        }
        Commands::Pack {
            input,
            output,
            max_size,
            padding,
            extrude,
            no_trim,
            align_to_grid,
            grid_cell_size,
            no_slice_grid_cells,
            allow_rotation,
            no_power_of_two,
            no_square,
            split_mode,
            output_format,
            debug_json,
            json,
        } => {
            let config = PackConfig {
                max_size,
                padding,
                extrude,
                trim: !no_trim,
                align_to_grid,
                grid_cell_size,
                slice_grid_cells: !no_slice_grid_cells,
                allow_rotation,
                power_of_two: !no_power_of_two,
                square: !no_square,
                split_mode,
                output_format,
                debug_json: debug_json || output_format == OutputFormat::JsonDebug,
            }
            .normalized();

            let result = pack_folder(&input, &output, config)?;
            for log in &result.logs {
                println!("[{}] {}", log.level, log.message);
            }
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!(
                    "Packed {} sprites into {} atlas file(s).",
                    result.total_sprites, result.total_atlases
                );
                for atlas in result.atlases {
                    println!(
                        "{} | {}x{} | usage {:.1}% | sprites {}",
                        atlas.image_path,
                        atlas.width,
                        atlas.height,
                        atlas.usage * 100.0,
                        atlas.sprites.len()
                    );
                }
            }
        }
    }

    Ok(())
}

fn parse_padding(value: &str) -> std::result::Result<u32, String> {
    parse_choice(value, &[0, 1, 2, 4, 8], "padding")
}

fn parse_extrude(value: &str) -> std::result::Result<u32, String> {
    parse_choice(value, &[0, 1, 2, 4], "extrude")
}

fn parse_positive_u32(value: &str) -> std::result::Result<u32, String> {
    let parsed = value
        .parse::<u32>()
        .map_err(|err| format!("invalid value: {err}"))?;
    if parsed > 0 {
        Ok(parsed)
    } else {
        Err("value must be greater than 0".to_string())
    }
}

fn parse_choice(value: &str, choices: &[u32], label: &str) -> std::result::Result<u32, String> {
    let parsed = value
        .parse::<u32>()
        .map_err(|err| format!("invalid {label}: {err}"))?;
    if choices.contains(&parsed) {
        Ok(parsed)
    } else {
        Err(format!("{label} must be one of {choices:?}"))
    }
}
