<script setup lang="ts">
import { defineAsyncComponent, onMounted, onUnmounted, shallowRef } from 'vue'
import { Button } from '@/components/ui/button'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { nativeCall } from '@/services/native'
import { authInvalidEvent, cloudUser } from '@/features/auth/state'
import type { CloudUser } from '@/features/auth/state'
const WorkbenchLayout = defineAsyncComponent(() => import('@/features/workbench/WorkbenchLayout.vue'))
const CloudUpdates = defineAsyncComponent(() => import('@/features/skills/CloudUpdates.vue'))
const LoginPage = defineAsyncComponent(() => import('@/features/auth/LoginPage.vue'))
const ready = shallowRef(false)
const error = shallowRef('')
function invalid() { cloudUser.value = null; error.value = ''; ready.value = true }
async function refresh() {
  error.value = ''
  try { cloudUser.value = await nativeCall<CloudUser>('cloud_me'); ready.value = true }
  catch (cause) { if ((cause as Error).message.startsWith('CLOUD_AUTH_INVALID:')) { invalid() } else { error.value = (cause as Error).message } }
}
function authenticated(user: CloudUser) { cloudUser.value = user; ready.value = true }
onMounted(() => { window.addEventListener(authInvalidEvent, invalid); void refresh() })
onUnmounted(() => window.removeEventListener(authInvalidEvent, invalid))
</script>
<template>
  <div v-if="ready && cloudUser" class="flex h-full min-h-0 flex-col"><CloudUpdates /><div class="min-h-0 flex-1"><WorkbenchLayout /></div></div>
  <LoginPage v-else-if="ready" @authenticated="authenticated" />
  <main v-else class="flex h-full flex-col items-center justify-center gap-4 p-8"><Alert v-if="error" variant="destructive"><AlertDescription>{{ error }}</AlertDescription></Alert><p v-else>正在检查登录状态…</p><Button v-if="error" @click="refresh">重试</Button></main>
</template>
