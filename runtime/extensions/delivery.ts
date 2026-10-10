import type { Lease } from "./workspace.js";
import { mkdir, copyFile, writeFile, readFile, lstat } from "node:fs/promises";
import { createHash } from "node:crypto";
import { dirname, resolve } from "node:path";
import { context, contained, safeRelative, runCommand } from "./commands.js";

export interface DeliveryInput { files: { source: string; target: string }[]; commandIds: string[]; targetDirectoryHint: string }
/** 从当前任务上下文取得 staging；模型不能自行指定其他任务或既有版本目录。 */
export async function prepareDelivery(toolCallId: string, input: DeliveryInput, signal?: AbortSignal, lease?: Lease) {
  const task = context();
  if (!input.files.length || !input.commandIds.length) { throw new Error("交付需要文件和本任务成功命令记录"); }
  const targetNames = new Set<string>();
  const sourceNames = new Set<string>();
  const files: { path: string; size: number; sha256: string }[] = [];
  const entries: { source: string; target: string }[] = [];
  await mkdir(resolve(task.stagingDirectory, "files")); // 已存在即拒绝，避免混入旧文件。
  for (const item of input.files) {
    if (signal?.aborted) { throw new Error("交付已取消"); }
    const sourceName = safeRelative(item.source);
    const target = safeRelative(item.target);
    const key = target.toLowerCase();
    if (targetNames.has(key) || sourceNames.has(sourceName.toLowerCase())) { throw new Error("交付包含重复文件或安装目标"); }
    targetNames.add(key); sourceNames.add(sourceName.toLowerCase());
    const source = await contained(task.sourceDirectory, resolve(task.sourceDirectory, sourceName));
    if ((await lstat(resolve(task.sourceDirectory, sourceName))).isSymbolicLink()) { throw new Error("交付不接受链接文件"); }
    const destination = resolve(task.stagingDirectory, "files", target);
    await mkdir(dirname(destination), { recursive: true });
    await copyFile(source, destination);
    const bytes = await readFile(destination);
    files.push({ path: target, size: bytes.length, sha256: createHash("sha256").update(bytes).digest("hex") });
    entries.push({ source: target, target });
  }
  const manifestPath = resolve(task.stagingDirectory, "install.json");
  const zipPath = resolve(task.stagingDirectory, "mod.zip");
  await writeFile(manifestPath, JSON.stringify({ targetDirectoryHint: input.targetDirectoryHint, entries }, null, 2), { flag: "wx" });
  await writeFile(resolve(task.stagingDirectory, "files.json"), JSON.stringify(files, null, 2), { flag: "wx" });
  const scriptPath = resolve(task.stagingDirectory, "package.ps1");
  await writeFile(scriptPath, 'param([string]$Files,[string]$Zip)\n$ErrorActionPreference="Stop"\nAdd-Type -AssemblyName System.IO.Compression.FileSystem\n[IO.Compression.ZipFile]::CreateFromDirectory($Files,$Zip)\n', { flag: "wx" });
  const receipt = await runCommand(toolCallId, "powershell.exe", ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File", scriptPath, resolve(task.stagingDirectory, "files"), zipPath], ".", signal,lease);
  if (receipt.status !== "succeeded") { throw new Error(`ZIP 打包失败，日志：${receipt.logPath}`); }
  return { taskId: task.taskId, stagingId: task.stagingId, commandIds: [...input.commandIds, receipt.commandId], manifestPath, zipPath, commands: [receipt] };
}
