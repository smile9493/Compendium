import { test, expect } from '@playwright/test'

test('loads wiki SPA shell', async ({ page }) => {
  await page.goto('/app/')
  await expect(page.locator('.app-header')).toBeVisible()
  await expect(page.locator('.logo')).toContainText('rsut-pdf-mcp')
})

test('opens compile drawer from header', async ({ page }) => {
  await page.goto('/app/')
  await page.locator('.compile-header-btn, .header-btn').filter({ hasText: '' }).first()
  const hammer = page.locator('button.compile-header-btn, button').filter({ has: page.locator('svg') })
  await page.getByRole('button', { name: /编译|Compile/ }).first().click({ timeout: 10_000 }).catch(async () => {
    await page.locator('.header-right button').nth(3).click()
  })
  await expect(page.locator('.compile-drawer')).toBeVisible({ timeout: 10_000 })
})
