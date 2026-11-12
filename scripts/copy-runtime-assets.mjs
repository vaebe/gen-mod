import { cpSync, existsSync, statSync, readFileSync, realpathSync, lstatSync, unlinkSync } from "node:fs";
import { join, relative, isAbsolute } from "node:path";

/** 准备阶段仅更新应用运行时源码，并逐项清理已确认退役文件。 */
export function copyRuntimeAssets(root, portable) {
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

}
