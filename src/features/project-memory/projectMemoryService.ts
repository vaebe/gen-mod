import type { ProjectContext } from './types'
import { nativeCall } from '@/services/native'

/** 项目记忆命令的参数与结果对应 Rust project_context / game_inspection 入口。 */
export const projectMemoryService = {
  get(modId: string): Promise<ProjectContext> {
    return nativeCall('get_project_context', { modId })
  },
  applyInspection(modId: string, reportId: string, expectedContextRevision: number): Promise<ProjectContext> {
    return nativeCall('apply_inspection', { modId, reportId, expectedContextRevision })
  },
}
