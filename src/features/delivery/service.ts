import type { DeliveryMetadata } from '@/contracts/delivery'
import type { InstallationRecord, InstallHint, InstallPreview } from '@/contracts/install'
import type { ModProject } from '@/contracts/mod'
import { nativeCall } from '@/services/native'

/** 交付与安装命令的参数/结果在能力边界绑定；原生继续校验归属、路径和预览令牌。 */
export const deliveryService = {
  exportVersion(modId: string, versionId: string): Promise<string | null> {
    return nativeCall('export_version', { modId, versionId })
  },
  openDirectory(modId: string, versionId?: string | null): Promise<void> {
    return nativeCall('open_mod_directory', { modId, versionId })
  },
  deleteVersion(modId: string, versionId: string): Promise<ModProject> {
    return nativeCall('delete_version', { modId, versionId })
  },
  metadata(modId: string, versionId: string): Promise<DeliveryMetadata[]> {
    return nativeCall('get_delivery_metadata', { modId, versionId })
  },
}

export const installationService = {
  hint(modId: string, versionId: string): Promise<InstallHint> {
    return nativeCall('get_install_hint', { modId, versionId })
  },
  pickDirectory(): Promise<string | null> {
    return nativeCall('pick_directory')
  },
  preview(modId: string, versionId: string, targetDirectory: string): Promise<InstallPreview> {
    return nativeCall('preview_install', { modId, versionId, targetDirectory })
  },
  install(previewId: string): Promise<InstallationRecord> {
    return nativeCall('install_version', { previewId })
  },
}
