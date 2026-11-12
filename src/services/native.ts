import { invoke } from '@tauri-apps/api/core'

/** 原生错误统一为可展示的 Error，调用方负责保留用户输入。 */
export async function nativeCall<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args)
  }
  catch (error) {
    const message = typeof error === 'string' ? error : String(error)
    if (message.startsWith('CLOUD_AUTH_INVALID:')) {
      window.dispatchEvent(new Event('genmod-auth-invalid'))
    }
    throw new Error(message)
  }
}
