/** 源码目录的直接子项；blocked 表示禁止访问的链接或特殊文件。 */
export interface SourceEntry {
  name: string
  path: string
  kind: 'directory' | 'file' | 'blocked'
}

/** 只读预览；无法展示全文时保留大小和具体原因。 */
export interface SourcePreview {
  path: string
  size: number
  text: string | null
  /** 支持的位图 MIME；图片字节通过独立的二进制接口获取。 */
  imageMimeType: string | null
  previewLimit: number
  unavailableReason: 'tooLarge' | 'binary' | 'encoding' | null
}
