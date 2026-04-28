from __future__ import annotations

from pathlib import Path
from typing import Dict, Iterable, List, Tuple

from PIL import Image

from .types import FileTreeNode, ScanResult, SourceImage, as_posix

IMAGE_EXTENSIONS = {".png", ".jpg", ".jpeg", ".webp", ".bmp"}


def is_supported_image(path: Path) -> bool:
    return path.suffix.lower() in IMAGE_EXTENSIONS


def scan_folder(path: str | Path) -> ScanResult:
    root_path = Path(path).expanduser().resolve()
    if not root_path.exists():
        raise FileNotFoundError(f"Input directory does not exist: {root_path}")
    if not root_path.is_dir():
        raise NotADirectoryError(f"Input path is not a directory: {root_path}")

    images: List[SourceImage] = []
    warnings: List[str] = []

    files = sorted(
        (item for item in root_path.rglob("*") if item.is_file() and is_supported_image(item)),
        key=lambda p: as_posix(p.relative_to(root_path)).lower(),
    )

    for idx, file_path in enumerate(files):
        rel_path = as_posix(file_path.relative_to(root_path))
        width = 0
        height = 0
        readable = True
        error = None
        try:
            with Image.open(file_path) as image:
                width, height = image.size
        except Exception as exc:  # Pillow exposes several decoder-specific errors.
            readable = False
            error = str(exc)
            warnings.append(f"Failed to read image metadata: {rel_path}: {exc}")

        file_size = file_path.stat().st_size
        images.append(
            SourceImage(
                id=idx,
                name=file_path.name,
                abs_path=str(file_path),
                rel_path=rel_path,
                width=width,
                height=height,
                file_size=file_size,
                readable=readable,
                error=error,
            )
        )

    tree = build_file_tree(root_path, images)
    return ScanResult(root=tree, images=images, total_images=len(images), warnings=warnings)


def build_file_tree(root_path: Path, images: Iterable[SourceImage]) -> FileTreeNode:
    root = FileTreeNode(name=root_path.name or str(root_path), path="", type="directory")
    nodes: Dict[Tuple[str, ...], FileTreeNode] = {(): root}

    for image in images:
        parts = tuple(Path(image.rel_path).parts)
        parent_parts: Tuple[str, ...] = ()
        for depth, part in enumerate(parts[:-1], start=1):
            current_parts = parts[:depth]
            if current_parts not in nodes:
                node = FileTreeNode(
                    name=part,
                    path="/".join(current_parts),
                    type="directory",
                )
                nodes[current_parts] = node
                nodes[parent_parts].children.append(node)
            nodes[current_parts].image_count += 1
            parent_parts = current_parts

        root.image_count += 1
        image_node = FileTreeNode(
            name=parts[-1] if parts else image.name,
            path=image.rel_path,
            type="image",
            image_count=1,
        )
        nodes[parent_parts].children.append(image_node)

    sort_tree(root)
    return root


def sort_tree(node: FileTreeNode) -> None:
    node.children.sort(key=lambda child: (child.type != "directory", child.name.lower()))
    for child in node.children:
        sort_tree(child)
