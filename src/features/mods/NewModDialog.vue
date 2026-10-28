<script setup lang="ts">
import type { SkillView } from '@/contracts/skill'
import { shallowRef, watch } from 'vue'
import { useRouter } from 'vue-router'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Field, FieldDescription, FieldError, FieldGroup, FieldLabel } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Textarea } from '@/components/ui/textarea'
import { modService, nativeCall } from '@/services/native'
import { useProjectsStore } from '@/stores/projects'

const open = defineModel<boolean>('open', { default: false })
const name = shallowRef('')
const requirement = shallowRef('')
const busy = shallowRef(false)
const error = shallowRef('')
const router = useRouter()
const projects = useProjectsStore()
const games = shallowRef<{ id: string, name: string, description: string }[]>([])
const gameId = shallowRef('')
watch(open, async (value) => {
  if (!value) {
    return
  }
  try {
    const [synced, skills] = await Promise.all([nativeCall<{ id: string, name: string, description: string }[]>('cloud_games'), nativeCall<SkillView[]>('list_skills')])
    games.value = synced.filter(game => skills.some(skill => skill.source === 'official' && skill.category === 'game' && skill.gameId === game.id && skill.enabled && !skill.error))
    gameId.value = games.value[0]?.id ?? ''
  }
  catch (cause) { error.value = (cause as Error).message }
})
/** 创建失败保留输入，原生保存成功后才进入工作台。 */
async function create(): Promise<void> {
  busy.value = true
  error.value = ''
  try {
    const project = await modService.create(name.value, gameId.value, requirement.value)
    projects.upsert(project)
    open.value = false
    name.value = ''
    requirement.value = ''
    await router.push(`/mods/${project.modId}/chat`)
  }
  catch (cause) { error.value = (cause as Error).message }
  finally { busy.value = false }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogContent class="flex max-h-[calc(100dvh-2rem)] flex-col gap-6 sm:max-w-xl" @interact-outside="busy && $event.preventDefault()" @escape-key-down="busy && $event.preventDefault()">
      <DialogHeader class="shrink-0 gap-2 pr-6">
        <DialogTitle>新建 MOD</DialogTitle>
        <DialogDescription class="leading-6">
          源码与每次交付会保存在独立的本地项目目录。
        </DialogDescription>
      </DialogHeader>
      <form class="flex min-h-0 flex-col gap-5" @submit.prevent="create">
        <FieldGroup class="min-h-0 gap-5 overflow-y-auto px-1 pb-1">
          <Field :data-disabled="busy">
            <FieldLabel for="mod-name">
              项目名称
            </FieldLabel>
            <Input id="mod-name" v-model="name" maxlength="120" required autofocus :disabled="busy" placeholder="例如：仓库信息面板" />
          </Field>
          <Field :data-disabled="busy">
            <FieldLabel for="game-skill">
              游戏
            </FieldLabel>
            <Select v-model="gameId" :disabled="busy">
              <SelectTrigger id="game-skill" class="w-full min-w-0" aria-describedby="game-skill-help">
                <SelectValue placeholder="请选择游戏">
                  <span class="truncate">{{ gameId }}</span>
                </SelectValue>
              </SelectTrigger>
              <SelectContent align="start" class="w-(--reka-select-trigger-width)">
                <SelectGroup>
                  <SelectItem v-for="game in games" :key="game.id" :value="game.id" :text-value="game.name" class="items-start py-3 [&>span:last-child]:min-w-0 [&>span:last-child]:flex-1">
                    <div class="min-w-0 space-y-1 whitespace-normal">
                      <p class="break-all font-medium">
                        {{ game.name }}
                      </p>
                      <p class="break-words text-xs leading-5 text-muted-foreground">
                        {{ game.description }}
                      </p>
                    </div>
                  </SelectItem>
                </SelectGroup>
              </SelectContent>
            </Select>
            <FieldDescription id="game-skill-help">
              无需安装游戏即可开始开发。
            </FieldDescription>
          </Field>
          <Field :data-disabled="busy">
            <div class="flex items-center justify-between gap-2">
              <FieldLabel for="mod-requirement">
                你想做什么？
              </FieldLabel>
              <span class="text-xs text-muted-foreground">选填</span>
            </div>
            <Textarea id="mod-requirement" v-model="requirement" class="min-h-32 resize-y leading-6" :disabled="busy" rows="4" placeholder="描述希望改变的行为、界面或规则…" />
          </Field>
          <FieldError v-if="error" class="break-words">
            {{ error }}
          </FieldError>
        </FieldGroup>
        <div class="flex shrink-0 justify-end gap-2 border-t pt-4">
          <Button type="button" variant="outline" :disabled="busy" @click="open = false">
            取消
          </Button>
          <Button type="submit" :disabled="busy || !name.trim() || !gameId">
            {{ busy ? '正在创建…' : '创建项目' }}
          </Button>
        </div>
      </form>
    </DialogContent>
  </Dialog>
</template>
