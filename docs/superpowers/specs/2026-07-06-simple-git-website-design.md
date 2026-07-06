# @napi-rs/simple-git website — Design Spec

**Date:** 2026-07-06
**Status:** Approved (brainstorming)
**Author:** LongYinan / Claude
**Target:** `simple-git.napi.rs`

---

## 1. Overview

Build a marketing + docs website for `@napi-rs/simple-git`, closely modeled on the
existing `image.napi.rs` site (source lives at `../Image/website`). Same framework,
same craft, same section rhythm — content and interactive showpiece re-cast for a
native Git library instead of an image library.

The site lives as a self-contained `website/` Yarn workspace inside the
`simple-git` repo and deploys to Cloudflare via `void deploy`.

### Goals
- Communicate what the library is (native libgit2 binding for Node, no `git` shell-out).
- Lead with the flagship use case: fast "last updated on" dates for doc-site generators.
- Show the ~30× benchmark, the full `Repository` toolbox, typed errors, and 15 prebuilt platforms.
- A single Getting Started docs page; deep API stays in the GitHub README / `index.d.ts`.

### Non-goals (out of scope)
- No WASM in-browser playground (simple-git has **no** `wasm32-wasi` build; git ops need FS/network).
  Therefore **no** COOP/COEP/CORP headers, no worker/engine/protocol plumbing.
- No Recipes docs page (link out to GitHub README + `index.d.ts`). *(A full hosted **API Reference** at `/docs/api` was added as a follow-on — see §11; it supersedes the "no full API-reference" part of this non-goal.)*
- No changelog page (can be added later).
- No analytics by default (GA/Plausible can be added later).
- The library source (`src/`, `index.js`, `index.d.ts`, native build) is **untouched**.

---

## 2. Stack (clone of image.napi.rs)

| Concern | Choice |
|---|---|
| Framework | `void` (VoidZero SSR meta-framework) `0.10.x` — `output: "server"` |
| React | `@void/react`, React 19 |
| Markdown | `@void/md` (docs page + prose theme CSS) |
| Build | Vite 8 (Rolldown) |
| Styling | Tailwind CSS v4 (CSS-first `@theme` in `app.css`, no JS config) |
| Syntax highlight | Shiki with **JS regex engine** (`createJavaScriptRegexEngine({ forgiving: true })`) — workerd-safe; `github-dark`; module-level singleton in `lib/highlight.ts` |
| Fonts | `@fontsource-variable/{space-grotesk,inter,jetbrains-mono}` self-hosted woff2 |
| Runtime target | Cloudflare `workerd` |

**Plugin order** (`vite.config.ts`): `voidPlugin()` → `voidReact()` → `voidMarkdown()` → `tailwindcss()`.
No dev COOP/COEP middleware, no WASM path fixes (not needed).

**Versions** — pin to whatever `../Image/website/package.json` uses at build time
(void `0.10.2`, `@void/react`/`@void/md` `0.10.2`, react `^19.2.1`, vite `^8.0.0`,
tailwindcss `4.3.2`, `@tailwindcss/vite` `4.3.2`). Verify latest with `npm info` before pinning.

---

## 3. Repository layout

```
simple-git/
├─ package.json                 # add "workspaces": ["website"]
├─ src/ …                       # library — UNTOUCHED
└─ website/                     # new Yarn workspace, self-contained
   ├─ package.json
   ├─ void.json                 # SSR config
   ├─ vite.config.ts
   ├─ tsconfig.json
   ├─ app.css                   # Tailwind v4 @theme tokens
   ├─ lib/highlight.ts          # Shiki singleton (JS regex engine)
   ├─ pages/
   │  ├─ layout.tsx             # root: header + footer + mobile nav
   │  ├─ index.tsx              # landing (React, composes sections)
   │  ├─ index.server.ts        # loader: pre-highlight code samples (Shiki), head
   │  ├─ _components/           # section + UI components (see §4)
   │  ├─ _data/                 # benchmarks, features, platforms, docSites, samples
   │  └─ docs/
   │     ├─ layout.tsx          # docs shell (centered prose)
   │     └─ index.md            # Getting Started (@void/md, frontmatter title/description)
   ├─ public/
   │  ├─ favicon.svg            # new brand mark (see §6)
   │  ├─ fonts/*.woff2          # self-hosted variable fonts
   │  └─ og.svg / og.png        # optional OG image
   └─ (scripts/ optional — OG raster only)
```

**Safe to add:** the package's `files`/`.npmignore` already restrict the published npm
tarball to `index.js` + `index.d.ts` + `.node` prebuilds, so `website/` is never
published. The native (cargo/napi) build is independent of the site's JS deps.

**Workspace note:** root gains `"workspaces": ["website"]`. This only affects JS dep
hoisting; it does not touch the napi/cargo build. Root already uses `yarn@4.17.0`.

Routes: `/` (landing), `/docs` (Getting Started), `/docs/api` (API Reference — added as a follow-on, §11).

---

## 4. Landing page anatomy

`pages/index.tsx` composes 8 sections in order. `index.server.ts` `loader`
pre-highlights all code samples server-side (Shiki) and passes HTML as props
(mirrors image's pattern). Each section is a component in `pages/_components/`.

```
HEADER      @napi-rs/simple-git      Docs · GitHub · npm       (blurred translucent, CSS mobile drawer)

01 HERO
   eyebrow:  NATIVE NODE ADDON · POWERED BY RUST · libgit2
   H1:       "Git for Node, at native speed."   (accent word: "native")
   tagline:  Open, inspect, stage, commit, blame, branch and push real repositories
             through libgit2 — no `git` shell-out, with JS-native Date / number / Buffer types.
   buttons:  [ Get started → /docs ]  [ GitHub ]  [ npm ]
   InstallSwitcher: npm | yarn | pnpm | bun   (tabs, localStorage-persisted, copy button)
   right:    CodeBlock — flagship "last updated" snippet
   stat tiles (CountUp):  "29× faster"  ·  "15 platforms"  ·  "0 dependencies"

02 BENCHMARK          eyebrow "01 — BENCHMARK", title "~30× faster than shelling out"
   1000× getFileLatestModifiedDate:
     git CLI (child_process)   ██████████████████ 1.9 s
     @napi-rs/simple-git       █ 65 ms
   big CountUp "29×"; animated horizontal bars (IntersectionObserver width animation)
   caption: vs spawning `git log` as a child process (source: performance.mjs)

03 WHY / DOC-SITE     eyebrow "02 — WHY", title "Powers 'Last updated on' for your docs"
   mock doc-page footer chip:  "Last updated on Jul 6, 2026 by LongYinan"
   code:  repo.getFilesLatestModified([...paths])   // bulk, single history walk
   used-by row:  Nextra · Docusaurus · Starlight · Fumadocs · Rspress   (name + inline logo/glyph)
   copy: doc generators compute "last updated" per page from git history; simple-git does it
         in-process, fast, no child_process per file.

04 FEATURES           eyebrow "03 — FEATURES", title "A full Git toolbox"   (3×3 card grid)
   see §5 features list (9 cards)

05 CODE / API         eyebrow "04 — API", title "Show me the code"   (tabbed CodeBlock)
   tabs: [ Status ] [ Stage & commit ] [ Blame ] [ Push ] [ Typed errors ]
   Shiki-highlighted, snippets verbatim from README (see §5 samples)

06 PLATFORMS          eyebrow "05 — PLATFORMS", title "Prebuilt everywhere"
   responsive grid of the 15 napi triples grouped by OS:
     Windows (x64 · x86 · ARM64) · macOS (Intel · Apple Silicon) ·
     Linux glibc & musl (x64 · ARM64 · ARMv7 · ppc64le · s390x) · Android · FreeBSD
   caption: "npm install pulls a ready binary — no compiler, no node-gyp."  Node ≥ 10.

07 CTA BAND           eyebrow "GET STARTED", title "Add it in one line"
   InstallSwitcher (compact) + [ Read the docs ] [ GitHub ]

FOOTER      brand + tagline · links (Docs / GitHub / npm / napi.rs) · "Built with napi-rs · MIT licensed"
```

**Interactive components** (cloned from image's toolkit, no WASM):
`InstallSwitcher` (tabs + localStorage + copy), `CountUp`, `Reveal` (scroll reveal,
gated behind `html.js` for no-JS SEO safety), animated benchmark bars, `CodeBlock`
(renders pre-highlighted HTML via `dangerouslySetInnerHTML`), `TabbedCodeBlock`,
CSS-only mobile nav drawer (checkbox `:checked` toggle).

---

## 5. Content (grounded in the repo — verbatim where noted)

### Feature cards (9) — `_data/features.ts`
1. **Native libgit2 speed** — Talks to Git through compiled Rust/libgit2, not a child-process `git` shell-out.
2. **"Last updated" dates for docs** — `getFileLatestModifiedDate` / `getFileLastModifiedDate` / `getFilesLatestModified` power "last updated on" stamps in Nextra, Docusaurus, Starlight, Fumadocs, Rspress.
3. **Full repository toolbox** — Init, open, discover, clone; status, stage, commit, blame, diff, branch, checkout, tag, refs, remotes, revwalk — one cohesive `Repository` API.
4. **Off-main-thread async** — `cloneAsync`, `commitAsync`, `fetchAsync`, `pushAsync`, plus async status/blame/file-date variants, each accepting an `AbortSignal`.
5. **First-class TypeScript** — Hand-annotated `index.d.ts` with JS-native types: `Date`, `number`, `Buffer` instead of raw libgit2 primitives.
6. **Typed, catchable errors** — Every failure carries a stable `GitErrorCode`, narrowable with the total, never-throwing `isGitError()` guard.
7. **Prebuilt for 15 platforms** — Windows (x64/x86/ARM64), macOS (Intel + Apple Silicon), Linux glibc & musl, Android, FreeBSD — no compiler needed.
8. **Push/fetch with real creds & progress** — SSH-agent, SSH-key, userpass via `Cred`; transfer + per-ref update callbacks via `RemoteCallbacks`.
9. **Deterministic cleanup** — `dispose()` / `free()` release native handles eagerly (frees Windows packfile fds); `using` supported.

### Benchmark — `_data/benchmarks.ts`
- Task: `getFileLatestModifiedDate` × 1000.
- `git` CLI via `child_process.exec`: **1.9 s**. `@napi-rs/simple-git`: **65 ms**. → **~29×**.
- Source: README "Performance" section + `performance.mjs`. Only baseline is the raw `git` CLI child process.

### Platforms (15 triples) — `_data/platforms.ts`
`x86_64-pc-windows-msvc`, `i686-pc-windows-msvc`, `aarch64-pc-windows-msvc`,
`x86_64-apple-darwin`, `aarch64-apple-darwin`,
`x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-gnu`,
`aarch64-unknown-linux-musl`, `armv7-unknown-linux-gnueabihf`,
`aarch64-linux-android`, `armv7-linux-androideabi`, `x86_64-unknown-freebsd`,
`powerpc64le-unknown-linux-gnu`, `s390x-unknown-linux-gnu`. Node engine `>= 10`.

### Doc-site generators — `_data/docSites.ts`
Nextra, Docusaurus, Starlight, Fumadocs, Rspress (name + homepage URL; inline SVG or text glyph).

### Code samples (verbatim from README) — hero + tabbed section
Hero flagship snippet:
```ts
import { Repository } from '@napi-rs/simple-git'

const repo = new Repository('/path/to/repo')

// Last-modified commit time of `build.rs`, in ms since the Unix epoch.
const lastModified = repo.getFileLatestModifiedDate('build.rs')
console.log(new Date(lastModified)) // 2022-03-13T12:47:47.920Z

// Null-safe alternative: a `Date`, or `null` (never throws).
const date = repo.getFileLastModifiedDate('build.rs')
```
Tabs (all verbatim from README):
- **Status** — `repo.statuses()` / `statusFile` / `statusesAsync({ includeIgnored: true })`.
- **Stage & commit** — `config()` → `signature()` → `index().addPath` → `writeTree` → `commit(...)`.
- **Blame** — `blameFile('build.rs')` hunk loop + `blameLine(...)?.finalAuthorName`.
- **Push** — `findRemote('origin')` + `RemoteCallbacks().pushUpdateReference/pushTransferProgress` + `remote.push([...], new PushOptions()...)`.
- **Typed errors** — `try/catch` with `isGitError(e) && e.code === GitErrorCode.NotFound`.

(Full verbatim snippets are in the content research; implementers pull them from README.md.)

### Trust badges (optional, in footer or hero)
CI status, npm monthly downloads (`img.shields.io/npm/dm/@napi-rs/simple-git`), install size (packagephobia).

---

## 6. Design system

Clone image's craft; **new accent + cooler surfaces** for a distinct identity.

**Surfaces — cool slate near-black** (image uses warm brown-black; we go cool):
```
--color-bg:            #0a0b0d
--color-surface-1:     #101318
--color-surface-2:     #181c22
--color-fg:            #eef2f5
--color-muted:         #9aa4af
--color-faint:         #79828d
--color-border:        rgb(228 240 250 / .08)
--color-border-strong: rgb(228 240 250 / .15)
```

**Accent — bright teal-cyan** (the chosen new color; distinct from image's amber,
reads fast/native, stays in the napi.rs family where each package owns an accent):
```
--color-accent:        oklch(74% 0.13 190)   /* ≈ #14c7bd */
--color-accent-strong: oklch(80% 0.14 190)   /* ≈ #35e0d3 */
--color-accent-muted:  oklch(60% 0.10 190)
--color-accent-glow:   teal at low alpha (radial glow)
--color-accent-fg:     #04211f               /* dark ink on accent buttons */
```

**Fonts** (same trio as image, self-hosted): Space Grotesk (display `--font-display`),
Inter (body `--font-sans`), JetBrains Mono (mono `--font-mono`). Fluid type scale via
`clamp()` tokens (`--text-display-xl`, `--text-display-lg`, `--text-h2`, `--text-eyebrow`).

**Global treatment:** dark-only (`color-scheme: dark`, `data-theme="dark"`), faint SVG
film-grain overlay (`body::after`), radial accent glow class (`.accent-glow`), reusable
`@layer components` classes (`.container-page` max-w ~1140px, `.site-header`, `.eyebrow`, `.reveal`).

**Brand mark** — none exists; create `public/favicon.svg`: rounded square, accent-teal
background, a monospace/commit-node or branch glyph in `--color-accent-fg`. Wordmark
stays text: `@napi-rs/simple-git` in JetBrains Mono. Optional OG image (`og.svg`,
optionally rasterized to `og.png` via `@napi-rs/canvas` at build — optional, low priority).

---

## 7. `void.json` (SSR config)

```jsonc
{
  "output": "server",
  "head": {
    "titleTemplate": "%s | @napi-rs/simple-git",
    "htmlAttrs": { "lang": "en", "data-theme": "dark" },
    "link": [ /* favicon + font preloads */ ]
  },
  "routing": {
    "redirects": [ /* 308 trailing-slash normalize */ ],
    "revalidate": { "*": 0 }
  },
  "worker": { "compatibility_date": "<current>" }
}
```
**Omit** image's `routing.headers` COOP/COEP block and `/playground` / `/assets/*`
entries — no WASM, no cross-origin isolation needed.

---

## 8. Deployment

- `website/` is a Yarn workspace; root `package.json` gains `"workspaces": ["website"]`.
- CI: `.github/workflows/void-deploy.yml`, triggers on push to `main` when `website/**` changes.
  Node 24, `corepack enable`, `yarn install --immutable` at repo root, then
  `yarn void deploy --project simple-git` with `working-directory: website`.
  Auth via GitHub OIDC (`id-token: write`, audience `void`), `VOID_API_URL` default
  `https://api.void.cloud`. Mirrors `../Image/.github/workflows/void-deploy.yml`.
- Local: `yarn workspace <website> dev` / `build` / `preview`.

---

## 9. Acceptance criteria

1. `yarn install` at repo root succeeds with the new workspace; the native library build is unaffected.
2. `yarn build` in `website/` produces a working SSR bundle (`dist/client` + `dist/ssr`).
3. Landing page renders all 8 sections; code samples are Shiki-highlighted server-side; no runtime `WebAssembly.instantiate` calls (workerd-safe).
4. `/docs` renders the Getting Started markdown with the prose theme.
5. Benchmark bars, CountUp, InstallSwitcher, scroll-reveal, and mobile nav work; page is usable with JS disabled (content visible, no-JS-safe reveals).
6. Teal accent + cool slate surfaces applied via `app.css` tokens; fonts self-hosted and preloaded.
7. Responsive down to mobile; no horizontal body scroll.
8. `dev` runs without COEP/WASM errors. Deploy workflow present and configured for `simple-git`.
9. Optional: a Playwright smoke test (home + /docs load, key headings present) mirroring image's `test:e2e`.

---

## 10. Implementation order (for subagent-driven-development)

1. **Scaffold** — `website/` workspace: `package.json`, `void.json`, `vite.config.ts`, `tsconfig.json`, root `workspaces`. Verify `dev` boots a blank page.
2. **Design system** — `app.css` tokens (teal + cool slate), fonts, `favicon.svg`, root `layout.tsx` (header/footer/mobile nav), `lib/highlight.ts`.
3. **Data + samples** — `_data/*` files; wire `index.server.ts` Shiki loader.
4. **Landing sections** — Hero → Benchmark → Why/doc-site → Features → Code/API → Platforms → CTA → Footer, plus shared UI (`InstallSwitcher`, `CountUp`, `Reveal`, `CodeBlock`, `TabbedCodeBlock`).
5. **Docs** — `docs/layout.tsx` + `docs/index.md` Getting Started.
6. **Deploy + polish** — `void-deploy.yml`, responsive pass, optional OG image + Playwright smoke test.

Reference implementation to read side-by-side: `../Image/website` (esp. `pages/index.tsx`,
`pages/_components/*`, `app.css`, `lib/highlight.ts`, `void.json`, `vite.config.ts`,
`pages/layout.tsx`, `.github/workflows/void-deploy.yml`). Do **not** copy the `playground/`
tree or any COEP/WASM code.

---

## 11. API Reference page (`/docs/api`) — follow-on

> Added after §1–§10 shipped; supersedes the §1 non-goal "no full API-reference".
> Approved separately (brainstorming). This section is the merged copy of the original
> standalone `…-api-reference-design.md`. Its executable task breakdown is **Phase 3** of
> `docs/superpowers/plans/2026-07-06-simple-git-website-plan.md`.

### 11.1 Context

The site's `/docs` (Getting Started) "Full API" section only links out to the GitHub README,
`index.d.ts`, and npm; the site hosts no reference of its own. This adds a hosted,
comprehensive **API Reference** at `/docs/api`.

- **Source of truth:** `index.d.ts` (2360 lines) — hand-annotated, with rich JSDoc prose on
  ~100% of members: **25 classes** (`Repository` alone has ~73 methods), **13 interfaces**,
  **18 `const enum`s**, **3 functions**. The JSDoc is prose (no `@param`/`@returns`/`@example`
  tags).
- **`GitErrorCode` member meanings live only in `README.md`** (its `| Token | Meaning |`
  table), not in `index.d.ts`.
- **Modeled on** image.napi.rs (`../Image/website`), which hand-authors `pages/docs/api.md`
  plus a two-column sidebar docs layout. No TypeDoc/api-extractor tooling exists in either repo.

### 11.2 Decision (approved)

- **Hand-authored, curated markdown** — not auto-generated. "Curated" = authoring *style*
  (hand-written, editorially grouped prose), **not** reduced coverage.
- **Single `/docs/api` page** with an in-page TOC, inside a two-column **sidebar** docs shell.

### 11.3 Scope

**Comprehensive coverage of the public API surface**, hand-authored with editorial grouping:

- **`Repository`** — all public methods, grouped by concern: construction & static factories
  (`init`, `initBare`, `discover`, `openExt`, `clone`/`cloneAsync`/`cloneRecurse`); the file
  "last updated" family (`getFileLatestModifiedDate`, `getFileLastModifiedDate`,
  `getFileLatestModified`, `getFilesLatestModified(+Async)`, `getFileCreatedDate`); status;
  index & commit; blame; branches / checkout / references; remotes (fetch/push); tags; config &
  signature; revwalk & object lookup; disposal.
- **All other exported classes** — `Commit`, `Tree`, `TreeEntry`, `TreeIter`, `Blob`,
  `GitObject`, `Tag`, `Reference`, `Signature`, `Branch`, `Remote`, `Index`, `Config`,
  `RevWalk`, `Diff`, `DiffDelta`, `DiffFile`, `Deltas`, `Cred`, `RepoBuilder`, `FetchOptions`,
  `PushOptions`, `ProxyOptions`, `RemoteCallbacks` — each with its public methods/signatures.
- **All option / result interfaces** — `StatusOptions`, `CheckoutOptions`, `DiffOptions`,
  `BlameOptions`, `FileStatus`, `FileModification`, `BlameHunk`, `ConfigEntry`, `Progress`,
  `PushTransferProgress`, `PushUpdateReference`, `TagForeachItem`, `CredInfo`.
- **All 18 enums** with their members; **`GitErrorCode`** rendered with the meanings table
  sourced from the README.
- **The 3 functions** — `isGitError`, `credTypeContains`, `diffFlagsContains`.

**Not in scope:** modifying the library; TypeDoc/generator tooling; splitting into multiple
routes; documenting private/internal members.

### 11.4 Structure of `/docs/api` (one markdown page)

- Frontmatter `title: 'API Reference'` + `description`; `[[toc]]` for the in-page TOC.
- Short intro — the import root and how to read the page.
- `## Repository` (the subsections above).
- `## Git objects & handles` — the other classes.
- `## Options & result types` — the interfaces (+ the options classes
  `FetchOptions`/`PushOptions`/`ProxyOptions`/`RemoteCallbacks`).
- `## Enums` — all 18, including the `GitErrorCode` meanings table.
- `## Functions` — the 3 standalone functions.
- `## Error handling` — `isGitError` guard + how `GitErrorCode` / `AbortError` (`'Cancelled'`)
  relate (consistent with the Getting Started page).

### 11.5 Layout & navigation

- Upgrade `website/pages/docs/layout.tsx` from the single centered column (§3/§10 Phase 1) to a
  **two-column shell** mirroring `../Image/website/pages/docs/layout.tsx`: a hand-maintained
  `NAV` (`Getting Started → /docs`, `API Reference → /docs/api`), a mobile `<details>`
  disclosure nav (no JS), a desktop sticky `<aside>`, markdown in `<article class="void-md …">`.
  No active-link highlight (docs pages don't hydrate).
- New `website/pages/docs/api.server.ts` head: `title: 'API Reference'`, description, canonical
  `https://simple-git.napi.rs/docs/api`, `og:url`, `prerender = true`.
- Update the Getting Started "Full API" section to link **internally** to `/docs/api` (keep the
  external README/types links as "full source").
- Top-level site nav stays `Docs · GitHub · npm`; the API page is reached via the docs sidebar
  and the Getting Started link.

### 11.6 Grounding & quality (binding — reviewers use these as the attention lens)

1. **Library untouched.** Changes confined to `website/` (plus this spec + its plan under
   `docs/superpowers/`). No `src/`, `index.d.ts`, `index.js`, `Cargo.*`, root config.
2. **Every TS signature faithful to `index.d.ts`.** Reproduce parameter names, types,
   optionality, return types, and `*Async`/`AbortSignal` overloads as written. No invented
   methods, parameters, or types.
3. **Every description grounded** in the member's `index.d.ts` JSDoc or the README — no
   ungrounded capability/behavior claims. `GitErrorCode` meanings verbatim from the README table.
4. **Typed-error accuracy:** Git-layer errors carry a `GitErrorCode` (narrow with `isGitError`);
   an aborted `*Async` rejects with napi's `AbortError` (`code === 'Cancelled'`), which
   `isGitError` does not match. (Consistent with the already-shipped Getting Started + llms.txt
   wording.)
5. **No-JS safe & static.** The page renders fully with JS disabled; `prerender = true`; code
   highlighted at build time by `voidMarkdown()` (Shiki JS-regex engine, workerd-safe). No new
   runtime WASM, no analytics.
6. **workerd-safe:** the page and layout introduce no runtime `WebAssembly.instantiate` and no
   Node-only request-time APIs.

### 11.7 Done criteria (API Reference)

- `/docs/api` renders with comprehensive coverage of the surface listed under §11.3; every
  signature verified against `index.d.ts`.
- Two-column sidebar present on both `/docs` and `/docs/api`; mobile disclosure nav works with
  JS off.
- `api.server.ts` head + canonical `/docs/api` present; `prerender = true`; Getting Started
  links internally to `/docs/api`.
- `void:prepare` + `build` succeed; Playwright e2e passes (extend smoke to load `/docs/api`).
- No changes outside `website/` + `docs/superpowers/`.
