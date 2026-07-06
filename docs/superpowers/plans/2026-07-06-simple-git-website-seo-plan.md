# simple-git.napi.rs — SEO fixes — Implementation Plan

Fixes the findings from the multi-agent SEO audit (all adversarially verified against the
rendered HTML + source). Branch: `feat/website` (continues PR #146). Domain: **https://simple-git.napi.rs**.

Reference site (for how void does head/assets): `../Image/website`.

---

## Context

The simple-git website (`website/`, void SSR, routes `/` and `/docs`, Cloudflare workerd)
is engineered well but ships with no share/discovery meta layer. This plan wires it in:
OG/Twitter cards + image, canonical + og:url, JSON-LD, robots/sitemap, charset/theme-color,
prerender, plus a11y and CWV micro-fixes. Every change is workerd-safe (head config, static
`public/` assets, server-rendered tags, CSS). The library is NOT touched.

---

## Global Constraints (binding — reviewers use these as the lens)

1. **Scope:** edits only under `website/` (`void.json`, `pages/**`, `app.css`, `public/**`,
   `scripts/**`). Do NOT modify the library (`src/`, `index.js`, `index.d.ts`, native build),
   root config, `package.json`, or the deploy workflow.
2. **workerd-safe:** no runtime `WebAssembly.instantiate`, no Node-only APIs at request time.
   Static assets go in `website/public/` (served at web root — `favicon.svg` proves the path).
   Head tags go via `void.json` base head OR a route's `.server.ts` head export. JSON-LD is
   server-rendered in the layout.
3. **Absolute URLs:** canonical, `og:url`, `og:image`, `twitter:image`, and sitemap `<loc>`
   use `https://simple-git.napi.rs/...`. Use the **non-trailing-slash** `/docs` form (matches
   the existing `void.json` 308 redirect).
4. **void head gotchas (verified against `node_modules/void`):**
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
5. **No regressions:** the site must keep building (`void:prepare && build`), the Playwright
   e2e must keep passing, content stays visible with JS off, dark-only, no analytics. Do not
   regress the "already good" list: SSR crawlable HTML, one `<h1>`/route + logical outline,
   self-hosted preloaded fonts, `lang=en` + viewport, home↔docs cross-links, `rel=noreferrer`
   on external links, Shiki JS-regex highlighting.
6. **Content grounded:** JSON-LD and copy use real facts (name `@napi-rs/simple-git`, version
   `1.0.0`, MIT, author LongYinan / Brooooooklyn, repo + npm URLs, the 15 platforms, Node ≥ 10).
   No fabricated data.

**Verification note:** each task ends with `void:prepare && build` succeeding and (where the
task changes rendered output) a grep/inspection of the built/served HTML proving the tags/assets
are present and correct. Keep `test:e2e` green. Each task commits on `feat/website`.

---

## Task 1: Static assets — OG image, raster icons, robots.txt, sitemap.xml

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

## Task 2: Meta & social tag layer — `void.json` base + home route head

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

## Task 3: `/docs` route head — canonical, og:url, description

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

## Task 4: JSON-LD structured data

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

## Task 5: Rendering strategy — prerender, revalidate, Worker asset precedence

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

## Task 6: Accessibility & paint/CWV micro-fixes

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

## Done criteria (whole branch)

Rendered `/` and `/docs` heads carry: unique title/description (no backticks, docs ≤160),
canonical + og:url, full OG + Twitter card with a resolvable 1200×630 og.png, charset, theme-color,
apple-touch-icon; JSON-LD (SoftwareApplication + WebSite) present and valid on both routes;
`robots.txt` + `sitemap.xml` served; both routes prerendered (no per-request Shiki); skip link,
focus-visible, labeled navs, new-tab cues; hero LCP not JS-gated; film-grain blend removed; font
CLS mitigated. Build + Playwright e2e green; library untouched; no-JS safe; no analytics.
