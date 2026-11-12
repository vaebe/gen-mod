import { fileURLToPath, URL } from 'node:url'
import { defineConfig } from 'vitest/config'

export default defineConfig({
  resolve: { alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) } },
  test: { environment: 'node', include: ['src/services/pi/*.test.ts', 'src/features/sessions/*.test.ts', 'src/features/chat/*.test.ts', 'src/features/workspace/useSourceFileActions.test.ts', 'src/features/delivery/useVersionOperations.test.ts'] },
})
