<script setup lang="ts">
import { computed, onMounted, shallowRef } from "vue";
import type { SkillDefinition, SkillView } from "@/contracts/skill";
import { nativeCall } from "@/services/native";
import { useSessionStore } from "@/stores/session";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import SkillDetails from "./SkillDetails.vue";
const skills = shallowRef<SkillView[]>([]);
const error = shallowRef("");
const busy = shallowRef(false);
const directory = shallowRef("");
const metadata = shallowRef('{"name":"my-game-mod","description":"我的游戏 MOD 能力","category":"game","gameId":"my-game","version":"1.0.0","dependencies":["mod-workflow"],"path":"","excludePaths":[]}');
const session = useSessionStore();
const loaded = computed(() => session.client.connection === "connected" ? session.loadedSkills : []);
async function refresh(): Promise<void> {
  try {
    skills.value = await nativeCall<SkillView[]>("list_skills");
  } catch (cause) { error.value = (cause as Error).message; }
}
async function toggle(skill: SkillView): Promise<void> {
  busy.value = true; error.value = "";
  try { await nativeCall("set_skill_enabled", { name: skill.name, enabled: !skill.enabled }); await refresh(); }
  catch (cause) { error.value = (cause as Error).message; }
  finally { busy.value = false; }
}
async function importLocal(): Promise<void> {
  busy.value = true; error.value = "";
  try {
    const definition = JSON.parse(metadata.value) as SkillDefinition;
    await nativeCall("import_skill", { directory: directory.value, definition });
    directory.value = ""; await refresh();
  } catch (cause) { error.value = (cause as Error).message; }
  finally { busy.value = false; }
}
onMounted(refresh);
</script>
<template>
  <section class="mx-auto max-w-3xl space-y-6 px-6 py-8">
    <div><h1 class="text-xl font-semibold">Skills</h1><p class="mt-2 text-sm text-muted-foreground">每个项目只加载所选游戏能力及其通用依赖。变更在下次新会话生效，旧会话保留为只读历史。</p></div>
    <p v-if="error" class="text-sm text-destructive" role="alert">{{ error }}</p>
    <SkillDetails v-for="skill in skills" :key="skill.name" :skill="skill" :loaded="loaded.includes(skill.name)" :disabled="busy || session.working" @toggle="toggle(skill)" />
    <details class="rounded-lg border p-4"><summary class="cursor-pointer text-sm font-medium">导入本地能力</summary><form class="mt-4 space-y-3" @submit.prevent="importLocal"><label class="block text-sm">目录绝对路径<Input v-model="directory" class="mt-2" required /></label><label class="block text-sm">应用能力元数据<Textarea v-model="metadata" class="mt-2 font-mono text-xs" rows="7" required /></label><p class="text-xs text-muted-foreground">目录需包含标准 SKILL.md。只校验并复制文件，不执行导入脚本；内置能力不可覆盖。</p><Button :disabled="busy || !directory.trim()">导入</Button></form></details>
  </section>
</template>
