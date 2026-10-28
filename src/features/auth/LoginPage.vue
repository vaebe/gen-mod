<script setup lang="ts">
import { onMounted, reactive, shallowRef } from 'vue'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle, CardFooter } from '@/components/ui/card'
import { Field, FieldGroup, FieldLabel } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { nativeCall } from '@/services/native'
import type { CloudUser } from './state'
const emit = defineEmits<{ authenticated: [user: CloudUser] }>()
const form = reactive({ email: '', password: '', apiUrl: '' })
const registering = shallowRef(false)
const busy = shallowRef(false)
const error = shallowRef('')
onMounted(async () => { try { form.apiUrl = await nativeCall<string>('cloud_configuration') } catch (cause) { error.value = (cause as Error).message } })
async function submit() {
  busy.value = true; error.value = ''
  try { emit('authenticated', await nativeCall<CloudUser>(registering.value ? 'cloud_register' : 'cloud_login', form)) }
  catch (cause) { error.value = (cause as Error).message }
  finally { busy.value = false }
}
</script>
<template>
  <main class="flex h-full items-center justify-center overflow-auto p-6">
    <Card class="w-full max-w-md">
      <CardHeader><CardTitle>{{ registering ? '注册 GenMod' : '登录 GenMod' }}</CardTitle><CardDescription>登录后进入工作台，本机项目和 Skill 文件将保留。</CardDescription></CardHeader>
      <CardContent><form id="cloud-login" @submit.prevent="submit"><FieldGroup>
        <Field><FieldLabel for="cloud-api">服务地址</FieldLabel><Input id="cloud-api" v-model="form.apiUrl" type="url" required :disabled="busy" /></Field>
        <Field><FieldLabel for="cloud-email">邮箱</FieldLabel><Input id="cloud-email" v-model="form.email" type="email" autocomplete="email" required :disabled="busy" /></Field>
        <Field><FieldLabel for="cloud-password">密码</FieldLabel><Input id="cloud-password" v-model="form.password" type="password" :autocomplete="registering ? 'new-password' : 'current-password'" required :disabled="busy" /></Field>
        <Alert v-if="error" variant="destructive"><AlertDescription>{{ error }}</AlertDescription></Alert>
      </FieldGroup></form></CardContent>
      <CardFooter class="flex flex-col items-stretch gap-3"><Button form="cloud-login" :disabled="busy">{{ busy ? '正在提交…' : registering ? '注册并登录' : '登录' }}</Button><Button variant="ghost" :disabled="busy" @click="registering = !registering">{{ registering ? '已有账号，登录' : '没有账号，注册' }}</Button></CardFooter>
    </Card>
  </main>
</template>
