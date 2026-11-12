import type { DeleteProjectResult, ModProject } from '@/contracts/mod'
import { nativeCall } from '@/services/native'

export const modService = {
  recoveryIssues(): Promise<{ modId: string, kind: string }[]> {
    return nativeCall('list_recovery_issues')
  },
  describe(modId: string, description: string, expectedDescription: string): Promise<ModProject> {
    return nativeCall('update_mod_description', { modId, description, expectedDescription })
  },
  archive(modId: string, expectedRevision: number, archived: boolean): Promise<ModProject> {
    return nativeCall('set_mod_archived', { modId, expectedRevision, archived })
  },
  archived(): Promise<ModProject[]> {
    return nativeCall('list_archived_mods')
  },
  remove(modId: string, expectedRevision: number): Promise<DeleteProjectResult> {
    return nativeCall('delete_mod', { modId, expectedRevision })
  },
  create(name: string, gameId: string, description: string): Promise<ModProject> {
    return nativeCall('create_mod', { name, gameId, description })
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
