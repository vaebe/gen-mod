<script setup lang="ts">
import { computed, defineAsyncComponent, onMounted, reactive, shallowRef, watch } from "vue";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from "@/components/ui/dialog";
import { providerApis } from "@/contracts/config";
import type { ProviderConfiguration, SaveConfiguration } from "@/contracts/config";
import { useConfiguration } from "./useConfiguration";

const ProviderEditor = defineAsyncComponent(() => import("./ProviderEditor.vue"));
const { status, error, loading, saving, refresh, openDirectory, save } = useConfiguration();
const draft = reactive<SaveConfiguration>({ providers: [], defaultProvider: "", defaultModel: "" });
const baseline = shallowRef("");
const editing = shallowRef<string | null>(null);
const saved = shallowRef(false);
const currentProvider = computed(() => draft.providers.find(provider => provider.id === editing.value));
const existingIds = computed(() => draft.providers.map(provider => provider.id));
const dirty = computed(() => JSON.stringify(draft) !== baseline.value);
const busy = computed(() => loading.value || saving.value);
const modelOptions = computed(() => draft.providers.flatMap(provider => provider.models.map(model => ({
  value: JSON.stringify([provider.id, model.id]), label: `${provider.name} / ${model.name}`,
}))));
const defaultValue = computed({
  get: () => draft.defaultProvider && draft.defaultModel ? JSON.stringify([draft.defaultProvider, draft.defaultModel]) : "",
  set: (value: string) => {
    const [provider, model] = value ? JSON.parse(value) as [string, string] : ["", ""];
    draft.defaultProvider = provider;
    draft.defaultModel = model;
  },
});
function reset(): void {
  if (!status.value) { return; }
  draft.providers = status.value.providers.map(provider => ({ ...provider, models: provider.models.map(model => ({ ...model, input: [...model.input] })) }));
  draft.defaultProvider = status.value.defaultProvider;
  draft.defaultModel = status.value.defaultModel;
  baseline.value = JSON.stringify(draft);
  editing.value = null;
  saved.value = false;
}
watch(status, reset);
watch(dirty, () => { saved.value = false; });
function validateDefault(): void {
  if (!draft.providers.some(provider => provider.id === draft.defaultProvider && provider.models.some(model => model.id === draft.defaultModel))) {
    defaultValue.value = "";
  }
}
function apply(provider: ProviderConfiguration): void {
  const index = draft.providers.findIndex(item => item.id === provider.id);
  if (index < 0) { draft.providers.push(provider); }
  else { draft.providers[index] = provider; }
  validateDefault();
  editing.value = null;
}
function remove(id: string): void {
  draft.providers = draft.providers.filter(provider => provider.id !== id);
  validateDefault();
}
async function persist(): Promise<void> {
  if (busy.value || editing.value !== null) { return; }
  if (await save({ providers: draft.providers, defaultProvider: draft.defaultProvider, defaultModel: draft.defaultModel })) {
    reset();
    saved.value = true;
  }
}
function apiLabel(value: string): string { return providerApis.find(api => api.value === value)?.label ?? value; }
onMounted(() => { void refresh(); });
</script>

<template>
  <section class="h-full overflow-auto px-5 py-8 sm:px-8"><div class="mx-auto max-w-3xl space-y-6">
    <header><h1 class="text-xl font-semibold">设置</h1><p class="mt-2 text-sm leading-6 text-muted-foreground">管理模型服务，选择工作台使用的默认模型。</p></header>
    <p v-if="error" role="alert" class="text-sm text-destructive">{{ error }}</p>
    <p v-if="loading" role="status" class="text-sm text-muted-foreground">正在读取配置…</p>
    <Button v-if="!status && !loading" variant="outline" @click="refresh">重新读取</Button>
    <template v-if="status">
      <fieldset :disabled="busy" class="min-w-0 overflow-hidden rounded-xl border bg-card">
        <div class="flex items-center justify-between gap-4 border-b px-5 py-4"><div><h2 class="text-sm font-medium">模型服务 <span v-if="draft.providers.length" class="ml-1 text-muted-foreground">{{ draft.providers.length }}</span></h2><p class="mt-1 text-xs text-muted-foreground">连接配置和模型统一在这里管理。</p></div><Button size="sm" :disabled="editing !== null" @click="editing = ''">添加 provider</Button></div>
        <Dialog :open="editing !== null" @update:open="!$event && (editing = null)">
          <DialogContent class="flex max-h-[85vh] flex-col gap-0 overflow-hidden p-0 sm:max-w-2xl" @pointer-down-outside.prevent @interact-outside.prevent>
            <DialogHeader class="shrink-0 border-b px-6 py-5 pr-12 text-left"><DialogTitle>{{ currentProvider ? '编辑 provider' : '添加 provider' }}</DialogTitle><DialogDescription>填写连接信息和模型，添加到列表后统一保存。</DialogDescription></DialogHeader>
            <ProviderEditor v-if="editing !== null" :key="editing" :provider="currentProvider" :existing-ids="existingIds" @apply="apply" @cancel="editing = null" />
          </DialogContent>
        </Dialog>
        <div class="divide-y">
          <div v-if="!draft.providers.length" class="px-5 py-8"><p class="text-sm font-medium">尚未添加模型服务</p><p class="mt-2 text-sm leading-6 text-muted-foreground">点击右上方“添加 provider”，填写服务地址、API Key 和模型信息。</p></div>
          <article v-for="provider in draft.providers" :key="provider.id" class="p-5">
            <div class="flex flex-wrap items-start justify-between gap-3"><div class="min-w-0"><h3 class="font-medium">{{ provider.name }}</h3><p class="mt-1 break-all font-mono text-xs text-muted-foreground">{{ provider.id }}</p></div><div class="flex gap-1"><Button variant="ghost" size="sm" @click="editing = provider.id">编辑</Button><Button variant="ghost" size="sm" class="text-destructive" @click="remove(provider.id)">移除</Button></div></div>
            <div class="mt-3 flex flex-wrap gap-x-3 gap-y-1 text-xs text-muted-foreground"><span>{{ apiLabel(provider.api) }}</span><span>{{ provider.apiKey || provider.keyConfigured ? 'Key 已填写' : 'Key 未填写' }}</span><span class="break-all">{{ provider.baseUrl }}</span></div>
            <ul class="mt-4 divide-y rounded-lg bg-muted/30 px-3 text-sm"><li v-for="model in provider.models" :key="model.id" class="flex flex-wrap items-center justify-between gap-2 py-3"><span>{{ model.name }} <span class="ml-2 font-mono text-xs text-muted-foreground">{{ model.id }}</span></span><span class="text-xs text-muted-foreground">{{ model.contextWindow.toLocaleString() }} 上下文 · {{ model.maxTokens.toLocaleString() }} 输出{{ model.input.includes('image') ? ' · 图片' : '' }}{{ model.reasoning ? ' · 推理' : '' }}</span></li></ul>
          </article>
        </div>
        <div class="grid gap-3 border-t p-5 sm:grid-cols-[8rem_minmax(0,1fr)] sm:gap-5"><div><label for="default-model" class="text-sm font-medium">默认模型</label><p class="mt-1 text-xs leading-5 text-muted-foreground">用于后续新任务</p></div><div><select v-if="modelOptions.length" id="default-model" v-model="defaultValue" :disabled="editing !== null" class="h-9 w-full rounded-md border bg-background px-3 text-sm focus-visible:outline focus-visible:outline-ring"><option value="">请选择默认模型</option><option v-for="option in modelOptions" :key="option.value" :value="option.value">{{ option.label }}</option></select><p v-else class="py-2 text-sm text-muted-foreground">添加模型后可选择</p><p v-if="modelOptions.length" class="mt-2 text-xs leading-5 text-muted-foreground">保存后下次任务生效，正在执行的任务不受影响。</p></div></div>
        <div class="flex flex-wrap items-center justify-between gap-3 border-t bg-muted/20 px-5 py-4"><div class="text-xs"><span v-if="saved" role="status" class="text-green-700 dark:text-green-400">配置已保存</span><span v-else-if="dirty" class="text-muted-foreground">修改尚未保存</span><span v-else class="text-muted-foreground">{{ draft.providers.length ? '配置保存在本机' : '未配置时仍可保存需求和管理项目' }}</span></div><div class="ml-auto flex gap-2"><Button variant="outline" :disabled="!dirty && editing === null" @click="reset">放弃修改</Button><Button :disabled="!dirty || editing !== null || busy" @click="persist">{{ saving ? '保存中…' : '保存配置' }}</Button></div></div>
      </fieldset>
      <div class="rounded-xl border bg-card px-5 py-4"><div class="flex flex-wrap items-center justify-between gap-2"><h2 class="text-sm font-medium">内置运行时</h2><span class="text-xs text-muted-foreground">{{ status.runtimePrepared ? '资源已准备' : '资源尚未准备' }}</span></div><p class="mt-2 text-xs text-muted-foreground">pi {{ status.runtimeVersion }} · Node {{ status.nodeVersion }}</p><details class="mt-4 border-t pt-3"><summary class="cursor-pointer text-xs text-muted-foreground">本地配置文件</summary><div class="space-y-3 pt-3"><p class="text-xs leading-6 text-muted-foreground">API Key 以明文保存在本机，界面不会回显已保存的 Key。</p><p class="break-all font-mono text-[11px] text-muted-foreground">{{ status.configurationDirectory }}</p><div class="flex justify-end"><Button variant="outline" size="sm" @click="openDirectory">打开配置目录</Button></div></div></details></div>
    </template>
  </div></section>
</template>
