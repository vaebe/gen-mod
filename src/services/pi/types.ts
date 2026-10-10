/** 锁定 pi 1.1.0 的 RPC JSONL。内部桥事件与 pi 记录分开。 */
export interface PiRecord {
  type: string;
  id?: string;
  command?: string;
  success?: boolean;
  error?: string;
  data?: Record<string, unknown>;
  [key: string]: unknown;
}
export interface PiEnvelope {
  modId: string;
  sessionId: string;
  taskId?: string;
  generation: number;
  kind: "record" | "diagnostic" | "exit" | "task" | "artifact";
  record: PiRecord;
}
export interface PiSessionState {
  sessionId: string;
  sessionFile?: string;
  isStreaming: boolean;
  isCompacting: boolean;
  pendingMessageCount: number;
}
