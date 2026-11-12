<script setup lang="ts">
import type { Game } from '@/contracts/game'
import type { SkillView } from '@/contracts/skill'
import { computed, defineAsyncComponent, shallowRef, watch } from 'vue'
import { useRouter } from 'vue-router'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Field, FieldDescription, FieldError, FieldGroup, FieldLabel } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Textarea } from '@/components/ui/textarea'
import { gameService } from '@/features/games/service'
import { modService } from '@/features/projects/service'
import { skillsService } from '@/features/skills/service'
import { useProjectsStore } from '@/stores/projects'

const GameCover = defineAsyncComponent(() => import('../games/GameCover.vue'))
const open = defineModel<boolean>('open', { default: false })
const name = shallowRef('')
const description = shallowRef('')
const busy = shallowRef(false)
const error = shallowRef('')
const router = useRouter()
const projects = useProjectsStore()
const games = shallowRef<Game[]>([])
const skills = shallowRef<SkillView[]>([])
const loading = shallowRef(false)
const gameId = shallowRef('')
const gameOptions = computed(() => games.value.map((game) => {
  const skill = skills.value.find(skill => skill.source === 'official' && skill.category === 'game' && skill.gameId === game.id)
  const unavailableReason = game.supportStatus === 'planned'
    ? '待适配'
    : !skill ? '缺少官方游戏 Skill' : skill.error || (!skill.enabled ? '请在 Skills 中启用游戏能力' : '')
  return { ...game, unavailableReason }
}))
const selectedGame = computed(() => gameOptions.value.find(game => game.id === gameId.value))
const canCreate = computed(() => !loading.value && !!selectedGame.value && !selectedGame.value.unavailableReason)
watch(open, async (value, _previous, onCleanup) => {
  if (!value) {
    return
  }
  let current = true
  onCleanup(() => {
    current = false
  })
  loading.value = true
  error.value = ''
  try {
    const [catalog, availableSkills] = await Promise.all([gameService.list(), skillsService.list()])
    if (current) {
      games.value = catalog
      skills.value = availableSkills
      gameId.value = gameOptions.value.find(game => !game.unavailableReason)?.id ?? ''
    }
  }
  catch (cause) {
    if (current) {
      games.value = []
      gameId.value = ''
      error.value = (cause as Error).message
    }
  }
  finally {
    if (current) {
      loading.value = false
    }
  }
})
/** 创建失败保留输入，原生保存成功后才进入工作台。 */
async function create(): Promise<void> {
  if (busy.value || !canCreate.value) {
    return
  }
  busy.value = true
  error.value = ''
  try {
    const project = await modService.create(name.value, gameId.value, description.value)
    projects.upsert(project)
    open.value = false
    name.value = ''
    description.value = ''
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
            <Select v-model="gameId" :disabled="busy || loading">
              <SelectTrigger id="game-skill" class="w-full min-w-0" aria-describedby="game-skill-help">
                <SelectValue placeholder="请选择游戏">
                  <span class="truncate">{{ selectedGame?.name || (loading ? '正在读取本地游戏…' : '请选择游戏') }}</span>
                </SelectValue>
              </SelectTrigger>
              <SelectContent align="start" class="w-(--reka-select-trigger-width)">
                <SelectGroup>
                  <SelectItem v-for="game in gameOptions" :key="game.id" :value="game.id" :text-value="game.name" :disabled="!!game.unavailableReason" class="items-start py-3 [&>span:last-child]:min-w-0 [&>span:last-child]:flex-1">
                    <div class="flex items-start gap-3 whitespace-normal">
                      <GameCover :game-id="game.id" :name="game.name" />
                      <div class="min-w-0 space-y-1">
                        <p class="break-all font-medium">
                          {{ game.name }}
                        </p>
                        <p class="break-words text-xs leading-5 text-muted-foreground">
                          {{ game.description }}
                        </p>
                        <p class="text-xs" :class="game.supportStatus === 'supported' && game.unavailableReason ? 'text-destructive' : 'text-muted-foreground'">
                          {{ game.unavailableReason || '已适配' }}
                        </p>
                      </div>
                    </div>
                  </SelectItem>
                </SelectGroup>
              </SelectContent>
            </Select>
            <FieldDescription id="game-skill-help">
              游戏清单和封面均保存在本地；目前支持逃离鸭科夫与星露谷物语。开始编写无需安装游戏，编译及运行前置按游戏说明准备。
            </FieldDescription>
          </Field>
          <Field :data-disabled="busy">
            <div class="flex items-center justify-between gap-2">
              <FieldLabel for="mod-description">
                MOD 描述
              </FieldLabel>
              <span class="text-xs text-muted-foreground">选填</span>
            </div>
            <Textarea id="mod-description" v-model="description" class="min-h-32 resize-y leading-6" :disabled="busy" rows="4" placeholder="简要介绍这个 MOD 的用途和功能…" />
          </Field>
          <FieldError v-if="error" class="break-words">
            {{ error }}
          </FieldError>
        </FieldGroup>
        <div class="flex shrink-0 justify-end gap-2 border-t pt-4">
          <Button type="button" variant="outline" :disabled="busy" @click="open = false">
            取消
          </Button>
          <Button type="submit" :disabled="busy || !name.trim() || !canCreate">
            {{ busy ? '正在创建…' : '创建项目' }}
          </Button>
        </div>
      </form>
    </DialogContent>
  </Dialog>
</template>
