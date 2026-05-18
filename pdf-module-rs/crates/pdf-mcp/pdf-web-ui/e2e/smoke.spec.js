import { test, expect } from '@playwright/test'

test('loads wiki SPA shell', async ({ page }) => {
  await page.goto('/app/')
  await expect(page.locator('.app-header')).toBeVisible()
  await expect(page.locator('.logo')).toContainText('Compendium')
})

test('opens compile drawer from header', async ({ page }) => {
  await page.goto('/app/')
  await page.getByRole('button', { name: /Compile|编译/ }).click()
  await expect(page.locator('.compile-drawer')).toBeVisible({ timeout: 10_000 })
})
