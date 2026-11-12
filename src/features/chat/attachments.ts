import type { PromptAttachment } from '@/contracts/session'

const supportedFormats = '支持 PNG、JPG、JPEG、WebP、GIF 图片，以及 UTF-8 文本和源码文件'
const binaryExtensions = new Set(['zip', 'rar', '7z', 'dll', 'exe', 'msi', 'pdf', 'doc', 'docx', 'xls', 'xlsx', 'ppt', 'pptx', 'mp3', 'mp4', 'wav', 'ogg', 'ttf', 'woff', 'woff2', 'pak', 'unitypackage'])

/** 读取用户明确选择的文件；二进制只接受常见图片，其他内容须为 UTF-8 文本。 */
export async function readAttachments(files: File[], current: readonly PromptAttachment[]): Promise<PromptAttachment[]> {
  if (current.length + files.length > 6) {
    throw new Error(`最多添加 6 个附件，当前已有 ${current.length} 个，本次选择 ${files.length} 个。请减少文件后重新选择`)
  }
  if ([...current, ...files].reduce((total, file) => total + file.size, 0) > 20 * 1024 * 1024) {
    throw new Error('附件合计不能超过 20 MB，请减少文件或缩小文件后重新选择')
  }
  const added: PromptAttachment[] = []
  for (const file of files) {
    const extension = file.name.split('.').pop()?.toLowerCase()
    if (binaryExtensions.has(extension ?? '')) {
      throw new Error(`暂不支持「${file.name}」。${supportedFormats}，请重新选择`)
    }
    const mime = ({ png: 'image/png', jpg: 'image/jpeg', jpeg: 'image/jpeg', webp: 'image/webp', gif: 'image/gif' } as Record<string, string>)[extension ?? '']
    const base = { id: crypto.randomUUID(), name: file.name, size: file.size }
    if (mime) {
      if (file.size > 10 * 1024 * 1024) {
        throw new Error(`「${file.name}」超过单张图片 10 MB 的限制，请缩小图片后重新选择`)
      }
      const data = await new Promise<string>((resolve, reject) => {
        const reader = new FileReader()
        reader.onload = () => resolve(String(reader.result).split(',')[1] ?? '')
        reader.onerror = () => reject(new Error(`无法读取「${file.name}」，请确认文件可访问后重新选择`))
        reader.readAsDataURL(file)
      })
      added.push({ ...base, type: 'image', mimeType: mime, data })
    }
    else {
      if (file.size > 1024 * 1024) {
        throw new Error(`「${file.name}」超过单个文本 1 MB 的限制，请截取相关内容后重新选择`)
      }
      let text: string
      try {
        text = new TextDecoder('utf-8', { fatal: true }).decode(await file.arrayBuffer())
      }
      catch { throw new Error(`「${file.name}」无法作为 UTF-8 文本读取。${supportedFormats}，文本文件请转为 UTF-8 后重新选择`) }
      if (text.includes('\0')) {
        throw new Error(`「${file.name}」是暂不支持的二进制文件。${supportedFormats}，请重新选择`)
      }
      added.push({ ...base, type: 'text', text })
    }
  }
  return added
}
