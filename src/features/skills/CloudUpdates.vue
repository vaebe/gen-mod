<script setup lang="ts">
import { onMounted, shallowRef } from 'vue'
import { Button } from '@/components/ui/button'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogFooter } from '@/components/ui/dialog'
import { nativeCall } from '@/services/native'
import { cloudUser, authInvalidEvent } from '@/features/auth/state'
import { useProjectsStore } from '@/stores/projects'
interface Updates { newGames: { id: string, name: string }[], versions: { skillId: string, name: string, version: string, fileId: string }[] }
const updates = shallowRef<Updates>({ newGames: [], versions: [] })
const open = shallowRef(false)
const busy = shallowRef(false)
const error = shallowRef('')
const status = shallowRef('')
const projects = useProjectsStore()
async function check() {
  busy.value = true; error.value = ''; status.value = ''
  try { updates.value = await nativeCall<Updates>('cloud_check_updates'); await projects.refresh(); open.value = !!(updates.value.newGames.length || updates.value.versions.length); if (!open.value) { status.value = '官方目录已是最新' } }
  catch (cause) { error.value = (cause as Error).message }
  finally { busy.value = false }
}
async function confirm() {
  busy.value = true; error.value = ''
  try {
    await nativeCall('cloud_confirm_updates', { gameIds: updates.value.newGames.map(game => game.id), fileIds: updates.value.versions.map(version => version.fileId) })
    open.value = false; status.value = '更新已确认'; await projects.refresh(); window.dispatchEvent(new Event('genmod-catalog-updated'))
  }
  catch (cause) { error.value = (cause as Error).message }
  finally { busy.value = false }
}
async function logout() { try { await nativeCall('cloud_logout'); window.dispatchEvent(new Event(authInvalidEvent)) } catch (cause) { error.value = (cause as Error).message } }
onMounted(check)
</script>
<template>
  <div class="shrink-0 border-b px-4 py-2">
    <div class="flex items-center justify-end gap-3"><span class="truncate text-xs text-muted-foreground">{{ status || cloudUser?.email }}</span><Button variant="outline" size="sm" :disabled="busy" @click="check">检查官方更新</Button><Button variant="ghost" size="sm" :disabled="busy" @click="logout">退出登录</Button></div>
    <Alert v-if="error" variant="destructive" class="mt-2"><AlertDescription>{{ error }}</AlertDescription></Alert>
  </div>
  <Dialog v-model:open="open">
    <DialogContent class="max-h-[85vh] overflow-y-auto" @interact-outside="busy && $event.preventDefault()" @escape-key-down="busy && $event.preventDefault()">
      <DialogHeader><DialogTitle>发现官方游戏或 Skill 更新</DialogTitle><DialogDescription>请确认以下变化后下载。</DialogDescription></DialogHeader>
      <ul class="flex flex-col gap-1 text-sm"><li v-for="game in updates.newGames" :key="game.id">新游戏：{{ game.name }}</li><li v-for="version in updates.versions" :key="version.fileId">{{ version.name }} · {{ version.version }}</li></ul>
      <ul class="list-disc pl-5 text-sm leading-6 text-muted-foreground">
        <li>确认后按 fileId 下载 zip，解压到该 Skill 的新版本目录，不覆盖正在使用的目录。</li>
        <li>之后新建的会话使用此次确认的版本。</li>
        <li>已开始的任务继续使用开始时的内容哈希和目录。</li>
        <li>快照与当前已确认版本不同的会话变为只读，不能发新任务。</li>
        <li>本机旧版本、导入和其他文件全部保留，不会删除。取消或关闭不下载、不切换版本。</li>
      </ul>
      <Alert v-if="error" variant="destructive"><AlertDescription>{{ error }}</AlertDescription></Alert>
      <DialogFooter><Button variant="outline" :disabled="busy" @click="open = false">暂不更新</Button><Button :disabled="busy" @click="confirm">{{ busy ? '正在下载并校验…' : '确认更新' }}</Button></DialogFooter>
    </DialogContent>
  </Dialog>
</template>
