import { nativeCall } from '@/services/native'

export const chatService = {
  openMessageLink(input: { modId: string, link: string }): Promise<void> {
    return nativeCall('open_message_link', input)
  },
}
