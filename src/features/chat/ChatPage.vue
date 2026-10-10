<script setup lang="ts">
import { computed, shallowRef, watch, onBeforeUnmount } from "vue";
import { RotateCw, Unplug } from "@lucide/vue";
import { onBeforeRouteLeave, onBeforeRouteUpdate, useRoute, useRouter } from "vue-router";
import type { ModProject } from "@/contracts/mod";
import type { TaskRecord } from "@/contracts/task";
import type { PromptAttachment } from "@/contracts/session";
import { readAttachments } from "./attachments";
import { modService, nativeCall } from "@/services/native";
import { Button } from "@/components/ui/button";
import { useDraft } from "./useDraft";
import { useConfiguration } from "@/features/settings/useConfiguration";
import { useSessionStore } from "@/stores/session";
import { useProjectsStore } from "@/stores/projects";
import MessageTimeline from "./MessageTimeline.vue";
import ChatComposer from "./ChatComposer.vue";

const route = useRoute();
const router = useRouter();
const project = shallowRef<ModProject>();
const error = shallowRef("");
const opening = shallowRef(false);
const loadingProject = shallowRef(true);
const attachments = computed({ get: () => session.attachments, set: (value: PromptAttachment[]) => { session.attachments = value; } });
const attachmentError = shallowRef("");
const readingAttachments = shallowRef(false);
const sending = shallowRef(false);
let attachmentGeneration = 0;
const tasks = shallowRef<TaskRecord[]>([]);
const draft = useDraft();
const { requirement, error: saveError } = draft;
const configuration = useConfiguration();
const { status } = configuration;
const session = useSessionStore();
const projects = useProjectsStore();
const connectionIssue = computed(() => error.value || session.error || configuration.error.value);
const runtimeClosed = computed(() => connectionIssue.value === "pi 进程已退出");
const reconnectId = computed(() => session.sessionId || String(route.query.session ?? ""));
watch(() => session.modId + "/" + session.sessionId, () => {
  attachmentGeneration += 1; attachmentError.value = ""; readingAttachments.value = false;
});
/** 切换会话期间完成的文件读取不得附加到另一个会话。 */
async function attachFiles(files: File[]): Promise<void> {
  if (readingAttachments.value) { return; }
  const generation = attachmentGeneration;
  readingAttachments.value = true;
  attachmentError.value = "";
  try {
    const added = await readAttachments(files, attachments.value);
    if (generation === attachmentGeneration) { attachments.value = [...attachments.value, ...added]; }
  } catch (cause) { if (generation === attachmentGeneration) { attachmentError.value = (cause as Error).message; } }
  finally { if (generation === attachmentGeneration) { readingAttachments.value = false; } }
}
let loadGeneration = 0;
let saveTimer: ReturnType<typeof setTimeout> | undefined;
watch(requirement, () => {
  if (saveTimer) { clearTimeout(saveTimer); }
  saveTimer = setTimeout(() => { void draft.flush(); }, 600);
});

async function openSession(id?: string, readOnly = false): Promise<void> {
  if (!project.value || !await draft.flush()) { return; }
  const epoch = loadGeneration;
  opening.value = true;
  error.value = "";
  try {
    const modId = project.value.modId;
    await session.open(modId, id, readOnly);
    if (epoch !== loadGeneration || project.value?.modId !== modId || (id && session.sessionId !== id)) { return; }
    const metadata = await nativeCall<{ draft: string }>("read_session_metadata",{modId,sessionId:session.sessionId});
    if (epoch !== loadGeneration) { return; }
    draft.load(modId,metadata.draft,session.sessionId);
    const records = await nativeCall<TaskRecord[]>("list_mod_tasks", { modId });
    if (epoch !== loadGeneration) { return; }
    tasks.value = records;
    session.task = tasks.value.find((item) => item.context.sessionId === session.sessionId)?.context;
    project.value = await modService.get(project.value.modId);
    projects.upsert(project.value);
    session.readOnly = project.value.sessions.find(item => item.sessionId === session.sessionId)?.readOnly ?? readOnly;
  } catch (cause) { error.value = (cause as Error).message; }
  finally { if (epoch === loadGeneration) { opening.value = false; } }
}

/** 重连只打开已选中的会话，不允许空 ID 触发新建。 */
async function reconnect(): Promise<void> {
  if (opening.value || session.busy || !reconnectId.value) { return; }
  await openSession(reconnectId.value, session.readOnly);
}

watch(() => [String(route.params.modId), String(route.query.session ?? ""), String(route.query.new ?? "")], async ([id, requested, fresh]) => {
  if (!id) { return; }
  const generation = ++loadGeneration;
  loadingProject.value = true;
  const changedProject = project.value?.modId !== id;
  if (changedProject) { project.value = undefined; tasks.value = []; }
  error.value = "";
  try {
    const [value] = await Promise.all([modService.get(id), configuration.refresh()]);
    if (generation !== loadGeneration) { return; }
    project.value = value;

    const previous = fresh ? undefined : requested ? value.sessions.find(item => item.sessionId === requested) : value.sessions[value.sessions.length - 1];
    if (requested && !previous && !fresh) { throw new Error("该会话已删除或不属于当前项目"); }
    if (!fresh && previous?.sessionId === session.sessionId && session.modId === id && session.stateFor(id,previous.sessionId)?.initialized) {
      session.readOnly = previous.readOnly;
      const metadata = await nativeCall<{draft: string}>("read_session_metadata",{modId:id,sessionId:previous.sessionId});
      if (generation !== loadGeneration) { return; }
      draft.load(id,metadata.draft,previous.sessionId);
      const records = await nativeCall<TaskRecord[]>("list_mod_tasks", { modId: id });
      if (generation !== loadGeneration) { return; }
      tasks.value = records;
      return;
    }
    await openSession(previous?.sessionId, previous?.readOnly);
    if (generation === loadGeneration && session.sessionId && !error.value) {
      await router.replace({ path: route.path, query: { session: session.sessionId } });
    }
  } catch (cause) { if (generation === loadGeneration) { error.value = (cause as Error).message; } }
  finally { if (generation === loadGeneration) { loadingProject.value = false; } }
}, { immediate: true });

async function send(): Promise<void> {
  if (sending.value || readingAttachments.value || opening.value || (session.busy && !session.canSteer)) { return; }
  const submittingKey = session.selectedKey;
  const target = session.stateFor(session.modId,session.sessionId);
  if (!target || (!requirement.value.trim() && !attachments.value.length)) { return; }
  const submitted = requirement.value;
  const selected = attachments.value;
  const textFiles = selected.filter(file => file.type === "text").map(file => `附件文件：${file.name}\n${file.text}`).join("\n\n");
  const message = [submitted.trim() || (selected.length ? "请根据附件继续处理当前 MOD。" : ""), textFiles].filter(Boolean).join("\n\n");
  const images = selected.filter(file => file.type === "image").map(file => ({ type: "image" as const, data: file.data, mimeType: file.mimeType }));
  const previousIds = new Set(target.chat.messages.map(item => item.id));
  sending.value = true;
  const submission = draft.captureSubmitted();
  target.attachments = target.attachments.filter(file => !selected.some(sent => sent.id === file.id));
  let restore = false;
  error.value = "";
  try {
    await target.send(message, images, submission.save);
  } catch (cause) {
    const confirmed = target.chat.messages.some(item => !previousIds.has(item.id) && item.role === "user" && (item.final || (item.inputStatus && !["submitted", "failed", "unknown"].includes(item.inputStatus))));
    restore = !confirmed && !/超时|未确认/.test((cause as Error).message);
    if (restore) { target.attachments = [...selected, ...target.attachments.filter(file => !selected.some(sent => sent.id === file.id))]; }
    if (session.selectedKey === submittingKey) { error.value = (cause as Error).message; }
  } finally {
    try {
      const saved = await submission.finish(restore);
      if (!saved && !restore && session.selectedKey === submittingKey) { error.value = "消息已提交，草稿保存失败，请勿重复发送"; }
    } finally { sending.value = false; }
  }
}

async function leave(): Promise<boolean> {
  if (saveTimer) { clearTimeout(saveTimer); saveTimer = undefined; }
  return draft.flush();
}
/** 原生终态事件之后读取任务结束时间及新版本，不用前端猜测完成。 */
watch(() => [session.changeRevision, session.artifactRevision, session.task?.taskId], async () => {
  const id = project.value?.modId;
  if (!id || session.modId !== id) { return; }
  try {
    const records = await nativeCall<TaskRecord[]>("list_mod_tasks", { modId: id });
    const value = await modService.get(id);
    if (project.value?.modId !== id) { return; }
    tasks.value = records;
    project.value = value;
    projects.upsert(value);
  } catch (cause) { error.value = (cause as Error).message; }
});
onBeforeUnmount(() => { if (saveTimer) { clearTimeout(saveTimer); } });
onBeforeRouteLeave(leave);
onBeforeRouteUpdate(leave);
</script>

<template>
  <div class="flex h-full min-h-0 flex-col">
    <div v-if="connectionIssue" :role="runtimeClosed ? 'status' : 'alert'" class="flex shrink-0 items-center gap-3 border-b bg-muted/30 px-5 py-2 text-xs">
      <Unplug v-if="runtimeClosed" class="size-3.5 shrink-0 text-muted-foreground" />
      <span class="min-w-0 flex-1 break-words" :class="runtimeClosed ? 'text-muted-foreground' : 'text-destructive'">{{ runtimeClosed ? '会话连接已断开' : connectionIssue }}</span>
      <Button v-if="session.restartAvailable" variant="outline" size="sm" class="h-7 shrink-0 gap-1.5 rounded-md text-xs" :disabled="opening" @click="session.restart().catch(cause => error = cause.message)"><RotateCw class="size-3.5" />重启运行时</Button>
      <Button v-else-if="project && reconnectId" variant="outline" size="sm" class="h-7 shrink-0 gap-1.5 rounded-md text-xs" :disabled="opening || session.busy" @click="reconnect"><RotateCw class="size-3.5" />{{ opening ? '正在连接…' : '重新连接' }}</Button>
    </div>
    <p v-if="opening" class="px-5 py-3 text-xs text-muted-foreground">正在恢复会话历史…</p>
    <MessageTimeline v-if="project && session.modId === project.modId && (session.chat.messages.length || session.working)" :key="`${session.modId}-${session.sessionId}`" :messages="session.chat.messages" :tools="session.chat.tools" :mod-id="String(route.params.modId)" :tasks="tasks.filter(item => item.context.sessionId === session.sessionId)" :working="session.working" :started-at="session.startedAt" :ended-at="session.endedAt" :activity="session.activity" :last-activity-at="session.lastActivityAt" />
    <div v-else-if="loadingProject || opening" role="status" class="flex min-h-0 flex-1 items-center justify-center text-sm text-muted-foreground">正在加载会话…</div>
    <div v-else class="flex min-h-0 flex-1 items-center justify-center overflow-auto px-8 py-10"><div class="max-w-md text-center"><p class="mb-3 text-xs text-muted-foreground">{{ project?.gameSkillName }}</p><h2 class="text-xl font-semibold">你的 MOD，想从哪里开始？</h2><p class="mt-3 text-sm leading-7 text-muted-foreground">描述希望实现的行为。源码会保存在项目中，每次成功交付形成独立版本。</p></div></div>
    <div v-if="session.steeringInputs.some(input => ['unknown', 'handled', 'submitted', 'queued'].includes(input.status) && !session.chat.messages.some(message => message.inputId === input.inputId))" role="status" class="max-h-28 shrink-0 overflow-auto border-t px-5 py-2 text-xs text-muted-foreground">
      <p v-for="input in session.steeringInputs.filter(input => ['unknown', 'handled', 'submitted', 'queued'].includes(input.status) && !session.chat.messages.some(message => message.inputId === input.inputId))" :key="input.inputId" class="truncate" :title="input.message">补充要求{{ input.status === 'handled' ? '已处理，未关联到正式历史' : '尚未确认执行' }}，不会自动重发：{{ input.message }}</p>
    </div>
    <p v-if="session.readOnly" class="border-t px-5 py-5 text-xs text-muted-foreground">此会话使用旧的能力上下文，只能查看历史。创建新会话继续开发。</p>
    <ChatComposer v-else-if="project" v-model="requirement" :attachments="attachments" :reading-attachments="readingAttachments" :working="session.working" :can-steer="session.canSteer && !sending" :stopping="session.stopping" :busy="session.busy || sending" :configured="!!status?.keyConfigured && !opening" :model="status?.model || 'step-5-preview'" :error="attachmentError || saveError" :thinking-level="session.thinkingLevel" :thinking-levels="session.thinkingLevels" :context-usage="session.contextUsage" :compacting="session.compacting" :can-compact="!!session.chat.messages.length" @files="attachFiles" @remove-attachment="attachments = attachments.filter(file => file.id !== $event)" @send="send" @save="draft.flush" @cancel="session.abort" @thinking="session.setThinking" @compact="session.compact" />
  </div>
</template>
