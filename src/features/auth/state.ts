import type { CloudUser } from './types'
import { shallowRef } from 'vue'

export const cloudUser = shallowRef<CloudUser | null>(null)
export const authInvalidEvent = 'genmod-auth-invalid'
