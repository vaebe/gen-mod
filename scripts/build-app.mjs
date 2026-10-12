import { spawn } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { collectBuildArtifacts } from "./collect-build-artifacts.mjs";

const root = fileURLToPath(new URL("../", import.meta.url));
const args = process.argv.slice(2);
// package.json 是唯一版本来源，构建前同步程序与安装包版本。
const { version } = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
if (typeof version !== "string" || !/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(version)) {
  throw new Error("package.json 中的 version 必须是有效的版本号");
}
const cargoPath = join(root, "src-tauri/Cargo.toml");
const cargo = readFileSync(cargoPath, "utf8");
const packageSection = /(^\[package\][^\S\r\n]*\r?\n)([\s\S]*?)(?=^\[|(?![\s\S]))/m;
const section = cargo.match(packageSection);
if (!section || !/^version\s*=\s*"[^"]*"/m.test(section[2])) {
  throw new Error("Cargo.toml 的 [package] 中缺少 version");
}
const nextCargo = cargo.replace(packageSection, (_, header, body) =>
  header + body.replace(/^(version\s*=\s*)"[^"]*"/m, `$1"${version}"`));
const configPath = join(root, "src-tauri/tauri.conf.json");
const configSource = readFileSync(configPath, "utf8");
const config = JSON.parse(configSource);
if (nextCargo !== cargo) {
  writeFileSync(cargoPath, nextCargo);
}
if (config.version !== version) {
  config.version = version;
  writeFileSync(configPath, `${JSON.stringify(config, null, 2)}\n`);
}
console.log(`构建版本：${version}`);
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
