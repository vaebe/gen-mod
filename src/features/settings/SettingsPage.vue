<script setup lang="ts">
import type { ProviderConfiguration, SaveConfiguration } from '@/contracts/config'
import { computed, defineAsyncComponent, onMounted, reactive, shallowRef, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from '@/components/ui/empty'
import { Field, FieldContent, FieldDescription, FieldGroup, FieldLabel } from '@/components/ui/field'
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { providerApis } from '@/contracts/config'
import { useConfiguration } from './useConfiguration'

const ProviderEditor = defineAsyncComponent(() => import('./ProviderEditor.vue'))
const LongTermMemorySettings = defineAsyncComponent(() => import('../knowledge/LongTermMemorySettings.vue'))
const route = useRoute()
const router = useRouter()
const tab = computed({
  get: () => route.query.tab === 'memory' ? 'memory' : 'model',
  set: (value: string) => { void router.replace({ query: { ...route.query, tab: value } }) },
})
const memoryId = computed(() => typeof route.query.memoryId === 'string' ? route.query.memoryId : undefined)
const { status, error, loading, saving, refresh, openDirectory, save } = useConfiguration()
const draft = reactive<SaveConfiguration>({ providers: [], defaultProvider: '', defaultModel: '' })
const baseline = shallowRef('')
const editing = shallowRef<string | null>(null)
const saved = shallowRef(false)
const currentProvider = computed(() => draft.providers.find(provider => provider.id === editing.value))
const existingIds = computed(() => draft.providers.map(provider => provider.id))
const dirty = computed(() => JSON.stringify(draft) !== baseline.value)
const busy = computed(() => loading.value || saving.value)
const modelOptions = computed(() => draft.providers.flatMap(provider => provider.models.map(model => ({
  value: JSON.stringify([provider.id, model.id]),
  label: `${provider.name} / ${model.name}`,
}))))
const defaultValue = computed({
  get: () => draft.defaultProvider && draft.defaultModel ? JSON.stringify([draft.defaultProvider, draft.defaultModel]) : '',
  set: (value: string) => {
    const [provider, model] = value ? JSON.parse(value) as [string, string] : ['', '']
    draft.defaultProvider = provider
    draft.defaultModel = model
  },
})
function reset(): void {
  if (!status.value) {
    return
  }
  draft.providers = status.value.providers.map(provider => ({ ...provider, models: provider.models.map(model => ({ ...model, input: [...model.input] })) }))
  draft.defaultProvider = status.value.defaultProvider
  draft.defaultModel = status.value.defaultModel
  baseline.value = JSON.stringify(draft)
  editing.value = null
  saved.value = false
}
watch(status, reset)
watch(dirty, () => {
  saved.value = false
})
function validateDefault(): void {
  if (!draft.providers.some(provider => provider.id === draft.defaultProvider && provider.models.some(model => model.id === draft.defaultModel))) {
    defaultValue.value = ''
  }
}
function apply(provider: ProviderConfiguration): void {
  const index = draft.providers.findIndex(item => item.id === provider.id)
  if (index < 0) {
    draft.providers.push(provider)
  }
  else { draft.providers[index] = provider }
  validateDefault()
  editing.value = null
}
function remove(id: string): void {
  draft.providers = draft.providers.filter(provider => provider.id !== id)
  validateDefault()
}
async function persist(): Promise<void> {
  if (busy.value || editing.value !== null) {
    return
  }
  if (await save({ providers: draft.providers, defaultProvider: draft.defaultProvider, defaultModel: draft.defaultModel })) {
    reset()
    saved.value = true
  }
}
function apiLabel(value: string): string {
  return providerApis.find(api => api.value === value)?.label ?? value
}
onMounted(() => {
  void refresh()
})
</script>

<template>
  <section class="studio-page flex min-h-0 flex-col overflow-hidden">
    <div class="studio-content flex min-h-0 flex-1 flex-col gap-6">
      <header class="shrink-0 border-b pb-8">
        <h1 class="studio-title">
          设置
        </h1><p class="mt-2 text-sm leading-6 text-muted-foreground">
          管理模型服务和跨项目长期记忆。
        </p>
      </header>
      <Tabs v-model="tab" class="flex min-h-0 flex-1 flex-col gap-5">
        <TabsList class="shrink-0">
          <TabsTrigger value="model">
            模型服务
          </TabsTrigger>
          <TabsTrigger value="memory">
            长期记忆
          </TabsTrigger>
        </TabsList>
        <TabsContent v-show="tab === 'model'" value="model" force-mount class="flex min-h-0 flex-1 flex-col overflow-hidden">
          <div class="min-h-0 flex-1 space-y-6 overflow-y-auto pb-6">
            <p v-if="error && !status" role="alert" class="text-sm text-destructive">
              {{ error }}
            </p>
            <p v-if="loading" role="status" class="text-sm text-muted-foreground">
              正在读取配置…
            </p>
            <Button v-if="!status && !loading" variant="outline" @click="refresh">
              重新读取
            </Button>
            <template v-if="status">
              <fieldset :disabled="busy" class="min-w-0 overflow-hidden rounded-xl border bg-card">
                <div class="flex items-center justify-between gap-4 border-b px-5 py-4">
                  <div>
                    <h2 class="text-sm font-medium">
                      模型服务 <span v-if="draft.providers.length" class="ml-1 text-muted-foreground">{{ draft.providers.length }}</span>
                    </h2><p class="mt-1 text-xs text-muted-foreground">
                      连接配置和模型统一在这里管理。
                    </p>
                  </div><Button size="sm" :disabled="editing !== null" @click="editing = ''">
                    添加模型服务
                  </Button>
                </div>
                <Dialog :open="editing !== null" @update:open="!$event && (editing = null)">
                  <DialogContent class="flex max-h-[85vh] flex-col gap-0 overflow-hidden p-0 sm:max-w-2xl" @pointer-down-outside.prevent @interact-outside.prevent>
                    <DialogHeader class="shrink-0 border-b px-6 py-5 pr-12 text-left">
                      <DialogTitle>{{ currentProvider ? '编辑模型服务' : '添加模型服务' }}</DialogTitle><DialogDescription>填写连接信息和模型。添加或更新到列表后，仍需点击“保存配置”才能生效。</DialogDescription>
                    </DialogHeader>
                    <ProviderEditor v-if="editing !== null" :key="editing" :provider="currentProvider" :existing-ids="existingIds" @apply="apply" @cancel="editing = null" />
                  </DialogContent>
                </Dialog>
                <div class="divide-y">
                  <Empty v-if="!draft.providers.length">
                    <EmptyHeader>
                      <EmptyTitle>
                        尚未添加模型服务
                      </EmptyTitle><EmptyDescription>
                        点击右上方“添加模型服务”，填写服务地址、API Key 和模型信息。
                      </EmptyDescription>
                    </EmptyHeader>
                  </Empty>
                  <article v-for="provider in draft.providers" :key="provider.id" class="p-5">
                    <div class="flex flex-wrap items-start justify-between gap-3">
                      <div class="min-w-0">
                        <h3 class="font-medium">
                          {{ provider.name }}
                        </h3><p class="mt-1 break-all font-mono text-xs text-muted-foreground">
                          {{ provider.id }}
                        </p>
                      </div><div class="flex gap-1">
                        <Button variant="ghost" size="sm" @click="editing = provider.id">
                          编辑
                        </Button><Button variant="ghost" size="sm" class="text-destructive" @click="remove(provider.id)">
                          移除
                        </Button>
                      </div>
                    </div>
                    <div class="mt-3 flex flex-wrap gap-x-3 gap-y-1 text-xs text-muted-foreground">
                      <span>{{ apiLabel(provider.api) }}</span><span>{{ provider.apiKey || provider.keyConfigured ? 'Key 已填写' : 'Key 未填写' }}</span><span class="break-all">{{ provider.baseUrl }}</span>
                    </div>
                    <ul class="mt-4 divide-y rounded-lg bg-muted/30 px-3 text-sm">
                      <li v-for="model in provider.models" :key="model.id" class="flex flex-wrap items-center justify-between gap-2 py-3">
                        <span>{{ model.name }} <span class="ml-2 font-mono text-xs text-muted-foreground">{{ model.id }}</span></span><span class="text-xs text-muted-foreground">{{ model.contextWindow.toLocaleString() }} 上下文 · {{ model.maxTokens.toLocaleString() }} 输出{{ model.input.includes('image') ? ' · 图片' : '' }}{{ model.reasoning ? ' · 推理' : '' }}</span>
                      </li>
                    </ul>
                  </article>
                </div>
                <FieldGroup class="border-t p-5">
                  <Field orientation="responsive" :data-disabled="busy || editing !== null">
                    <FieldContent>
                      <FieldLabel for="default-model">
                        默认模型
                      </FieldLabel><FieldDescription>
                        用于后续新任务
                      </FieldDescription>
                    </FieldContent><FieldContent class="min-w-0 sm:flex-3">
                      <Select v-if="modelOptions.length" v-model="defaultValue" :disabled="busy || editing !== null">
                        <SelectTrigger id="default-model" class="w-full">
                          <SelectValue placeholder="请选择默认模型" />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectGroup>
                            <SelectItem :value="JSON.stringify(['', ''])">
                              请选择默认模型
                            </SelectItem>
                            <SelectItem v-for="option in modelOptions" :key="option.value" :value="option.value">
                              {{ option.label }}
                            </SelectItem>
                          </SelectGroup>
                        </SelectContent>
                      </Select><FieldDescription v-else>
                        添加模型后可选择
                      </FieldDescription><FieldDescription v-if="modelOptions.length">
                        保存后下次任务生效，正在执行的任务不受影响。
                      </FieldDescription>
                    </FieldContent>
                  </Field>
                </FieldGroup>
              </fieldset>
              <Card>
                <CardHeader>
                  <div class="flex flex-wrap items-center justify-between gap-2">
                    <CardTitle>内置运行时</CardTitle><span class="text-xs text-muted-foreground">{{ status.runtimePrepared ? '资源已准备' : '资源尚未准备' }}</span>
                  </div>
                  <CardDescription>pi {{ status.runtimeVersion }} · Node {{ status.nodeVersion }}</CardDescription>
                </CardHeader><CardContent>
                  <Collapsible>
                    <CollapsibleTrigger as-child>
                      <Button type="button" variant="ghost" size="sm">
                        本地配置文件
                      </Button>
                    </CollapsibleTrigger><CollapsibleContent>
                      <div class="flex flex-col gap-3 pt-3">
                        <p class="text-xs leading-6 text-muted-foreground">
                          API Key 以明文保存在本机，界面不会回显已保存的 Key。
                        </p><p class="break-all font-mono text-[11px] text-muted-foreground">
                          {{ status.configurationDirectory }}
                        </p><div class="flex justify-end">
                          <Button variant="outline" size="sm" @click="openDirectory">
                            打开配置目录
                          </Button>
                        </div>
                      </div>
                    </CollapsibleContent>
                  </Collapsible>
                </CardContent>
              </Card>
            </template>
          </div>
          <div v-if="status" class="flex shrink-0 flex-wrap items-center justify-between gap-3 border-t bg-background py-4">
            <p v-if="error" role="alert" class="w-full break-words text-sm text-destructive">
              {{ error }}
            </p>
            <div class="min-w-0 flex-1 text-xs leading-5">
              <span v-if="saved" role="status" class="text-green-700 dark:text-green-400">配置已保存</span><span v-else-if="dirty" role="status" class="text-muted-foreground">修改尚未保存，离开设置页将丢弃修改。</span><span v-else class="text-muted-foreground">{{ draft.providers.length ? '配置保存在本机' : '未配置时仍可保存需求和管理项目' }}</span>
            </div><div class="ml-auto flex shrink-0 gap-2">
              <Button variant="outline" :disabled="busy || (!dirty && editing === null)" @click="reset">
                放弃修改
              </Button><Button :disabled="!dirty || editing !== null || busy" @click="persist">
                {{ saving ? '保存中…' : '保存配置' }}
              </Button>
            </div>
          </div>
        </TabsContent>
        <TabsContent v-show="tab === 'memory'" value="memory" force-mount class="min-h-0 flex-1 overflow-y-auto pb-6">
          <LongTermMemorySettings :memory-id="memoryId" />
        </TabsContent>
      </Tabs>
    </div>
  </section>
</template>
