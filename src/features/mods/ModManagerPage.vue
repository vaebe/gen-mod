<script setup lang="ts">
import { computed, shallowRef, watch } from "vue";
import { useRoute } from "vue-router";
import type { ModProject } from "@/contracts/mod";
import { modService, nativeCall } from "@/services/native";
import { useSessionStore } from "@/stores/session";
import { useProjectsStore } from "@/stores/projects";
import { Button } from "@/components/ui/button";
import { displayPath } from "@/lib/displayPath";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import VersionList from "./VersionList.vue";
import ArtifactPanel from "./ArtifactPanel.vue";
import InstallDialog from "./InstallDialog.vue";
const route = useRoute();
const project = shallowRef<ModProject>();
const selectedId = shallowRef("");
const selected = computed(() => project.value?.versions.find((v) => v.versionId === selectedId.value));
const error = shallowRef("");
const notice = shallowRef("");
const busy = shallowRef(false);
const installing = shallowRef(false);
const deleting = shallowRef(false);
/** 只标记仍在目标状态中已知的当前版本，历史安装成功不等于当前安装。 */
const installedIds = computed(() => project.value?.installedTargets.filter(target => target.state === "known" && target.versionId).map(target => target.versionId!) ?? []);
const previousTarget = computed(() => {
  const value = project.value;
  return value?.lastDeploymentDirectory ?? value?.installations[value.installations.length - 1]?.targetDirectory;
});
const projects = useProjectsStore();
const session = useSessionStore();
let generation = 0;
async function refresh(): Promise<void> {
  const id = String(route.params.modId); const current = ++generation;
  try {
    const result = await modService.get(id);
    if (current !== generation) { return; }
    project.value = result; projects.upsert(result);
    if (!result.versions.some(version => version.versionId === selectedId.value)) { selectedId.value = result.latestVersionId ?? ""; }
  } catch (cause) { if (current === generation) { error.value = (cause as Error).message; } }
}
watch(() => String(route.params.modId), () => { project.value = undefined; selectedId.value = ""; error.value = ""; notice.value = ""; void refresh(); }, { immediate: true });
watch(() => projects.projects.find(item => item.modId === String(route.params.modId))?.versions.length, refresh);
async function action(kind: "source" | "version" | "export"): Promise<void> {
  if (!project.value) { return; }
  busy.value = true; error.value = "";
  try {
    if (kind === "export") { const path = await nativeCall<string | null>("export_version", { modId: project.value.modId, versionId: selectedId.value }); if (path) { notice.value = `已导出：${path}`; } }
    else { await nativeCall("open_mod_directory", { modId: project.value.modId, versionId: kind === "version" ? selectedId.value : null }); }
  } catch (cause) { error.value = (cause as Error).message; }
  finally { busy.value = false; }
}
function rememberTarget(directory: string): void {
  if (!project.value) { return; }
  project.value = { ...project.value, lastDeploymentDirectory: directory };
  projects.upsert(project.value);
}
/** 删除选定的应用内版本包；原生负责目录边界与元数据更新。 */
async function deleteVersion(): Promise<void> {
  if (!project.value || !selected.value || busy.value) { return; }
  const id = selected.value.versionId;
  busy.value = true; error.value = "";
  try {
    const result = await nativeCall<ModProject>("delete_version", { modId: project.value.modId, versionId: id });
    project.value = result; projects.upsert(result);
    selectedId.value = result.latestVersionId ?? "";
    session.artifactRevision += 1;
    deleting.value = false;
    notice.value = "版本已删除，已部署文件和安装记录保留。";
  } catch (cause) { error.value = (cause as Error).message; deleting.value = false; await refresh(); }
  finally { busy.value = false; }
}
</script>
<template><section class="mx-auto max-w-5xl space-y-6 px-6 py-8"><div class="flex flex-wrap items-start justify-between gap-3"><div><h1 class="text-xl font-semibold">{{ project?.name ?? '版本管理' }}</h1><p class="mt-2 text-sm text-muted-foreground">成功交付按版本保存，安装与导出始终使用选定版本。</p></div><div class="flex gap-2"><Button variant="outline" :disabled="busy || !project" @click="action('source')">打开源码</Button><Button v-if="project" as-child variant="ghost"><RouterLink :to="`/mods/${project.modId}/chat`">返回对话</RouterLink></Button></div></div><p v-if="error" role="alert" class="text-sm text-destructive">{{ error }}</p><p v-if="notice" class="break-all text-sm">{{ notice }}</p><template v-if="project"><div class="grid gap-6 lg:grid-cols-[250px_1fr]"><VersionList :versions="project.versions" :selected="selectedId" :latest="project.latestVersionId" :installed="installedIds" @select="selectedId = $event" /><ArtifactPanel v-if="selected" :version="selected" :busy="busy" @delete="deleting = true" @install="installing = true" @open="action('version')" @export="action('export')" /></div><section class="rounded-lg border p-4"><h2 class="text-sm font-medium">目标当前安装状态</h2><p v-if="!project.installedTargets.length" class="mt-3 text-xs text-muted-foreground">尚未安装任何版本。</p><div v-for="target in project.installedTargets" :key="target.targetDirectory" class="mt-3 text-xs"><p class="break-all font-mono">{{ displayPath(target.targetDirectory) }}</p><p class="mt-1" :class="target.state === 'uncertain' && 'text-destructive'">{{ target.state === 'uncertain' ? '目标状态不确定，请检查部署文件' : `当前版本 ${target.versionId?.slice(0, 8)}` }}<span v-if="target.state === 'known' && target.versionId !== project.latestVersionId" class="ml-2 text-muted-foreground">最新版本待安装</span></p></div></section><details class="rounded-lg border p-4"><summary class="cursor-pointer text-sm">安装记录（{{ project.installations.length }}）</summary><div v-for="record in [...project.installations].reverse()" :key="record.installationId" class="mt-3 border-t pt-3 text-xs"><p>{{ record.versionId.slice(0, 8) }} · {{ record.status === 'succeeded' ? '成功' : '失败' }} · {{ new Date(record.attemptedAt).toLocaleString() }}</p><p class="mt-1 break-all">{{ displayPath(record.targetDirectory) }}</p><p v-if="record.error" class="mt-1 text-destructive">{{ record.error }}{{ record.rollback === 'notAttempted' ? ' · 未备份、未回退' : record.rollback === 'notNeeded' ? '' : ` · 回退 ${record.rollback}` }}</p><p v-if="record.backupDirectory" class="mt-1 break-all text-muted-foreground">备份 {{ displayPath(record.backupDirectory) }}</p></div></details><InstallDialog v-if="selected" v-model:open="installing" :mod-id="project.modId" :version-id="selected.versionId" :previous-target="previousTarget" @target-selected="rememberTarget" @installed="refresh" /></template><Dialog :open="deleting" @update:open="value => !busy && (deleting = value)"><DialogContent @interact-outside="busy && $event.preventDefault()" @escape-key-down="busy && $event.preventDefault()"><DialogHeader><DialogTitle>删除版本 {{ selectedId.slice(0, 8) }}</DialogTitle><DialogDescription>删除应用内的版本文件和 ZIP，无法撤销。已部署到游戏目录的文件、源码及安装记录会保留。</DialogDescription></DialogHeader><p v-if="installedIds.includes(selectedId)" class="text-sm text-muted-foreground">此版本仍有已安装记录；删除后仍可继续使用已部署的文件。</p><div class="flex justify-end gap-2"><Button variant="ghost" :disabled="busy" @click="deleting = false">取消</Button><Button variant="destructive" :disabled="busy" @click="deleteVersion">{{ busy ? '正在删除…' : '删除版本' }}</Button></div></DialogContent></Dialog></section></template>
