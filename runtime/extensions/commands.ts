import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { access, mkdir, open, realpath } from "node:fs/promises";
import { delimiter, dirname, isAbsolute, join, relative, resolve, sep } from "node:path";

export interface TaskContext { taskId: string; modId: string; sessionId: string; generation: number; sourceDirectory: string; stagingId: string; stagingDirectory: string; createdAt: string }
export interface Receipt { taskId: string; commandId: string; toolCallId: string; exitCode: number | null; status: "succeeded" | "failed" | "cancelled"; logPath: string; program: string; args: string[] }
export function context(): TaskContext {
  const raw = process.env.GEN_MOD_TASK_CONTEXT;
  if (!raw) { throw new Error("此连接没有原生任务上下文，不能执行命令或交付"); }
  return JSON.parse(raw) as TaskContext;
}
export async function contained(root: string, path: string): Promise<string> {
  const base = await realpath(root);
  const actual = await realpath(path);
  const relation = relative(base, actual);
  if (isAbsolute(relation) || relation === ".." || relation.startsWith(`..${sep}`)) { throw new Error("文件路径位于允许目录之外"); }
  return actual;
}
export function safeRelative(value: string): string {
  if (!value || isAbsolute(value) || value.includes(":") || value.includes("\\") || value.split("/").some((v) => !v || v === "." || v === "..")) { throw new Error("需使用有效相对路径和 / 分隔符"); }
  return value;
}

let environmentInfo: Promise<{ shell?: string; dotnet?: string }> | undefined;
/** 每个任务进程只探测一次现有程序路径，不执行安装或试探命令。 */
export function commandEnvironment(): Promise<{ shell?: string; dotnet?: string }> {
  environmentInfo ??= (async () => {
    const path = process.env.PATH ?? process.env.Path ?? "";
    const directories = path.split(delimiter).filter(Boolean).map(directory => directory.replace(/^"|"$/g, ""));
    const programFiles = process.env.ProgramFiles;
    const windows = process.env.SystemRoot;
    async function locate(name: string, extra: string[]): Promise<string | undefined> {
      for (const file of [...directories.map(directory => join(directory, name)), ...extra]) {
        try { await access(file); return file; } catch { /* 继续查找已安装程序。 */ }
      }
      return undefined;
    }
    if (process.platform !== "win32") { return {}; }
    const pwsh = await locate("pwsh.exe", programFiles ? [join(programFiles, "PowerShell/7/pwsh.exe")] : []);
    const shell = pwsh ?? await locate("powershell.exe", windows ? [join(windows, "System32/WindowsPowerShell/v1.0/powershell.exe")] : []);
    const dotnet = await locate("dotnet.exe", programFiles ? [join(programFiles, "dotnet/dotnet.exe")] : []);
    return { shell, dotnet };
  })();
  return environmentInfo;
}

/** 不拼接 shell 字符串；进程继承原生 Job，取消时通过 taskkill 收回命令后代。 */
export async function runCommand(toolCallId: string, program: string, args: string[], cwd: string, signal?: AbortSignal): Promise<Receipt> {
  const task = context();
  const directory = await contained(task.sourceDirectory, resolve(task.sourceDirectory, cwd));
  const commandId = randomUUID();
  const logPath = resolve(task.stagingDirectory, "../../commands", `${commandId}.log`);
  await mkdir(dirname(logPath), { recursive: true });
  const log = await open(logPath, "wx");
  const secret = process.env.STEPFUN_API_KEY ?? "";
  const environment = { ...process.env };
  delete environment.STEPFUN_API_KEY;
  const available = await commandEnvironment();
  // GUI 进程的旧 PATH 可能不含已安装 SDK；命令子进程补充实际找到的目录。
  const pathKey = Object.keys(environment).find(key => key.toLowerCase() === "path") ?? "PATH";
  environment[pathKey] = [...new Set([available.shell, available.dotnet].filter((path): path is string => !!path).map(dirname)), environment[pathKey] ?? ""].join(delimiter);
  const redact = (value: string) => secret ? value.replaceAll(secret, "[redacted]") : value;
  const child = spawn(program, args, { cwd: directory, env: environment, shell: false, windowsHide: true, stdio: ["ignore", "pipe", "pipe"] });
  let cancelled = signal?.aborted ?? false;
  let output = Promise.resolve();
  // 按流保存全文；相邻分片合并后脱敏，避免 Key 跨 chunk 被写入日志。
  function capture(stream: NodeJS.ReadableStream, label: string): void {
    let tail = "";
    stream.setEncoding("utf8");
    stream.on("data", (chunk: string) => {
      const combined = tail + chunk;
      const keep = secret ? Math.max(0, secret.length - 1) : 0;
      let boundary = Math.max(0, combined.length - keep);
      if (secret) {
        const crossing = combined.lastIndexOf(secret, boundary);
        if (crossing >= 0 && crossing + secret.length > boundary) { boundary = crossing; }
      }
      const safe = redact(combined.slice(0, boundary));
      tail = combined.slice(boundary);
      output = output.then(async () => { await log.write(`[${label}] ${safe}`); });
    });
    stream.on("end", () => { const safe = redact(tail); output = output.then(async () => { await log.write(safe); }); });
  }
  if (child.stdout) { capture(child.stdout, "stdout"); }
  if (child.stderr) { capture(child.stderr, "stderr"); }
  const cancel = () => {
    cancelled = true;
    if (process.platform === "win32" && child.pid) {
      const killer = spawn("taskkill.exe", ["/PID", String(child.pid), "/T", "/F"], { windowsHide: true, stdio: "ignore" });
      killer.on("error", () => { child.kill(); });
    } else { child.kill(); }
  };
  signal?.addEventListener("abort", cancel, { once: true });
  if (cancelled) { cancel(); }
  let exitCode: number | null = null;
  let startupError: Error | undefined;
  try {
    exitCode = await new Promise<number | null>((done) => { child.once("error", (error) => { startupError = error; }); child.once("close", done); });
    await output;
    if (startupError) { await log.write(redact(startupError.message)); }
  } catch (cause) { await log.write(redact(String(cause))); }
  finally { signal?.removeEventListener("abort", cancel); await log.close(); }
  return { taskId: task.taskId, commandId, toolCallId, exitCode, status: cancelled ? "cancelled" : exitCode === 0 ? "succeeded" : "failed", logPath, program, args };
}
