<script setup lang="ts">
import type { KnowledgeNote, MemoryEntryInput } from '@/contracts/knowledge'
import { reactive, shallowRef, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { Field, FieldDescription, FieldGroup, FieldLabel } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Textarea } from '@/components/ui/textarea'

const props = defineProps<{ note?: KnowledgeNote, games: { id: string, name: string }[], busy: boolean, error: string }>()
const emit = defineEmits<{ save: [input: MemoryEntryInput], status: [value: 'active' | 'archived' | 'deleted'], reload: [] }>()
const draft = reactive<MemoryEntryInput>({ title: '', content: '', gameId: '*', versionScope: 'independent' })
const scopeOpen = shallowRef(false)
watch(() => props.note, (note) => {
  Object.assign(draft, { title: note?.title ?? '', content: note?.content ?? '', gameId: note?.gameId ?? '*', versionScope: note?.versionScope ?? 'independent', gameVersion: note?.gameVersion ?? '' })
}, { immediate: true })
watch(() => draft.gameId, (value) => {
  if (value === '*') {
    draft.versionScope = 'independent'
    draft.gameVersion = ''
  }
})
function save(): void {
  emit('save', { ...draft, gameVersion: draft.versionScope === 'exact' ? draft.gameVersion?.trim() : undefined })
}
</script>

<template>
  <form class="flex flex-col gap-4 rounded-lg border p-4" @submit.prevent="save">
    <h3 class="font-medium">
      {{ note ? '编辑长期记忆' : '新增长期记忆' }}
    </h3>
    <FieldGroup>
      <Field>
        <FieldLabel for="memory-title">
          标题
        </FieldLabel>
        <Input id="memory-title" v-model="draft.title" required maxlength="120" :disabled="busy" />
      </Field>
      <Field>
        <FieldLabel for="memory-content">
          内容
        </FieldLabel>
        <Textarea id="memory-content" v-model="draft.content" required maxlength="2000" :disabled="busy" class="min-h-40" />
        <FieldDescription>
          用户维护的内容会受到保护，AI 不会静默覆盖或恢复已删除的内容。
        </FieldDescription>
      </Field>
    </FieldGroup>
    <Collapsible v-model:open="scopeOpen">
      <CollapsibleTrigger as-child>
        <Button type="button" variant="ghost" size="sm">
          适用范围：{{ draft.gameId === '*' ? '通用' : games.find(game => game.id === draft.gameId)?.name ?? draft.gameId }} · {{ draft.versionScope === 'exact' ? draft.gameVersion : draft.versionScope === 'unknown' ? '版本待核对' : '不依赖版本' }}
        </Button>
      </CollapsibleTrigger>
      <CollapsibleContent>
        <FieldGroup class="pt-3">
          <Field>
            <FieldLabel for="memory-game">
              适用游戏
            </FieldLabel>
            <Select v-model="draft.gameId" :disabled="busy">
              <SelectTrigger id="memory-game">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  <SelectItem value="*">
                    通用
                  </SelectItem>
                  <SelectItem v-for="game in games" :key="game.id" :value="game.id">
                    {{ game.name }}
                  </SelectItem>
                  <SelectItem v-if="draft.gameId !== '*' && !games.some(game => game.id === draft.gameId)" :value="draft.gameId">
                    {{ draft.gameId }}
                  </SelectItem>
                </SelectGroup>
              </SelectContent>
            </Select>
          </Field>
          <Field v-if="draft.gameId !== '*'">
            <FieldLabel for="memory-scope">
              版本范围
            </FieldLabel>
            <Select v-model="draft.versionScope" :disabled="busy">
              <SelectTrigger id="memory-scope">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  <SelectItem value="independent">
                    不依赖版本
                  </SelectItem>
                  <SelectItem value="unknown">
                    版本待核对
                  </SelectItem>
                  <SelectItem value="exact">
                    指定版本
                  </SelectItem>
                </SelectGroup>
              </SelectContent>
            </Select>
          </Field>
          <Field v-if="draft.versionScope === 'exact'">
            <FieldLabel for="memory-version">
              游戏版本
            </FieldLabel>
            <Input id="memory-version" v-model="draft.gameVersion" required :disabled="busy" />
          </Field>
        </FieldGroup>
      </CollapsibleContent>
    </Collapsible>
    <p v-if="note" class="text-xs text-muted-foreground">
      {{ note.originProjectName ? `来源项目：${note.originProjectName} · ` : '' }}{{ note.updatedAt ?? note.createdAt ?? '历史记录' }} · {{ note.userEdited ? '用户维护' : 'AI 自动保存' }} · {{ note.sourceRefs.length }} 条来源记录
    </p>
    <div v-if="error" role="alert" class="flex flex-col items-start gap-2 text-sm text-destructive">
      <p>
        {{ error }}
      </p>
      <Button v-if="note" type="button" variant="outline" size="sm" :disabled="busy" @click="emit('reload')">
        重新载入（放弃当前输入）
      </Button>
    </div>
    <div class="flex flex-wrap gap-2">
      <Button type="submit" :disabled="busy">
        {{ busy ? '处理中…' : '保存记忆' }}
      </Button>
      <template v-if="note">
        <Button type="button" variant="outline" :disabled="busy" @click="emit('status', note.status === 'active' && note.allowReuse ? 'archived' : 'active')">
          {{ note.status === 'active' && note.allowReuse ? '停用' : '启用复用' }}
        </Button>
        <Button type="button" variant="destructive" :disabled="busy" @click="emit('status', 'deleted')">
          删除
        </Button>
      </template>
    </div>
  </form>
</template>
