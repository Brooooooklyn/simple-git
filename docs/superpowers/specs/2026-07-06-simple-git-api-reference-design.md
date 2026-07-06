# @napi-rs/simple-git website — API Reference page — Design

## Context

The website (`website/`, void SSR) currently has one docs page — `/docs` (Getting Started). Its "Full API" section only links out to the GitHub README, `index.d.ts`, and npm; the site hosts no reference of its own. This adds a hosted, comprehensive **API Reference** at `/docs/api`.

- **Source of truth:** `index.d.ts` (2360 lines) — hand-annotated, with rich JSDoc prose on ~100% of members: **25 classes** (`Repository` alone has ~73 methods), **13 interfaces**, **18 `const enum`s**, **3 functions**. The JSDoc is prose (no `@param`/`@returns`/`@example` tags).
- **`GitErrorCode` member meanings live only in `README.md`** (its `| Token | Meaning |` table), not in `index.d.ts`.
- **Modeled on** image.napi.rs (`../Image/website`), which hand-authors `pages/docs/api.md` plus a two-column sidebar docs layout. No TypeDoc/api-extractor tooling exists in either repo.

## Decision (approved)

- **Hand-authored, curated markdown** — not auto-generated. "Curated" = authoring *style* (hand-written, editorially grouped prose), **not** reduced coverage.
- **Single `/docs/api` page** with an in-page TOC, inside a two-column **sidebar** docs shell.

## Scope

**Comprehensive coverage of the public API surface**, hand-authored with editorial grouping:

- **`Repository`** — all public methods, grouped by concern: construction & static factories (`init`, `initBare`, `discover`, `openExt`, `clone`/`cloneAsync`/`cloneRecurse`); the file "last updated" family (`getFileLatestModifiedDate`, `getFileLastModifiedDate`, `getFileLatestModified`, `getFilesLatestModified(+Async)`, `getFileCreatedDate`); status; index & commit; blame; branches / checkout / references; remotes (fetch/push); tags; config & signature; revwalk & object lookup; disposal.
- **All other exported classes** — `Commit`, `Tree`, `TreeEntry`, `TreeIter`, `Blob`, `GitObject`, `Tag`, `Reference`, `Signature`, `Branch`, `Remote`, `Index`, `Config`, `RevWalk`, `Diff`, `DiffDelta`, `DiffFile`, `Deltas`, `Cred`, `RepoBuilder`, `FetchOptions`, `PushOptions`, `ProxyOptions`, `RemoteCallbacks` — each with its public methods/signatures.
- **All option / result interfaces** — `StatusOptions`, `CheckoutOptions`, `DiffOptions`, `BlameOptions`, `FileStatus`, `FileModification`, `BlameHunk`, `ConfigEntry`, `Progress`, `PushTransferProgress`, `PushUpdateReference`, `TagForeachItem`, `CredInfo`.
- **All 18 enums** with their members; **`GitErrorCode`** rendered with the meanings table sourced from the README.
- **The 3 functions** — `isGitError`, `credTypeContains`, `diffFlagsContains`.

**Not in scope:** modifying the library; TypeDoc/generator tooling; splitting into multiple routes; documenting private/internal members.

## Structure of `/docs/api` (one markdown page)

- Frontmatter `title: 'API Reference'` + `description`; `[[toc]]` for the in-page TOC.
- Short intro — the import root and how to read the page.
- `## Repository` (the subsections above).
- `## Git objects & handles` — the other classes.
- `## Options & result types` — the interfaces (+ the options classes `FetchOptions`/`PushOptions`/`ProxyOptions`/`RemoteCallbacks`).
- `## Enums` — all 18, including the `GitErrorCode` meanings table.
- `## Functions` — the 3 standalone functions.
- `## Error handling` — `isGitError` guard + how `GitErrorCode` / `AbortError` (`'Cancelled'`) relate (consistent with the Getting Started page).

## Layout & navigation

- Upgrade `website/pages/docs/layout.tsx` from the single centered column to a **two-column shell** mirroring `../Image/website/pages/docs/layout.tsx`: a hand-maintained `NAV` (`Getting Started → /docs`, `API Reference → /docs/api`), a mobile `<details>` disclosure nav (no JS), a desktop sticky `<aside>`, markdown in `<article class="void-md …">`. No active-link highlight (docs pages don't hydrate).
- New `website/pages/docs/api.server.ts` head: `title: 'API Reference'`, description, canonical `https://simple-git.napi.rs/docs/api`, `og:url`, `prerender = true`.
- Update the Getting Started "Full API" section to link **internally** to `/docs/api` (keep the external README/types links as "full source").
- Top-level site nav stays `Docs · GitHub · npm`; the API page is reached via the docs sidebar and the Getting Started link.

## Grounding & quality (binding — reviewers use these as the attention lens)

1. **Library untouched.** Changes confined to `website/` (plus this spec + its plan under `docs/superpowers/`). No `src/`, `index.d.ts`, `index.js`, `Cargo.*`, root config.
2. **Every TS signature faithful to `index.d.ts`.** Reproduce parameter names, types, optionality, return types, and `*Async`/`AbortSignal` overloads as written. No invented methods, parameters, or types.
3. **Every description grounded** in the member's `index.d.ts` JSDoc or the README — no ungrounded capability/behavior claims. `GitErrorCode` meanings verbatim from the README table.
4. **Typed-error accuracy:** Git-layer errors carry a `GitErrorCode` (narrow with `isGitError`); an aborted `*Async` rejects with napi's `AbortError` (`code === 'Cancelled'`), which `isGitError` does not match. (Consistent with the already-shipped Getting Started + llms.txt wording.)
5. **No-JS safe & static.** The page renders fully with JS disabled; `prerender = true`; code highlighted at build time by `voidMarkdown()` (Shiki JS-regex engine, workerd-safe). No new runtime WASM, no analytics.
6. **workerd-safe:** the page and layout introduce no runtime `WebAssembly.instantiate` and no Node-only request-time APIs.

## Done criteria (whole feature)

- `/docs/api` renders with comprehensive coverage of the surface listed under Scope; every signature verified against `index.d.ts`.
- Two-column sidebar present on both `/docs` and `/docs/api`; mobile disclosure nav works with JS off.
- `api.server.ts` head + canonical `/docs/api` present; `prerender = true`; Getting Started links internally to `/docs/api`.
- `void:prepare` + `build` succeed; Playwright e2e passes (extend smoke to load `/docs/api`).
- No changes outside `website/` + `docs/superpowers/`.
