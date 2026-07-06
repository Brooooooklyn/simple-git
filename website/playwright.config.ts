import { defineConfig } from '@playwright/test'

const BASE_URL = process.env.PLAYWRIGHT_BASE_URL ?? 'http://localhost:5173'

export default defineConfig({
  testDir: './e2e',
  timeout: 120_000,
  // Single, deterministic smoke run — serial keeps SSR compile + hydration off a
  // contended machine so the gate is reliably green.
  workers: 1,
  use: { baseURL: BASE_URL },
  // Auto-manage the server so the test is self-contained. `yarn dev` runs Vite with
  // the Void SSR pipeline, so `/` and `/docs` are served as real server-rendered HTML
  // (this is a `output: "server"` app — `vite preview` only serves static assets and
  // would 404 the SSR routes). Locally we reuse an already-running dev server for fast
  // iteration; in CI we always boot a fresh one so the gate never passes against a
  // stale server.
  webServer: {
    command: 'yarn dev',
    url: `${BASE_URL}/`,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
})
