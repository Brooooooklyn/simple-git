# @napi-rs/simple-git website — Implementation Plan

This plan covers the full **simple-git.napi.rs** website in two phases. **Phase 1**
builds the site; **Phase 2** adds SEO/discovery, accessibility and agent-friendliness.
Both phases are implemented on branch `feat/website` (PR #146).

---

## Context

Executes the approved spec: `docs/superpowers/specs/2026-07-06-simple-git-website-design.md`.
Read the spec for full rationale; this plan is the executable task breakdown. Reference
implementation to read side-by-side: `/Users/brooklyn/workspace/github/Image/website`
(the image.napi.rs site). Domain: **https://simple-git.napi.rs**.

We are cloning the architecture and craft of the `image.napi.rs` site
(`../Image/website`, framework = `void` by VoidZero) for the `@napi-rs/simple-git`
library, adapting all content for a native Git library. The site is a new,
self-contained `website/` Yarn workspace inside the `simple-git` repo — void SSR on
Cloudflare workerd, with two routes: `/` (marketing landing, 8 sections) and `/docs`
(Getting Started, one markdown page).

**Reference files in `../Image/website` are the source of truth for HOW.** Implementers
read the corresponding image file, then adapt it — strip everything playground/WASM,
swap content, apply our theme. Do NOT invent void APIs; mirror what the reference does.

**Phase 1** builds the site end to end. **Phase 2** is the follow-on phase: it fixes the
findings from a multi-agent SEO audit (all adversarially verified against the rendered
HTML + source). The site is engineered well but ships with no share/discovery meta layer,
so Phase 2 wires it in: OG/Twitter cards + image, canonical + og:url, JSON-LD,
robots/sitemap, charset/theme-color, prerender, plus a11y and CWV micro-fixes. Every
Phase 2 change is workerd-safe (head config, static `public/` assets, server-rendered
tags, CSS). The library is NOT touched.

---

## Global Constraints (binding — reviewers use these as the attention lens)

1. **Do NOT modify the library.** `src/`, `index.js`, `index.d.ts`, `Cargo.*`,
   `build.rs`, and the native build stay untouched. **Phase 1** root-level changes are
   limited to: (a) adding `"workspaces": ["website"]` to `package.json`, (b) the resulting
   `yarn.lock` update, and (c) **in Phase 1 Task 7 only**, adding the CI workflow file
   `.github/workflows/void-deploy.yml`. **All website ignore rules live in a new
   `website/.gitignore`, NOT the root `.gitignore`.** Nothing else at the repo root changes.
   **Phase 2** edits only under `website/` (`void.json`, `pages/**`, `app.css`, `public/**`,
   `scripts/**`) — do NOT modify the library, root config, `package.json`, or the deploy
   workflow.
2. **No WASM, no playground.** Do NOT copy image's `pages/playground/`, `worker.ts`,
   `_engine.ts`, `protocol.ts`, the COOP/COEP/CORP headers in `void.json`, the dev
   COEP middleware or WASM path fixes in `vite.config.ts`, or the `@napi-rs/image*`,
   `@napi-rs/canvas`, `buffer` deps. simple-git has no browser build.
3. **workerd-safe.** No runtime `WebAssembly.instantiate` anywhere, and no Node-only APIs
   at request time. All code samples are highlighted server-side with Shiki's
   **JavaScript regex engine** (`createJavaScriptRegexEngine({ forgiving: true })`),
   never the Oniguruma WASM engine. Static assets go in `website/public/` (served at web
   root — `favicon.svg` proves the path). Head tags go via `void.json` base head OR a
   route's `.server.ts` head export; JSON-LD is server-rendered in the layout.
4. **Theme:** dark-only. Accent = teal-cyan `--color-accent: oklch(74% 0.13 190)`
   (≈ `#14c7bd`); surfaces = cool slate near-black `--color-bg: #0a0b0d`. Full token
   list in the spec §6. Fonts: Space Grotesk (display), Inter (body), JetBrains Mono
   (mono), self-hosted.
5. **Content is grounded — no fabrication.** Benchmark = **1.9 s (git CLI child
   process) vs 65 ms (@napi-rs/simple-git), 1000× `getFileLatestModifiedDate`, ≈29×**
   (source: README Performance + `performance.mjs`). API names, method signatures, and
   the 15 platform triples come from the spec / `index.d.ts` / `package.json` verbatim.
   JSON-LD and copy use real facts (name `@napi-rs/simple-git`, version `1.0.0`, MIT,
   author LongYinan / Brooooooklyn, repo + npm URLs, the 15 platforms, Node ≥ 10).
   Do not invent APIs, numbers, logos, or testimonials.
6. **No-JS safe.** Content is visible and readable with JavaScript disabled; scroll
   reveals are gated behind an `html.js` class so no-JS users see content.
7. **No analytics.** Do not add Google Analytics or any tracker.
8. **Toolchain:** Node 24, Yarn 4.17 (`corepack`). Deps pinned to the reference
   `../Image/website/package.json` versions (void `0.10.2`, `@void/react` `0.10.2`,
   `@void/md` `0.10.2`, react `^19.2.1`, vite `^8.0.0`, tailwindcss `4.3.2`,
   `@tailwindcss/vite` `4.3.2`, `@fontsource-variable/*` `^5.2.x`, typescript `^6.0.3`,
   `@playwright/test` `^1.50.0`, `@types/*`). Verify with `npm info` if any fails to resolve.
9. **Absolute URLs (Phase 2):** canonical, `og:url`, `og:image`, `twitter:image`, and
   sitemap `<loc>` use `https://simple-git.napi.rs/...`. Use the **non-trailing-slash**
   `/docs` form (matches the existing `void.json` 308 redirect).
10. **void head gotchas (Phase 2, verified against `node_modules/void`):**
    - **Markdown frontmatter carries ONLY `title` + `description`.** Any `head`/`link`/`og`
      frontmatter key is silently dropped. Per-route canonical/og for `/docs` therefore MUST go
      in a co-located `website/pages/docs/index.server.ts` head export — NOT `index.md`
      frontmatter.
    - **Once a `docs/index.server.ts` head export exists, it REPLACES the frontmatter-derived
      title+description head** — so re-declare `title` and `description` in that server head.
    - **Base vs route:** `void.json` base `head.meta`/`head.link` apply to ALL routes; a route's
      head overrides entries with the same `name`/`property`. Put **route-invariant** tags in the
      base; keep **per-route** tags in the route head export. Mirror the shape of the EXISTING
      working `head` export in `website/pages/index.server.ts` and confirm the exact void head
      API in `node_modules/void` before writing new head code — do not invent an API.
    - void renders `<title>` first, then meta, then link (charset can't be forced first — that's
      fine, `<title>` is ASCII and within the 1 KB sniff window).
11. **No regressions (Phase 2):** the site must keep building (`void:prepare && build`), the
    Playwright e2e must keep passing, content stays visible with JS off, dark-only, no analytics.
    Do not regress the "already good" list: SSR crawlable HTML, one `<h1>`/route + logical
    outline, self-hosted preloaded fonts, `lang=en` + viewport, home↔docs cross-links,
    `rel=noreferrer` on external links, Shiki JS-regex highlighting.

**Verification note (Phase 1, applies to all tasks):** classic unit-test TDD does not fit a
visual SSR site. "Tests" here means: `yarn workspace website build` (and `void prepare`
if the reference requires it) completes without error, plus the task-specific render
/ content checks named in each task. The Playwright smoke test is built in Phase 1 Task 7.
Each task ends with a commit.

**Verification note (Phase 2):** each task ends with `void:prepare && build` succeeding and
(where the task changes rendered output) a grep/inspection of the built/served HTML proving
the tags/assets are present and correct. Keep `test:e2e` green. Each task commits on
`feat/website`.

---

## Phase 1 — Build the site

### Task 1: Scaffold the `website/` Yarn workspace

**Goal:** a `void` app that installs and builds, serving a minimal blank `/` and
`/docs` route. No theme or content yet.

**Read first (reference):** `../Image/website/package.json`, `void.json`,
`vite.config.ts`, `tsconfig.json`, and `../Image/.gitignore` (for `website/` ignore
entries). Note how routing/pages are wired, then reproduce the MINIMAL subset.

**Create:**
- `website/package.json` — `name: "@napi-rs/simple-git-website"`, `private: true`,
  `type: "module"`, scripts `dev`/`build`/`preview`/`void:prepare`/`deploy`/`test:e2e`
  copied from the reference. Dependencies: `void`, `@void/react`, `@void/md`, `react`,
  `react-dom`, `@fontsource-variable/{inter,jetbrains-mono,space-grotesk}`.
  devDependencies: `@tailwindcss/vite`, `tailwindcss`, `typescript`, `vite`,
  `@types/node`, `@types/react`, `@types/react-dom`, `@playwright/test`.
  **Exclude** `@napi-rs/image`, `@napi-rs/image-wasm32-wasi`, `@napi-rs/canvas`,
  `buffer`, `chalk`. Use the exact versions from the reference package.json.
- `website/void.json` — `output: "server"`; `head.titleTemplate: "%s | @napi-rs/simple-git"`;
  `head.htmlAttrs: { lang: "en", data-theme: "dark" }`; favicon link + font preloads
  (font preloads may be added in Task 2 once fonts land); `routing.revalidate: {"*": 0}`;
  trailing-slash 308 redirects; `worker.compatibility_date` = today (2026-07-06) or the
  reference value. **OMIT** any `routing.headers` COOP/COEP block and any `/playground`
  or `/assets/*` entries.
- `website/vite.config.ts` — plugin order `voidPlugin()` → `voidReact()` →
  `voidMarkdown()` → `tailwindcss()`. **OMIT** the COEP dev middleware, the WASM
  worker path-fix plugin, the `@napi-rs/image` → wasm alias, `worker` config, and any
  asset-generation plugins.
- `website/tsconfig.json` — copy/adapt from reference.
- `website/pages/index.tsx` — minimal placeholder that renders a single `<h1>` (real
  content comes in later tasks).
- `website/pages/docs/index.md` — minimal frontmatter (`title: Getting Started`) + a
  placeholder paragraph.
- `website/.gitignore` — a self-contained ignore file for the workspace's build
  artifacts (`node_modules/`, `dist/`, `.void`, `.wrangler`, `test-results`,
  `playwright-report`, `.playwright-mcp`, or whatever the reference ignores).
  Do **NOT** edit the root `.gitignore`.

**Modify:** root `package.json` — add `"workspaces": ["website"]`. Change NOTHING else
in root package.json, and do NOT touch the root `.gitignore`.

**Verify (acceptance):**
- `corepack enable` then `yarn install` at repo root succeeds; the native library is
  not rebuilt/affected.
- `yarn workspace @napi-rs/simple-git-website void:prepare` (if the reference has this
  step) then `yarn workspace @napi-rs/simple-git-website build` completes with no error.
- If a quick `dev` boot is feasible, confirm `/` renders the placeholder `<h1>`.
- Commit.

**Report** any deviation from the reference's void setup you had to make, and whether
`void:prepare` was required before build.

---

### Task 2: Design system + root layout

**Goal:** the visual foundation — theme tokens, fonts, favicon, Shiki, and the
header/footer/mobile-nav shell that wraps every route.

**Read first (reference):** `../Image/website/app.css` (full), `lib/highlight.ts`,
`pages/layout.tsx`, `pages/_components/Footer.tsx`, and how fonts are imported +
preloaded (`void.json` link preloads, `@fontsource-variable/*` imports).

**Create / modify:**
- `website/app.css` — Tailwind v4 CSS-first config: `@import 'tailwindcss'` + `@theme`
  block with our tokens (copy the token NAMES/structure from the reference, swap VALUES):
  ```
  --color-bg: #0a0b0d;  --color-surface-1: #101318;  --color-surface-2: #181c22;
  --color-fg: #eef2f5;  --color-muted: #9aa4af;  --color-faint: #79828d;
  --color-border: rgb(228 240 250 / .08);  --color-border-strong: rgb(228 240 250 / .15);
  --color-accent: oklch(74% 0.13 190);        /* ≈ #14c7bd */
  --color-accent-strong: oklch(80% 0.14 190); /* ≈ #35e0d3 */
  --color-accent-muted: oklch(60% 0.10 190);
  --color-accent-glow: <teal at low alpha>;   --color-accent-fg: #04211f;
  --font-display: 'Space Grotesk Variable', …;  --font-sans: 'Inter Variable', …;
  --font-mono: 'JetBrains Mono Variable', …;
  ```
  Plus the fluid type-scale `clamp()` tokens, `color-scheme: dark`, film-grain
  `body::after` overlay, `.accent-glow` radial glow, and the `@layer components`
  helpers (`.container-page` max-w ~1140px, `.site-header`, `.eyebrow`, `.reveal`,
  `.reveal` gated on `html.js`). Import the three `@fontsource-variable` packages.
- `website/lib/highlight.ts` — copy the reference verbatim (Shiki JS-regex-engine
  singleton, `github-dark`, `forgiving: true`). It is already workerd-safe and
  library-agnostic.
- `website/public/favicon.svg` — NEW brand mark: rounded square, teal (`--color-accent`)
  background, a simple monospace/commit-node or branch glyph in `--color-accent-fg`
  (`#04211f`). Keep it clean and legible at 16px.
- Self-host the fonts: ensure the three `@fontsource-variable` woff2 files are
  imported (via CSS import) and preloaded in `void.json` `head.link` the way the
  reference does.
- `website/pages/layout.tsx` — root layout: blurred translucent `.site-header` with the
  text wordmark `@napi-rs/simple-git` (JetBrains Mono) + nav (`Docs`, `GitHub`, `npm`),
  a CSS-only mobile nav drawer (checkbox `:checked` toggle, no JS), `{children}`, and a
  footer (brand + tagline + links `Docs`/`GitHub`/`npm`/`napi.rs` + "Built with napi-rs
  · MIT licensed"). Links: GitHub `https://github.com/Brooooooklyn/simple-git`, npm
  `https://www.npmjs.com/package/@napi-rs/simple-git`, napi.rs `https://napi.rs`.
  **No** Google Analytics injection (image has one — omit it).

**Verify (acceptance):**
- `yarn workspace @napi-rs/simple-git-website build` succeeds.
- The placeholder page shows the teal accent, the three fonts load, header + footer
  render, and the mobile drawer toggles with JS disabled.
- Commit.

---

### Task 3: Content data + shared UI components + Shiki loader

**Goal:** all `_data/*` content modules, the reusable UI primitives, and the
`index.server.ts` loader that pre-highlights code samples server-side.

**Read first (reference):** `../Image/website/pages/_data/*`, `pages/index.server.ts`,
and the reusable components `_components/{InstallSwitcher,_CountUp,_Reveal,CodeBlock}.tsx`
(exact names may vary — match the reference). Note the `.island.tsx` / `with { island }`
hydration pattern for interactive pieces.

**Create:**
- `website/pages/_data/benchmarks.ts` — the 1.9 s vs 65 ms, 1000-call, ≈29× data.
- `website/pages/_data/features.ts` — the 9 feature cards (titles + one-line
  descriptions) from spec §5.
- `website/pages/_data/platforms.ts` — the 15 napi triples grouped by OS (spec §5),
  Node `>= 10`.
- `website/pages/_data/docSites.ts` — Nextra, Docusaurus, Starlight, Fumadocs, Rspress
  (name + homepage URL). Use inline SVG or text glyphs; do NOT fetch remote logos.
- `website/pages/_data/samples.ts` — the code-sample strings (hero flagship snippet +
  the five tab snippets: Status / Stage & commit / Blame / Push / Typed errors), copied
  **verbatim from `README.md`** (see spec §5). Keep them as raw TS strings for Shiki.
- `website/pages/index.server.ts` — `loader` that runs `lib/highlight.ts` over every
  sample string and returns the highlighted HTML as props (mirror the reference).
  `head` for the landing (`title`, `description`).
- Shared UI in `website/pages/_components/`:
  - `InstallSwitcher.tsx` (island) — tabs npm/yarn/pnpm/bun, localStorage-persisted,
    copy button. Commands: `npm install @napi-rs/simple-git`, `yarn add …`,
    `pnpm add …`, `bun add …`.
  - `CountUp.tsx` (island) — animate a number into view (IntersectionObserver).
  - `Reveal.tsx` (island) — scroll reveal, gated behind `html.js`.
  - `CodeBlock.tsx` — renders pre-highlighted HTML via `dangerouslySetInnerHTML` + a
    copy button.
  - `TabbedCodeBlock.tsx` (island) — tabbed wrapper over several `CodeBlock`s.

**Verify (acceptance):**
- `yarn workspace @napi-rs/simple-git-website build` succeeds; data modules typecheck;
  the loader returns non-empty highlighted HTML (no runtime WASM).
- Commit.

**Report** the exact island-hydration convention the reference uses so later tasks match it.

---

### Task 4: Landing sections A — Hero, Benchmark, Doc-site showcase

**Goal:** the first three landing sections, wired into `index.tsx`.

**Read first (reference):** `../Image/website/pages/index.tsx`, and the section
components `_components/{Hero,Benchmarks,OptimizationShowcase}.tsx` (adapt structure,
replace content). Use the `_data` + `index.server.ts` props from Task 3.

**Create in `website/pages/_components/`:**
- `Hero.tsx` — eyebrow `NATIVE NODE ADDON · POWERED BY RUST · libgit2`; H1
  **"Git for Node, at native speed."** with the word **"native"** in accent color;
  tagline (spec §4); buttons `[ Get started ]`→`/docs`, `[ GitHub ]`, `[ npm ]`;
  `InstallSwitcher`; a `CodeBlock` showing the flagship "last updated" snippet; three
  stat tiles via `CountUp`: `29× faster`, `15 platforms`, `0 dependencies`.
- `Benchmark.tsx` — eyebrow `01 — BENCHMARK`, title "~30× faster than shelling out";
  two horizontal bars (git CLI 1.9 s = long bar; simple-git 65 ms = tiny bar) with
  IntersectionObserver width animation; a big `CountUp` `29×`; caption citing the
  `git log` child-process baseline (source: `performance.mjs`).
- `DocSiteShowcase.tsx` — eyebrow `02 — WHY`, title "Powers 'Last updated on' for your
  docs"; a mock doc-page footer chip ("Last updated on Jul 6, 2026 by LongYinan"); a
  `CodeBlock` of `repo.getFilesLatestModified([...paths])`; a row of the five doc-site
  generators from `_data/docSites.ts`; explanatory copy (spec §4).

**Modify:** `website/pages/index.tsx` — render `<Hero/> <Benchmark/> <DocSiteShowcase/>`
inside the layout, consuming the highlighted-sample props.

**Verify (acceptance):** build succeeds; the three sections render top-to-bottom with
theme applied and animations firing on scroll; content matches the grounded values.
Commit.

---

### Task 5: Landing sections B — Features, Code/API, Platforms, CTA

**Goal:** the remaining landing sections; `index.tsx` complete end-to-end.

**Read first (reference):** `../Image/website/pages/_components/{FormatMatrix,CodeSample,
CtaBand}.tsx` and the feature-grid pattern; adapt structure, replace content.

**Create in `website/pages/_components/`:**
- `Features.tsx` — eyebrow `03 — FEATURES`, title "A full Git toolbox"; a 3×3 responsive
  card grid from `_data/features.ts` (9 cards).
- `CodeSample.tsx` — eyebrow `04 — API`, title "Show me the code"; a `TabbedCodeBlock`
  with tabs `[Status] [Stage & commit] [Blame] [Push] [Typed errors]` consuming the
  highlighted samples.
- `PlatformMatrix.tsx` — eyebrow `05 — PLATFORMS`, title "Prebuilt everywhere"; a
  responsive grid of the 15 triples from `_data/platforms.ts` grouped by OS; caption
  "npm install pulls a ready binary — no compiler, no node-gyp. Node ≥ 10."
- `CtaBand.tsx` — eyebrow `GET STARTED`, title "Add it in one line"; a compact
  `InstallSwitcher` + `[ Read the docs ]`→`/docs` and `[ GitHub ]` buttons.

**Modify:** `website/pages/index.tsx` — append `<Features/> <CodeSample/>
<PlatformMatrix/> <CtaBand/>` after the Task-4 sections. Final section order matches
spec §4 exactly (Hero → Benchmark → DocSiteShowcase → Features → CodeSample →
PlatformMatrix → CtaBand → Footer[from layout]).

**Verify (acceptance):** build succeeds; the full landing page renders all 8 sections
in order; the API tabs switch; the platform grid shows all 15 triples; responsive with
no horizontal body scroll. Commit.

---

### Task 6: Docs — Getting Started page

**Goal:** the `/docs` route with the Getting Started markdown and its layout.

**Read first (reference):** `../Image/website/pages/docs/layout.tsx`,
`pages/docs/index.md`, and `@void/md` prose theme usage (`@void/md/theme-content.css`
or equivalent).

**Create / modify:**
- `website/pages/docs/layout.tsx` — a centered prose column shell (adapt the reference;
  a simple single-column layout is fine since we have one docs page — no multi-page
  sidebar required, but a minimal in-page nav is OK).
- `website/pages/docs/index.md` — replace the Task-1 placeholder with the full Getting
  Started content (spec §3): Install (npm/yarn/pnpm/bun) · Platform support (15 triples,
  Node ≥ 10, no toolchain) · 60-second example (open repo → last-modified date → status
  → commit) · Flagship doc-site "last updated" (`getFilesLatestModified` bulk +
  integration note) · Async & AbortSignal · Typed errors (`isGitError` + `GitErrorCode`)
  · Resource cleanup (`dispose()`/`free()`/`using`) · "Full API →" link to the GitHub
  README + `index.d.ts`. All code/API grounded in the spec; frontmatter `title` +
  `description`.

**Verify (acceptance):** build succeeds; `/docs` renders with the prose theme and code
blocks highlighted; all internal/external links resolve. Commit.

---

### Task 7: Deploy workflow + responsive polish + smoke test

**Goal:** CI deploy config, a final responsive/polish pass, and a Playwright smoke test.

**Read first (reference):** `../Image/.github/workflows/void-deploy.yml`,
`../Image/website/playwright.config.*` and any `e2e` test, to mirror the shape.

**Create / modify:**
- `.github/workflows/void-deploy.yml` — trigger on push to `main` when `website/**`
  changes; Node 24; `corepack enable`; `yarn install --immutable` at repo root;
  `yarn void deploy --project simple-git` with `working-directory: website`; auth
  via GitHub OIDC (`id-token: write`, audience `void`); `VOID_API_URL` default
  `https://api.void.cloud`. Mirror the reference exactly except the project name and the
  path filter.
- `website/playwright.config.ts` + `website/e2e/smoke.spec.ts` — a smoke test that
  builds/serves and asserts: `/` returns 200 and contains the H1 "Git for Node, at
  native speed." and the "01 — BENCHMARK" heading; `/docs` returns 200 and contains
  "Getting Started". Keep it minimal and deterministic.
- Responsive polish pass across all sections: verify no horizontal body scroll at mobile
  widths; wide content (platform grid, code blocks) scrolls within its own container;
  tap targets and the mobile drawer work.
- Optional (low priority, only if quick): a static `website/public/og.svg` +
  `void.json` OG meta tags. Do NOT add `@napi-rs/canvas` or a build-time raster pipeline.

**Verify (acceptance):** `yarn workspace @napi-rs/simple-git-website build` succeeds;
`yarn workspace @napi-rs/simple-git-website test:e2e` passes; the deploy workflow is
present and correctly configured for `simple-git`. Commit.

---

### Done criteria — Phase 1

All acceptance criteria in spec §9 met: workspace installs without touching the
library; landing renders 8 sections; `/docs` renders; theme + fonts applied; benchmark/
CountUp/InstallSwitcher/reveal/mobile-nav work; no-JS safe; workerd-safe (no runtime
WASM); responsive; deploy workflow configured; smoke test green.

---

## Phase 2 — SEO, accessibility & agent-friendliness

### Task 1: Static assets — OG image, raster icons, robots.txt, sitemap.xml

**Goal:** the static files the meta layer will reference.

**Create:**
- `website/public/og.png` — a **1200×630** social share image. Dark brand: bg `#0a0b0d`,
  teal accent `#14c7bd`, the wordmark `@napi-rs/simple-git` (mono), the tagline
  **"Git for Node, at native speed"**, and a small eyebrow like `NATIVE NODE ADDON · libgit2`.
  Author it as an SVG/HTML and rasterize to PNG. Reliable path: a small
  `website/scripts/gen-og.mjs` that uses the already-installed Playwright chromium to screenshot
  the SVG/HTML at 1200×630 (and also emit the icons below). Commit BOTH the generated PNG(s) and
  the script (so it's reproducible). Do NOT add `@napi-rs/canvas` or any new dependency.
- `website/public/apple-touch-icon.png` — **180×180**, rendered from `favicon.svg`.
- `website/public/favicon.png` — **32×32** PNG fallback, from `favicon.svg`.
- `website/public/robots.txt`:
  ```
  User-agent: *
  Allow: /

  Sitemap: https://simple-git.napi.rs/sitemap.xml
  ```
- `website/public/sitemap.xml`:
  ```xml
  <?xml version="1.0" encoding="UTF-8"?>
  <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
    <url><loc>https://simple-git.napi.rs/</loc><lastmod>2026-07-06</lastmod></url>
    <url><loc>https://simple-git.napi.rs/docs</loc><lastmod>2026-07-06</lastmod></url>
  </urlset>
  ```

**Verify:** the three PNGs exist with correct dimensions (report `file`/`sips -g pixelWidth`
output); `og.png` looks on-brand; `build` succeeds and the files land in `dist/client` (static).
Commit.

---

### Task 2: Meta & social tag layer — `void.json` base + home route head

**Goal:** the full head meta layer, with route-invariant tags in the base and per-route tags on
home; no duplicate tags.

**Read first:** `website/void.json` (current base head: viewport meta + icon/font-preload links),
`website/pages/index.server.ts` (current home `head` export: title/description/og:*/twitter:card),
and confirm the void head API/shape.

**Modify `website/void.json`** — add to base `head.meta` (route-invariant):
- `{ "charset": "utf-8" }`
- `{ "name": "theme-color", "content": "#0a0b0d" }`
- `{ "property": "og:type", "content": "website" }`
- `{ "property": "og:site_name", "content": "@napi-rs/simple-git" }`
- `{ "name": "twitter:card", "content": "summary_large_image" }`
- `{ "property": "og:image", "content": "https://simple-git.napi.rs/og.png" }`
- `{ "property": "og:image:width", "content": "1200" }`
- `{ "property": "og:image:height", "content": "630" }`
- `{ "property": "og:image:alt", "content": "@napi-rs/simple-git — Git for Node, at native speed" }`
- `{ "name": "twitter:image", "content": "https://simple-git.napi.rs/og.png" }`

and to base `head.link`: `{ "rel": "apple-touch-icon", "href": "/apple-touch-icon.png" }` and a
PNG icon fallback `{ "rel": "icon", "type": "image/png", "href": "/favicon.png" }` (keep the
existing SVG icon + font preloads).

**Modify `website/pages/index.server.ts` head export** (per-route):
- Add a `link: [{ rel: 'canonical', href: 'https://simple-git.napi.rs/' }]` (mirror the void
  head API for links).
- Add `{ property: 'og:url', content: 'https://simple-git.napi.rs/' }` to `meta`.
- Change `og:title` → `{ property: 'og:title', content: 'Git for Node, at native speed' }`
  (was the bare package name).
- Rewrite the `DESCRIPTION` const to lead with the flagship intent, add "Node.js" + "libgit2",
  and **drop the literal backticks**. Use exactly:
  `Native Git for Node.js via libgit2 — no git shell-out. Read a file's last-updated commit date, run status, blame, stage, commit, branch and push, all in-process.`
  (This const feeds both `<meta name=description>` and `og:description`.)
- **Delete** the now-duplicated base tags from this file: `og:type`, `og:site_name`, and
  `twitter:card` (they moved to `void.json` base; void appends, so leaving them duplicates).

**Verify:** served/built home HTML `<head>` contains exactly one each of canonical, og:url,
og:image, og:title (benefit copy), theme-color, charset, twitter:card, apple-touch-icon; the
meta description has no backticks; no duplicate og:type/og:site_name/twitter:card. Commit.

---

### Task 3: `/docs` route head — canonical, og:url, description

**Goal:** give `/docs` its own canonical + og:url and a right-length description, via a co-located
server head (frontmatter can't carry these).

**Read first:** `website/pages/docs/index.md` (frontmatter title/description), the home
`index.server.ts` head export (mirror its API), and Global Constraint 4.

**Create `website/pages/docs/index.server.ts`** — a head export that:
- Re-declares `title: 'Getting Started'` and the docs `description` (because the server head
  replaces the frontmatter-derived head). Trim the description to **≤160 chars** (the current
  one is 175 and gets clipped) — e.g. drop the trailing "Powered by libgit2 and Rust."
- Adds `link: [{ rel: 'canonical', href: 'https://simple-git.napi.rs/docs' }]`.
- Adds `meta: [{ name: 'description', ... }, { property: 'og:url', content: 'https://simple-git.napi.rs/docs' }]`.
- Optionally `og:title`/`og:description` for docs (else the base + `<title>`/description suffice).

**Modify `website/pages/docs/index.md`** — keep frontmatter title/description in sync (or note
that the server head now governs); ensure no content regression.

**Verify:** served `/docs` HTML `<head>` has canonical `.../docs`, og:url `.../docs`, a ≤160-char
description, AND inherits the base social tags (og:type/site_name/twitter:card/og:image/
theme-color). The docs page still renders its prose + title. Commit.

---

### Task 4: JSON-LD structured data

**Goal:** server-rendered schema.org JSON-LD so the library is eligible for rich results.

**Read first:** `website/pages/layout.tsx` (root layout wrapping both routes),
`package.json` + `LICENSE` for grounded facts.

**Modify `website/pages/layout.tsx`** — server-render two JSON-LD blocks (a
`<script type="application/ld+json" dangerouslySetInnerHTML={{ __html: JSON.stringify(...) }} />`
in the layout so both routes emit them). Ground every field in real data:
- `SoftwareApplication`: name `@napi-rs/simple-git`; description (reuse the site description);
  `applicationCategory` `DeveloperApplication`; `operatingSystem` `Windows, macOS, Linux, Android, FreeBSD`;
  `programmingLanguage` `["JavaScript","TypeScript","Rust"]`; `softwareVersion` `1.0.0`;
  `license` `https://opensource.org/licenses/MIT`; `codeRepository`
  `https://github.com/Brooooooklyn/simple-git`; `downloadUrl`
  `https://www.npmjs.com/package/@napi-rs/simple-git`; `author`
  `{ "@type":"Person","name":"LongYinan","url":"https://github.com/Brooooooklyn" }`;
  `offers` `{ "@type":"Offer","price":"0","priceCurrency":"USD" }`; `url`
  `https://simple-git.napi.rs`.
- `WebSite`: name `@napi-rs/simple-git`, url `https://simple-git.napi.rs`.

Escape correctly (`JSON.stringify` output inside the script is fine for JSON-LD). Note in a code
comment that `softwareVersion` is hand-maintained.

**Verify:** built HTML for both routes contains valid `application/ld+json`; paste the JSON
through `JSON.parse` (or a validator) to confirm it parses; fields match `package.json`. Commit.

---

### Task 5: Rendering strategy — prerender, revalidate, Worker asset precedence

**Goal:** stop re-rendering static pages per request; stop routing static assets through the SSR
Worker (as far as void allows in-repo).

**Read first:** `website/pages/index.server.ts:18` (`prerender = false` + its comment),
`website/void.json` (`routing.revalidate {"*":0}`), `node_modules/void/schema.json` (what
`worker`/`assets`/routing knobs void exposes), and `website/dist/ssr/wrangler.json`
(`assets.run_worker_first: ["/**"]`, generated by `void prepare`).

**Change:**
- `website/pages/index.server.ts:18` → `export const prerender = true` (update/remove the stale
  "never served from cache" comment). Add `export const prerender = true` to
  `website/pages/docs/index.server.ts` (from Task 3) as well, so `/docs` prerenders too.
- `website/void.json` → remove (or greatly raise) the `routing.revalidate {"*":0}` block so the
  static responses are edge-cacheable. (Islands still hydrate client-side; no interactivity lost.)
- **Worker asset precedence:** `dist/ssr/wrangler.json` sets `run_worker_first: ["/**"]`, routing
  every static asset (the 3 critical-path fonts, favicon, hashed assets) through the SSR Worker.
  Investigate whether `void.json` exposes a passthrough for the assets/`run_worker_first` config
  (check `node_modules/void/schema.json`). **If void exposes it**, set it to scope the Worker to
  SSR paths only, e.g. `["/*","!/fonts/*","!/favicon.svg","!/favicon.png","!/apple-touch-icon.png","!/og.png","!/robots.txt","!/sitemap.xml","!/assets/*"]` (do NOT use `["/","/docs"]` — that would
  break the `/docs/` 308 redirect). **If void does NOT expose it** (the file is regenerated by
  `void prepare`, so hand-edits don't survive), do NOT hand-edit the generated file — instead
  document the limitation clearly in the report and as a short note to add to the PR.

**Verify:** `void:prepare && build` succeeds; the built output for `/` and `/docs` is prerendered
static HTML (confirm the pages exist as static files or that build logs show prerender); code
samples are still highlighted; `test:e2e` still passes; islands still hydrate (CountUp/tabs/
InstallSwitcher). Report exactly what happened with the Worker-precedence knob. Commit.

---

### Task 6: Accessibility & paint/CWV micro-fixes

**Goal:** the remaining a11y + CWV nits, all small and localized.

**Read first:** `website/pages/layout.tsx` (`<main>` at ~:97, nav at ~:38/:82, inline js script
~:23), `website/pages/_components/Footer.tsx` (nav ~:20), `website/pages/_components/Hero.tsx`
(~:57 above-fold `<Reveal>`), `website/app.css` (`.js .reveal` ~:178-181, film-grain
`body::after` ~:114-123, `@font-face` ~:9-29, existing focus rule ~:247-250).

**Change:**
- **Skip link + main id** (WCAG A): add, as the first child of the layout's top fragment,
  `<a href="#main-content" className="sr-only focus:not-sr-only focus:absolute focus:left-3 focus:top-3 focus:z-[100] focus:rounded-lg focus:bg-(--color-accent) focus:px-4 focus:py-2 focus:text-(--color-accent-fg)">Skip to content</a>`, and change `<main>` → `<main id="main-content">`.
- **:focus-visible ring** (WCAG AA): add to `app.css` Base section (after the body rule)
  `:where(a, button, [role="tab"], summary, input):focus-visible { outline: 2px solid var(--color-accent); outline-offset: 2px; border-radius: 4px; }` (zero-specificity `:where` so it won't clash with the nav-toggle rule).
- **nav aria-labels:** `layout.tsx` header navs (:38 and drawer :82) → `aria-label="Primary"`;
  `Footer.tsx` nav (:20) → `aria-label="Footer"`.
- **External-link new-tab cue:** for external `target="_blank"` links (layout NAV, Footer links,
  and the external `<Button>`s in `Hero.tsx`/`CtaBand.tsx`/`DocSiteShowcase.tsx`), add a
  visually-hidden `<span className="sr-only"> (opens in a new tab)</span>` or an
  `aria-label="… (opens in a new tab)"`. Prefer centralizing (e.g. in the `Button`/nav-link
  render) over editing 5 files by hand if a clean shared point exists. `rel="noreferrer"` is
  already present — keep it.
- **Hero LCP:** replace the above-fold `<Reveal className="flex min-w-0 flex-col gap-6" delay={120}>` wrapper (Hero.tsx ~:57) with a plain `<div className="flex min-w-0 flex-col gap-6">`
  and drop the now-unused `Reveal` import. (Keeps the code block from being `opacity:0` until JS.)
  Keep `<Reveal>` on all below-the-fold sections.
- **Film-grain:** remove `mix-blend-mode: soft-light` from `body::after` (`app.css` ~:121) — the
  overlay is ~2% opacity, so drop the per-frame compositing cost (keep or drop the overlay itself;
  removing just the blend mode is the minimal fix).
- **Font CLS:** on the Space Grotesk + Inter (and JetBrains Mono) `@font-face` rules, reduce the
  swap-CLS risk. Preferred: add fallback-metric descriptors (`size-adjust`, `ascent-override`,
  `descent-override`, `line-gap-override`) using Fontsource's published fallback numbers for these
  families if readily available; otherwise set `font-display: optional` (with the existing preload
  the web font almost always wins first paint, and optional avoids a mid-view swap). Pick one,
  apply consistently, and say which in the report.

**Verify:** served HTML has the skip link + `<main id>`, the nav `aria-label`s, and the new-tab
cue on external links; `app.css` has the `:focus-visible` rule and no `mix-blend-mode` on the
grain; the hero code block is no longer `opacity:0` in the no-hydration HTML; `build` + `test:e2e`
pass; content still visible with JS off. Commit.

---

### Task 7: Agent-friendliness — llms.txt, llms-full.txt, robots pointer

**Goal:** expose the site/library to LLM agents via the llms.txt convention (llmstxt.org),
plus a full-text fetch and a robots pointer. Static assets only — served from `website/public/`.

**Read first:** website/pages/docs/index.md (the source of the full docs text),
/Users/brooklyn/workspace/github/simple-git/README.md and index.d.ts and package.json
(ground the API names / facts), and website/public/robots.txt.

**Create `website/public/llms.txt`** — follow the llmstxt.org format:
- H1: `# @napi-rs/simple-git`
- a blockquote one-line summary (reuse the site description: "Native Git for Node.js via
  libgit2 — no git shell-out. Read a file's last-updated commit date, run status, blame,
  stage, commit, branch and push, all in-process.")
- a short prose block: native libgit2 binding for Node (no `git` child process); flagship =
  fast per-file "last updated" commit dates for doc-site generators; v1.0.0, MIT, 15 prebuilt
  platform triples, Node ≥ 10.
- `## Documentation` — a list of `- [name](absolute-url): note` links: Getting Started
  (https://simple-git.napi.rs/docs), README
  (https://github.com/Brooooooklyn/simple-git#readme), Type definitions
  (https://github.com/Brooooooklyn/simple-git/blob/main/index.d.ts), npm
  (https://www.npmjs.com/package/@napi-rs/simple-git).
- `## Core API` — a concise bullet list of the real surface (Repository open/init/discover/
  clone; `getFileLatestModifiedDate` / `getFileLastModifiedDate` / `getFilesLatestModified`;
  `statuses`; index + `commit`; `blameFile`; branches/checkout; remotes; tags; `*Async` variants
  with `AbortSignal`; `isGitError` + `GitErrorCode`; `dispose()`/`free()`/`using`). Every name
  grounded in index.d.ts/README — no invented API.
- `## Install` — the four package managers (`npm install @napi-rs/simple-git`, yarn/pnpm/bun).
All URLs ABSOLUTE. No fabricated facts.

**Create `website/public/llms-full.txt`** — the FULL docs in one plain-markdown fetch: take the
content of website/pages/docs/index.md, strip the YAML frontmatter and the `[[toc]]` directive,
prefix with `# @napi-rs/simple-git — Getting Started` and the one-line summary. Keep all code
blocks intact. (Keep it to the docs content to avoid drift with the rendered docs page.)

**Modify `website/public/robots.txt`** — add a comment pointer after the `Sitemap:` line (a
comment, so no parser is affected):
`# LLM-friendly content: https://simple-git.napi.rs/llms.txt`
Keep the existing `User-agent`/`Allow`/`Sitemap` lines intact.

**Verify:** `build` succeeds and llms.txt + llms-full.txt land in `dist/client` (served at
`/llms.txt` and `/llms-full.txt`, same as robots.txt/sitemap.xml); both are valid markdown with
resolving absolute links; spot-check the API names against index.d.ts (no fabrication); `test:e2e`
still passes. Only `website/public/` changed. Commit.

---

### Done criteria — Phase 2

Rendered `/` and `/docs` heads carry: unique title/description (no backticks, docs ≤160),
canonical + og:url, full OG + Twitter card with a resolvable 1200×630 og.png, charset, theme-color,
apple-touch-icon; JSON-LD (SoftwareApplication + WebSite) present and valid on both routes;
`robots.txt` + `sitemap.xml` served; both routes prerendered (no per-request Shiki); skip link,
focus-visible, labeled navs, new-tab cues; hero LCP not JS-gated; film-grain blend removed; font
CLS mitigated. Build + Playwright e2e green; library untouched; no-JS safe; no analytics.
