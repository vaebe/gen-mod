<script setup lang="ts">
import type { SkillView } from '@/contracts/skill'
import { computed, defineAsyncComponent, onMounted, shallowRef, watch } from 'vue'
import { Button } from '@/components/ui/button'
import { Empty, EmptyDescription, EmptyHeader, EmptyTitle } from '@/components/ui/empty'
import { Field, FieldGroup, FieldLabel } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { nativeCall } from '@/services/native'
import { useLongTermMemory } from './useLongTermMemory'

const props = defineProps<{ memoryId?: string }>()
const MemoryEntryEditor = defineAsyncComponent(() => import('./MemoryEntryEditor.vue'))
const { notes, total, page, text, game, status, selected, editing, loading, saving, error, entryError, refresh, select, create, save, changeStatus } = useLongTermMemory()
const skills = shallowRef<SkillView[]>([])
const skillError = shallowRef('')
const games = computed(() => [...new Map(skills.value.filter(skill => skill.category === 'game' && skill.gameId).map(skill => [skill.gameId!, { id: skill.gameId!, name: skill.description || skill.gameId! }])).values()])
watch(() => props.memoryId, (id) => {
  if (id) {
    void select(id)
  }
}, { immediate: true })
onMounted(async () => {
  void refresh(1)
  try {
    skills.value = await nativeCall<SkillView[]>('list_skills')
  }
  catch (cause) { skillError.value = (cause as Error).message }
})
</script>

<template>
  <div class="flex flex-col gap-5">
    <div class="flex items-start justify-between gap-3">
      <p class="text-sm text-muted-foreground">
        跨项目复用的开发经验，可随时修改、停用或删除。来源项目移除后仍会保留。
      </p>
      <Button :disabled="saving" @click="create">
        新增记忆
      </Button>
    </div>
    <form @submit.prevent="refresh(1)">
      <FieldGroup class="grid gap-3 sm:grid-cols-3">
        <Field>
          <FieldLabel for="memory-search">
            搜索
          </FieldLabel>
          <Input id="memory-search" v-model="text" placeholder="标题或内容" />
        </Field>
        <Field>
          <FieldLabel for="memory-filter-game">
            游戏
          </FieldLabel>
          <Select v-model="game">
            <SelectTrigger id="memory-filter-game">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                <SelectItem value="all">
                  全部游戏
                </SelectItem>
                <SelectItem value="*">
                  通用
                </SelectItem>
                <SelectItem v-for="item in games" :key="item.id" :value="item.id">
                  {{ item.name }}
                </SelectItem>
              </SelectGroup>
            </SelectContent>
          </Select>
        </Field>
        <Field>
          <FieldLabel for="memory-filter-status">
            状态
          </FieldLabel>
          <Select v-model="status">
            <SelectTrigger id="memory-filter-status">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                <SelectItem value="all">
                  全部状态
                </SelectItem>
                <SelectItem value="active">
                  已启用
                </SelectItem>
                <SelectItem value="candidate">
                  历史候选
                </SelectItem>
                <SelectItem value="archived">
                  已停用
                </SelectItem>
              </SelectGroup>
            </SelectContent>
          </Select>
        </Field>
      </FieldGroup>
      <Button type="submit" variant="outline" class="mt-3" :disabled="loading">
        {{ loading ? '读取中…' : '筛选' }}
      </Button>
    </form>
    <p v-if="skillError" role="alert" class="text-sm text-destructive">
      游戏列表读取失败：{{ skillError }}
    </p>
    <div v-if="error" role="alert" class="flex items-center gap-3 text-sm text-destructive">
      {{ error }}
      <Button variant="outline" @click="refresh()">
        重试
      </Button>
    </div>
    <Empty v-if="!loading && !error && !notes.length">
      <EmptyHeader>
        <EmptyTitle>
          没有匹配的长期记忆
        </EmptyTitle>
        <EmptyDescription>
          可调整筛选条件，或手动新增。
        </EmptyDescription>
      </EmptyHeader>
    </Empty>
    <ul v-else class="flex flex-col gap-2">
      <li v-for="note in notes" :key="note.noteId">
        <Button variant="outline" class="h-auto w-full justify-start whitespace-normal py-3 text-left text-sm" :disabled="saving" @click="select(note.noteId)">
          <span class="flex min-w-0 flex-col gap-1">
            <span>
              {{ note.title }}
            </span>
            <span class="line-clamp-2 text-xs font-normal text-muted-foreground">
              {{ note.content }}
            </span>
            <span class="text-xs font-normal text-muted-foreground">
              {{ note.status === 'archived' ? '已停用' : note.status === 'candidate' ? '历史候选' : note.allowReuse ? '已启用' : '未启用复用' }} · {{ note.versionScope === 'exact' ? note.gameVersion : note.versionScope === 'unknown' ? '版本待核对' : '不依赖版本' }}
            </span>
          </span>
        </Button>
      </li>
    </ul>
    <div class="flex items-center justify-between gap-3 text-sm">
      <span>
        共 {{ total }} 条 · 第 {{ page }} 页
      </span>
      <div class="flex gap-2">
        <Button variant="outline" size="sm" :disabled="loading || page <= 1" @click="refresh(page - 1)">
          上一页
        </Button>
        <Button variant="outline" size="sm" :disabled="loading || page * 20 >= total" @click="refresh(page + 1)">
          下一页
        </Button>
      </div>
    </div>
    <MemoryEntryEditor v-if="editing" :key="selected?.noteId ?? 'new'" :note="selected" :games="games" :busy="saving" :error="entryError" @save="save" @status="changeStatus" @reload="selected && select(selected.noteId)" />
    <p v-else-if="entryError" role="alert" class="text-sm text-destructive">
      {{ entryError }}
    </p>
  </div>
</template>
