import type { DeleteProjectResult, ModProject } from '../contracts/mod'
import { invoke } from '@tauri-apps/api/core'

/** 原生错误统一为可展示的 Error，调用方负责保留用户输入。 */
export async function nativeCall<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args)
  }
  catch (error) {
    const message = typeof error === 'string' ? error : String(error)
    if (message.startsWith('CLOUD_AUTH_INVALID:')) { window.dispatchEvent(new Event('genmod-auth-invalid')) }
    throw new Error(message)
  }
}

export const modService = {
  archive(modId: string, expectedRevision: number, archived: boolean): Promise<ModProject> {
    return nativeCall('set_mod_archived', { modId, expectedRevision, archived })
  },
  archived(): Promise<ModProject[]> {
    return nativeCall('list_archived_mods')
  },
  remove(modId: string, expectedRevision: number): Promise<DeleteProjectResult> {
    return nativeCall('delete_mod', { modId, expectedRevision })
  },
  create(name: string, gameId: string, requirement: string): Promise<ModProject> {
    return nativeCall('create_mod', { name, gameId, requirement })
  },
  list(): Promise<ModProject[]> {
    return nativeCall('list_mods')
  },
  get(modId: string): Promise<ModProject> {
    return nativeCall('get_mod', { modId })
  },
  rename(modId: string, name: string): Promise<ModProject> {
    return nativeCall('rename_mod', { modId, name })
  },
  saveDraft(modId: string, requirement: string, sessionId?: string, expectedRequirement?: string): Promise<ModProject> {
    return nativeCall('save_mod_draft', { modId, requirement, sessionId, expectedRequirement })
  },
}
