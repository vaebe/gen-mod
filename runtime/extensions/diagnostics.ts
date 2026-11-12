import { context } from "./task-context.js";

// 仅 pi 主进程持有脱敏集合，命令后代不能继承这些密钥。
export const redactionKeys = [...new Set<string>(JSON.parse(process.env.GEN_MOD_REDACTION_KEYS ?? "[]") as string[])]
  .filter(Boolean).sort((left, right) => right.length - left.length);
export const redact = (value: string): string => redactionKeys.reduce((text, key) => text.replaceAll(key, "[redacted]"), value);
/** stderr 独立于工具回执，协调失败时原生仍可保存命令已执行的证据。 */
export function runtimeDiagnostic(event: string, fields: Record<string, unknown>): void {
  const task = context();
  const raw = JSON.stringify({ type: "command_diagnostic", event, timestamp: new Date().toISOString(), taskId: task.taskId, ...fields });
  process.stderr.write(`GEN_MOD_DIAGNOSTIC ${redact(raw)}\n`);
}
