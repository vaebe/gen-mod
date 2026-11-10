<script setup lang="ts">
import type { SteeringInput } from '@/contracts/session'
import type { FollowupInput } from '@/services/pi/followupQueue'
import { ArrowUp, Pencil, X } from '@lucide/vue'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'

defineProps<{ items: FollowupInput[], editingId?: string, pending: SteeringInput[], paused: boolean, busy: boolean, working: boolean, canSteer: boolean, canSend: boolean, hasDraft: boolean, readOnly: boolean, error: string }>()
defineEmits<{ edit: [id: string], remove: [id: string], deliver: [id: string], resume: [] }>()
const labels: Record<FollowupInput['status'], string> = { pending: '待发送', sending: '正在发送…', accepted: '已发送，等待接收', unknown: '结果未确认，请重新连接核对，勿重复发送', failed: '发送失败，可编辑后重新提交', interrupted: '任务已失败、取消或中断，请核对执行结果后编辑或撤回', handled: '已由扩展接收' }
</script>

<template>
  <div v-if="items.length || pending.length || error" class="shrink-0 px-4 pt-3 lg:px-8">
    <Card class="mx-auto flex max-w-3xl flex-col gap-2 py-3">
      <CardHeader class="px-4 py-0">
        <div class="flex items-center justify-between gap-2">
          <CardTitle>待发送 · {{ items.filter(item => item.status === 'pending').length }}</CardTitle>
          <Button v-if="paused && items.some(item => item.status === 'pending') && !readOnly" variant="ghost" size="sm" :disabled="busy || !!editingId || (!working && !canSend)" :title="editingId ? '请先提交编辑中的消息，或撤回原队列消息' : undefined" @click="$emit('resume')">
            继续队列
          </Button>
        </div>
        <CardDescription>{{ paused ? '队列已暂停，消息会保留' : '本轮结束后按顺序发送；引导会直接发送给当前任务' }}</CardDescription>
      </CardHeader>
      <CardContent class="max-h-48 overflow-y-auto px-4 py-0">
        <ul class="flex flex-col gap-3" aria-label="待发送消息">
          <li v-for="item in items" :key="item.id" class="flex items-start gap-3">
            <div class="min-w-0 flex-1">
              <p class="line-clamp-3 whitespace-pre-wrap break-words text-sm" :title="item.text">
                {{ item.text || '请根据附件继续处理当前 MOD。' }}
              </p>
              <p v-if="item.attachments.length" class="truncate text-xs text-muted-foreground" :title="item.attachments.map(file => file.name).join('、')">
                附件：{{ item.attachments.map(file => file.name).join('、') }}
              </p>
              <p role="status" class="text-xs text-muted-foreground">
                {{ item.id === editingId ? '编辑转移未完成，重新提交将替换此消息' : item.error || labels[item.status] }}
              </p>
            </div>
            <div v-if="['pending', 'failed', 'interrupted'].includes(item.status) && !readOnly" class="flex shrink-0 items-center gap-1">
              <Button variant="ghost" size="icon" :disabled="busy || hasDraft" aria-label="编辑消息" :title="hasDraft ? '请先处理输入框中的草稿' : '移回输入框编辑，重新提交后排到队尾'" @click="$emit('edit', item.id)">
                <Pencil />
              </Button>
              <Button variant="ghost" size="icon" :disabled="busy" aria-label="撤回消息" title="撤回" @click="$emit('remove', item.id)">
                <X />
              </Button>
              <Button v-if="item.status === 'pending'" variant="outline" size="sm" :disabled="busy || item.id === editingId || (working ? !canSteer : !canSend)" :title="working ? '立即发送给当前任务，在下一可处理时机生效' : '作为新一轮请求发送'" @click="$emit('deliver', item.id)">
                <ArrowUp data-icon="inline-start" />{{ working ? '引导' : '发送' }}
              </Button>
            </div>
          </li>
          <li v-for="input in pending" :key="input.inputId" class="flex flex-col gap-1">
            <p class="line-clamp-3 whitespace-pre-wrap break-words text-sm">
              {{ input.message }}
            </p>
            <p class="text-xs text-muted-foreground">
              {{ input.status === 'handled' ? '已由扩展接收' : input.status === 'unknown' ? '结果未确认，请重新连接核对' : input.status === 'rejected' ? '引导被拒绝' : '已投递，等待进入会话' }}；不可编辑或撤回
            </p>
          </li>
        </ul>
        <p v-if="error" role="alert" class="mt-2 text-xs text-destructive">
          {{ error }}
        </p>
      </CardContent>
    </Card>
  </div>
</template>
