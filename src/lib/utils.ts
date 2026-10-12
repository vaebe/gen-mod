import type { ClassValue } from 'clsx'
import { clsx } from 'clsx'
import { twMerge } from 'tailwind-merge'

/** 合并组件默认样式与调用方覆盖。 */
export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs))
}
