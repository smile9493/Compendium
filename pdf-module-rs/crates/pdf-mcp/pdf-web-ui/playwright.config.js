import { defineConfig } from '@playwright/test'

export default defineConfig({
  testDir: './e2e',
  timeout: 60_000,
  use: {
    baseURL: 'http://127.0.0.1:5173',
    trace: 'on-first-retry',
  },
  webServer: {
    command: process.env.CI ? 'npm run preview -- --port 5173' : 'npm run dev',
    url: 'http://127.0.0.1:5173/app/',
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
})
