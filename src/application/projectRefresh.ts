import type { PiEnvelope } from '@/services/pi/types'
import { useDeliveriesStore } from '@/features/delivery/useDeliveries'
import { invalidateProjectMemory } from '@/features/project-memory/useProjectMemory'
import { invalidateVerification } from '@/features/verification/useVerification'
import { invalidateSource } from '@/features/workspace/useTaskChanges'

/** 只接收 client 已核对身份和代次的事件，不读取当前选中会话。 */
export function coordinateProjectEvent(event: PiEnvelope): Promise<unknown> | void {
  if (event.kind === 'artifact' || event.kind === 'exit' || (event.kind === 'task' && ['source_changed', 'changes_updated'].includes(event.record.type))) {
    invalidateSource(event.modId)
  }
  if (event.kind === 'task' && event.record.type === 'project_memory_updated') {
    invalidateProjectMemory(event.modId)
  }
  if (event.kind === 'task' && event.record.type === 'feedback_draft_updated') {
    invalidateVerification(event.modId)
  }
  if (event.kind === 'artifact' || event.kind === 'exit' || (event.kind === 'record' && event.record.type === 'agent_settled') || (event.kind === 'task' && ['source_changed', 'changes_updated'].includes(event.record.type))) {
    return useDeliveriesStore().refresh(event.modId)
  }
}
