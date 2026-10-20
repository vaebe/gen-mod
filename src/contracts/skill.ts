/** 会话和版本保留当时加载的能力内容，避免更新后误用新定义。 */
export interface SkillSnapshot {
  name: string
  version: string
  contentHash: string
}

export interface SkillDefinition {
  name: string
  description: string
  category: 'common' | 'game'
  gameId: string | null
  version: string
  dependencies: string[]
  path: string
  excludePaths: string[]
  requiredCommandScripts?: string[]
  inspectionFile?: string
}
export interface SkillView extends SkillDefinition {
  source: 'builtin' | 'user'
  enabled: boolean
  contentHash: string
  scripts: string[]
  error?: string | null
}
