# @napi-rs/simple-git website — Implementation Plan

Executes the approved spec: `docs/superpowers/specs/2026-07-06-simple-git-website-design.md`.
Branch: `feat/website`. Reference implementation to read side-by-side:
`/Users/brooklyn/workspace/github/Image/website` (the image.napi.rs site).

Read the spec for full rationale. This plan is the executable task breakdown.

---

## Context

We are cloning the architecture and craft of the `image.napi.rs` site
(`../Image/website`, framework = `void` by VoidZero) for the `@napi-rs/simple-git`
library, adapting all content for a native Git library. The site is a new,
self-contained `website/` Yarn workspace inside the `simple-git` repo. Two routes:
`/` (marketing landing, 8 sections) and `/docs` (Getting Started, one markdown page).

**Reference files in `../Image/website` are the source of truth for HOW.** Implementers
read the corresponding image file, then adapt it — strip everything playground/WASM,
swap content, apply our theme. Do NOT invent void APIs; mirror what the reference does.

---

## Global Constraints (binding — reviewers use these as the attention lens)

1. **Do NOT modify the library.** `src/`, `index.js`, `index.d.ts`, `Cargo.*`,
   `build.rs`, and the native build stay untouched. Root-level changes are limited to:
   (a) adding `"workspaces": ["website"]` to `package.json`, (b) the resulting
   `yarn.lock` update, and (c) **in Task 7 only**, adding the CI workflow file
   `.github/workflows/void-deploy.yml`. **All website ignore rules live in a new
   `website/.gitignore`, NOT the root `.gitignore`.** Nothing else at the repo root changes.
2. **No WASM, no playground.** Do NOT copy image's `pages/playground/`, `worker.ts`,
   `_engine.ts`, `protocol.ts`, the COOP/COEP/CORP headers in `void.json`, the dev
   COEP middleware or WASM path fixes in `vite.config.ts`, or the `@napi-rs/image*`,
   `@napi-rs/canvas`, `buffer` deps. simple-git has no browser build.
3. **workerd-safe highlighting.** All code samples are highlighted server-side with
   Shiki's **JavaScript regex engine** (`createJavaScriptRegexEngine({ forgiving: true })`),
   never the Oniguruma WASM engine. No runtime `WebAssembly.instantiate` anywhere.
4. **Theme:** dark-only. Accent = teal-cyan `--color-accent: oklch(74% 0.13 190)`
   (≈ `#14c7bd`); surfaces = cool slate near-black `--color-bg: #0a0b0d`. Full token
   list in the spec §6. Fonts: Space Grotesk (display), Inter (body), JetBrains Mono
   (mono), self-hosted.
5. **Content is grounded — no fabrication.** Benchmark = **1.9 s (git CLI child
   process) vs 65 ms (@napi-rs/simple-git), 1000× `getFileLatestModifiedDate`, ≈29×**
   (source: README Performance + `performance.mjs`). API names, method signatures, and
   the 15 platform triples come from the spec / `index.d.ts` / `package.json` verbatim.
   Do not invent APIs, numbers, logos, or testimonials.
6. **No-JS safe.** Content is visible and readable with JavaScript disabled; scroll
   reveals are gated behind an `html.js` class so no-JS users see content.
7. **No analytics.** Do not add Google Analytics or any tracker.
8. **Toolchain:** Node 24, Yarn 4.17 (`corepack`). Deps pinned to the reference
   `../Image/website/package.json` versions (void `0.10.2`, `@void/react` `0.10.2`,
   `@void/md` `0.10.2`, react `^19.2.1`, vite `^8.0.0`, tailwindcss `4.3.2`,
   `@tailwindcss/vite` `4.3.2`, `@fontsource-variable/*` `^5.2.x`, typescript `^6.0.3`,
   `@playwright/test` `^1.50.0`, `@types/*`). Verify with `npm info` if any fails to resolve.

**Verification note (applies to all tasks):** classic unit-test TDD does not fit a
visual SSR site. "Tests" here means: `yarn workspace website build` (and `void prepare`
if the reference requires it) completes without error, plus the task-specific render
/ content checks named in each task. The Playwright smoke test is built in Task 7.
Each task ends with a commit.

---

## Task 1: Scaffold the `website/` Yarn workspace

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

## Task 2: Design system + root layout

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

## Task 3: Content data + shared UI components + Shiki loader

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

## Task 4: Landing sections A — Hero, Benchmark, Doc-site showcase

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

## Task 5: Landing sections B — Features, Code/API, Platforms, CTA

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

## Task 6: Docs — Getting Started page

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

## Task 7: Deploy workflow + responsive polish + smoke test

**Goal:** CI deploy config, a final responsive/polish pass, and a Playwright smoke test.

**Read first (reference):** `../Image/.github/workflows/void-deploy.yml`,
`../Image/website/playwright.config.*` and any `e2e` test, to mirror the shape.

**Create / modify:**
- `.github/workflows/void-deploy.yml` — trigger on push to `main` when `website/**`
  changes; Node 24; `corepack enable`; `yarn install --immutable` at repo root;
  `yarn void deploy --project napi-simple-git` with `working-directory: website`; auth
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
present and correctly configured for `napi-simple-git`. Commit.

---

## Done criteria (whole branch)

All acceptance criteria in spec §9 met: workspace installs without touching the
library; landing renders 8 sections; `/docs` renders; theme + fonts applied; benchmark/
CountUp/InstallSwitcher/reveal/mobile-nav work; no-JS safe; workerd-safe (no runtime
WASM); responsive; deploy workflow configured; smoke test green.
