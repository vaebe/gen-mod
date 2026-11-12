import type { ConfigurationStatus, SaveConfiguration } from './types'
import { nativeCall } from '@/services/native'

/** 配置命令只返回脱敏状态；原生校验及持久化失败沿用 nativeCall 的 Error。 */
export const configurationService = {
  read(): Promise<ConfigurationStatus> {
    return nativeCall('get_configuration')
  },
  save(configuration: SaveConfiguration): Promise<ConfigurationStatus> {
    return nativeCall('save_configuration', { configuration })
  },
  openDirectory(): Promise<void> {
    return nativeCall('open_configuration_directory')
  },
}
