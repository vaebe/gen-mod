export interface TaskContext {
  taskId: string;
  modId: string;
  sessionId: string;
  generation: number;
  sourceDirectory: string;
  stagingId: string;
  stagingDirectory: string;
  createdAt: string;
}
export interface CommandReceipt {
  taskId: string;
  commandId: string;
  toolCallId: string;
  exitCode: number | null;
  status: "succeeded" | "failed" | "cancelled";
  logPath: string;
  program?: string;
  args?: string[];
}
export interface TaskRecord {
  finishedAt?: string | null;
  context: TaskContext;
  status: "preparing" | "running" | "finished" | "failed" | "cancelled";
  commands: CommandReceipt[];
  registeredVersionId?: string;
}
