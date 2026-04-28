#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use hollowatlas::core::packer::pack_folder as core_pack_folder;
use hollowatlas::core::packer::preview_folder as core_preview_folder;
use hollowatlas::core::scanner::scan_folder as core_scan_folder;
use hollowatlas::core::types::{PackConfig, PackResult, ScanResult};
use serde::{Deserialize, Serialize};
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectFilePayload {
    #[serde(default = "default_project_version")]
    version: u32,
    #[serde(default)]
    input_path: String,
    #[serde(default)]
    output_path: String,
    #[serde(default)]
    config: PackConfig,
    #[serde(default = "default_show_bounds")]
    show_bounds: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedRecentProject {
    path: String,
    name: String,
    last_opened_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecentProjectEntry {
    path: String,
    name: String,
    last_opened_at: u64,
    exists: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct RecentProjectsPayload {
    #[serde(default)]
    recent_projects: Vec<PersistedRecentProject>,
}

fn default_project_version() -> u32 {
    1
}

fn default_show_bounds() -> bool {
    true
}

fn recent_projects_file_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|err| format!("Failed to resolve app config directory: {err}"))?;
    fs::create_dir_all(&config_dir)
        .map_err(|err| format!("Failed to create app config directory: {err}"))?;
    Ok(config_dir.join("recent-projects.json"))
}

fn read_recent_projects(app: &tauri::AppHandle) -> Result<Vec<PersistedRecentProject>, String> {
    let file_path = recent_projects_file_path(app)?;
    if !file_path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&file_path)
        .map_err(|err| format!("Failed to read recent project list: {err}"))?;
    let payload: RecentProjectsPayload = serde_json::from_str(&content)
        .map_err(|err| format!("Failed to parse recent project list: {err}"))?;
    Ok(payload.recent_projects)
}

fn write_recent_projects(
    app: &tauri::AppHandle,
    projects: &[PersistedRecentProject],
) -> Result<(), String> {
    let file_path = recent_projects_file_path(app)?;
    let payload = RecentProjectsPayload {
        recent_projects: projects.to_vec(),
    };
    let content = serde_json::to_string_pretty(&payload)
        .map_err(|err| format!("Failed to serialize recent project list: {err}"))?;
    fs::write(file_path, content)
        .map_err(|err| format!("Failed to save recent project list: {err}"))
}

fn project_display_name(path: &str) -> String {
    let file_path = PathBuf::from(path);
    file_path
        .file_stem()
        .and_then(|name| name.to_str())
        .or_else(|| file_path.file_name().and_then(|name| name.to_str()))
        .unwrap_or(path)
        .to_string()
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn to_recent_entries(projects: Vec<PersistedRecentProject>) -> Vec<RecentProjectEntry> {
    projects
        .into_iter()
        .map(|project| RecentProjectEntry {
            exists: PathBuf::from(&project.path).exists(),
            path: project.path,
            name: project.name,
            last_opened_at: project.last_opened_at,
        })
        .collect()
}

#[tauri::command]
fn scan_folder(path: String) -> Result<ScanResult, String> {
    core_scan_folder(path).map_err(|err| err.to_string())
}

#[tauri::command]
fn pack_folder(
    input_path: String,
    output_path: String,
    config: PackConfig,
) -> Result<PackResult, String> {
    core_pack_folder(input_path, output_path, config).map_err(|err| err.to_string())
}

#[tauri::command]
fn preview_folder(input_path: String, config: PackConfig) -> Result<PackResult, String> {
    core_preview_folder(input_path, config).map_err(|err| err.to_string())
}

#[tauri::command]
fn read_image_data_url(path: String) -> Result<String, String> {
    let bytes = fs::read(&path).map_err(|err| format!("Failed to read image: {err}"))?;
    Ok(format!("data:image/png;base64,{}", STANDARD.encode(bytes)))
}

#[tauri::command]
fn save_project_file(path: String, mut project: ProjectFilePayload) -> Result<(), String> {
    let path = PathBuf::from(path);
    if let Some(parent) = path.parent().filter(|value| !value.as_os_str().is_empty()) {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Failed to create project folder: {err}"))?;
    }

    project.config = project.config.normalized();
    let content = serde_json::to_string_pretty(&project)
        .map_err(|err| format!("Failed to serialize project: {err}"))?;
    fs::write(&path, content).map_err(|err| format!("Failed to save project file: {err}"))?;
    Ok(())
}

#[tauri::command]
fn load_project_file(path: String) -> Result<ProjectFilePayload, String> {
    let content =
        fs::read_to_string(&path).map_err(|err| format!("Failed to read project file: {err}"))?;
    let mut project: ProjectFilePayload = serde_json::from_str(&content)
        .map_err(|err| format!("Failed to parse project file: {err}"))?;
    project.config = project.config.normalized();
    Ok(project)
}

#[tauri::command]
fn get_recent_projects(app: tauri::AppHandle) -> Result<Vec<RecentProjectEntry>, String> {
    read_recent_projects(&app).map(to_recent_entries)
}

#[tauri::command]
fn record_recent_project(
    app: tauri::AppHandle,
    path: String,
) -> Result<Vec<RecentProjectEntry>, String> {
    let normalized_path = path.trim().to_string();
    if normalized_path.is_empty() {
        return Ok(to_recent_entries(read_recent_projects(&app)?));
    }

    let mut projects = read_recent_projects(&app)?;
    projects.retain(|project| project.path != normalized_path);
    projects.insert(
        0,
        PersistedRecentProject {
            name: project_display_name(&normalized_path),
            path: normalized_path,
            last_opened_at: current_timestamp(),
        },
    );
    projects.truncate(12);

    write_recent_projects(&app, &projects)?;
    Ok(to_recent_entries(projects))
}

#[tauri::command]
fn clear_recent_projects(app: tauri::AppHandle) -> Result<(), String> {
    write_recent_projects(&app, &[])
}

#[tauri::command]
fn open_output_folder(path: String) -> Result<(), String> {
    let path = PathBuf::from(path);
    fs::create_dir_all(&path).map_err(|err| format!("Failed to create output folder: {err}"))?;

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|err| format!("Failed to open folder: {err}"))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|err| format!("Failed to open folder: {err}"))?;
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|err| format!("Failed to open folder: {err}"))?;
    }

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            scan_folder,
            pack_folder,
            preview_folder,
            read_image_data_url,
            save_project_file,
            load_project_file,
            get_recent_projects,
            record_recent_project,
            clear_recent_projects,
            open_output_folder
        ])
        .run(tauri::generate_context!())
        .expect("failed to run HollowAtlas");
}
