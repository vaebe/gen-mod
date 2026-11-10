import type { FollowupState } from './followupQueue'
import type { ImageInput, PromptAttachment } from '@/contracts/session'

export interface PendingSubmission {
  requestId: string
  modId: string
  sessionId: string
  text: string
  message: string
  attachments: PromptAttachment[]
  images: ImageInput[]
  timestamp: number
}

/** 只保存尚未确认的提交；IndexedDB 避免图片附件超过 localStorage 配额。 */
async function database(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open('gen-mod-pending-submissions', 1)
    request.onupgradeneeded = () => request.result.createObjectStore('submissions')
    request.onsuccess = () => resolve(request.result)
    request.onerror = () => reject(request.error)
  })
}
async function access<T>(modId: string, sessionId: string, mode: IDBTransactionMode, action: (store: IDBObjectStore, key: string) => IDBRequest<T>): Promise<T> {
  const db = await database()
  try {
    return await new Promise<T>((resolve, reject) => {
      const tx = db.transaction('submissions', mode)
      const request = action(tx.objectStore('submissions'), `${modId}/${sessionId}`)
      tx.oncomplete = () => resolve(request.result)
      tx.onerror = () => reject(tx.error)
      tx.onabort = () => reject(tx.error ?? new Error('未确认提交保存失败'))
    })
  }
  finally { db.close() }
}
export const pendingSubmission = {
  read: (modId: string, sessionId: string) => access<PendingSubmission | undefined>(modId, sessionId, 'readonly', (store, key) => store.get(key)),
  save: (value: PendingSubmission) => access(value.modId, value.sessionId, 'readwrite', (store, key) => store.put(value, key)),
  async remove(modId: string, sessionId: string, requestId: string): Promise<void> {
    const db = await database()
    try {
      await new Promise<void>((resolve, reject) => {
        const tx = db.transaction('submissions', 'readwrite')
        const store = tx.objectStore('submissions')
        const key = `${modId}/${sessionId}`
        const request = store.get(key)
        request.onsuccess = () => {
          if ((request.result as PendingSubmission | undefined)?.requestId === requestId) {
            store.delete(key)
          }
        }
        tx.oncomplete = () => resolve()
        tx.onerror = () => reject(tx.error)
        tx.onabort = () => reject(tx.error ?? new Error('提交保留内容清理失败'))
      })
    }
    finally { db.close() }
  },
}

// 与单次提交使用不同的键；图片仍保存在 IndexedDB，不受 localStorage 配额限制。
export const followupStorage = {
  read: (modId: string, sessionId: string) => access<FollowupState | undefined>(modId, `${sessionId}/followups`, 'readonly', (store, key) => store.get(key)),
  save: (modId: string, sessionId: string, value: FollowupState) => access(modId, `${sessionId}/followups`, 'readwrite', (store, key) => store.put(JSON.parse(JSON.stringify(value)), key)),
}
