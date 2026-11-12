import type { FileChange, FileDiff, TaskFileOperation } from '@/contracts/changes'
import type { SourceEntry, SourceImportItem, SourceMutationResult, SourcePreview, SourceTransferItem } from '@/contracts/source'
import { nativeCall } from '@/services/native'

interface Project { modId: string }
type Task = Project & { taskId: string }
type Entry = Project & { relativePath: string }
type CreateEntry = Project & { parentDirectory: string, name: string }
export interface CreatedSourceEntry { path: string, sourceRevision: number }

/** MOD 工作文件 IPC 边界；原生命令继续负责可写状态、路径和磁盘冲突校验。 */
export const workspaceService = {
  listTaskChanges: (args: Task): Promise<FileChange[]> => nativeCall('list_task_changes', { ...args }),
  listTaskOperations: (args: Task): Promise<TaskFileOperation[]> => nativeCall('list_task_operations', { ...args }),
  getTaskDiff: (args: Task & { relativePath: string }): Promise<FileDiff> => nativeCall('get_task_file_diff', { ...args }),
  listDirectory: (args: Entry): Promise<SourceEntry[]> => nativeCall('list_source_directory', { ...args }),
  readFile: (args: Entry): Promise<SourcePreview> => nativeCall('read_source_file', { ...args }),
  readImage: (args: Entry): Promise<ArrayBuffer> => nativeCall('read_source_image', { ...args }),
  revealFile: (args: Entry): Promise<void> => nativeCall('reveal_source_file', { ...args }),
  saveText: (args: Project & { path: string, expected: string, text: string }): Promise<void> => nativeCall('save_source_text', { ...args }),
  transferEntries: (args: Project & { operation: 'copy' | 'move' | 'import' | 'export', items: SourceTransferItem[], finalize?: SourceTransferItem[] }): Promise<SourceMutationResult> => nativeCall('transfer_source_entries', { ...args }),
  moveFile: (args: Project & { source: string, targetDirectory: string, targetName: string }): Promise<SourceMutationResult> => nativeCall('move_source_file', { ...args }),
  importFiles: (args: Project & { targetDirectory: string, items: SourceImportItem[] }): Promise<SourceMutationResult> => nativeCall('import_source_files', { ...args }),
  createFile: (args: CreateEntry): Promise<CreatedSourceEntry> => nativeCall('create_source_file', { ...args }),
  createDirectory: (args: CreateEntry): Promise<CreatedSourceEntry> => nativeCall('create_source_directory', { ...args }),
  entryAction: (args: Project & { path: string, action: 'open' | 'reveal' | 'path' | 'reference' }): Promise<string> => nativeCall('source_entry_action', { ...args }),
  recycleEntries: (args: Project & { entries: string[] }): Promise<SourceMutationResult> => nativeCall('recycle_source_entries', { ...args }),
  pickImportFiles: (): Promise<string[] | null> => nativeCall('pick_source_import_files'),
  pickDirectory: (): Promise<string | null> => nativeCall('pick_directory'),
}
