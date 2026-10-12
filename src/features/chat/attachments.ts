import type { PromptAttachment } from '@/contracts/session'

/** 读取用户明确选择的文件；二进制只接受常见图片，其他内容须为 UTF-8 文本。 */
export async function readAttachments(files: File[], current: PromptAttachment[]): Promise<PromptAttachment[]> {
  if (current.length + files.length > 6) {
    throw new Error('最多添加 6 个附件')
  }
  if ([...current, ...files].reduce((total, file) => total + file.size, 0) > 20 * 1024 * 1024) {
    throw new Error('附件合计不能超过 20 MB')
  }
  const added: PromptAttachment[] = []
  for (const file of files) {
    const extension = file.name.split('.').pop()?.toLowerCase()
    const mime = ({ png: 'image/png', jpg: 'image/jpeg', jpeg: 'image/jpeg', webp: 'image/webp', gif: 'image/gif' } as Record<string, string>)[extension ?? '']
    const base = { id: crypto.randomUUID(), name: file.name, size: file.size }
    if (mime) {
      if (file.size > 10 * 1024 * 1024) {
        throw new Error(`${file.name} 超过图片大小上限 10 MB`)
      }
      const data = await new Promise<string>((resolve, reject) => {
        const reader = new FileReader()
        reader.onload = () => resolve(String(reader.result).split(',')[1] ?? '')
        reader.onerror = () => reject(new Error(`无法读取 ${file.name}`))
        reader.readAsDataURL(file)
      })
      added.push({ ...base, type: 'image', mimeType: mime, data })
    }
    else {
      if (file.size > 1024 * 1024) {
        throw new Error(`${file.name} 超过文本大小上限 1 MB`)
      }
      let text: string
      try {
        text = new TextDecoder('utf-8', { fatal: true }).decode(await file.arrayBuffer())
      }
      catch { throw new Error(`${file.name} 不是 UTF-8 文本文件`) }
      if (text.includes('\0')) {
        throw new Error(`${file.name} 是二进制文件，暂不支持`)
      }
      added.push({ ...base, type: 'text', text })
    }
  }
  return added
}
