import { Channel } from "@tauri-apps/api/core";
import { shallowRef } from "vue";
import { nativeCall } from "@/services/native";
import type { PiEnvelope, PiRecord } from "./types";
import type { TaskContext } from "@/contracts/task";
import type { ImageInput, ThinkingLevel } from "@/contracts/session";

type Pending = { resolve: (record: PiRecord) => void; reject: (error: Error) => void; timer: ReturnType<typeof setTimeout> };

/** 单连接请求匹配器，命令超时不重发，也不认定 Agent 已停止。 */
export function createPiClient() {
  const connection = shallowRef<"disconnected" | "connecting" | "connected" | "error">("disconnected");
  const submissionState = shallowRef<"idle" | "submitting" | "running" | "queued" | "finished" | "failed">("idle");
  const active = shallowRef(false);
  const generation = shallowRef(0);
  const pending = new Map<string, Pending>();
  const listeners = new Set<(event: PiEnvelope) => void>();
  let settledSeen = false;
  let minimumGeneration = 0;

  function rejectAll(message: string): void {
    for (const request of pending.values()) {
      clearTimeout(request.timer);
      request.reject(new Error(message));
    }
    pending.clear();
  }

  function receive(event: PiEnvelope): void {
    if (event.generation < Math.max(generation.value, minimumGeneration)) { return; }
    if (event.generation > generation.value) {
      generation.value = event.generation;
    }
    const record = event.record;
    if (event.kind === "exit") {
      connection.value = "disconnected";
      active.value = false;
      submissionState.value = "finished";
      rejectAll("pi 进程已退出");
    } else if (event.kind === "record") {
      if (record.type === "agent_start") { active.value = true; }
      if (record.type === "agent_settled") {
        settledSeen = true;
        active.value = false;
        if (submissionState.value !== "submitting") { submissionState.value = "finished"; }
      }
      if (record.type === "response" && record.id) {
        const request = pending.get(record.id);
        if (request) {
          clearTimeout(request.timer);
          pending.delete(record.id);
          if (record.success) { request.resolve(record); }
          else { request.reject(new Error(record.error || "RPC 命令失败")); }
        }
      }
    }
    for (const listener of listeners) { listener(event); }
  }

  function channel(): Channel<PiEnvelope> {
    const result = new Channel<PiEnvelope>();
    result.onmessage = receive;
    return result;
  }

  async function connect(modId: string, sessionId?: string): Promise<void> {
    rejectAll("正在建立新连接");
    minimumGeneration = generation.value + 1;
    connection.value = "connecting";
    try {
      const result = await nativeCall<number>("connect_pi", { modId, sessionId, onEvent: channel() });
      generation.value = result;
      connection.value = "connected";
    } catch (error) { minimumGeneration = generation.value; connection.value = "error"; throw error; }
  }

  function request(type: string, fields: Record<string, unknown> = {}, timeoutMs = 30_000): Promise<PiRecord> {
    const id = crypto.randomUUID();
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        pending.delete(id);
        reject(new Error("RPC 响应超时；任务可能仍在运行，请查询状态"));
      }, timeoutMs);
      pending.set(id, { resolve, reject, timer });
      nativeCall<void>("send_pi_command", { generation: generation.value, command: { ...fields, type, id } }).catch((error: Error) => {
        const item = pending.get(id);
        if (!item) { return; }
        clearTimeout(item.timer);
        pending.delete(id);
        reject(error);
      });
    });
  }

  /** 原生创建并绑定任务后才提交 prompt，普通 RPC 不能绕开此入口。 */
  async function prompt(modId: string, sessionId: string, requirement: string, thinkingLevel: ThinkingLevel, images: ImageInput[] = []): Promise<TaskContext> {
    rejectAll("正在开始新任务");
    minimumGeneration = generation.value + 1;
    submissionState.value = "submitting";
    settledSeen = false;
    connection.value = "connecting";
    const id = crypto.randomUUID();
    let response: Promise<PiRecord> | undefined;
    try {
      // 原生在返回前可能已发响应，先注册 pending。
      response = new Promise<PiRecord>((resolve, reject) => {
        const timer = setTimeout(() => { pending.delete(id); reject(new Error("需求提交响应超时，未自动重发")); }, 30_000);
        pending.set(id, { resolve, reject, timer });
      });
      // 捕获提前失败，避免等待原生启动期间产生未处理拒绝。
      void response.catch(() => undefined);
      const context = await nativeCall<TaskContext>("start_mod_task", { modId, sessionId, requirement, thinkingLevel, images, requestId: id, onEvent: channel() });
      generation.value = context.generation;
      connection.value = "connected";
      const record = await response;
      const disposition = record.data?.disposition;
      if (disposition === "handled") { submissionState.value = "finished"; }
      else if (settledSeen) { submissionState.value = "finished"; }
      else if (disposition === "queued") { submissionState.value = "queued"; }
      else if (disposition === "started") { submissionState.value = "running"; active.value = true; }
      else { throw new Error("pi 返回未知 prompt disposition"); }
      return context;
    } catch (error) {
      minimumGeneration = generation.value;
      const item = pending.get(id);
      if (item) { clearTimeout(item.timer); pending.delete(id); item.reject(error as Error); }
      submissionState.value = "failed";
      if (connection.value === "connecting") { connection.value = "error"; }
      throw error;
    }
  }

  async function disconnect(): Promise<void> {
    await nativeCall("disconnect_pi");
    rejectAll("连接已关闭");
    connection.value = "disconnected";
    active.value = false;
    submissionState.value = "idle";
  }
  function subscribe(listener: (event: PiEnvelope) => void): () => void {
    listeners.add(listener);
    return () => { listeners.delete(listener); };
  }
  return { connection, submissionState, active, generation, connect, prompt, request, disconnect, subscribe };
}
