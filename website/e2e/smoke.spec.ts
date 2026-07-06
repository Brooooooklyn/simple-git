import { test, expect } from '@playwright/test'

// Deterministic smoke test: the two real routes render server-side with the exact
// grounded copy. Assertions match the ACTUAL rendered text — the landing H1 is split
// across a nested <span> ("Git for Node, at <span>native</span> speed."), so we match
// on the H1 element's normalized text content rather than a raw-HTML substring, and the
// benchmark eyebrow is "01 — BENCHMARK" (an em dash, U+2014, from SectionHeader).

test('landing (/) renders 200 with the hero H1 and the benchmark eyebrow', async ({ page }) => {
  const res = await page.goto('/')
  expect(res?.status()).toBe(200)

  await expect(page.locator('h1')).toContainText('Git for Node, at native speed.')
  await expect(page.getByText('01 — BENCHMARK')).toBeVisible()
})

test('docs (/docs) renders 200 with the Getting Started heading', async ({ page }) => {
  const res = await page.goto('/docs')
  expect(res?.status()).toBe(200)

  await expect(page.getByRole('heading', { name: 'Getting Started', level: 1 })).toBeVisible()
})
