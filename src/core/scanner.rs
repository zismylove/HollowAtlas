use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use walkdir::WalkDir;

use crate::core::types::{path_to_posix, FileTreeNode, ScanResult, SourceImage};

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp", "bmp"];

pub fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| IMAGE_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn scan_folder(path: impl AsRef<Path>) -> Result<ScanResult> {
    let root_path = canonicalize_input_directory(path.as_ref())?;
    let mut warnings = Vec::new();
    let images = scan_root_images(&root_path, None, 0, &mut warnings)?;

    let root = build_file_tree(&root_path, &images);
    Ok(ScanResult {
        total_images: images.len(),
        root,
        images,
        warnings,
    })
}

pub fn scan_folders<I, P>(paths: I) -> Result<ScanResult>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let root_paths = collect_unique_input_directories(paths)?;
    if root_paths.len() == 1 {
        return scan_folder(&root_paths[0]);
    }

    let labels = unique_root_labels(&root_paths);
    let mut images = Vec::new();
    let mut warnings = Vec::new();

    for (root_path, label) in root_paths.iter().zip(labels.iter()) {
        let mut root_images =
            scan_root_images(root_path, Some(label), images.len(), &mut warnings)?;
        images.append(&mut root_images);
    }

    let root = build_multi_file_tree(&images);
    Ok(ScanResult {
        total_images: images.len(),
        root,
        images,
        warnings,
    })
}

fn canonicalize_input_directory(path: &Path) -> Result<PathBuf> {
    let root_path = path
        .canonicalize()
        .with_context(|| format!("Input directory does not exist: {}", path.display()))?;

    if !root_path.is_dir() {
        bail!("Input path is not a directory: {}", root_path.display());
    }

    Ok(root_path)
}

fn collect_unique_input_directories<I, P>(paths: I) -> Result<Vec<PathBuf>>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let mut root_paths = Vec::new();
    let mut seen = BTreeSet::new();

    for path in paths {
        if path.as_ref().as_os_str().is_empty() {
            continue;
        }

        let root_path = canonicalize_input_directory(path.as_ref())?;
        let key = path_to_posix(&root_path).to_ascii_lowercase();
        if seen.insert(key) {
            root_paths.push(root_path);
        }
    }

    if root_paths.is_empty() {
        bail!("Choose at least one input directory.");
    }

    Ok(root_paths)
}

fn unique_root_labels(root_paths: &[PathBuf]) -> Vec<String> {
    let mut labels = Vec::with_capacity(root_paths.len());
    let mut used = BTreeSet::new();

    for root_path in root_paths {
        let base_name = root_path
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.trim().is_empty())
            .unwrap_or("input")
            .to_string();
        let mut label = base_name.clone();
        let mut index = 2usize;

        while !used.insert(label.to_ascii_lowercase()) {
            label = format!("{base_name}_{index}");
            index += 1;
        }

        labels.push(label);
    }

    labels
}

fn scan_root_images(
    root_path: &Path,
    rel_prefix: Option<&str>,
    id_offset: usize,
    warnings: &mut Vec<String>,
) -> Result<Vec<SourceImage>> {
    let mut files: Vec<PathBuf> = WalkDir::new(root_path)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| is_supported_image(path))
        .collect();

    files.sort_by_key(|path| {
        path.strip_prefix(root_path)
            .map(path_to_posix)
            .unwrap_or_else(|_| path_to_posix(path))
            .to_ascii_lowercase()
    });

    let mut images = Vec::with_capacity(files.len());

    for (id, file_path) in files.iter().enumerate() {
        let local_rel_path = file_path
            .strip_prefix(root_path)
            .map(path_to_posix)
            .unwrap_or_else(|_| path_to_posix(file_path));
        let rel_path = rel_prefix
            .map(|prefix| format!("{prefix}/{local_rel_path}"))
            .unwrap_or(local_rel_path);
        let file_size = fs::metadata(file_path)?.len();

        let (width, height, readable, error) = match image::image_dimensions(file_path) {
            Ok((width, height)) => (width, height, true, None),
            Err(err) => {
                warnings.push(format!("Failed to read image metadata: {rel_path}: {err}"));
                (0, 0, false, Some(err.to_string()))
            }
        };

        images.push(SourceImage {
            id: id_offset + id,
            name: file_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_string(),
            abs_path: file_path.to_string_lossy().to_string(),
            rel_path,
            width,
            height,
            file_size,
            readable,
            error,
        });
    }

    Ok(images)
}

pub fn build_file_tree(root_path: &Path, images: &[SourceImage]) -> FileTreeNode {
    let root_name = root_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_else(|| root_path.to_str().unwrap_or("root"))
        .to_string();
    let mut root = FileTreeNode::directory(root_name, "");

    for image in images {
        let parts: Vec<&str> = image.rel_path.split('/').collect();
        insert_image_node(&mut root, &parts, "");
    }

    update_counts_and_sort(&mut root);
    root
}

fn build_multi_file_tree(images: &[SourceImage]) -> FileTreeNode {
    let mut root = FileTreeNode::directory("Input Folders", "");

    for image in images {
        let parts: Vec<&str> = image.rel_path.split('/').collect();
        insert_image_node(&mut root, &parts, "");
    }

    update_counts_and_sort(&mut root);
    root
}

fn insert_image_node(node: &mut FileTreeNode, parts: &[&str], prefix: &str) {
    if parts.is_empty() {
        return;
    }

    if parts.len() == 1 {
        let path = if prefix.is_empty() {
            parts[0].to_string()
        } else {
            format!("{prefix}/{}", parts[0])
        };
        node.children.push(FileTreeNode::image(parts[0], path));
        return;
    }

    let dir_name = parts[0];
    let dir_path = if prefix.is_empty() {
        dir_name.to_string()
    } else {
        format!("{prefix}/{dir_name}")
    };

    let index = node
        .children
        .iter()
        .position(|child| child.node_type == "directory" && child.name == dir_name);

    let child_index = match index {
        Some(index) => index,
        None => {
            node.children
                .push(FileTreeNode::directory(dir_name, dir_path.clone()));
            node.children.len() - 1
        }
    };

    insert_image_node(&mut node.children[child_index], &parts[1..], &dir_path);
}

fn update_counts_and_sort(node: &mut FileTreeNode) -> usize {
    if node.node_type == "image" {
        node.image_count = 1;
        return 1;
    }

    let mut count = 0;
    for child in &mut node.children {
        count += update_counts_and_sort(child);
    }
    node.image_count = count;
    node.children.sort_by_key(|child| {
        (
            child.node_type != "directory",
            child.name.to_ascii_lowercase(),
        )
    });
    count
}
