<script setup lang="ts">
import type { CloudUser } from './types'
import { ArrowRight, LoaderCircle } from '@lucide/vue'
import { defineAsyncComponent, reactive, shallowRef } from 'vue'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card'
import { Field, FieldGroup, FieldLabel } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { authService } from './service'

const emit = defineEmits<{ authenticated: [user: CloudUser] }>()

const AuthScene = defineAsyncComponent(() => import('./AuthScene.vue'))

const form = reactive({ email: '', password: '', confirmPassword: '' })
const registering = shallowRef(false)
const busy = shallowRef(false)
const error = shallowRef('')

function toggleMode() {
  registering.value = !registering.value
  form.confirmPassword = ''
  error.value = ''
}

async function submit() {
  if (busy.value) {
    return
  }
  error.value = ''
  if (registering.value && form.password !== form.confirmPassword) {
    error.value = '两次输入的密码不一致'
    return
  }
  busy.value = true
  try {
    const user = await (registering.value ? authService.register : authService.login)({
      email: form.email.trim(),
      password: form.password,
    })
    emit('authenticated', user)
  }
  catch (cause) {
    error.value = (cause as Error).message
  }
  finally {
    busy.value = false
  }
}
</script>

<template>
  <main class="auth-page">
    <div class="auth-content">
      <AuthScene />
      <div class="auth-form-area">
        <Card class="auth-card flex w-full flex-col">
          <CardHeader class="gap-3 px-8 pb-7 pt-8">
            <CardTitle>{{ registering ? '创建账号' : '欢迎回来' }}</CardTitle>
            <CardDescription>
              {{ registering ? '创建账号，开始你的 MOD 创作。' : '登录账号，继续你的创作。' }}
            </CardDescription>
          </CardHeader>
          <CardContent class="px-8">
            <form id="cloud-login" @submit.prevent="submit">
              <FieldGroup>
                <Field>
                  <FieldLabel for="cloud-email">
                    邮箱
                  </FieldLabel>
                  <Input id="cloud-email" v-model="form.email" class="h-10" type="email" autocomplete="email" required :disabled="busy" />
                </Field>
                <Field>
                  <FieldLabel for="cloud-password">
                    密码
                  </FieldLabel>
                  <Input id="cloud-password" v-model="form.password" class="h-10" type="password" :autocomplete="registering ? 'new-password' : 'current-password'" required :disabled="busy" />
                </Field>
                <Field v-if="registering">
                  <FieldLabel for="cloud-confirm-password">
                    确认密码
                  </FieldLabel>
                  <Input id="cloud-confirm-password" v-model="form.confirmPassword" class="h-10" type="password" autocomplete="new-password" required :disabled="busy" />
                </Field>
                <Alert v-if="error" variant="destructive">
                  <AlertDescription>{{ error }}</AlertDescription>
                </Alert>
              </FieldGroup>
            </form>
          </CardContent>
          <CardFooter class="mt-auto flex flex-col items-stretch gap-2 px-8 pb-8">
            <Button form="cloud-login" size="lg" :disabled="busy">
              <LoaderCircle v-if="busy" class="auth-spinner animate-spin" aria-hidden="true" />
              {{ busy ? registering ? '正在注册…' : '正在登录…' : registering ? '创建账号并登录' : '进入工作台' }}
              <ArrowRight v-if="!busy" data-icon="inline-end" aria-hidden="true" />
            </Button>
            <Button variant="ghost" :disabled="busy" @click="toggleMode">
              {{ registering ? '已有账号，登录' : '没有账号，注册' }}
            </Button>
          </CardFooter>
        </Card>
        <p class="auth-local-note">
          本机项目与创作文件会保留。
        </p>
      </div>
    </div>
  </main>
</template>

<style scoped>
.auth-page {
  position: relative;
  height: 100%;
  overflow: auto;
  background: linear-gradient(115deg, oklch(0.975 0.014 168), var(--background) 72%);
}

.auth-page::before {
  position: absolute;
  top: 0;
  left: 0;
  width: 40%;
  height: 3px;
  background: linear-gradient(to right, oklch(0.72 0.12 170), transparent);
  content: '';
}

.auth-content {
  position: relative;
  display: grid;
  grid-template-columns: minmax(0, 1.1fr) minmax(320px, 0.9fr);
  align-items: center;
  gap: clamp(36px, 6vw, 100px);
  width: 100%;
  max-width: 1180px;
  min-height: 100%;
  margin-inline: auto;
  padding: 48px clamp(32px, 5vw, 72px);
}

.auth-form-area {
  display: flex;
  width: 100%;
  flex-direction: column;
  align-items: center;
  gap: 18px;
}
.auth-card {
  max-width: 390px;
  min-height: 422px;
  box-shadow:
    0 2px 8px oklch(0.3 0.02 170 / 0.025),
    0 24px 64px -24px oklch(0.3 0.02 170 / 0.16);
}
.auth-local-note {
  color: var(--muted-foreground);
  font-size: 11px;
}

@media (max-width: 900px) {
  .auth-content {
    grid-template-columns: minmax(0, 1fr);
    align-content: center;
    gap: 28px;
    padding: 32px 24px;
  }
  .auth-page {
    background: linear-gradient(150deg, oklch(0.96 0.025 168), var(--background) 65%);
  }
}

@media (prefers-reduced-motion: reduce) {
  .auth-spinner {
    animation: none;
  }
}
</style>
