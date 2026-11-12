import type { ModProject } from '@/contracts/mod'
import type { ContextUsage, RuntimeLogEntry, SessionIdentity, SteeringInput, ThinkingLevel } from '@/contracts/session'
import type { TaskRecord } from '@/contracts/task'
import type { SessionModelState } from '@/features/settings/types'
import type { SubmissionResult } from '@/services/pi/types'
import { nativeCall } from '@/services/native'

export interface SessionMetadata {
  draft: string
  thinkingLevel: ThinkingLevel
  modelState: SessionModelState
  generation: number
  contextUsage: ContextUsage | null
}

export const sessionService = {
  metadata(identity: SessionIdentity): Promise<SessionMetadata> {
    return nativeCall('read_session_metadata', { ...identity })
  },
  history(identity: SessionIdentity): Promise<{ messages: unknown[] }> {
    return nativeCall('read_session_history', { ...identity })
  },
  tasks(identity: { modId: string }): Promise<TaskRecord[]> {
    return nativeCall('list_mod_tasks', { ...identity })
  },
  create(identity: { modId: string }): Promise<string> {
    return nativeCall('create_mod_session', { ...identity })
  },
  rename(input: SessionIdentity & { name: string }): Promise<ModProject> {
    return nativeCall('rename_mod_session', { ...input })
  },
  remove(identity: SessionIdentity): Promise<ModProject> {
    return nativeCall('delete_mod_session', { ...identity })
  },
  steeringInputs(identity: SessionIdentity): Promise<SteeringInput[]> {
    return nativeCall('list_steering_inputs', { ...identity })
  },
  submission(input: SessionIdentity & { requestId: string }): Promise<SubmissionResult> {
    return nativeCall('read_submission_result', { ...input })
  },
  saveThinking(input: SessionIdentity & { level: ThinkingLevel }): Promise<void> {
    return nativeCall('save_session_thinking', { ...input })
  },
  runtimeLogs(identity: SessionIdentity): Promise<RuntimeLogEntry[]> {
    return nativeCall('read_session_runtime_logs', { ...identity })
  },
  openTaskLogs(input: { modId: string, taskId: string }): Promise<void> {
    return nativeCall('open_task_logs', { ...input })
  },
}
