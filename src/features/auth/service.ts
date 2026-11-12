import type { CloudCredentials, CloudUser } from './types'
import { nativeCall } from '@/services/native'
import { authInvalidEvent } from './state'

/** 令牌仅留在原生层；认证失效由 nativeCall 按现有错误前缀通知界面。 */
export const authService = {
  login(credentials: CloudCredentials): Promise<CloudUser> {
    return nativeCall('cloud_login', { ...credentials })
  },
  register(credentials: CloudCredentials): Promise<CloudUser> {
    return nativeCall('cloud_register', { ...credentials })
  },
  currentUser(): Promise<CloudUser> {
    return nativeCall('cloud_me')
  },
  async logout(): Promise<void> {
    await nativeCall<void>('cloud_logout')
    window.dispatchEvent(new Event(authInvalidEvent))
  },
}
