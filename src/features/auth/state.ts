import { shallowRef } from 'vue'
export interface CloudUser { email: string, role: 'admin' | 'user' }
export const cloudUser = shallowRef<CloudUser | null>(null)
export const authInvalidEvent = 'genmod-auth-invalid'
