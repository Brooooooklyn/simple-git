# @napi-rs/simple-git website — API Reference — Implementation Plan

Design: `docs/superpowers/specs/2026-07-06-simple-git-api-reference-design.md`.
Branch: `feat/website` (PR #146). Build via subagent-driven development, each task codex-gated on grounding.

## Context

Add a hosted, comprehensive **API Reference** at `/docs/api` — a single hand-authored markdown page inside a new two-column sidebar docs shell. The site's `/docs` is currently a single Getting Started page whose "Full API" section only links out.

- **Source of truth:** `index.d.ts` (2360 lines). Key ranges: `Repository` class `907`–`1503` (~73 methods); the other 24 classes across `3`–`1724`; 13 interfaces `1747`–`2352`; 18 `const enum`s `1730`–`2291`; 3 functions (`credTypeContains` `1935`, `diffFlagsContains` `1991`, `isGitError` `2161`). JSDoc is rich prose (no `@param`/`@returns` tags).
- **`GitErrorCode` member meanings** live only in `README.md`'s `| Token | Meaning |` table (`README.md:690`–`720`).
- **Layout reference:** `../Image/website/pages/docs/layout.tsx` (two-column sidebar) and `../Image/website/pages/docs/api.md` (hand-authored page shape).
- **Rendering:** markdown routes via `voidMarkdown()` (`website/vite.config.ts:11`), static/non-hydrating, Shiki JS-regex highlight at build; `[[toc]]` expands to an in-page TOC; co-located `*.server.ts` head + `layout.tsx` shell.

## Global Constraints (binding — reviewers use these as the attention lens)

1. **Library untouched.** All changes confined to `website/` (plus this plan + the spec under `docs/superpowers/`). No `src/`, `index.d.ts`, `index.js`, `Cargo.*`, `build.rs`, root config, or CI.
2. **Every TS signature faithful to `index.d.ts`** — verbatim parameter names, types, optionality (`?`), return types, and every `*Async`/`AbortSignal` overload. No invented methods, parameters, types, or overloads. When in doubt, copy the declaration text from `index.d.ts`.
3. **Every description grounded** in the member's `index.d.ts` JSDoc or `README.md`. No ungrounded capability/behavior/performance claims. `GitErrorCode` meanings verbatim from the README table.
4. **Typed-error accuracy:** Git-layer errors carry a `GitErrorCode` (narrow with `isGitError`); an aborted `*Async` rejects with napi's `AbortError` (`code === 'Cancelled'`), NOT a `GitErrorCode`, and `isGitError` returns `false` for it. Consistent with the shipped Getting Started + llms.txt wording.
5. **No-JS safe & static.** Renders fully with JS disabled; `prerender = true`; highlight at build via `voidMarkdown()` (Shiki JS-regex, workerd-safe). No new runtime WASM, no analytics, no COOP/COEP.
6. **Top-level site nav unchanged** — stays `Docs · GitHub · npm` (`website/pages/layout.tsx`). API is reached via the docs sidebar + the Getting Started "Full API" link only.
7. **npm links** use `https://npmx.dev/package/@napi-rs/simple-git` (already the site convention). Any new npm link uses npmx.dev, not npmjs.com.

## Task 1: Docs sidebar layout + `/docs/api` scaffold

**Goal:** stand up the two-column docs shell and an empty-but-structured `/docs/api` page; no API content yet.
- Upgrade `website/pages/docs/layout.tsx` from the single centered column to a **two-column sidebar shell** mirroring `../Image/website/pages/docs/layout.tsx`: a hand-maintained `NAV` array `[{ label: 'Getting Started', href: '/docs' }, { label: 'API Reference', href: '/docs/api' }]`, a mobile `<details>` disclosure nav (works with JS off), a desktop sticky `<aside>`, markdown rendered in `<article className="void-md …">`. No active-link highlight (pages don't hydrate). Keep the existing prose `max-width` for readability.
- Create `website/pages/docs/api.md`: frontmatter `title: 'API Reference'` + a ≤160-char `description`; a `[[toc]]`; a short intro (the `import … from '@napi-rs/simple-git'` root + one line on how the page is organized); then the six top-level section headers as empty scaffolding — `## Repository`, `## Git objects & handles`, `## Options & result types`, `## Enums`, `## Functions`, `## Error handling` — each with a one-line lead-in (content lands in Tasks 2–7). No `TBD`/`TODO` text.
- Create `website/pages/docs/api.server.ts` mirroring `index.server.ts`: `export const prerender = true`; a `defineHead()` with `title: 'API Reference'`, the description, canonical `https://simple-git.napi.rs/docs/api`, `og:url`, `og:title`.
- Update the Getting Started "Full API" section (`website/pages/docs/index.md`) to link **internally** to `/docs/api` (keep the external README/`index.d.ts` links as "full source").
- Extend the Playwright smoke test to load `/docs/api` and assert it renders (title + sidebar present).
- **Verify:** `void:prepare` + `build` succeed; `/docs` and `/docs/api` both render with the sidebar; mobile `<details>` nav works with JS disabled; e2e passes.

## Task 2: `Repository` reference — part A

Author the `## Repository` section for these method groups, signatures verbatim from `index.d.ts:907`–~`1200`, prose from each method's JSDoc:
- **Construction & static factories:** `new Repository(gitDir)`, `init`, `initBare`, `discover`, `open`/`openExt`, `clone`, `cloneAsync`, `cloneRecurse`.
- **File "last updated" dates** (the headline family): `getFileLatestModifiedDate`, `getFileLastModifiedDate`, `getFileLatestModified`, `getFilesLatestModified`, `getFilesLatestModifiedAsync`, `getFileCreatedDate` (+ any async twins present).
- **Status:** `statuses`, `statusFile`, `statusesAsync`.
- **Index & commit:** `index()`, `commit`, `commitAsync`, `blob`, `blobPath`, `findTree`, `findCommit` (and related object writers/lookups in range).
- **Blame:** `blameFile`, `blameLine`, `blameFileAsync`.
Use a consistent per-method shape: an `H3`/`H4` heading, a fenced `ts` signature block copied from `index.d.ts`, then the grounded description. Group with `H3` sub-headers per concern.
- **Verify:** every signature matches `index.d.ts`; build succeeds.

## Task 3: `Repository` reference — part B

Continue the `## Repository` section, signatures verbatim from `index.d.ts:~1200`–`1503`:
- **Branches / checkout / references:** `branches`, `branch`, `findBranch`, `checkoutTree`, `checkoutHead`, `checkoutIndex`, `setHead`, `setHeadDetached`, `reference`, `referenceSymbolic`, and other ref methods in range.
- **Remotes:** `remotes`, `findRemote`, `remote`, `remoteWithFetch`, `remoteAnonymous` (fetch/push happen on `Remote`).
- **Tags:** `tag`, `tagAnnotation`, `tagLightweight`, `findTag`, `findTagByPrefix`, `tagNames`, `tagForeach`, `tagDelete`.
- **Config & signature:** `config()`, `signature()`.
- **Revwalk & object lookup:** `revwalk`, `findObject`/`findBlob`/etc. present in range.
- **Disposal:** `dispose`/`free` + the `using` opt-in note (`index.d.ts:908`–`943`).
- **Verify:** every signature matches `index.d.ts`; the full `## Repository` section (Tasks 2+3) is coherent; build succeeds.

## Task 4: Git objects & handles — object model classes

Author the object-model portion of `## Git objects & handles` for: `Commit`, `Tree`, `TreeEntry`, `TreeIter`, `Blob`, `GitObject`, `Tag`, `Reference`, `Signature`, `Branch`. For each: an `H3` with the class one-liner (from its class JSDoc), then its public methods/accessors as fenced `ts` signatures + grounded prose. Note `TreeIter` `extends Iterator<…>`.
- **Verify:** signatures match `index.d.ts`; build succeeds.

## Task 5: Git objects & handles — operation & remote classes

Continue `## Git objects & handles` for: `Remote` (incl. `fetch`/`push`), `Index`, `Config`, `RevWalk` (`extends Iterator`), `Diff`, `DiffDelta`, `DiffFile`, `Deltas` (`extends Iterator`), `Cred`, `RepoBuilder`, and the options **classes** `FetchOptions`, `PushOptions`, `ProxyOptions`, `RemoteCallbacks`. Same per-class shape.
- **Verify:** signatures match `index.d.ts`; build succeeds.

## Task 6: Interfaces + Enums + Functions

- **`## Options & result types`** — the 13 interfaces (`index.d.ts:1747`–`2352`): `StatusOptions`, `CheckoutOptions`, `DiffOptions`, `BlameOptions`, `FileStatus`, `FileModification`, `BlameHunk`, `ConfigEntry`, `Progress`, `PushTransferProgress`, `PushUpdateReference`, `TagForeachItem`, `CredInfo`. Each as an `H3` with a fenced `ts` field list + per-field grounded notes (fields have inline JSDoc).
- **`## Enums`** — all 18 `const enum`s (`1730`–`2291`) with members. For **`GitErrorCode`**, render the members with the **meanings table from `README.md:690`–`720`** (verbatim). For flag enums (`DiffFlags`, `CredentialType`, `RemoteUpdateFlags`), note they are bitflags and mention the `*Contains` helpers.
- **`## Functions`** — `isGitError`, `credTypeContains`, `diffFlagsContains` (`index.d.ts` signatures + JSDoc).
- **Verify:** signatures/members match `index.d.ts`; the `GitErrorCode` table matches the README; build succeeds.

## Task 7: Error handling section + polish + verification

- **`## Error handling`** — a short narrative: `isGitError` guard, `GitErrorCode` for Git-layer errors, and the `AbortError`/`'Cancelled'` cancellation path (Constraint 4). Cross-link the `GitErrorCode` enum + `isGitError` function anchors.
- **Whole-page audit:** every fenced signature on `/docs/api` re-verified against `index.d.ts`; TOC + all in-page anchors resolve; the six sections are complete and consistently formatted; sidebar links correct.
- Confirm the Playwright smoke covers `/docs/api`; run `void:prepare` + `build` + e2e.
- **Verify:** no changes outside `website/` + `docs/superpowers/`.

## Done criteria (whole feature)

- `/docs/api` renders comprehensive coverage of the surface in the spec; every signature verified against `index.d.ts`; `GitErrorCode` table matches the README.
- Two-column sidebar on `/docs` and `/docs/api`; mobile disclosure nav works with JS off.
- `api.server.ts` head + canonical `/docs/api`; `prerender = true`; Getting Started links internally to `/docs/api`.
- `void:prepare` + `build` + Playwright e2e (incl. `/docs/api`) pass. No changes outside `website/` + `docs/superpowers/`.
