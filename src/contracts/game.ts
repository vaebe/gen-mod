import type { InstallHint } from './install'

export interface Game {
  installHint?: InstallHint
  id: string
  steamAppId: number
  name: string
  description: string
  cover: string
  icon: string
  sortOrder: number
  supportStatus: 'supported' | 'planned'
}
