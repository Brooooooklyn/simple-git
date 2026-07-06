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

test('docs api (/docs/api) renders 200 with the heading and the docs sidebar', async ({ page }) => {
  const res = await page.goto('/docs/api')
  expect(res?.status()).toBe(200)

  await expect(page.getByRole('heading', { name: 'API Reference', level: 1 })).toBeVisible()

  // Assert the markdown BODY rendered (not just the frontmatter H1): the `## Repository`
  // and `## Error handling` section headings are real H2s produced by the page content.
  await expect(page.getByRole('heading', { name: 'Repository', level: 2 })).toBeVisible()
  await expect(page.getByRole('heading', { name: 'Error handling', level: 2 })).toBeVisible()

  // The two-column docs shell: a desktop sidebar <nav aria-label="Documentation"> with
  // links to both docs routes. The mobile <details> nav shares the label, so scope the
  // link assertion to the visible (desktop) sidebar to keep it deterministic.
  const sidebar = page.getByRole('navigation', { name: 'Documentation' }).filter({ visible: true })
  await expect(sidebar.getByRole('link', { name: 'Getting Started' })).toBeVisible()
  await expect(sidebar.getByRole('link', { name: 'API Reference' })).toBeVisible()
})
