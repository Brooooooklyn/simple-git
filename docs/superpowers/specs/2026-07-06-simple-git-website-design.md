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
- No full API-reference or Recipes docs pages (link out to GitHub README + `index.d.ts`).
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

Routes: `/` (landing), `/docs` (Getting Started).

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
  `yarn void deploy --project napi-simple-git` with `working-directory: website`.
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
8. `dev` runs without COEP/WASM errors. Deploy workflow present and configured for `napi-simple-git`.
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
