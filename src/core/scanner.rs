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
    let root_path = path.as_ref().canonicalize().with_context(|| {
        format!(
            "Input directory does not exist: {}",
            path.as_ref().display()
        )
    })?;

    if !root_path.is_dir() {
        bail!("Input path is not a directory: {}", root_path.display());
    }

    let mut files: Vec<PathBuf> = WalkDir::new(&root_path)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| is_supported_image(path))
        .collect();

    files.sort_by_key(|path| {
        path.strip_prefix(&root_path)
            .map(path_to_posix)
            .unwrap_or_else(|_| path_to_posix(path))
            .to_ascii_lowercase()
    });

    let mut images = Vec::with_capacity(files.len());
    let mut warnings = Vec::new();

    for (id, file_path) in files.iter().enumerate() {
        let rel_path = file_path
            .strip_prefix(&root_path)
            .map(path_to_posix)
            .unwrap_or_else(|_| path_to_posix(file_path));
        let file_size = fs::metadata(file_path)?.len();

        let (width, height, readable, error) = match image::image_dimensions(file_path) {
            Ok((width, height)) => (width, height, true, None),
            Err(err) => {
                warnings.push(format!("Failed to read image metadata: {rel_path}: {err}"));
                (0, 0, false, Some(err.to_string()))
            }
        };

        images.push(SourceImage {
            id,
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

    let root = build_file_tree(&root_path, &images);
    Ok(ScanResult {
        total_images: images.len(),
        root,
        images,
        warnings,
    })
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
