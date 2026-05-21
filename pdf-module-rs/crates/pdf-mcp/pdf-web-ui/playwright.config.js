import { defineConfig } from '@playwright/test'

export default defineConfig({
  testDir: './e2e',
  timeout: 60_000,
  use: {
    baseURL: 'http://127.0.0.1:5173',
    trace: 'on-first-retry',
  },
  // CI starts the preview server manually before the test step.
  // Dev uses vite dev via webServer for hot-reload.
  webServer: process.env.CI
    ? undefined
    : {
        command: 'npm run dev',
        url: 'http://127.0.0.1:5173/app/',
        reuseExistingServer: !process.env.CI,
        timeout: 120_000,
      },
})
