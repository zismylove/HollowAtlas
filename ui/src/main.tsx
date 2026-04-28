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
  split_mode: "all_in_one",
  output_format: "godot_tpsheet",
  debug_json: false,
};

function defaultLogs(): LogMessage[] {
  return [{ level: "info", message: "Ready." }];
}

function trimTrailingSeparators(path: string) {
  return path.replace(/[\\/]+$/, "");
}

function joinPath(base: string, leaf: string) {
  if (!base) return leaf;
  return `${trimTrailingSeparators(base)}/${leaf}`;
}

function buildDefaultOutputPath(inputPath: string) {
  return inputPath ? joinPath(inputPath, "atlas_out") : "";
}

function buildDefaultProjectPath(inputPath: string) {
  const cleaned = trimTrailingSeparators(inputPath);
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

function formatRecentTime(timestamp: number) {
  return new Intl.DateTimeFormat(undefined, {
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
  const [inputPath, setInputPath] = useState("");
  const [outputPath, setOutputPath] = useState("");
  const [projectPath, setProjectPath] = useState("");
  const [recentProjects, setRecentProjects] = useState<RecentProject[]>([]);
  const [recentMenuOpen, setRecentMenuOpen] = useState(false);
  const [scan, setScan] = useState<ScanResult | null>(null);
  const [result, setResult] = useState<PackResult | null>(null);
  const [config, setConfig] = useState<PackConfig>(normalizePackConfig(defaultConfig));
  const [logs, setLogs] = useState<LogMessage[]>(defaultLogs);
  const [busy, setBusy] = useState(false);
  const [showBounds, setShowBounds] = useState(true);
  const [dragActive, setDragActive] = useState(false);
  const previewRequestRef = useRef(0);
  const recentMenuRef = useRef<HTMLDivElement | null>(null);

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

  const scanFolder = useCallback(async (path: string) => {
    if (!path) return;
    setBusy(true);
    setScan(null);
    setResult(null);
    try {
      pushLog("info", `Scan folder: ${path}`);
      const nextScan = await invoke<ScanResult>("scan_folder", { path });
      setScan(nextScan);
      pushLog("success", `Images found: ${nextScan.total_images}`);
      nextScan.warnings.forEach((warning) => pushLog("warning", warning));
    } catch (error) {
      pushLog("error", String(error));
    } finally {
      setBusy(false);
    }
  }, [pushLog]);

  const generatePreview = useCallback(async (path: string, nextConfig: PackConfig) => {
    if (!path) return;
    const requestId = ++previewRequestRef.current;
    setBusy(true);
    try {
      const nextResult = await invoke<PackResult>("preview_folder", {
        inputPath: path,
        config: nextConfig,
      });
      if (previewRequestRef.current !== requestId) return;
      setResult(nextResult);
    } catch (error) {
      if (previewRequestRef.current !== requestId) return;
      pushLog("error", `Preview failed: ${String(error)}`);
    } finally {
      if (previewRequestRef.current === requestId) {
        setBusy(false);
      }
    }
  }, [pushLog]);

  const loadInputFolder = useCallback(async (path: string) => {
    const currentDefault = buildDefaultOutputPath(inputPath);
    const nextOutputPath = !outputPath || outputPath === currentDefault
      ? buildDefaultOutputPath(path)
      : outputPath;

    setInputPath(path);
    setOutputPath(nextOutputPath);
    await scanFolder(path);
  }, [inputPath, outputPath, scanFolder]);

  const loadProjectFromPath = useCallback(async (path: string) => {
    const project = await invoke<ProjectFile>("load_project_file", { path });
    const nextConfig = normalizePackConfig({ ...defaultConfig, ...project.config });
    const nextOutputPath = project.output_path || buildDefaultOutputPath(project.input_path);

    setProjectPath(path);
    setInputPath(project.input_path);
    setOutputPath(nextOutputPath);
    setConfig(nextConfig);
    setShowBounds(project.show_bounds ?? true);
    setResult(null);

    if (project.input_path) {
      await scanFolder(project.input_path);
      await rememberRecentProject(path);
      pushLog("success", `Project loaded: ${path}`);
    } else {
      setScan(null);
      await rememberRecentProject(path);
      pushLog("warning", "Project loaded, but no input folder is configured.");
    }
  }, [pushLog, rememberRecentProject, scanFolder]);

  async function chooseInput() {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected !== "string") return;
    await loadInputFolder(selected);
  }

  async function chooseOutput() {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string") setOutputPath(selected);
  }

  async function pack() {
    if (!inputPath) {
      pushLog("warning", "Choose an input folder first.");
      return;
    }
    previewRequestRef.current += 1;
    const finalOutput = outputPath || buildDefaultOutputPath(inputPath);
    setOutputPath(finalOutput);
    setBusy(true);
    try {
      pushLog("info", "Pack started.");
      const nextResult = await invoke<PackResult>("pack_folder", {
        inputPath,
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
    const finalOutput = outputPath || buildDefaultOutputPath(inputPath);
    if (!finalOutput) {
      pushLog("warning", "Choose an output folder first.");
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
      filters: [{ name: "HollowAtlas Project", extensions: [projectExtension, "json"] }],
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
        title: "Save HollowAtlas Project",
        defaultPath: projectPath || buildDefaultProjectPath(inputPath),
        filters: [{ name: "HollowAtlas Project", extensions: [projectExtension] }],
      });
      if (!selected) return;

      const finalPath = ensureProjectExtension(selected);
      const project: ProjectFile = {
        version: 1,
        input_path: inputPath,
        output_path: outputPath,
        config,
        show_bounds: showBounds,
      };

      await invoke("save_project_file", { path: finalPath, project });
      setProjectPath(finalPath);
      await rememberRecentProject(finalPath);
      setRecentMenuOpen(false);
      pushLog("success", `Project saved: ${finalPath}`);
    } catch (error) {
      pushLog("error", String(error));
    }
  }

  async function clearRecentProjectList() {
    try {
      await invoke("clear_recent_projects");
      setRecentProjects([]);
      setRecentMenuOpen(false);
      pushLog("success", "Recent project list cleared.");
    } catch (error) {
      pushLog("error", String(error));
    }
  }

  function clear() {
    previewRequestRef.current += 1;
    setInputPath("");
    setOutputPath("");
    setProjectPath("");
    setScan(null);
    setResult(null);
    setConfig(normalizePackConfig({ ...defaultConfig }));
    setShowBounds(true);
    setLogs(defaultLogs());
    setDragActive(false);
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
        const droppedPath = event.payload.paths[0];
        if (!droppedPath) return;

        pushLog("info", `Dropped path: ${droppedPath}`);
        loadInputFolder(droppedPath).catch((error) => pushLog("error", String(error)));
      })
      .then((dispose) => {
        if (cancelled) {
          dispose();
        } else {
          unlisten = dispose;
        }
      })
      .catch((error) => pushLog("error", `Failed to enable drag and drop: ${String(error)}`));

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [loadInputFolder, pushLog]);

  useEffect(() => {
    if (!scan || !inputPath) return;

    const timer = window.setTimeout(() => {
      generatePreview(inputPath, config).catch((error) => {
        pushLog("error", `Preview failed: ${String(error)}`);
      });
    }, 120);

    return () => {
      window.clearTimeout(timer);
      previewRequestRef.current += 1;
    };
  }, [scan, inputPath, config, generatePreview, pushLog]);

  return (
    <main className="app">
      <header className="toolbar">
        <div className="split-button" ref={recentMenuRef}>
          <button onClick={openProject} title="打开项目文件">
            <FolderOpen size={18} />
            <span>Open Project</span>
          </button>
          <button
            className="split-toggle"
            onClick={() => setRecentMenuOpen((value) => !value)}
            title="最近项目"
          >
            <ChevronDown size={16} />
          </button>
          {recentMenuOpen && (
            <div className="recent-menu">
              <div className="recent-menu-header">Recent Projects</div>
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
                        {project.exists ? formatRecentTime(project.last_opened_at) : "Missing file"}
                      </small>
                    </button>
                  ))}
                </div>
              ) : (
                <div className="recent-menu-empty">No recent projects yet.</div>
              )}
              <button className="recent-menu-clear" onClick={clearRecentProjectList}>
                Clear Recent
              </button>
            </div>
          )}
        </div>
        <button onClick={saveProject} title="保存项目文件">
          <Save size={18} />
          <span>Save Project</span>
        </button>
        <button onClick={chooseInput} title="选择输入目录">
          <FolderOpen size={18} />
          <span>Input</span>
        </button>
        <button className="primary" onClick={pack} disabled={busy} title="开始打包">
          {busy ? <Loader2 className="spin" size={18} /> : <Play size={18} />}
          <span>Pack</span>
        </button>
        <button onClick={openOutputFolder} title="打开输出目录">
          <Boxes size={18} />
          <span>Open</span>
        </button>
        <button onClick={clear} title="清空当前项目">
          <Trash2 size={18} />
        </button>
        <div className="pathline">
          <span><strong>Project:</strong>{projectPath || "No project file"}</span>
          <span><strong>Input:</strong>{inputPath || "No input folder"}</span>
          <span><strong>Output:</strong>{outputPath || buildDefaultOutputPath(inputPath) || "No output folder"}</span>
        </div>
      </header>

      <section className="workspace">
        <FileTree scan={scan} />
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
        />
        <OptionsPanel
          config={config}
          setConfig={setConfig}
          inputPath={inputPath}
          outputPath={outputPath}
          setOutputPath={setOutputPath}
          chooseOutput={chooseOutput}
          showBounds={showBounds}
          setShowBounds={setShowBounds}
        />
        {dragActive && (
          <div className="drop-overlay">
            <div className="drop-overlay-content">
              <FolderOpen size={36} />
              <span>Release a folder to load it as the input source</span>
            </div>
          </div>
        )}
      </section>

      <LogPanel logs={logs} />
    </main>
  );
}

function FileTree({ scan }: { scan: ScanResult | null }) {
  return (
    <aside className="panel tree-panel">
      <div className="panel-title">
        <FolderOpen size={17} />
        <span>Files</span>
      </div>
      <div className="tree-scroll">
        {scan ? <TreeNode node={scan.root} root /> : <div className="empty">No folder selected</div>}
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
  inputPath,
  outputPath,
  setOutputPath,
  chooseOutput,
  showBounds,
  setShowBounds,
}: {
  config: PackConfig;
  setConfig: React.Dispatch<React.SetStateAction<PackConfig>>;
  inputPath: string;
  outputPath: string;
  setOutputPath: (value: string) => void;
  chooseOutput: () => Promise<void>;
  showBounds: boolean;
  setShowBounds: (value: boolean) => void;
}) {
  function update<K extends keyof PackConfig>(key: K, value: PackConfig[K]) {
    setConfig((current) => normalizePackConfig({ ...current, [key]: value }));
  }

  const defaultOutputPath = buildDefaultOutputPath(inputPath);
  const maxSizeOptions = buildMaxSizeOptions(config);

  return (
    <aside className="panel options-panel">
      <div className="panel-title">
        <Settings2 size={17} />
        <span>Options</span>
      </div>
      <label className="path-field">
        <span>Output Folder</span>
        <div className="path-field-row">
          <input
            value={outputPath}
            onChange={(event) => setOutputPath(event.target.value)}
            placeholder={defaultOutputPath || "Select output folder"}
          />
          <button type="button" onClick={chooseOutput}>Browse</button>
        </div>
        <small>
          {defaultOutputPath
            ? `Leave blank to use ${defaultOutputPath}`
            : "Select an input folder first to generate the default output path."}
        </small>
      </label>
      <Toggle label="Align to Tile Grid" value={config.align_to_grid} setValue={(value) => update("align_to_grid", value)} />
      {config.align_to_grid && (
        <>
          <label>
            <span>Grid Cell Size</span>
            <select value={config.grid_cell_size} onChange={(event) => update("grid_cell_size", Number(event.target.value))}>
              {[16, 24, 32, 48, 64, 96, 128].map((value) => (
                <option key={value} value={value}>
                  {value} x {value}
                </option>
              ))}
            </select>
            <small>
              Sprites snap to whole {config.grid_cell_size}x{config.grid_cell_size} cells. Tileset workflows usually work best with trim off and padding/extrude set to 0.
            </small>
          </label>
          <Toggle label="Slice Grid Cells" value={config.slice_grid_cells} setValue={(value) => update("slice_grid_cells", value)} />
          <label>
            <small>
              {config.slice_grid_cells
                ? "Enabled: transparent grid holes split the image into occupied grid regions, while fully connected blocks stay merged."
                : "Disabled: keep each source image as one sprite, but only trim transparent area by whole grid cells around the outside."}
            </small>
          </label>
        </>
      )}
      <label>
        <span>Max Size</span>
        <select value={config.max_size} onChange={(event) => update("max_size", Number(event.target.value))}>
          {maxSizeOptions.map((value) => (
            <option key={value} value={value}>
              {value}
            </option>
          ))}
        </select>
        {config.align_to_grid && (
          <small>
            In grid mode, max size is normalized to a multiple of {config.grid_cell_size}. With Power of Two on, it uses power-of-two cell counts.
          </small>
        )}
      </label>
      <label>
        <span>Padding</span>
        <select value={config.padding} onChange={(event) => update("padding", Number(event.target.value))}>
          {[0, 1, 2, 4, 8].map((value) => (
            <option key={value} value={value}>
              {value}
            </option>
          ))}
        </select>
      </label>
      <label>
        <span>Extrude</span>
        <select value={config.extrude} onChange={(event) => update("extrude", Number(event.target.value))}>
          {[0, 1, 2, 4].map((value) => (
            <option key={value} value={value}>
              {value}
            </option>
          ))}
        </select>
      </label>
      <Toggle label="Trim Transparent" value={config.trim} setValue={(value) => update("trim", value)} />
      <Toggle label="Allow Rotation" value={config.allow_rotation} setValue={(value) => update("allow_rotation", value)} />
      <Toggle label="Power of Two" value={config.power_of_two} setValue={(value) => update("power_of_two", value)} />
      <Toggle label="Square Atlas" value={config.square} setValue={(value) => update("square", value)} />
      <label>
        <span>Split Mode</span>
        <select value={config.split_mode} onChange={(event) => update("split_mode", event.target.value as PackConfig["split_mode"])}>
          <option value="all_in_one">all_in_one</option>
          <option value="by_first_level_folder">by_first_level_folder</option>
        </select>
      </label>
      <label>
        <span>Output Format</span>
        <select value={config.output_format} onChange={(event) => update("output_format", event.target.value as PackConfig["output_format"])}>
          <option value="godot_tpsheet">godot_tpsheet</option>
          <option value="json_debug">json_debug</option>
        </select>
      </label>
      <Toggle label="Debug JSON" value={config.debug_json} setValue={(value) => update("debug_json", value)} />
      <Toggle label="Show Bounds" value={showBounds} setValue={setShowBounds} />
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
        `${item.atlas.width}x${item.atlas.height}  ${item.atlas.sprites.length} sprites  ${(item.atlas.usage * 100).toFixed(1)}%`,
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
  }, [alignToGrid, atlases, canvasSize.height, canvasSize.width, gridCellSize, layout.items, loadedImages, offset.x, offset.y, showBounds, zoom]);

  const totalSprites = useMemo(() => atlases.reduce((sum, atlas) => sum + atlas.sprites.length, 0), [atlases]);
  const showRecentProjects = !busy && !loadingPreview && !hasScan && recentProjects.length > 0;

  const emptyMessage = loadError || (busy || loadingPreview
    ? "Generating atlas preview..."
    : hasScan
      ? "Preview will appear here after the folder is processed."
      : "Drop a folder or choose Input.");
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
          <span>Atlas Wall</span>
        </div>
        <div className="preview-actions">
          <span className="zoom-label">{Math.round(zoom * 100)}%</span>
          <button onClick={() => applyZoom(0.85)} title="缩小">
            <ZoomOut size={17} />
          </button>
          <button onClick={() => applyZoom(1.15)} title="放大">
            <ZoomIn size={17} />
          </button>
          <button
            onClick={fitView}
            title="重置缩放"
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
                  <strong>Recent Projects</strong>
                  <button type="button" className="recent-panel-clear" onClick={clearRecentProjects}>Clear</button>
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
                      <small>{project.exists ? formatRecentTime(project.last_opened_at) : "Missing file"}</small>
                    </button>
                  ))}
                </div>
              </div>
            )}
          </div>
        )}
      </div>
      <div className="status-strip">
        <span>{atlases.length} atlas</span>
        <span>{totalSprites} sprites</span>
        {alignToGrid && <span>{`Grid ${gridCellSize}x${gridCellSize}`}</span>}
        <span>Wheel = zoom</span>
        <span>Drag = pan</span>
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
