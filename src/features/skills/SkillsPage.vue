<script setup lang="ts">
import type { SkillDefinition, SkillView } from '@/contracts/skill'
import { defineAsyncComponent, onMounted, shallowRef } from 'vue'
import { Button } from '@/components/ui/button'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { Field, FieldDescription, FieldError, FieldGroup, FieldLabel } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { useSessionStore } from '@/features/sessions/useSessionStore'
import { skillsService } from '@/features/skills/service'
import SkillDetails from './SkillDetails.vue'

const SkillContentDialog = defineAsyncComponent(() => import('./SkillContentDialog.vue'))
const selectedSkill = shallowRef<SkillView | null>(null)
const detailOpen = shallowRef(false)
const skills = shallowRef<SkillView[]>([])
const error = shallowRef('')
const busy = shallowRef(false)
const directory = shallowRef('')
const metadata = shallowRef('{"name":"my-local-skill","description":"本机导入的通用 Skill","category":"common","gameId":null,"version":"1.0.0","dependencies":[],"path":"","excludePaths":[]}')
const session = useSessionStore()
async function refresh(): Promise<void> {
  try {
    skills.value = await skillsService.list()
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
    await skillsService.setEnabled(skill.name, enabled)
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
    const selected = await skillsService.pickDirectory()
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
    await skillsService.importLocal(directory.value, definition)
    directory.value = ''
    await refresh()
  }
  catch (cause) { error.value = (cause as Error).message }
  finally { busy.value = false }
}
onMounted(refresh)
</script>

<template>
  <section class="studio-page min-h-0">
    <div class="studio-content space-y-6">
      <header class="border-b pb-8">
        <h1 class="studio-title">
          Skills
        </h1><p class="mt-2 max-w-2xl text-sm leading-6 text-muted-foreground">
          管理 AI 制作 MOD 时使用的通用能力与对应游戏能力。修改启用状态后，下次新会话生效；旧会话保留为只读历史。
        </p>
      </header>
      <FieldError v-if="error">
        {{ error }}
      </FieldError>
      <ul class="divide-y overflow-hidden rounded-xl border bg-card px-2">
        <SkillDetails v-for="skill in skills" :key="skill.name" :skill="skill" :disabled="busy || session.anyBusy" @toggle="toggle(skill, $event)" @select="showSkill(skill)" />
      </ul>
      <Collapsible class="rounded-xl border bg-card p-4">
        <CollapsibleTrigger as-child>
          <Button type="button" variant="ghost" size="sm">
            导入本地能力
          </Button>
        </CollapsibleTrigger><CollapsibleContent>
          <form class="mt-4" @submit.prevent="importLocal">
            <FieldGroup>
              <Field :data-disabled="busy">
                <FieldLabel for="skill-directory">
                  能力目录
                </FieldLabel><div class="flex gap-2">
                  <Input id="skill-directory" v-model="directory" class="min-w-0 flex-1" :disabled="busy" placeholder="选择包含 SKILL.md 的目录" required /><Button type="button" variant="outline" :disabled="busy || session.anyBusy" @click="chooseDirectory">
                    选择目录
                  </Button>
                </div>
              </Field>
              <Field :data-disabled="busy">
                <FieldLabel for="skill-metadata">
                  能力配置（JSON）
                </FieldLabel>
                <Textarea id="skill-metadata" v-model="metadata" class="font-mono" :disabled="busy" rows="7" required aria-describedby="skill-import-help" />
                <FieldDescription id="skill-import-help">
                  目录需包含标准 SKILL.md。游戏 Skill 只能绑定本地清单中的游戏，名称不能与官方 Skill 或已有导入重复。只校验并复制，不执行脚本。
                </FieldDescription>
              </Field><Button type="submit" class="self-start" :disabled="busy || session.anyBusy || !directory.trim()">
                导入
              </Button>
            </FieldGroup>
          </form>
        </CollapsibleContent>
      </Collapsible>
    </div>
    <SkillContentDialog v-if="selectedSkill" v-model:open="detailOpen" :skill="selectedSkill" />
  </section>
</template>
