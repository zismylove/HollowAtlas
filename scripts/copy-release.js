import { access, copyFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const filePath = fileURLToPath(import.meta.url);
const scriptDir = path.dirname(filePath);
const rootDir = path.resolve(scriptDir, "..");
const sourceName = process.platform === "win32" ? "hollowatlas-gui.exe" : "hollowatlas-gui";
const targetName = process.platform === "win32" ? "HollowAtlas.exe" : "HollowAtlas";
const sourcePath = path.join(rootDir, "src-tauri", "target", "release", sourceName);
const targetPath = path.join(rootDir, targetName);

try {
  await access(sourcePath);
  await copyFile(sourcePath, targetPath);
  console.log(`Copied ${sourceName} to ${targetName}`);
} catch (error) {
  console.error(`Failed to copy release binary from ${sourcePath}`);
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
}
