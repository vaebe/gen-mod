import { spawn } from "node:child_process";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { collectBuildArtifacts } from "./collect-build-artifacts.mjs";

const root = fileURLToPath(new URL("../", import.meta.url));
const args = process.argv.slice(2);
const child = spawn(process.execPath, [join(root, "node_modules/@tauri-apps/cli/tauri.js"), "build", ...args], {
  cwd: root,
  stdio: "inherit",
  windowsHide: true,
});
const exitCode = await new Promise((resolve, reject) => {
  child.once("error", reject);
  child.once("close", code => resolve(code ?? 1));
});
if (exitCode !== 0) {
  process.exit(exitCode);
}
// 仅默认 Windows x64 release 构建收集固定位置产物，其余参数原样交给 Tauri。
if (args.length === 0) {
  collectBuildArtifacts();
}
