<script lang="ts">
import manifest from 'material-icon-theme/dist/material-icons.json'
import { computed } from 'vue'

// 使用包内 SVG URL，让 Vite 将图标作为本地资源打包，不依赖 CDN。
const assets = import.meta.glob<string>('/node_modules/material-icon-theme/icons/*.svg', { eager: true, query: '?url&no-inline', import: 'default' })
const definitions: Record<string, { iconPath: string }> = manifest.iconDefinitions
const fileNames: Record<string, string> = manifest.fileNames
const extensions: Record<string, string> = manifest.fileExtensions
const folders: Record<string, string> = manifest.folderNames
const openFolders: Record<string, string> = manifest.folderNamesExpanded
</script>

<script setup lang="ts">
const props = defineProps<{ name: string, directory?: boolean, expanded?: boolean }>()

function iconUrl(light: boolean): string {
  const name = props.name.toLowerCase()
  const variant = light ? manifest.light : undefined
  let icon: string | undefined
  if (props.directory) {
    const names: Record<string, string> | undefined = props.expanded ? variant?.folderNamesExpanded : variant?.folderNames
    icon = names?.[name] ?? (props.expanded ? openFolders[name] : folders[name])
  }
  else {
    const names: Record<string, string> | undefined = variant?.fileNames
    const suffixes: Record<string, string> | undefined = variant?.fileExtensions
    icon = names?.[name] ?? fileNames[name]
    // 先匹配完整文件名，再从最长复合扩展名开始匹配，例如 d.ts。
    const parts = name.split('.')
    for (let index = 1; !icon && index < parts.length; index++) {
      const suffix = parts.slice(index).join('.')
      icon = suffixes?.[suffix] ?? extensions[suffix]
    }
  }
  const fallback = props.directory ? (props.expanded ? manifest.folderExpanded : manifest.folder) : manifest.file
  const filename = definitions[icon ?? fallback]?.iconPath.split('/').at(-1)
  return assets[`/node_modules/material-icon-theme/icons/${filename}`]
    ?? assets[`/node_modules/material-icon-theme/icons/${fallback}.svg`]!
}

const lightIcon = computed(() => iconUrl(true))
const darkIcon = computed(() => iconUrl(false))
</script>

<template>
  <span aria-hidden="true" class="pointer-events-none inline-flex size-4 shrink-0">
    <img :src="lightIcon" alt="" draggable="false" class="size-full dark:hidden">
    <img :src="darkIcon" alt="" draggable="false" class="hidden size-full dark:block">
  </span>
</template>
