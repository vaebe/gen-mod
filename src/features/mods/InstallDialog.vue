<script setup lang="ts">
import { shallowRef, watch } from "vue";
import type { InstallPreview } from "@/contracts/install";
import type { InstallationRecord } from "@/contracts/mod";
import { nativeCall } from "@/services/native";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { displayPath } from "@/lib/displayPath";
const props = defineProps<{ modId: string; versionId: string; previousTarget?: string }>();
const open = defineModel<boolean>("open", { default: false });
const emit = defineEmits<{ installed: [record: InstallationRecord]; targetSelected: [directory: string] }>();
const preview = shallowRef<InstallPreview>();
const busy = shallowRef(false);
const error = shallowRef("");
watch(() => [open.value, props.versionId], () => { preview.value = undefined; error.value = ""; });
async function choose(previousTarget?: string): Promise<void> {
  busy.value = true; error.value = ""; preview.value = undefined;
  try {
    const target = previousTarget ?? await nativeCall<string | null>("pick_directory");
    if (target) { preview.value = await nativeCall<InstallPreview>("preview_install", { modId: props.modId, versionId: props.versionId, targetDirectory: target }); emit("targetSelected", preview.value.targetDirectory); }
  } catch (cause) { error.value = (cause as Error).message; }
  finally { busy.value = false; }
}
async function install(): Promise<void> {
  if (!preview.value) { return; }
  busy.value = true; error.value = "";
  try {
    const result = await nativeCall<InstallationRecord>("install_version", { previewId: preview.value.previewId });
    emit("installed", result);
    if (result.status === "succeeded") { open.value = false; }
    else { error.value = `${result.error ?? '安装失败'}${result.rollback === 'notAttempted' ? '；文件可能已部分覆盖，本次没有备份，请检查目标目录。' : ''}`; }
    preview.value = undefined;
  } catch (cause) { error.value = (cause as Error).message; preview.value = undefined; }
  finally { busy.value = false; }
}
</script>
<template>
  <Dialog v-model:open="open"><DialogContent class="max-w-2xl" @interact-outside="busy && $event.preventDefault()" @escape-key-down="busy && $event.preventDefault()">
    <DialogHeader><DialogTitle>安装选定版本</DialogTitle><DialogDescription>版本 {{ versionId.slice(0, 8) }}。安装会直接覆盖清单中的同名文件，不创建备份。</DialogDescription></DialogHeader>
    <div v-if="previousTarget && !preview" class="space-y-3 rounded-lg border bg-muted/30 p-4"><p class="text-sm">使用上次的部署位置，还是重新选择？</p><p class="break-all text-xs text-muted-foreground">{{ displayPath(previousTarget) }}</p><div class="flex flex-wrap justify-end gap-2"><Button :disabled="busy" @click="choose(previousTarget)">使用之前的位置</Button><Button variant="outline" :disabled="busy" @click="choose()">重新选择</Button></div></div>
    <Button v-else class="justify-self-end" variant="outline" :disabled="busy" @click="choose()">{{ preview ? '重新选择目标目录' : '选择目标目录' }}</Button>
    <template v-if="preview"><p class="break-all text-xs">{{ displayPath(preview.targetDirectory) }}</p><div class="max-h-72 space-y-3 overflow-auto rounded-md border p-3"><div v-for="entry in preview.entries" :key="entry.target" class="text-xs"><p class="break-all font-mono">{{ displayPath(entry.source) }}</p><p class="mt-1 break-all font-mono">→ {{ displayPath(entry.target) }}</p><p class="mt-1 text-muted-foreground">{{ entry.overwrite ? '覆盖已有文件' : '新增文件' }}</p></div></div><Button class="justify-self-end" :disabled="busy" @click="install">{{ busy ? '正在安装…' : '确认安装' }}</Button></template>
    <p v-if="error" class="text-sm text-destructive" role="alert">{{ error }}</p>
  </DialogContent></Dialog>
</template>
