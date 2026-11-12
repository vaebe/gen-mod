import type { SkillDefinition, SkillUpdates, SkillView } from '@/contracts/skill'
import { nativeCall } from '@/services/native'

export const skillsService = {
  list(): Promise<SkillView[]> { return nativeCall('list_skills') },
  setEnabled(name: string, enabled: boolean): Promise<void> { return nativeCall('set_skill_enabled', { name, enabled }) },
  pickDirectory(): Promise<string | null> { return nativeCall('pick_directory') },
  importLocal(directory: string, definition: SkillDefinition): Promise<void> { return nativeCall('import_skill', { directory, definition }) },
  content(name: string): Promise<string> { return nativeCall('read_skill_content', { name }) },
  reveal(name: string): Promise<void> { return nativeCall('reveal_skill_file', { name }) },
  checkUpdates(): Promise<SkillUpdates> { return nativeCall('cloud_check_updates') },
  confirmUpdates(gameIds: string[], fileIds: string[]): Promise<void> { return nativeCall('cloud_confirm_updates', { gameIds, fileIds }) },
}
