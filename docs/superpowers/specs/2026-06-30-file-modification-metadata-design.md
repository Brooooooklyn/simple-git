# File modification metadata (author + bulk) — Design

Date: 2026-06-30
Status: Approved (Approach A)
Package: `@napi-rs/simple-git` (napi-rs binding over `git2` 0.21 / libgit2)

## Problem

Consumers (Nextra, Docusaurus, Astro Starlight, Fumadocs, Rspress) call
`getFileLatestModifiedDate(Async)` per page to render "Last updated on" lines.
Two documented, still-unmet needs:

1. **Author identity.** The method returns a bare `number`. Sites want "last updated
   *and by whom*". (Docusaurus#2798; e18e/ecosystem-issues#216; confirmed by
   Sebastien Lorber / Docusaurus on X: *"its current API doesn't permit to retrieve
   the last update author, although it could be easy to add"*.)
2. **Bulk, fast.** One independent `HEAD`→root revwalk per file → ~24s builds on an
   11k-doc site (Docusaurus#11208). They want "read the Git history for many files,
   as fast as possible ... time/author for each file" in a single pass.

The enrichment site already holds the answer: `get_file_modified_date`
(`src/repo.rs:939-983`) holds the live `git2::Commit` at line 966 and discards
everything except `commit.time().seconds() * 1000`.

## Goals

- Add an **enriched single-file** result carrying author + committer identity + commit id + summary, alongside the existing modified timestamp.
- Add a **bulk** method that resolves an explicit list of paths in **one** history walk, with early-exit once every requested path is resolved.
- Sync + async (AbortSignal) variants for both.
- **Non-breaking**: existing `getFileLatestModifiedDate(Async)` / `getFileCreatedDate(Async)` keep their exact signatures and return values.

## Non-goals (this spec)

- `git log --follow` rename detection (pre-existing TODO at `src/repo.rs:989`; the existing modified-date method already ignores renames — we match that).
- "Whole tree at HEAD" bulk mode (explicit path list only).
- Fixing `getFileCreatedDate`'s full-history scan (rank 5 — separate follow-up).
- Per-file commit log / contributors list (separate follow-up).

## Approach A (chosen)

A flat `#[napi(object)]` data struct of owned `String`/`i64` values, built at the
existing enrichment site. It is `Send`, so it drops into the existing `Task` async
pattern with only a type swap, and needs **no** `SharedReference`/lifetime plumbing
(unlike returning a `Commit`/`Signature` object). Walk logic lives in one internal
function; the legacy `i64` method delegates to it, so there is **zero behavior drift**.

## API

```ts
/** Last commit that modified a file, with identity. All times are ms since epoch (UTC, offset ignored). */
export interface FileModification {
  /** Committer time, ms since epoch. Identical to getFileLatestModifiedDate's value. Equals committerTime. */
  timestamp: number
  /** 40-char hex OID of the last commit that modified the file. */
  commitId: string
  /** Commit summary (first line). Undefined if not valid UTF-8. */
  summary?: string
  /** Author name. Undefined if not valid UTF-8. */
  authorName?: string
  /** Author email. Undefined if not valid UTF-8. */
  authorEmail?: string
  /** Author time, ms since epoch. */
  authorTime: number
  /** Committer name. Undefined if not valid UTF-8. */
  committerName?: string
  /** Committer email. Undefined if not valid UTF-8. */
  committerEmail?: string
  /** Committer time, ms since epoch. Equals timestamp. */
  committerTime: number
}

export class Repository {
  // unchanged (kept byte-for-byte):
  getFileLatestModifiedDate(filepath: string): number
  getFileLatestModifiedDateAsync(filepath: string, signal?: AbortSignal): Promise<number>
  getFileCreatedDate(filepath: string): number
  getFileCreatedDateAsync(filepath: string, signal?: AbortSignal): Promise<number>

  // new — enriched single file (null when the path has no history, instead of throwing):
  getFileLatestModification(filepath: string): FileModification | null
  getFileLatestModificationAsync(filepath: string, signal?: AbortSignal): Promise<FileModification | null>

  // new — bulk, one walk, early-exit:
  getFilesLatestModification(filepaths: Array<string>): Record<string, FileModification | null>
  getFilesLatestModificationAsync(filepaths: Array<string>, signal?: AbortSignal): Promise<Record<string, FileModification | null>>
}
```

Notes:
- `timestamp` is retained as the headline field so `.timestamp` is a drop-in for the
  legacy method. It equals `committerTime`; the mild, documented redundancy keeps
  author/committer symmetric.
- Single-file new method returns `null` (not a throw) when no commit touched the path.
  The legacy method's throw-on-missing behavior is preserved separately.
- Bulk: every input path appears as a key; never-committed paths map to `null`.
- Paths are **repo-root-relative, forward-slash** (same constraint as the existing
  pathspec-based method).

## Algorithms

### `build_modification(&git2::Commit) -> FileModification`
Reads `commit.id()`, `commit.author()`, `commit.committer()`, `commit.summary()`,
`commit.time()`. Extracts owned `String`/`i64` (times = `Signature::when().seconds() * 1000`).

### `get_file_modification(repo, filepath) -> Option<FileModification>`
Refactor of the current `get_file_modified_date`: same revwalk (`push_head`,
`Sort::TIME & TOPOLOGICAL`), same pathspec diff, same merge-commit skip and root-commit
handling. Only change: at the find site, return `Some(build_modification(&commit))`
instead of `Some(commit.time().seconds() * 1000)`.

### `get_files_modification(repo, &[String]) -> HashMap<String, Option<FileModification>>`
```
result = { path: None for path in input }
unresolved = set(input)            // normalized
diff_opts.pathspec = each input path; disable_pathspec_match(false)
revwalk from HEAD, Sort::TIME & TOPOLOGICAL          // newest first
for oid in revwalk:
    if unresolved empty: break                       // early-exit
    commit = find_commit(oid)
    match commit.parent_count():
      1 => diff(commit.tree vs parent.tree, diff_opts)
           for delta in diff.deltas():
               p = delta.new_file().path() (fallback old_file().path())
               if p in unresolved:
                   result[p] = Some(build_modification(&commit)); unresolved.remove(p)
      0 => for p in unresolved.clone():               // root commit
               if commit.tree().get_path(p).is_ok():
                   result[p] = Some(build_modification(&commit)); unresolved.remove(p)
      _ => skip                                        // merge
return result
```
Exact-string match against `unresolved` (not glob/prefix). Newest commit wins (= last modified).

### Legacy delegation
`getFileLatestModifiedDate` and `GitDateTask::compute` call `get_file_modification(...)`,
then `.map(|m| m.timestamp)` + existing `expect_not_null`. Same value, same throw.

## Async

Two new `Task` impls mirroring `GitDateTask` (`RwLock<Reference<Repository>>` + `unsafe impl Send`):
- `GitModificationTask { repo, filepath }` → `Output = JsValue = Option<FileModification>`.
- `GitBulkModificationTask { repo, filepaths: Vec<String> }` → `Output = JsValue = HashMap<String, Option<FileModification>>`.

`FileModification` is plain owned data → `Send`. `#[napi(object)]` derives the napi
conversions, so it works as both `Task::Output` and `Task::JsValue`; `Option<_>` →
`T | null`; `HashMap<String, _>` → JS `Record`.

## Module layout

New file `src/file_modification.rs`: the `FileModification` struct, `build_modification`,
`get_file_modification`, `get_files_modification`. `src/lib.rs` gains `mod file_modification;`.
`src/repo.rs` keeps the `#[napi]` Repository methods + the two `Task` structs (they
reference `Repository`) and imports from the new module. (repo.rs is already ~1010 lines;
isolating the walk logic improves boundaries.)

## Build / generated artifacts

`yarn build` (`napi build --platform --release`) regenerates `index.d.ts` + `index.js`
and the `*.node` binary. The `FileModification` interface and new methods appear
automatically. Tests run against the rebuilt addon, so each TDD cycle rebuilds.

## Testing (ava, TDD)

New `__test__/modification.spec.mjs` (existing `repo.spec.mjs` untouched):
1. `getFileLatestModification('build.rs')`: non-null; `.timestamp === getFileLatestModifiedDate('build.rs')`; `.commitId` matches `/^[0-9a-f]{40}$/`; `authorName`/`authorEmail` non-empty; `committerTime === timestamp`.
2. async parity: `getFileLatestModificationAsync('build.rs')` deep-equals the sync result.
3. missing path → `null`.
4. bulk: `getFilesLatestModification(['build.rs','Cargo.toml'])` — both keys present, each deep-equals that path's single-file result; a bogus path key → `null`.
5. bulk async parity.
6. back-compat: existing `repo.spec.mjs` still green (no edits to it).

## Risk register

- **napi `HashMap<String, Option<#[napi(object)]>>` return** — expected supported; confirm with the first GREEN of test #4.
- **`#[napi(object)]` as `Task::JsValue`** — `#[napi(object)]` derives `ToNapiValue`; confirm with test #2/#5 GREEN.
- **pathspec exact-match in bulk** — guarded by explicit `unresolved.contains(path)` check, not pathspec semantics.
- **Time unit consistency** — all new times ms (×1000), offset ignored, matching the existing method.
```
