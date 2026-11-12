export interface TaskContext { contextSnapshotId: string; memoryBootstrapPath?: string; memoryBootstrapSha256?: string; taskId: string; modId: string; sessionId: string; generation: number; sourceDirectory: string; stagingId: string; stagingDirectory: string; createdAt: string }
export function context(): TaskContext {
  const raw = process.env.GEN_MOD_TASK_CONTEXT;
  if (!raw) { throw new Error("此连接没有原生任务上下文，不能执行命令或交付"); }
  return JSON.parse(raw) as TaskContext;
}
