<script setup lang="ts">
import type { ModelForm } from '@/contracts/config'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'

const models = defineModel<ModelForm[]>({ required: true })
function add(): void {
  models.value = [...models.value, { id: '', name: '', contextWindow: '', maxTokens: '', reasoning: false, input: ['text'] }]
}
function image(index: number, enabled: boolean): void {
  models.value = models.value.map((model, position) => position === index ? { ...model, input: enabled ? ['text', 'image'] : ['text'] } : model)
}
</script>

<template>
  <div class="space-y-4">
    <div class="flex items-center justify-between gap-3">
      <h3 class="text-sm font-medium">
        模型
      </h3><Button type="button" size="sm" variant="outline" @click="add">
        添加模型
      </Button>
    </div>
    <p v-if="!models.length" class="rounded-lg bg-muted/40 px-4 py-4 text-sm leading-6 text-muted-foreground">
      尚未添加模型。点击右侧“添加模型”，按服务商规格填写参数。
    </p>
    <section v-for="(model, index) in models" :key="index" class="space-y-4 rounded-lg border p-4" :aria-label="`模型 ${index + 1}`">
      <div class="flex items-center justify-between">
        <span class="text-xs text-muted-foreground">模型 {{ index + 1 }}</span><Button type="button" variant="ghost" size="sm" class="text-destructive" @click="models = models.filter((_, position) => position !== index)">
          移除
        </Button>
      </div>
      <div class="grid gap-4 sm:grid-cols-2">
        <label class="flex flex-col gap-2 text-sm"><span>模型 ID</span><Input v-model="model.id" required placeholder="服务商提供的模型 ID" /></label>
        <label class="flex flex-col gap-2 text-sm"><span>显示名称</span><Input v-model="model.name" required placeholder="便于识别的名称" /></label>
        <label class="flex flex-col gap-2 text-sm"><span>上下文长度</span><Input v-model.number="model.contextWindow" type="number" required min="1" max="4294967295" step="1" placeholder="Token 数，按模型规格填写" /></label>
        <label class="flex flex-col gap-2 text-sm"><span>最大输出</span><Input v-model.number="model.maxTokens" type="number" required min="1" :max="model.contextWindow || 4294967295" step="1" placeholder="Token 数，按模型规格填写" /></label>
      </div>
      <div class="flex flex-wrap gap-x-6 gap-y-3 text-sm">
        <label class="flex items-center gap-2"><input type="checkbox" class="size-4 accent-primary" :checked="model.input.includes('image')" @change="image(index, ($event.target as HTMLInputElement).checked)">支持图片输入</label>
        <label class="flex items-center gap-2"><input v-model="model.reasoning" type="checkbox" class="size-4 accent-primary">支持推理</label>
      </div>
    </section>
  </div>
</template>
