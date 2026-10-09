<script setup lang="ts">
import { onMounted } from "vue";
import { Button } from "@/components/ui/button";
import { useConfiguration } from "./useConfiguration";
const { status, error, loading, refresh, openDirectory } = useConfiguration();
onMounted(() => { void refresh(); });
</script>

<template>
  <section class="h-full overflow-auto px-6 py-10"><div class="mx-auto max-w-3xl">
    <h1 class="text-xl font-semibold">设置</h1>
    <p class="mt-2 text-sm text-muted-foreground">模型配置来自 env 文件，API Key 不在界面中显示。</p>
    <p v-if="error" role="alert" class="mt-6 text-sm text-destructive">{{ error }}</p>
    <div v-if="status" class="mt-8 rounded-xl border bg-card p-6">
      <h2 class="text-sm font-medium">模型连接</h2>
      <dl class="mt-5 grid grid-cols-[7rem_1fr] gap-x-4 gap-y-4 text-sm">
        <dt class="text-muted-foreground">模型</dt><dd>{{ status.model }}</dd>
        <dt class="text-muted-foreground">协议</dt><dd>OpenAI Responses</dd>
        <dt class="text-muted-foreground">Base URL</dt><dd class="break-all font-mono text-xs">{{ status.baseUrl }}</dd>
        <dt class="text-muted-foreground">API Key</dt><dd :class="status.keyConfigured ? 'text-green-700' : 'text-muted-foreground'">{{ status.keyConfigured ? '已配置' : '尚未配置' }}</dd>
        <dt class="text-muted-foreground">env 文件</dt><dd class="break-all font-mono text-xs">{{ status.envPath }}</dd>
      </dl>
      <div class="mt-6 flex gap-3"><Button variant="outline" @click="openDirectory">打开配置目录</Button><Button variant="outline" :disabled="loading" @click="refresh">{{ loading ? '读取中…' : '刷新配置状态' }}</Button></div>
      <p class="mt-4 text-xs leading-6 text-muted-foreground">修改 env 后，下次启动任务时读取新配置。未配置 Key 时仍可保存需求、查看项目和管理已有版本。</p>
    </div>
    <div v-if="status" class="mt-5 rounded-xl border bg-card p-6"><h2 class="text-sm font-medium">内置运行时</h2><p class="mt-3 text-sm text-muted-foreground">pi {{ status.runtimeVersion }} · Node {{ status.nodeVersion }}</p><p class="mt-2 text-sm">{{ status.runtimePrepared ? '资源已准备' : '资源尚未准备' }}</p></div>
  </div></section>
</template>
