// Generates the static social/share raster assets for the site:
//   public/og.png               1200x630  — social share card
//   public/apple-touch-icon.png  180x180  — iOS home-screen icon
//   public/favicon.png            32x32    — PNG favicon fallback
//
// Reproducible: run `node scripts/gen-og.mjs` (or `yarn workspace
// @napi-rs/simple-git-website exec node scripts/gen-og.mjs`) from the website
// workspace. Uses the Playwright chromium already installed for the e2e tests
// (imported from the workspace's @playwright/test) — NO new dependency. If the
// chromium binary is missing, run:
//   yarn workspace @napi-rs/simple-git-website exec playwright install chromium
//
// The OG card is authored as a self-contained HTML document (brand tokens copied
// from app.css) with the site's JetBrains Mono + Space Grotesk variable fonts
// embedded as base64 @font-face data URIs, so rendering does not depend on any
// system-installed font. Chromium screenshots it at an exact 1200x630 clip.

import { readFile } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { chromium } from '@playwright/test'

const __dirname = dirname(fileURLToPath(import.meta.url))
const publicDir = join(__dirname, '..', 'public')
const fontsDir = join(publicDir, 'fonts')

// Brand tokens (kept in sync with website/app.css @theme).
const BG = '#0a0b0d'
const ACCENT = '#14c7bd'
const ACCENT_STRONG = '#35e0d3'
const ACCENT_FG = '#04211f'
const FG = '#eef2f5'
const MUTED = '#9aa4af'
const FAINT = '#79828d'

async function fontDataUri(file) {
  const buf = await readFile(join(fontsDir, file))
  return `data:font/woff2;base64,${buf.toString('base64')}`
}

// The logo mark, reusing the exact geometry of public/favicon.svg. The 0 0 32 32
// viewBox scales cleanly to any target `size`.
function logoMark(size) {
  return `
    <svg width="${size}" height="${size}" viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg">
      <rect width="32" height="32" rx="8" fill="${ACCENT}"/>
      <g fill="none" stroke="${ACCENT_FG}" stroke-width="2.8" stroke-linecap="round" stroke-linejoin="round">
        <path d="M11 11.7 V20.3"/>
        <path d="M11 16 C 11 13.2 13.4 13 18.3 13"/>
      </g>
      <g fill="${ACCENT_FG}">
        <circle cx="11" cy="9" r="2.7"/>
        <circle cx="11" cy="23" r="2.7"/>
        <circle cx="21" cy="13" r="2.7"/>
      </g>
    </svg>`
}

async function ogHtml() {
  const [mono, display] = await Promise.all([
    fontDataUri('jetbrains-mono-var.woff2'),
    fontDataUri('space-grotesk-var.woff2'),
  ])
  return `<!doctype html>
<html>
<head>
<meta charset="utf-8">
<style>
  @font-face {
    font-family: 'JetBrains Mono';
    font-weight: 100 800;
    font-display: block;
    src: url('${mono}') format('woff2');
  }
  @font-face {
    font-family: 'Space Grotesk';
    font-weight: 300 700;
    font-display: block;
    src: url('${display}') format('woff2');
  }
  * { margin: 0; padding: 0; box-sizing: border-box; }
  html, body { width: 1200px; height: 630px; }
  body {
    background: ${BG};
    color: ${FG};
    font-family: 'Space Grotesk', sans-serif;
    -webkit-font-smoothing: antialiased;
    overflow: hidden;
  }
  .card {
    position: relative;
    width: 1200px;
    height: 630px;
    padding: 84px 88px;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    overflow: hidden;
  }
  /* teal glow top-right */
  .glow {
    position: absolute;
    top: -320px;
    right: -260px;
    width: 900px;
    height: 900px;
    background: radial-gradient(circle, rgba(20,199,189,0.20) 0%, rgba(20,199,189,0.06) 38%, transparent 68%);
    pointer-events: none;
  }
  /* subtle inset frame */
  .frame {
    position: absolute;
    inset: 22px;
    border: 1px solid rgba(228,240,250,0.08);
    border-radius: 24px;
    pointer-events: none;
  }
  .top { display: flex; align-items: center; gap: 22px; position: relative; }
  .top .name {
    font-family: 'JetBrains Mono', monospace;
    font-size: 26px;
    font-weight: 600;
    letter-spacing: -0.01em;
    color: ${FG};
  }
  .top .name .at { color: ${MUTED}; }
  .eyebrow {
    font-family: 'JetBrains Mono', monospace;
    font-size: 21px;
    font-weight: 500;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: ${ACCENT};
  }
  .wordmark {
    font-family: 'JetBrains Mono', monospace;
    font-size: 96px;
    font-weight: 700;
    letter-spacing: -0.03em;
    line-height: 1;
    white-space: nowrap;
  }
  .wordmark .scope { color: ${MUTED}; }
  .wordmark .pkg { color: ${ACCENT_STRONG}; }
  .tagline {
    font-family: 'Space Grotesk', sans-serif;
    font-size: 46px;
    font-weight: 500;
    letter-spacing: -0.01em;
    color: ${FG};
  }
  .mid { position: relative; display: flex; flex-direction: column; gap: 26px; }
  .bottom {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .url {
    font-family: 'JetBrains Mono', monospace;
    font-size: 24px;
    font-weight: 500;
    color: ${FAINT};
  }
  .url .slash { color: ${ACCENT}; }
  .badges { display: flex; gap: 12px; }
  .badge {
    font-family: 'JetBrains Mono', monospace;
    font-size: 20px;
    font-weight: 500;
    letter-spacing: 0.02em;
    color: ${MUTED};
    padding: 9px 18px;
    border: 1px solid rgba(228,240,250,0.12);
    border-radius: 999px;
  }
  .badge.accent {
    color: ${ACCENT_FG};
    background: ${ACCENT};
    border-color: ${ACCENT};
    font-weight: 600;
  }
</style>
</head>
<body>
  <div class="card">
    <div class="glow"></div>
    <div class="frame"></div>

    <div class="top">
      ${logoMark(56)}
      <div class="name"><span class="at">@napi-rs/</span>simple-git</div>
    </div>

    <div class="mid">
      <div class="eyebrow">Native Node Addon · Powered by Rust · libgit2</div>
      <div class="wordmark"><span class="scope">@napi-rs/</span><span class="pkg">simple-git</span></div>
      <div class="tagline">Git for Node, at native speed</div>
    </div>

    <div class="bottom">
      <div class="url">simple-git<span class="slash">.</span>napi<span class="slash">.</span>rs</div>
      <div class="badges">
        <div class="badge">Async I/O</div>
        <div class="badge">Type-safe</div>
        <div class="badge accent">15 platforms</div>
      </div>
    </div>
  </div>
</body>
</html>`
}

async function iconHtml(svg, size) {
  return `<!doctype html>
<html>
<head><meta charset="utf-8">
<style>
  * { margin: 0; padding: 0; }
  html, body { width: ${size}px; height: ${size}px; background: ${BG}; }
  svg { display: block; width: ${size}px; height: ${size}px; }
</style>
</head>
<body>${svg}</body>
</html>`
}

async function main() {
  const favicon = await readFile(join(publicDir, 'favicon.svg'), 'utf8')

  const browser = await chromium.launch()
  try {
    // OG card — exact 1200x630, deviceScaleFactor 1.
    {
      const page = await browser.newPage({ viewport: { width: 1200, height: 630 }, deviceScaleFactor: 1 })
      await page.setContent(await ogHtml(), { waitUntil: 'load' })
      await page.evaluate(() => document.fonts.ready)
      await page.screenshot({
        path: join(publicDir, 'og.png'),
        clip: { x: 0, y: 0, width: 1200, height: 630 },
      })
      await page.close()
      console.log('wrote public/og.png (1200x630)')
    }

    // Raster icons rendered from favicon.svg at exact target sizes.
    for (const [file, size] of [
      ['apple-touch-icon.png', 180],
      ['favicon.png', 32],
    ]) {
      const page = await browser.newPage({ viewport: { width: size, height: size }, deviceScaleFactor: 1 })
      await page.setContent(await iconHtml(favicon, size), { waitUntil: 'load' })
      await page.screenshot({
        path: join(publicDir, file),
        clip: { x: 0, y: 0, width: size, height: size },
      })
      await page.close()
      console.log(`wrote public/${file} (${size}x${size})`)
    }
  } finally {
    await browser.close()
  }
}

main().catch((err) => {
  console.error(err)
  process.exit(1)
})
