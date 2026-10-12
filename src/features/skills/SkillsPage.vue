<script setup lang="ts">
import type { SkillDefinition, SkillView } from '@/contracts/skill'
import { defineAsyncComponent, onMounted, shallowRef } from 'vue'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { nativeCall } from '@/services/native'
import { useSessionStore } from '@/stores/session'
import SkillDetails from './SkillDetails.vue'

const SkillContentDialog = defineAsyncComponent(() => import('./SkillContentDialog.vue'))
const selectedSkill = shallowRef<SkillView | null>(null)
const detailOpen = shallowRef(false)
const skills = shallowRef<SkillView[]>([])
const error = shallowRef('')
const busy = shallowRef(false)
const directory = shallowRef('')
const metadata = shallowRef('{"name":"my-game-mod","description":"我的游戏 MOD 能力","category":"game","gameId":"my-game","version":"1.0.0","dependencies":["mod-workflow"],"path":"","excludePaths":[]}')
const session = useSessionStore()
async function refresh(): Promise<void> {
  try {
    skills.value = await nativeCall<SkillView[]>('list_skills')
  }
  catch (cause) { error.value = (cause as Error).message }
}
/** 保存开关状态后刷新列表，失败时保留原状态。 */
async function toggle(skill: SkillView, enabled: boolean): Promise<void> {
  if (busy.value || session.anyBusy) {
    return
  }
  busy.value = true
  error.value = ''
  try {
    await nativeCall('set_skill_enabled', { name: skill.name, enabled })
    await refresh()
  }
  catch (cause) { error.value = (cause as Error).message }
  finally { busy.value = false }
}
/** 取消系统目录选择时保留原来的目录。 */
async function chooseDirectory(): Promise<void> {
  busy.value = true
  error.value = ''
  try {
    const selected = await nativeCall<string | null>('pick_directory')
    if (selected) {
      directory.value = selected
    }
  }
  catch (cause) { error.value = (cause as Error).message }
  finally { busy.value = false }
}
/** 只打开详情，不改变能力的启用状态。 */
function showSkill(skill: SkillView): void {
  selectedSkill.value = skill
  detailOpen.value = true
}
async function importLocal(): Promise<void> {
  if (busy.value || session.anyBusy) {
    return
  }
  busy.value = true
  error.value = ''
  try {
    const definition = JSON.parse(metadata.value) as SkillDefinition
    await nativeCall('import_skill', { directory: directory.value, definition })
    directory.value = ''
    await refresh()
  }
  catch (cause) { error.value = (cause as Error).message }
  finally { busy.value = false }
}
onMounted(refresh)
</script>

<template>
  <section class="h-full min-h-0 overflow-y-auto">
    <div class="mx-auto max-w-3xl space-y-6 px-6 py-8">
      <div>
        <h1 class="text-xl font-semibold">
          Skills
        </h1><p class="mt-2 text-sm text-muted-foreground">
          每个项目只加载所选游戏能力及其通用依赖。变更在下次新会话生效，旧会话保留为只读历史。
        </p>
      </div>
      <p v-if="error" class="text-sm text-destructive" role="alert">
        {{ error }}
      </p>
      <ul class="divide-y border-y">
        <SkillDetails v-for="skill in skills" :key="skill.name" :skill="skill" :disabled="busy || session.anyBusy" @toggle="toggle(skill, $event)" @select="showSkill(skill)" />
      </ul>
      <details class="rounded-lg border p-4">
        <summary class="cursor-pointer text-sm font-medium">
          导入本地能力
        </summary><form class="mt-4 space-y-3" @submit.prevent="importLocal">
          <div class="space-y-2">
            <label for="skill-directory" class="block text-sm">能力目录</label><div class="flex gap-2">
              <Input id="skill-directory" v-model="directory" class="min-w-0 flex-1" :disabled="busy" placeholder="选择包含 SKILL.md 的目录" required /><Button type="button" variant="outline" :disabled="busy || session.anyBusy" @click="chooseDirectory">
                选择目录
              </Button>
            </div>
          </div>
          <label class="block text-sm">应用能力元数据<Textarea v-model="metadata" class="mt-2 font-mono text-xs" :disabled="busy" rows="7" required /></label><p class="text-xs text-muted-foreground">
            目录需包含标准 SKILL.md。只校验并复制文件，不执行导入脚本；内置能力不可覆盖。
          </p><Button :disabled="busy || session.anyBusy || !directory.trim()">
            导入
          </Button>
        </form>
      </details>
    </div>
    <SkillContentDialog v-if="selectedSkill" v-model:open="detailOpen" :skill="selectedSkill" />
  </section>
</template>
