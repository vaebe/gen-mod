/** 仅规范界面展示，保留原始路径供原生文件操作使用。 */
export function displayPath(path: string): string {
  const prefix = '\\\\?\\'
  if (path.startsWith(`${prefix}UNC\\`)) {
    return `\\\\${path.slice(prefix.length + 4)}`
  }
  if (path.startsWith(prefix) && /^[a-z]:\\/i.test(path.slice(prefix.length))) {
    return path.slice(prefix.length)
  }
  return path
}
