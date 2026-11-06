export interface Game {
  id: string
  steamAppId: number
  name: string
  description: string
  cover: string
  icon: string
  sortOrder: number
  supportStatus: 'supported' | 'planned'
}
