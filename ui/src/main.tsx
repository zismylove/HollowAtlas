import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import {
  Boxes,
  Check,
  ChevronDown,
  ChevronRight,
  Eye,
  FilePlus2,
  FolderOpen,
  Image as ImageIcon,
  Loader2,
  Play,
  RotateCcw,
  Save,
  Settings2,
  SquareDashedMousePointer,
  Trash2,
  ZoomIn,
  ZoomOut,
} from "lucide-react";
import type {
  AtlasResult,
  FileTreeNode,
  LogMessage,
  PackConfig,
  PackResult,
  ProjectFile,
  RecentProject,
  ScanResult,
} from "./types";
import "./styles.css";

const projectExtension = "hollowatlasproj";
const defaultGridCellSize = 48;
const baseMaxSizeOptions = [512, 1024, 2048, 4096, 8192];
const gridCellCountOptions = [4, 8, 16, 32, 64, 128];

const translations = {
  en: {
    ready: "Ready.",
    language: "Language",
    chinese: "Chinese",
    english: "English",
    newProject: "New Project",
    openProject: "Open Project",
    recentProjects: "Recent Projects",
    noRecentProjects: "No recent projects yet.",
    clearRecent: "Clear Recent",
    saveProject: "Save Project",
    input: "Input",
    pack: "Pack",
    reveal: "Reveal",
    project: "Project:",
    output: "Output:",
    noProjectFile: "No project file",
    noInputFolders: "No input folders",
    noOutputFile: "No output file",
    missingFile: "Missing file",
    files: "Files",
    noFoldersSelected: "No folders selected",
    releaseFolders: "Release folders to add them as input sources",
    options: "Options",
    outputFile: "Output File",
    browse: "Browse",
    selectOutputFile: "Select Output File",
    selectOutputPlaceholder: "Select output file",
    selectInputFirst: "Select input folders first to generate the default output file.",
    leaveBlankPrefix: "Leave blank to use",
    alignToGrid: "Align to Tile Grid",
    gridCellSize: "Grid Cell Size",
    gridCellSizeHelp: "Sprites snap to whole {size}x{size} cells. Tileset workflows usually work best with trim off and padding/extrude set to 0.",
    sliceGridCells: "Slice Grid Cells",
    sliceGridCellsOn: "Enabled: transparent grid holes split the image into occupied grid regions, while fully connected blocks stay merged.",
    sliceGridCellsOff: "Disabled: keep each source image as one sprite, but only trim transparent area by whole grid cells around the outside.",
    maxSize: "Max Size",
    maxSizeGridHelp: "In grid mode, max size is normalized to a multiple of {size}. With Power of Two on, it uses power-of-two cell counts.",
    padding: "Padding",
    extrude: "Extrude",
    trimTransparent: "Trim Transparent",
    allowRotation: "Allow Rotation",
    powerOfTwo: "Power of Two",
    squareAtlas: "Square Atlas",
    separateInputFolders: "Separate Input Folders",
    separateInputFoldersHelp: "When multiple folders are loaded, each input folder is packed into its own atlas image instead of sharing one combined layout.",
    forceMaxAtlasSize: "Force Max Atlas Size",
    forceMaxAtlasSizeHelp: "Every output atlas page keeps the selected Max Size, even if only a few sprites are placed on that page.",
    splitMode: "Split Mode",
    outputFormat: "Output Format",
    debugJson: "Debug JSON",
    showBounds: "Show Bounds",
    atlasWall: "Atlas Wall",
    zoomOut: "Zoom out",
    zoomIn: "Zoom in",
    resetZoom: "Reset zoom",
    generatingPreview: "Generating atlas preview...",
    previewAfterProcessing: "Preview will appear here after the input folders are processed.",
    dropFoldersOrInput: "Drop folders or choose Input.",
    atlasCount: "atlas",
    spritesCount: "sprites",
    wheelZoom: "Wheel = zoom",
    dragPan: "Drag = pan",
    clear: "Clear",
    newProjectTitle: "New blank project",
    openProjectTitle: "Open project file",
    recentProjectsTitle: "Recent projects",
    saveProjectTitle: "Save project file",
    projectFileType: "HollowAtlas Project",
    saveProjectDialogTitle: "Save HollowAtlas Project",
    chooseInputTitle: "Choose input folders",
    packTitle: "Start packing",
    revealTitle: "Open output folder",
    clearTitle: "Clear current project",
    scanFoldersLog: "Scan folders:",
    imagesFoundLog: "Images found:",
    projectLoadedLog: "Project loaded:",
    projectNoInputsLog: "Project loaded, but no input folders are configured.",
    chooseInputWarning: "Choose at least one input folder first.",
    packStartedLog: "Pack started.",
    chooseOutputWarning: "Choose an output file first.",
    projectSavedLog: "Project saved:",
    recentClearedLog: "Recent project list cleared.",
    newBlankProjectLog: "New blank project ready.",
    droppedPathsLog: "Dropped {count} path(s).",
    previewFailedLog: "Preview failed:",
    dragDropFailedLog: "Failed to enable drag and drop:",
  },
  zh: {
    ready: "准备就绪。",
    language: "Language",
    chinese: "中文",
    english: "English",
    newProject: "新建项目",
    openProject: "打开项目",
    recentProjects: "最近项目",
    noRecentProjects: "还没有最近项目。",
    clearRecent: "清空最近",
    saveProject: "保存项目",
    input: "输入目录",
    pack: "打包",
    reveal: "打开输出",
    project: "项目：",
    output: "输出：",
    noProjectFile: "未选择项目文件",
    noInputFolders: "未选择输入目录",
    noOutputFile: "未设置输出文件",
    missingFile: "文件不存在",
    files: "文件",
    noFoldersSelected: "未选择目录",
    releaseFolders: "松开鼠标，将目录加入输入列表",
    options: "选项",
    outputFile: "输出文件",
    browse: "浏览",
    selectOutputFile: "选择输出文件",
    selectOutputPlaceholder: "选择输出文件",
    selectInputFirst: "请先选择输入目录，程序会生成默认输出文件。",
    leaveBlankPrefix: "留空则使用",
    alignToGrid: "对齐 Tile 网格",
    gridCellSize: "网格尺寸",
    gridCellSizeHelp: "精灵会对齐到完整 {size}x{size} 格子。Tileset 流程通常建议关闭裁剪，并将 padding/extrude 设为 0。",
    sliceGridCells: "切分网格格子",
    sliceGridCellsOn: "开启：透明网格空洞会参与形状打包，连续的实心块仍保持合并。",
    sliceGridCellsOff: "关闭：每张源图保持为一个精灵，只按外围完整透明格裁剪。",
    maxSize: "最大尺寸",
    maxSizeGridHelp: "网格模式下，最大尺寸会规范为 {size} 的倍数；开启 2 的幂时会使用 2 的幂格子数。",
    padding: "间距",
    extrude: "边缘扩展",
    trimTransparent: "裁剪透明边",
    allowRotation: "允许旋转",
    powerOfTwo: "2 的幂尺寸",
    squareAtlas: "正方形图集",
    separateInputFolders: "输入目录分别打包",
    separateInputFoldersHelp: "加载多个目录时，每个输入目录会单独打包成自己的 atlas 图页，而不是共享一个合并布局。",
    forceMaxAtlasSize: "强制最大图集尺寸",
    forceMaxAtlasSizeHelp: "每张输出 atlas 都保持当前最大尺寸；即使只有少量图片，也会输出同样大小的空白图页。",
    splitMode: "拆分模式",
    outputFormat: "输出格式",
    debugJson: "调试 JSON",
    showBounds: "显示边框",
    atlasWall: "图集预览",
    zoomOut: "缩小",
    zoomIn: "放大",
    resetZoom: "重置缩放",
    generatingPreview: "正在生成图集预览...",
    previewAfterProcessing: "输入目录处理完成后会在这里显示预览。",
    dropFoldersOrInput: "拖入目录，或点击输入目录。",
    atlasCount: "张图集",
    spritesCount: "个精灵",
    wheelZoom: "滚轮 = 缩放",
    dragPan: "拖拽 = 平移",
    clear: "清空",
    newProjectTitle: "新建空白项目",
    openProjectTitle: "打开项目文件",
    recentProjectsTitle: "最近项目",
    saveProjectTitle: "保存项目文件",
    projectFileType: "HollowAtlas 项目",
    saveProjectDialogTitle: "保存 HollowAtlas 项目",
    chooseInputTitle: "选择输入目录",
    packTitle: "开始打包",
    revealTitle: "打开输出文件所在目录",
    clearTitle: "清空当前项目",
    scanFoldersLog: "扫描目录：",
    imagesFoundLog: "发现图片：",
    projectLoadedLog: "项目已打开：",
    projectNoInputsLog: "项目已打开，但没有配置输入目录。",
    chooseInputWarning: "请先选择至少一个输入目录。",
    packStartedLog: "开始打包。",
    chooseOutputWarning: "请先选择输出文件。",
    projectSavedLog: "项目已保存：",
    recentClearedLog: "最近项目列表已清空。",
    newBlankProjectLog: "新的空白项目已准备好。",
    droppedPathsLog: "已拖入 {count} 个路径。",
    previewFailedLog: "预览失败：",
    dragDropFailedLog: "启用拖拽失败：",
  },
} as const;

type Language = keyof typeof translations;
type TranslationKey = keyof typeof translations.en;

function translate(language: Language, key: TranslationKey) {
  return translations[language][key] ?? translations.en[key];
}

function makeTranslator(language: Language) {
  return (key: TranslationKey) => translate(language, key);
}

function normalizeLanguage(value: string | null | undefined): Language {
  return value === "en" ? "en" : "zh";
}

const defaultConfig: PackConfig = {
  max_size: 2048,
  padding: 2,
  extrude: 1,
  trim: true,
  align_to_grid: false,
  grid_cell_size: defaultGridCellSize,
  slice_grid_cells: true,
  allow_rotation: false,
  power_of_two: true,
  square: true,
  separate_input_folders: false,
  force_max_atlas_size: false,
  split_mode: "all_in_one",
  output_format: "godot_tpsheet",
  debug_json: false,
};

function defaultLogs(language: Language = "zh"): LogMessage[] {
  return [{ level: "info", message: translate(language, "ready") }];
}

function trimTrailingSeparators(path: string) {
  return path.replace(/[\\/]+$/, "");
}

function normalizeInputPaths(paths: string | string[] | null | undefined) {
  const rawPaths = Array.isArray(paths) ? paths : paths ? [paths] : [];
  const normalized: string[] = [];
  const seen = new Set<string>();

  rawPaths.forEach((path) => {
    const trimmed = path.trim();
    if (!trimmed) return;

    const key = trimTrailingSeparators(trimmed).toLocaleLowerCase();
    if (seen.has(key)) return;

    seen.add(key);
    normalized.push(trimmed);
  });

  return normalized;
}

function mergeInputPaths(current: string[], incoming: string[]) {
  return normalizeInputPaths([...current, ...incoming]);
}

function primaryInputPath(paths: string | string[] | null | undefined) {
  return normalizeInputPaths(paths)[0] ?? "";
}

function formatInputPaths(paths: string[], language: Language = "zh") {
  if (paths.length === 0) return "";
  if (paths.length === 1) return paths[0];
  return language === "zh" ? `${paths.length} 个目录：${paths.join("; ")}` : `${paths.length} folders: ${paths.join("; ")}`;
}

function joinPath(base: string, leaf: string) {
  if (!base) return leaf;
  return `${trimTrailingSeparators(base)}/${leaf}`;
}

function outputExtensionForFormat(format: PackConfig["output_format"]) {
  return format === "json_debug" ? "json" : "tpsheet";
}

function outputFilterName(format: PackConfig["output_format"], language: Language) {
  if (format === "json_debug") return language === "zh" ? "JSON 输出" : "JSON Output";
  return language === "zh" ? "Godot TexturePacker 表" : "Godot TexturePacker Sheet";
}

function buildDefaultOutputPath(inputPaths: string | string[], config: PackConfig = defaultConfig) {
  const inputPath = primaryInputPath(inputPaths);
  return inputPath ? joinPath(inputPath, `atlas.${outputExtensionForFormat(config.output_format)}`) : "";
}

function ensureOutputExtension(path: string, format: PackConfig["output_format"]) {
  if (!path) return path;
  const extension = outputExtensionForFormat(format);
  if (/\.(tpsheet|json)$/i.test(path)) {
    return path.replace(/\.(tpsheet|json)$/i, `.${extension}`);
  }
  return `${path}.${extension}`;
}

function buildDefaultProjectPath(inputPaths: string | string[]) {
  const cleaned = trimTrailingSeparators(primaryInputPath(inputPaths));
  if (!cleaned) return `hollowatlas.${projectExtension}`;
  const parts = cleaned.split(/[\\/]/).filter(Boolean);
  const folderName = parts.length > 0 ? parts[parts.length - 1] : "hollowatlas";
  return joinPath(cleaned, `${folderName}.${projectExtension}`);
}

function ensureProjectExtension(path: string) {
  return /\.(hollowatlasproj|json)$/i.test(path) ? path : `${path}.${projectExtension}`;
}

function clamp(value: number, min: number, max: number) {
  return Math.min(max, Math.max(min, value));
}

function previousPowerOfTwo(value: number) {
  let current = 1;
  while (current * 2 <= value) current *= 2;
  return current;
}

function normalizePackConfig(config: PackConfig): PackConfig {
  const gridCellSize = Math.max(1, config.grid_cell_size || defaultGridCellSize);
  let maxSize = Math.max(1, config.max_size || 1);

  if (config.align_to_grid) {
    const availableCells = Math.max(1, Math.floor(maxSize / gridCellSize));
    const normalizedCells = config.power_of_two ? previousPowerOfTwo(availableCells) : availableCells;
    maxSize = normalizedCells * gridCellSize;
  }

  return {
    ...config,
    grid_cell_size: gridCellSize,
    max_size: maxSize,
    allow_rotation: config.align_to_grid ? false : config.allow_rotation,
  };
}

function buildMaxSizeOptions(config: PackConfig) {
  if (!config.align_to_grid) {
    return baseMaxSizeOptions;
  }

  const options = gridCellCountOptions
    .map((cellCount) => cellCount * Math.max(1, config.grid_cell_size))
    .filter((value) => value <= 8192);
  if (!options.includes(config.max_size)) {
    options.push(config.max_size);
  }
  return options.sort((left, right) => left - right);
}

function formatRecentTime(timestamp: number, language: Language = "zh") {
  return new Intl.DateTimeFormat(language === "zh" ? "zh-CN" : undefined, {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(timestamp * 1000));
}

type AtlasPlacement = {
  atlas: AtlasResult;
  index: number;
  x: number;
  y: number;
  imageY: number;
};

function layoutAtlasWall(atlases: AtlasResult[]) {
  const gap = 48;
  const padding = 48;
  const titleHeight = 28;
  const totalArea = atlases.reduce((sum, atlas) => sum + atlas.width * atlas.height, 0);
  const rowTarget = clamp(Math.round(Math.sqrt(Math.max(totalArea, 1)) * 1.6), 1200, 3600);

  const items: AtlasPlacement[] = [];
  let cursorX = padding;
  let cursorY = padding;
  let rowHeight = 0;
  let maxRight = padding;

  atlases.forEach((atlas, index) => {
    const blockWidth = atlas.width;
    const blockHeight = atlas.height + titleHeight;

    if (cursorX > padding && cursorX + blockWidth > rowTarget) {
      cursorX = padding;
      cursorY += rowHeight + gap;
      rowHeight = 0;
    }

    items.push({
      atlas,
      index,
      x: cursorX,
      y: cursorY,
      imageY: cursorY + titleHeight,
    });

    cursorX += blockWidth + gap;
    rowHeight = Math.max(rowHeight, blockHeight);
    maxRight = Math.max(maxRight, cursorX - gap);
  });

  return {
    items,
    contentWidth: Math.max(maxRight + padding, padding * 2),
    contentHeight: Math.max(cursorY + rowHeight + padding, padding * 2),
    titleHeight,
  };
}

function drawCheckerboard(context: CanvasRenderingContext2D, x: number, y: number, width: number, height: number) {
  const cell = 24;
  context.save();
  context.beginPath();
  context.rect(x, y, width, height);
  context.clip();
  context.fillStyle = "#eef1f4";
  context.fillRect(x, y, width, height);

  for (let row = 0; row * cell < height; row += 1) {
    for (let column = 0; column * cell < width; column += 1) {
      if ((row + column) % 2 === 0) {
        context.fillStyle = "#dfe5ec";
      } else {
        context.fillStyle = "#eef1f4";
      }
      context.fillRect(x + column * cell, y + row * cell, cell, cell);
    }
  }

  context.restore();
}

function drawGridOverlay(
  context: CanvasRenderingContext2D,
  x: number,
  y: number,
  width: number,
  height: number,
  cellSize: number,
) {
  const cell = Math.max(1, cellSize);
  context.save();
  context.beginPath();
  context.rect(x, y, width, height);
  context.clip();

  context.strokeStyle = "rgba(30, 92, 168, 0.55)";
  context.lineWidth = 1;

  for (let offsetX = 0; offsetX <= width; offsetX += cell) {
    context.beginPath();
    context.moveTo(x + offsetX + 0.5, y);
    context.lineTo(x + offsetX + 0.5, y + height);
    context.stroke();
  }

  for (let offsetY = 0; offsetY <= height; offsetY += cell) {
    context.beginPath();
    context.moveTo(x, y + offsetY + 0.5);
    context.lineTo(x + width, y + offsetY + 0.5);
    context.stroke();
  }

  context.restore();
}

function App() {
  const [language, setLanguage] = useState<Language>("zh");
  const t = useMemo(() => makeTranslator(language), [language]);
  const [inputPaths, setInputPaths] = useState<string[]>([]);
  const [outputPath, setOutputPath] = useState("");
  const [projectPath, setProjectPath] = useState("");
  const [recentProjects, setRecentProjects] = useState<RecentProject[]>([]);
  const [recentMenuOpen, setRecentMenuOpen] = useState(false);
  const [scan, setScan] = useState<ScanResult | null>(null);
  const [result, setResult] = useState<PackResult | null>(null);
  const [config, setConfig] = useState<PackConfig>(normalizePackConfig(defaultConfig));
  const [logs, setLogs] = useState<LogMessage[]>(() => defaultLogs(language));
  const [busy, setBusy] = useState(false);
  const [showBounds, setShowBounds] = useState(true);
  const [dragActive, setDragActive] = useState(false);
  const previewRequestRef = useRef(0);
  const recentMenuRef = useRef<HTMLDivElement | null>(null);
  const inputPath = inputPaths[0] ?? "";

  const pushLog = useCallback((level: LogMessage["level"], message: string) => {
    setLogs((current) => [...current, { level, message }]);
  }, []);

  const refreshRecentProjects = useCallback(async () => {
    const projects = await invoke<RecentProject[]>("get_recent_projects");
    setRecentProjects(projects);
    return projects;
  }, []);

  const rememberRecentProject = useCallback(async (path: string) => {
    const projects = await invoke<RecentProject[]>("record_recent_project", { path });
    setRecentProjects(projects);
    return projects;
  }, []);

  const scanFolders = useCallback(async (paths: string[]) => {
    const nextPaths = normalizeInputPaths(paths);
    if (nextPaths.length === 0) return;
    setBusy(true);
    setScan(null);
    setResult(null);
    try {
      pushLog("info", `${t("scanFoldersLog")} ${formatInputPaths(nextPaths, language)}`);
      const nextScan = await invoke<ScanResult>("scan_folders", { paths: nextPaths });
      setScan(nextScan);
      pushLog("success", `${t("imagesFoundLog")} ${nextScan.total_images}`);
      nextScan.warnings.forEach((warning) => pushLog("warning", warning));
    } catch (error) {
      pushLog("error", String(error));
    } finally {
      setBusy(false);
    }
  }, [language, pushLog, t]);

  const generatePreview = useCallback(async (paths: string[], nextConfig: PackConfig) => {
    const nextPaths = normalizeInputPaths(paths);
    if (nextPaths.length === 0) return;
    const requestId = ++previewRequestRef.current;
    setBusy(true);
    try {
      const nextResult = await invoke<PackResult>("preview_folders", {
        inputPaths: nextPaths,
        config: nextConfig,
      });
      if (previewRequestRef.current !== requestId) return;
      setResult(nextResult);
    } catch (error) {
      if (previewRequestRef.current !== requestId) return;
      pushLog("error", `${t("previewFailedLog")} ${String(error)}`);
    } finally {
      if (previewRequestRef.current === requestId) {
        setBusy(false);
      }
    }
  }, [pushLog, t]);

  const loadInputFolders = useCallback(async (paths: string[], options: { append?: boolean } = {}) => {
    const incomingPaths = normalizeInputPaths(paths);
    if (incomingPaths.length === 0) return;

    const nextInputPaths = options.append ? mergeInputPaths(inputPaths, incomingPaths) : incomingPaths;
    const currentDefault = buildDefaultOutputPath(inputPaths, config);
    const nextOutputPath = !outputPath || outputPath === currentDefault
      ? buildDefaultOutputPath(nextInputPaths, config)
      : outputPath;

    setInputPaths(nextInputPaths);
    setOutputPath(nextOutputPath);
    await scanFolders(nextInputPaths);
  }, [config, inputPaths, outputPath, scanFolders]);

  const loadProjectFromPath = useCallback(async (path: string) => {
    const project = await invoke<ProjectFile>("load_project_file", { path });
    const nextConfig = normalizePackConfig({ ...defaultConfig, ...project.config });
    const nextInputPaths = normalizeInputPaths(
      project.input_paths && project.input_paths.length > 0 ? project.input_paths : project.input_path,
    );
    const nextOutputPath = project.output_path || buildDefaultOutputPath(nextInputPaths, nextConfig);

    setProjectPath(path);
    setInputPaths(nextInputPaths);
    setOutputPath(nextOutputPath);
    setConfig(nextConfig);
    setLanguage(normalizeLanguage(project.language));
    setShowBounds(project.show_bounds ?? true);
    setResult(null);

    if (nextInputPaths.length > 0) {
      await scanFolders(nextInputPaths);
      await rememberRecentProject(path);
      pushLog("success", `${t("projectLoadedLog")} ${path}`);
    } else {
      setScan(null);
      await rememberRecentProject(path);
      pushLog("warning", t("projectNoInputsLog"));
    }
  }, [pushLog, rememberRecentProject, scanFolders, t]);

  async function chooseInput() {
    const selected = await open({ directory: true, multiple: true });
    const selectedPaths = normalizeInputPaths(selected);
    if (selectedPaths.length === 0) return;
    await loadInputFolders(selectedPaths);
  }

  async function chooseOutput() {
    const extension = outputExtensionForFormat(config.output_format);
    const selected = await saveDialog({
      title: t("selectOutputFile"),
      defaultPath: outputPath || buildDefaultOutputPath(inputPaths, config),
      filters: [{ name: outputFilterName(config.output_format, language), extensions: [extension] }],
    });
    if (typeof selected === "string") setOutputPath(ensureOutputExtension(selected, config.output_format));
  }

  async function pack() {
    if (inputPaths.length === 0) {
      pushLog("warning", t("chooseInputWarning"));
      return;
    }
    previewRequestRef.current += 1;
    const finalOutput = ensureOutputExtension(outputPath || buildDefaultOutputPath(inputPaths, config), config.output_format);
    setOutputPath(finalOutput);
    setBusy(true);
    try {
      pushLog("info", t("packStartedLog"));
      const nextResult = await invoke<PackResult>("pack_folders", {
        inputPaths,
        outputPath: finalOutput,
        config,
      });
      setResult(nextResult);
      nextResult.logs.forEach((log) => pushLog(log.level, log.message));
    } catch (error) {
      pushLog("error", String(error));
    } finally {
      setBusy(false);
    }
  }

  async function openOutputFolder() {
    const finalOutput = ensureOutputExtension(outputPath || buildDefaultOutputPath(inputPaths, config), config.output_format);
    if (!finalOutput) {
      pushLog("warning", t("chooseOutputWarning"));
      return;
    }
    setOutputPath(finalOutput);
    try {
      await invoke("open_output_folder", { path: finalOutput });
    } catch (error) {
      pushLog("error", String(error));
    }
  }

  async function openProject() {
    const selected = await open({
      multiple: false,
      filters: [{ name: t("projectFileType"), extensions: [projectExtension, "json"] }],
    });
    if (typeof selected !== "string") return;

    try {
      await loadProjectFromPath(selected);
      setRecentMenuOpen(false);
    } catch (error) {
      pushLog("error", String(error));
    }
  }

  async function openRecentProject(path: string) {
    try {
      await loadProjectFromPath(path);
      setRecentMenuOpen(false);
    } catch (error) {
      pushLog("error", String(error));
      try {
        await refreshRecentProjects();
      } catch (refreshError) {
        pushLog("error", String(refreshError));
      }
    }
  }

  async function saveProject() {
    try {
      const selected = projectPath || await saveDialog({
        title: t("saveProjectDialogTitle"),
        defaultPath: projectPath || buildDefaultProjectPath(inputPaths),
        filters: [{ name: t("projectFileType"), extensions: [projectExtension] }],
      });
      if (!selected) return;

      const finalPath = ensureProjectExtension(selected);
      const project: ProjectFile = {
        version: 1,
        input_path: inputPath,
        input_paths: inputPaths,
        output_path: outputPath,
        config,
        show_bounds: showBounds,
        language,
      };

      await invoke("save_project_file", { path: finalPath, project });
      setProjectPath(finalPath);
      await rememberRecentProject(finalPath);
      setRecentMenuOpen(false);
      pushLog("success", `${t("projectSavedLog")} ${finalPath}`);
    } catch (error) {
      pushLog("error", String(error));
    }
  }

  async function clearRecentProjectList() {
    try {
      await invoke("clear_recent_projects");
      setRecentProjects([]);
      setRecentMenuOpen(false);
      pushLog("success", t("recentClearedLog"));
    } catch (error) {
      pushLog("error", String(error));
    }
  }

  function clear() {
    previewRequestRef.current += 1;
    setInputPaths([]);
    setOutputPath("");
    setProjectPath("");
    setScan(null);
    setResult(null);
    setConfig(normalizePackConfig({ ...defaultConfig }));
    setShowBounds(true);
    setLogs(defaultLogs(language));
    setDragActive(false);
  }

  function newProject() {
    clear();
    setRecentMenuOpen(false);
    pushLog("success", t("newBlankProjectLog"));
  }

  useEffect(() => {
    refreshRecentProjects().catch((error) => pushLog("error", String(error)));
  }, [pushLog, refreshRecentProjects]);

  useEffect(() => {
    if (!recentMenuOpen) return;

    function handlePointerDown(event: PointerEvent) {
      if (!recentMenuRef.current?.contains(event.target as Node)) {
        setRecentMenuOpen(false);
      }
    }

    window.addEventListener("pointerdown", handlePointerDown);
    return () => window.removeEventListener("pointerdown", handlePointerDown);
  }, [recentMenuOpen]);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let cancelled = false;

    getCurrentWebviewWindow()
      .onDragDropEvent((event) => {
        if (event.payload.type === "enter" || event.payload.type === "over") {
          setDragActive(true);
          return;
        }

        if (event.payload.type === "leave") {
          setDragActive(false);
          return;
        }

        setDragActive(false);
        const droppedPaths = normalizeInputPaths(event.payload.paths);
        if (droppedPaths.length === 0) return;

        pushLog("info", t("droppedPathsLog").replace("{count}", String(droppedPaths.length)));
        loadInputFolders(droppedPaths, { append: true }).catch((error: unknown) => pushLog("error", String(error)));
      })
      .then((dispose) => {
        if (cancelled) {
          dispose();
        } else {
          unlisten = dispose;
        }
      })
      .catch((error) => pushLog("error", `${t("dragDropFailedLog")} ${String(error)}`));

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [loadInputFolders, pushLog, t]);

  useEffect(() => {
    if (!scan || inputPaths.length === 0) return;

    const timer = window.setTimeout(() => {
      generatePreview(inputPaths, config).catch((error) => {
        pushLog("error", `${t("previewFailedLog")} ${String(error)}`);
      });
    }, 120);

    return () => {
      window.clearTimeout(timer);
      previewRequestRef.current += 1;
    };
  }, [scan, inputPaths, config, generatePreview, pushLog, t]);

  return (
    <main className="app">
      <header className="toolbar">
        <label className="language-picker">
          <span>{t("language")}</span>
          <select value={language} onChange={(event) => setLanguage(normalizeLanguage(event.target.value))}>
            <option value="zh">{t("chinese")}</option>
            <option value="en">{t("english")}</option>
          </select>
        </label>
        <button className="new-project" onClick={newProject} title={t("newProjectTitle")}>
          <FilePlus2 size={18} />
          <span>{t("newProject")}</span>
        </button>
        <div className="split-button" ref={recentMenuRef}>
          <button onClick={openProject} title={t("openProjectTitle")}>
            <FolderOpen size={18} />
            <span>{t("openProject")}</span>
          </button>
          <button
            className="split-toggle"
            onClick={() => setRecentMenuOpen((value) => !value)}
            title={t("recentProjectsTitle")}
          >
            <ChevronDown size={16} />
          </button>
          {recentMenuOpen && (
            <div className="recent-menu">
              <div className="recent-menu-header">{t("recentProjects")}</div>
              {recentProjects.length > 0 ? (
                <div className="recent-menu-list">
                  {recentProjects.map((project) => (
                    <button
                      key={project.path}
                      className={`recent-menu-item ${project.exists ? "" : "missing"}`}
                      onClick={() => openRecentProject(project.path)}
                      title={project.path}
                    >
                      <strong>{project.name}</strong>
                      <span>{project.path}</span>
                      <small>
                        {project.exists ? formatRecentTime(project.last_opened_at, language) : t("missingFile")}
                      </small>
                    </button>
                  ))}
                </div>
              ) : (
                <div className="recent-menu-empty">{t("noRecentProjects")}</div>
              )}
              <button className="recent-menu-clear" onClick={clearRecentProjectList}>
                {t("clearRecent")}
              </button>
            </div>
          )}
        </div>
        <button onClick={saveProject} title={t("saveProjectTitle")}>
          <Save size={18} />
          <span>{t("saveProject")}</span>
        </button>
        <button onClick={chooseInput} title={t("chooseInputTitle")}>
          <FolderOpen size={18} />
          <span>{t("input")}</span>
        </button>
        <button className="primary" onClick={pack} disabled={busy} title={t("packTitle")}>
          {busy ? <Loader2 className="spin" size={18} /> : <Play size={18} />}
          <span>{t("pack")}</span>
        </button>
        <button onClick={openOutputFolder} title={t("revealTitle")}>
          <Boxes size={18} />
          <span>{t("reveal")}</span>
        </button>
        <button onClick={clear} title={t("clearTitle")}>
          <Trash2 size={18} />
        </button>
        <div className="pathline">
          <span><strong>{t("project")}</strong>{projectPath || t("noProjectFile")}</span>
          <span title={inputPaths.join("\n")}><strong>{t("input")}:</strong>{formatInputPaths(inputPaths, language) || t("noInputFolders")}</span>
          <span><strong>{t("output")}</strong>{outputPath || buildDefaultOutputPath(inputPaths, config) || t("noOutputFile")}</span>
        </div>
      </header>

      <section className="workspace">
        <FileTree scan={scan} t={t} />
        <AtlasPreview
          atlases={result?.atlases ?? []}
          busy={busy}
          hasScan={Boolean(scan)}
          showBounds={showBounds}
          alignToGrid={config.align_to_grid}
          gridCellSize={config.grid_cell_size}
          recentProjects={recentProjects}
          openRecentProject={openRecentProject}
          clearRecentProjects={clearRecentProjectList}
          language={language}
          t={t}
        />
        <OptionsPanel
          config={config}
          setConfig={setConfig}
          inputPaths={inputPaths}
          outputPath={outputPath}
          setOutputPath={setOutputPath}
          chooseOutput={chooseOutput}
          showBounds={showBounds}
          setShowBounds={setShowBounds}
          language={language}
          t={t}
        />
        {dragActive && (
          <div className="drop-overlay">
            <div className="drop-overlay-content">
              <FolderOpen size={36} />
              <span>{t("releaseFolders")}</span>
            </div>
          </div>
        )}
      </section>

      <LogPanel logs={logs} />
    </main>
  );
}

function FileTree({ scan, t }: { scan: ScanResult | null; t: (key: TranslationKey) => string }) {
  return (
    <aside className="panel tree-panel">
      <div className="panel-title">
        <FolderOpen size={17} />
        <span>{t("files")}</span>
      </div>
      <div className="tree-scroll">
        {scan ? <TreeNode node={scan.root} root /> : <div className="empty">{t("noFoldersSelected")}</div>}
      </div>
    </aside>
  );
}

function TreeNode({ node, root = false }: { node: FileTreeNode; root?: boolean }) {
  const [openNode, setOpenNode] = useState(root);
  const isDirectory = node.type === "directory";
  return (
    <div className="tree-node">
      <button
        className={`tree-row ${isDirectory ? "directory" : "image"}`}
        onClick={() => isDirectory && setOpenNode((value) => !value)}
      >
        {isDirectory ? (
          openNode ? <ChevronDown size={15} /> : <ChevronRight size={15} />
        ) : (
          <ImageIcon size={15} />
        )}
        <span title={node.path || node.name}>{node.name}</span>
        {isDirectory && <b>{node.imageCount}</b>}
      </button>
      {isDirectory && openNode && (
        <div className="tree-children">
          {node.children.map((child) => (
            <TreeNode key={`${child.type}:${child.path}`} node={child} />
          ))}
        </div>
      )}
    </div>
  );
}

function OptionsPanel({
  config,
  setConfig,
  inputPaths,
  outputPath,
  setOutputPath,
  chooseOutput,
  showBounds,
  setShowBounds,
  language,
  t,
}: {
  config: PackConfig;
  setConfig: React.Dispatch<React.SetStateAction<PackConfig>>;
  inputPaths: string[];
  outputPath: string;
  setOutputPath: React.Dispatch<React.SetStateAction<string>>;
  chooseOutput: () => Promise<void>;
  showBounds: boolean;
  setShowBounds: (value: boolean) => void;
  language: Language;
  t: (key: TranslationKey) => string;
}) {
  function update<K extends keyof PackConfig>(key: K, value: PackConfig[K]) {
    if (key === "output_format") {
      const nextFormat = value as PackConfig["output_format"];
      const currentDefault = buildDefaultOutputPath(inputPaths, config);
      setOutputPath((current) => {
        if (!current) return current;
        if (current === currentDefault || /\.(tpsheet|json)$/i.test(current)) {
          return ensureOutputExtension(current, nextFormat);
        }
        return current;
      });
    }
    setConfig((current) => normalizePackConfig({ ...current, [key]: value }));
  }

  const defaultOutputPath = buildDefaultOutputPath(inputPaths, config);
  const maxSizeOptions = buildMaxSizeOptions(config);

  return (
    <aside className="panel options-panel">
      <div className="panel-title">
        <Settings2 size={17} />
        <span>{t("options")}</span>
      </div>
      <label className="path-field">
        <span>{t("outputFile")}</span>
        <div className="path-field-row">
          <input
            value={outputPath}
            onChange={(event) => setOutputPath(event.target.value)}
            placeholder={defaultOutputPath || t("selectOutputPlaceholder")}
          />
          <button type="button" onClick={chooseOutput}>{t("browse")}</button>
        </div>
        <small>
          {defaultOutputPath
            ? `${t("leaveBlankPrefix")} ${defaultOutputPath}`
            : t("selectInputFirst")}
        </small>
      </label>
      <Toggle label={t("alignToGrid")} value={config.align_to_grid} setValue={(value) => update("align_to_grid", value)} />
      {config.align_to_grid && (
        <>
          <label>
            <span>{t("gridCellSize")}</span>
            <select value={config.grid_cell_size} onChange={(event) => update("grid_cell_size", Number(event.target.value))}>
              {[16, 24, 32, 48, 64, 96, 128].map((value) => (
                <option key={value} value={value}>
                  {value} x {value}
                </option>
              ))}
            </select>
            <small>
              {t("gridCellSizeHelp").replace(/\{size\}/g, String(config.grid_cell_size))}
            </small>
          </label>
          <Toggle label={t("sliceGridCells")} value={config.slice_grid_cells} setValue={(value) => update("slice_grid_cells", value)} />
          <label>
            <small>
              {config.slice_grid_cells
                ? t("sliceGridCellsOn")
                : t("sliceGridCellsOff")}
            </small>
          </label>
        </>
      )}
      <label>
        <span>{t("maxSize")}</span>
        <select value={config.max_size} onChange={(event) => update("max_size", Number(event.target.value))}>
          {maxSizeOptions.map((value) => (
            <option key={value} value={value}>
              {value}
            </option>
          ))}
        </select>
        {config.align_to_grid && (
          <small>
            {t("maxSizeGridHelp").replace("{size}", String(config.grid_cell_size))}
          </small>
        )}
      </label>
      <label>
        <span>{t("padding")}</span>
        <select value={config.padding} onChange={(event) => update("padding", Number(event.target.value))}>
          {[0, 1, 2, 4, 8].map((value) => (
            <option key={value} value={value}>
              {value}
            </option>
          ))}
        </select>
      </label>
      <label>
        <span>{t("extrude")}</span>
        <select value={config.extrude} onChange={(event) => update("extrude", Number(event.target.value))}>
          {[0, 1, 2, 4].map((value) => (
            <option key={value} value={value}>
              {value}
            </option>
          ))}
        </select>
      </label>
      <Toggle label={t("trimTransparent")} value={config.trim} setValue={(value) => update("trim", value)} />
      <Toggle label={t("allowRotation")} value={config.allow_rotation} setValue={(value) => update("allow_rotation", value)} />
      <Toggle label={t("powerOfTwo")} value={config.power_of_two} setValue={(value) => update("power_of_two", value)} />
      <Toggle label={t("squareAtlas")} value={config.square} setValue={(value) => update("square", value)} />
      <Toggle label={t("separateInputFolders")} value={config.separate_input_folders} setValue={(value) => update("separate_input_folders", value)} />
      <label>
        <small>
          {t("separateInputFoldersHelp")}
        </small>
      </label>
      <Toggle label={t("forceMaxAtlasSize")} value={config.force_max_atlas_size} setValue={(value) => update("force_max_atlas_size", value)} />
      <label>
        <small>
          {t("forceMaxAtlasSizeHelp")}
        </small>
      </label>
      <label>
        <span>{t("splitMode")}</span>
        <select value={config.split_mode} onChange={(event) => update("split_mode", event.target.value as PackConfig["split_mode"])}>
          <option value="all_in_one">{language === "zh" ? "合并打包" : "all_in_one"}</option>
          <option value="by_first_level_folder">{language === "zh" ? "按一级目录" : "by_first_level_folder"}</option>
        </select>
      </label>
      <label>
        <span>{t("outputFormat")}</span>
        <select value={config.output_format} onChange={(event) => update("output_format", event.target.value as PackConfig["output_format"])}>
          <option value="godot_tpsheet">godot_tpsheet</option>
          <option value="json_debug">json_debug</option>
        </select>
      </label>
      <Toggle label={t("debugJson")} value={config.debug_json} setValue={(value) => update("debug_json", value)} />
      <Toggle label={t("showBounds")} value={showBounds} setValue={setShowBounds} />
    </aside>
  );
}

function Toggle({ label, value, setValue }: { label: string; value: boolean; setValue: (value: boolean) => void }) {
  return (
    <button className={`toggle ${value ? "on" : ""}`} onClick={() => setValue(!value)}>
      <span>{label}</span>
      <i>{value && <Check size={14} />}</i>
    </button>
  );
}

function AtlasPreview({
  atlases,
  busy,
  hasScan,
  showBounds,
  alignToGrid,
  gridCellSize,
  recentProjects,
  openRecentProject,
  clearRecentProjects,
  language,
  t,
}: {
  atlases: AtlasResult[];
  busy: boolean;
  hasScan: boolean;
  showBounds: boolean;
  alignToGrid: boolean;
  gridCellSize: number;
  recentProjects: RecentProject[];
  openRecentProject: (path: string) => Promise<void>;
  clearRecentProjects: () => Promise<void>;
  language: Language;
  t: (key: TranslationKey) => string;
}) {
  const [loadedImages, setLoadedImages] = useState<Map<string, HTMLImageElement>>(new Map());
  const [loadError, setLoadError] = useState("");
  const [loadingPreview, setLoadingPreview] = useState(false);
  const [zoom, setZoom] = useState(1);
  const [offset, setOffset] = useState({ x: 0, y: 0 });
  const [isPanning, setIsPanning] = useState(false);
  const [canvasSize, setCanvasSize] = useState({ width: 0, height: 0 });
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const canvasWrapRef = useRef<HTMLDivElement | null>(null);
  const dragRef = useRef<{ x: number; y: number } | null>(null);
  const zoomRef = useRef(1);
  const offsetRef = useRef({ x: 0, y: 0 });
  const fitKeyRef = useRef("");
  const layout = useMemo(() => layoutAtlasWall(atlases), [atlases]);

  useEffect(() => {
    zoomRef.current = zoom;
  }, [zoom]);

  useEffect(() => {
    offsetRef.current = offset;
  }, [offset]);

  useEffect(() => {
    if (atlases.length === 0) {
      fitKeyRef.current = "";
      setLoadedImages(new Map());
      setLoadError("");
      setLoadingPreview(false);
      return;
    }

    let cancelled = false;
    setLoadingPreview(true);
    setLoadError("");
    setLoadedImages(new Map());

    Promise.allSettled(
      atlases.map(async (atlas) => {
        const dataUrl = await invoke<string>("read_image_data_url", { path: atlas.image_path });
        const image = await new Promise<HTMLImageElement>((resolve, reject) => {
          const nextImage = new Image();
          nextImage.onload = () => resolve(nextImage);
          nextImage.onerror = () => reject(new Error(`Failed to render atlas image: ${atlas.image_path}`));
          nextImage.src = dataUrl;
        });
        return { path: atlas.image_path, image };
      }),
    ).then((results) => {
      if (cancelled) return;

      const nextImages = new Map<string, HTMLImageElement>();
      const errors: string[] = [];

      results.forEach((result) => {
        if (result.status === "fulfilled") {
          nextImages.set(result.value.path, result.value.image);
        } else {
          errors.push(result.reason instanceof Error ? result.reason.message : String(result.reason));
        }
      });

      setLoadedImages(nextImages);
      setLoadError(errors[0] ?? "");
      setLoadingPreview(false);
    });

    return () => {
      cancelled = true;
    };
  }, [atlases]);

  useEffect(() => {
    const element = canvasWrapRef.current;
    if (!element) return;

    const updateSize = () => {
      setCanvasSize({
        width: element.clientWidth,
        height: element.clientHeight,
      });
    };

    updateSize();
    const observer = new ResizeObserver(updateSize);
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    if (atlases.length === 0 || canvasSize.width === 0 || canvasSize.height === 0) return;

    const fitKey = atlases.map((atlas) => `${atlas.image_path}:${atlas.width}x${atlas.height}`).join("|");
    if (fitKeyRef.current === fitKey) return;

    const nextZoom = clamp(
      Math.min(
        (canvasSize.width - 96) / Math.max(layout.contentWidth, 1),
        (canvasSize.height - 96) / Math.max(layout.contentHeight, 1),
        1,
      ),
      0.08,
      1,
    );
    const nextOffset = {
      x: (canvasSize.width - layout.contentWidth * nextZoom) / 2,
      y: (canvasSize.height - layout.contentHeight * nextZoom) / 2,
    };

    fitKeyRef.current = fitKey;
    zoomRef.current = nextZoom;
    offsetRef.current = nextOffset;
    setZoom(nextZoom);
    setOffset(nextOffset);
  }, [atlases, canvasSize.height, canvasSize.width, layout.contentHeight, layout.contentWidth]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const width = Math.max(1, Math.floor(canvasSize.width));
    const height = Math.max(1, Math.floor(canvasSize.height));
    const ratio = window.devicePixelRatio || 1;

    canvas.width = Math.max(1, Math.floor(width * ratio));
    canvas.height = Math.max(1, Math.floor(height * ratio));

    const context = canvas.getContext("2d");
    if (!context) return;

    context.setTransform(1, 0, 0, 1, 0, 0);
    context.clearRect(0, 0, canvas.width, canvas.height);
    context.scale(ratio, ratio);
    context.fillStyle = "#ffffff";
    context.fillRect(0, 0, width, height);
    context.imageSmoothingEnabled = false;

    if (atlases.length === 0) return;

    context.save();
    context.translate(offset.x, offset.y);
    context.scale(zoom, zoom);
    context.textBaseline = "top";

    layout.items.forEach((item) => {
      const image = loadedImages.get(item.atlas.image_path);

      context.fillStyle = "#243143";
      context.font = '700 18px "Segoe UI", sans-serif';
      context.fillText(`atlas_${item.index}`, item.x, item.y);

      context.fillStyle = "#5e6c7d";
      context.font = '12px "Segoe UI", sans-serif';
      context.fillText(
        `${item.atlas.width}x${item.atlas.height}  ${item.atlas.sprites.length} ${t("spritesCount")}  ${(item.atlas.usage * 100).toFixed(1)}%`,
        item.x + 92,
        item.y + 4,
      );

      drawCheckerboard(context, item.x, item.imageY, item.atlas.width, item.atlas.height);
      context.strokeStyle = "rgba(33, 46, 64, 0.22)";
      context.lineWidth = 1;
      context.strokeRect(item.x + 0.5, item.imageY + 0.5, item.atlas.width - 1, item.atlas.height - 1);

      if (image) {
        context.drawImage(image, item.x, item.imageY, item.atlas.width, item.atlas.height);
      }

      if (alignToGrid) {
        drawGridOverlay(
          context,
          item.x,
          item.imageY,
          item.atlas.width,
          item.atlas.height,
          gridCellSize,
        );
      }

      if (showBounds) {
        context.strokeStyle = "rgba(28, 112, 204, 0.95)";
        context.lineWidth = 1;
        item.atlas.sprites.forEach((sprite) => {
          context.strokeRect(
            item.x + sprite.x + 0.5,
            item.imageY + sprite.y + 0.5,
            Math.max(1, sprite.w - 1),
            Math.max(1, sprite.h - 1),
          );
        });
      }
    });

    context.restore();
  }, [alignToGrid, atlases, canvasSize.height, canvasSize.width, gridCellSize, layout.items, loadedImages, offset.x, offset.y, showBounds, t, zoom]);

  const totalSprites = useMemo(() => atlases.reduce((sum, atlas) => sum + atlas.sprites.length, 0), [atlases]);
  const showRecentProjects = !busy && !loadingPreview && !hasScan && recentProjects.length > 0;

  const emptyMessage = loadError || (busy || loadingPreview
    ? t("generatingPreview")
    : hasScan
      ? t("previewAfterProcessing")
      : t("dropFoldersOrInput"));
  const showOverlay = atlases.length === 0 || (loadedImages.size === 0 && (loadingPreview || Boolean(loadError)));

  function applyZoom(factor: number, anchor?: { x: number; y: number }) {
    const currentZoom = zoomRef.current;
    const nextZoom = clamp(currentZoom * factor, 0.05, 8);

    if (anchor) {
      const currentOffset = offsetRef.current;
      const worldX = (anchor.x - currentOffset.x) / currentZoom;
      const worldY = (anchor.y - currentOffset.y) / currentZoom;
      const nextOffset = {
        x: anchor.x - worldX * nextZoom,
        y: anchor.y - worldY * nextZoom,
      };
      offsetRef.current = nextOffset;
      setOffset(nextOffset);
    }

    zoomRef.current = nextZoom;
    setZoom(nextZoom);
  }

  function fitView() {
    if (atlases.length === 0 || canvasSize.width === 0 || canvasSize.height === 0) return;

    const nextZoom = clamp(
      Math.min(
        (canvasSize.width - 96) / Math.max(layout.contentWidth, 1),
        (canvasSize.height - 96) / Math.max(layout.contentHeight, 1),
        1,
      ),
      0.08,
      1,
    );
    const nextOffset = {
      x: (canvasSize.width - layout.contentWidth * nextZoom) / 2,
      y: (canvasSize.height - layout.contentHeight * nextZoom) / 2,
    };

    zoomRef.current = nextZoom;
    offsetRef.current = nextOffset;
    setZoom(nextZoom);
    setOffset(nextOffset);
  }

  return (
    <section className="preview-panel">
      <div className="preview-toolbar">
        <div className="panel-title">
          <Eye size={17} />
          <span>{t("atlasWall")}</span>
        </div>
        <div className="preview-actions">
          <span className="zoom-label">{Math.round(zoom * 100)}%</span>
          <button onClick={() => applyZoom(0.85)} title={t("zoomOut")}>
            <ZoomOut size={17} />
          </button>
          <button onClick={() => applyZoom(1.15)} title={t("zoomIn")}>
            <ZoomIn size={17} />
          </button>
          <button
            onClick={fitView}
            title={t("resetZoom")}
          >
            <RotateCcw size={17} />
          </button>
        </div>
      </div>
      <div
        ref={canvasWrapRef}
        className={`preview-canvas ${isPanning ? "dragging" : ""}`}
        onWheel={(event) => {
          if (atlases.length === 0) return;
          event.preventDefault();
          const rect = event.currentTarget.getBoundingClientRect();
          applyZoom(event.deltaY < 0 ? 1.08 : 0.92, {
            x: event.clientX - rect.left,
            y: event.clientY - rect.top,
          });
        }}
        onMouseDown={(event) => {
          if (event.button !== 0) return;
          event.preventDefault();
          dragRef.current = { x: event.clientX, y: event.clientY };
          setIsPanning(true);
        }}
        onMouseMove={(event) => {
          if (!dragRef.current) return;
          const dx = event.clientX - dragRef.current.x;
          const dy = event.clientY - dragRef.current.y;
          dragRef.current = { x: event.clientX, y: event.clientY };
          setOffset((current) => {
            const next = { x: current.x + dx, y: current.y + dy };
            offsetRef.current = next;
            return next;
          });
        }}
        onMouseUp={() => {
          dragRef.current = null;
          setIsPanning(false);
        }}
        onMouseLeave={() => {
          dragRef.current = null;
          setIsPanning(false);
        }}
      >
        <canvas ref={canvasRef} className="preview-draw" />
        {showOverlay && (
          <div className="empty large preview-empty">
            {busy || loadingPreview ? <Loader2 className="spin" size={42} /> : <SquareDashedMousePointer size={42} />}
            <span>{emptyMessage}</span>
            {showRecentProjects && (
              <div className="recent-panel">
                <div className="recent-panel-header">
                  <strong>{t("recentProjects")}</strong>
                  <button type="button" className="recent-panel-clear" onClick={clearRecentProjects}>{t("clear")}</button>
                </div>
                <div className="recent-panel-list">
                  {recentProjects.map((project) => (
                    <button
                      key={project.path}
                      type="button"
                      className={`recent-panel-item ${project.exists ? "" : "missing"}`}
                      onClick={() => openRecentProject(project.path)}
                      title={project.path}
                    >
                      <strong>{project.name}</strong>
                      <span>{project.path}</span>
                      <small>{project.exists ? formatRecentTime(project.last_opened_at, language) : t("missingFile")}</small>
                    </button>
                  ))}
                </div>
              </div>
            )}
          </div>
        )}
      </div>
      <div className="status-strip">
        <span>{atlases.length} {t("atlasCount")}</span>
        <span>{totalSprites} {t("spritesCount")}</span>
        {alignToGrid && <span>{`Grid ${gridCellSize}x${gridCellSize}`}</span>}
        <span>{t("wheelZoom")}</span>
        <span>{t("dragPan")}</span>
      </div>
    </section>
  );
}

function LogPanel({ logs }: { logs: LogMessage[] }) {
  const ref = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    ref.current?.scrollTo({ top: ref.current.scrollHeight });
  }, [logs]);

  return (
    <section className="log-panel" ref={ref}>
      {logs.map((log, index) => (
        <div key={`${index}:${log.message}`} className={`log ${log.level}`}>
          <b>[{log.level}]</b>
          <span>{log.message}</span>
        </div>
      ))}
    </section>
  );
}

createRoot(document.getElementById("root")!).render(<App />);
