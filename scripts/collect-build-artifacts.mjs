import { copyFileSync, mkdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));

/** 收集当前版本 Windows x64 安装包，不混入历史版本或中间文件。 */
export function collectBuildArtifacts() {
  const config = JSON.parse(readFileSync(join(root, "src-tauri/tauri.conf.json"), "utf8"));
  const filename = `${config.productName}_${config.version}_x64-setup.exe`;
  const source = join(root, "src-tauri/target/release/bundle/nsis", filename);
  const destination = join(root, "build-output", filename);
  mkdirSync(join(root, "build-output"), { recursive: true });
  copyFileSync(source, destination);
  console.log(`安装包已收集：${destination}`);
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  collectBuildArtifacts();
}
