import { createHash } from "node:crypto";
import { readFileSync, writeFileSync, mkdirSync, existsSync, renameSync, cpSync, readdirSync, statSync, realpathSync, lstatSync, unlinkSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { isAbsolute, join, relative } from "node:path";
import { spawnSync } from "node:child_process";
import { materialize } from "./materialize-runtime.mjs";

const root = fileURLToPath(new URL("../", import.meta.url));
const lock = JSON.parse(readFileSync(join(root, "runtime/runtime-lock.json"), "utf8"));
const resources = join(root, "runtime/resources");
const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");

function run(program, args) {
  const result = spawnSync(program, args, { cwd: root, stdio: "inherit", windowsHide: true });
  if (result.error || result.status !== 0) {
    throw new Error(`运行时准备命令失败：${program} (${result.status ?? result.error?.message})`);
  }
}

if (process.platform !== lock.node.platform || process.arch !== lock.node.arch) {
  throw new Error(`当前锁文件仅支持 ${lock.node.platform}/${lock.node.arch}`);
}
mkdirSync(resources, { recursive: true });
const archive = join(resources, `${lock.node.directory}.zip`);
if (!existsSync(archive)) {
  console.log(`下载固定 Node ${lock.node.version}…`);
  const response = await fetch(lock.node.url, { signal: AbortSignal.timeout(120_000) });
  if (!response.ok) { throw new Error(`Node 下载失败：HTTP ${response.status}`); }
  const bytes = Buffer.from(await response.arrayBuffer());
  if (hash(bytes) !== lock.node.sha256) { throw new Error("Node 下载校验失败"); }
  writeFileSync(`${archive}.tmp`, bytes);
  renameSync(`${archive}.tmp`, archive);
}
if (hash(readFileSync(archive)) !== lock.node.sha256) { throw new Error("本地 Node ZIP 校验失败"); }
const node = join(resources, lock.node.directory, "node.exe");
if (!existsSync(node)) {
  // 文件路径只作为参数，不插入 PowerShell 代码；脚本通过位置参数获取。
  const expandScript = join(resources, "expand-node.ps1");
  writeFileSync(expandScript, "param([string]$Archive,[string]$Destination)\n$ErrorActionPreference='Stop'\nExpand-Archive -LiteralPath $Archive -DestinationPath $Destination\n");
  run("powershell.exe", ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File", expandScript, archive, resources]);
}
const deploy = join(resources, `pi-v${lock.pi.version}`);
const deploymentMarker = join(deploy, ".deployment-complete");
if (!existsSync(deploymentMarker)) {
  if (existsSync(deploy)) {
    // 只移动本脚本在固定 resources 目录内生成的未完成部署，保留失败现场。
    const previous = join(root, "work", `pi-incomplete-${Date.now()}`);
    if (!relative(resources, deploy).startsWith("..") && !relative(join(root, "work"), previous).startsWith("..")) {
      mkdirSync(join(root, "work"), { recursive: true });
      renameSync(deploy, previous);
    } else { throw new Error("部署恢复路径越界"); }
  }
  // 使用项目固定的 pnpm，不运行依赖生命周期脚本，不引入第二份锁文件。
  run(process.execPath, [join(root, "node_modules/pnpm/bin/pnpm.cjs"), "--filter", "@gen-mod/pi-runtime", "deploy", "--prod", "--legacy", "--ignore-scripts", deploy]);
  writeFileSync(deploymentMarker, lock.pi.version);
}
const installed = JSON.parse(readFileSync(join(deploy, "node_modules", lock.pi.package, "package.json"), "utf8"));
if (installed.version !== lock.pi.version) { throw new Error("部署的 pi 版本与锁文件不一致"); }
const portable = join(resources, `pi-portable-v${lock.pi.version}`);
const portableMarker = join(portable, ".deployment-complete");
const portableRevision = `${lock.pi.version}:no-source-maps-v1`;
if (!existsSync(portableMarker) || readFileSync(portableMarker, "utf8") !== portableRevision) {
  if (existsSync(portable)) {
    // 规则更新时保留旧目录，避免覆盖复制后仍留下被排除的文件。
    const previous = join(root, "work", `pi-portable-before-slim-${Date.now()}`);
    if (!relative(resources, portable).startsWith("..") && !relative(join(root, "work"), previous).startsWith("..")) {
      mkdirSync(join(root, "work"), { recursive: true });
      renameSync(portable, previous);
    } else { throw new Error("运行时恢复路径越界"); }
  }
  console.log("实体化 pi 运行时依赖闭包…");
  const count = materialize(deploy, portable);
  writeFileSync(portableMarker, portableRevision);
  console.log(`复制 ${count} 个运行时依赖目录（不含页面依赖闭包）`);
}
// Tauri 按资源修改时间触发 Rust 重建，内容相同时避免重新复制。
const runtimeCopyOptions = {
  recursive: true,
  filter: (source, destination) => statSync(source).isDirectory() || !existsSync(destination) || !readFileSync(source).equals(readFileSync(destination)),
};
if (existsSync(join(root, "runtime/extensions"))) {
  // cpSync 不移除旧文件；只清理本轮明确退役的两个分发文件。
  for (const name of ["extensions/assets.ts", "helpers/process-image.ps1"]) {
    const target = join(portable, name);
    if (!existsSync(target)) { continue; }
    const relation = relative(realpathSync(portable), realpathSync(target));
    if (isAbsolute(relation) || relation.startsWith("..") || lstatSync(target).isSymbolicLink() || !lstatSync(target).isFile()) { throw new Error("退役资源路径异常，未删除"); }
    unlinkSync(target);
  }
  cpSync(join(root, "runtime/extensions"), join(portable, "extensions"), runtimeCopyOptions);
}
cpSync(join(root, "runtime/helpers"), join(portable, "helpers"), runtimeCopyOptions);

// 记录可审计的资源哈希，发布前检查完整性；不复制项目 env 或用户资料。
const files = [];
const visited = new Set();
function inventory(directory, output = files, base = resources) {
  const canonical = realpathSync(directory);
  if (visited.has(canonical)) { return; }
  visited.add(canonical);
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.name === "node.zip" || entry.name === "resource-manifest.json" || entry.name === ".bin" || !existsSync(path)) { continue; }
    const info = statSync(path);
    if (info.isDirectory()) { inventory(path, output, base); }
    else { output.push({ path: relative(base, path).replaceAll("\\", "/"), size: info.size, sha256: hash(readFileSync(path)) }); }
  }
}
// 与 Tauri 的 Node 文件白名单一致，不将 npm/corepack 等开发工具计入分发清单。
for (const name of ["node.exe", "LICENSE"]) {
  const path = join(resources, lock.node.directory, name);
  const bytes = readFileSync(path);
  files.push({ path: relative(resources, path).replaceAll("\\", "/"), size: bytes.length, sha256: hash(bytes) });
}
inventory(portable);
const skillFiles = [];
inventory(join(root, "skills"), skillFiles, join(root, "skills"));
const manifestPath = join(resources, "resource-manifest.json");
const manifest = JSON.stringify({ pi: lock.pi.version, node: lock.node.version, files, skills: skillFiles }, null, 2);
if (!existsSync(manifestPath) || readFileSync(manifestPath, "utf8") !== manifest) {
  writeFileSync(manifestPath, manifest);
}
console.log(`已准备 Node ${lock.node.version} / pi ${lock.pi.version}，${files.length} 个运行时文件、${skillFiles.length} 个 skills 文件`);
