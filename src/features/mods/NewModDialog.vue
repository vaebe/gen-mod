<script setup lang="ts">
import { shallowRef, watch } from "vue";
import { useRouter } from "vue-router";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { modService, nativeCall } from "@/services/native";
import type { SkillView } from "@/contracts/skill";
import { useProjectsStore } from "@/stores/projects";
const open = defineModel<boolean>("open", { default: false });
const name = shallowRef("");
const requirement = shallowRef("");
const busy = shallowRef(false);
const error = shallowRef("");
const router = useRouter();
const projects = useProjectsStore();
const games = shallowRef<SkillView[]>([]);
const gameSkill = shallowRef("");
watch(open, async (value) => {
  if (!value) { return; }
  try {
    games.value = (await nativeCall<SkillView[]>("list_skills")).filter((v) => v.category === "game" && v.enabled && !v.error);
    gameSkill.value = games.value[0]?.name ?? "";
  } catch (cause) { error.value = (cause as Error).message; }
});
/** 创建失败保留输入，原生保存成功后才进入工作台。 */
async function create(): Promise<void> {
  busy.value = true;
  error.value = "";
  try {
    const project = await modService.create(name.value, gameSkill.value, requirement.value);
    projects.upsert(project);
    open.value = false;
    name.value = "";
    requirement.value = "";
    await router.push(`/mods/${project.modId}/chat`);
  } catch (cause) { error.value = (cause as Error).message; }
  finally { busy.value = false; }
}
</script>
<template>
  <Dialog v-model:open="open"><DialogContent @interact-outside="busy && $event.preventDefault()" @escape-key-down="busy && $event.preventDefault()"><DialogHeader><DialogTitle>新建 MOD</DialogTitle><DialogDescription>源码与每次交付会保存在独立的本地项目目录。</DialogDescription></DialogHeader>
    <form class="space-y-4" @submit.prevent="create">
      <div class="space-y-2"><label for="mod-name" class="text-sm font-medium">项目名称</label><Input id="mod-name" v-model="name" maxlength="120" required autofocus :disabled="busy" placeholder="例如：仓库信息面板" /></div>
      <div class="space-y-2"><label class="text-sm font-medium" for="game-skill">游戏能力</label><select id="game-skill" v-model="gameSkill" class="h-9 w-full rounded-md border bg-background px-3 text-sm" :disabled="busy"><option v-for="game in games" :key="game.name" :value="game.name">{{ game.name }} · {{ game.description }}</option></select><p class="text-xs text-muted-foreground">无需安装游戏即可开始开发。</p></div>
      <div class="space-y-2"><label for="mod-requirement" class="text-sm font-medium">你想做什么？</label><Textarea id="mod-requirement" v-model="requirement" :disabled="busy" rows="5" placeholder="描述希望改变的行为、界面或规则…" /></div>
      <p v-if="error" role="alert" class="text-sm text-destructive">{{ error }}</p><div class="flex justify-end"><Button type="submit" :disabled="busy || !name.trim() || !gameSkill">{{ busy ? '正在创建…' : '创建项目' }}</Button></div>
    </form>
  </DialogContent></Dialog>
</template>
