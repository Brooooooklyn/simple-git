# Design: preserve the 0.1.x file-date API + add a robust twin

Date: 2026-07-02
Branch: `feat/api-1.0-breaking-sweep` (PR #143)
Status: approved (user said "go")

## Problem

The 1.0 sweep retyped the two most-consumed file-date accessors:

| method | 0.1.x | 1.0 HEAD |
|---|---|---|
| `getFileLatestModifiedDate` | `number` (ms epoch), **throws** on missing | `Date \| null`, `null` on missing |
| `getFileCreatedDate` | `number` (ms epoch), **throws** on missing | `Date \| null`, `null` on missing |

`getFileLatestModifiedDate` is the dominant consumer entry point (doc-site
"Last updated" generators — Nextra/Docusaurus/Starlight/Fumadocs/Rspress).
Retyping `number → Date | null` silently breaks every such consumer, and the
`new Date(null) → 1970-01-01` foot-gun makes the break quiet rather than loud.

## Goal

Preserve the exact 0.1.x shape of the widely-used getters (no break), and offer
the improved null-safe/Date-typed behavior under a new name.

## Non-goals

- No change to the rich `getFileLatestModified[Async]` (FileModification) or bulk
  `getFilesLatestModified[Async]` — they already ship in 1.0 and are the richest option.
- No `Date | null` twin for created-date (YAGNI — the doc-site niche uses
  last-modified, not created; a `getFileCreationDate` can be added later if needed).
- No behavior change to the underlying history walk / helpers.

## API surface (after)

```
PRESERVE — revert to exact 0.1.x (number ms, THROWS on missing)
  getFileLatestModifiedDate(f): number
  getFileLatestModifiedDateAsync(f, signal?): Promise<number>
  getFileCreatedDate(f): number
  getFileCreatedDateAsync(f, signal?): Promise<number>

NEW robust — null-safe, Date-typed (today's 1.0 behavior, renamed)
  getFileLastModifiedDate(f): Date | null
  getFileLastModifiedDateAsync(f, signal?): Promise<Date | null>

UNCHANGED — richest option, already in 1.0
  getFileLatestModified[Async](f): FileModification | null
  getFilesLatestModified[Async](fs): Record<string, FileModification | null>
```

Behavior contract:
- `getFileLatestModifiedDate` / `getFileCreatedDate` (and `*Async`): return ms
  since epoch as a JS `number`; **throw** when no commit in history touched the
  path (0.1.x messages: `Failed to get commit for [f]` / `Failed to get created date for [f]`).
- `getFileLastModifiedDate` (and `*Async`): return a `Date`, or `null` when no
  commit touched the path — never throws for the missing case.
- `getFileLastModifiedDate(f)?.getTime() === getFileLatestModifiedDate(f)` for any committed file (same instant, different type).

## Implementation

All in `src/repo.rs`. The current `Date | null` impls become the new `…Last…`
methods; the `number`-throwing 0.1.x impls are restored under the original names.

**Sync methods:**
- `get_file_latest_modified_date` → `Result<i64>`:
  `get_file_modification(self.inner()?, &filepath).convert_without_message()
   .and_then(|v| v.map(|m| m.committer_time.timestamp_millis())
   .expect_not_null(format!("Failed to get commit for [{filepath}]")))`
- **new** `get_file_last_modified_date` → `Result<Option<DateTime<Utc>>>`: the
  current body — `…map(|v| v.map(|m| m.committer_time))`.
- `get_file_created_date` → `Result<i64>`:
  `get_file_created_date(self.inner()?, &filepath).convert_without_message()
   .and_then(|v| v.map(|d| d.timestamp_millis())
   .expect_not_null(format!("Failed to get created date for [{filepath}]")))`

**Async tasks** (structs + `Task` impls, same file):
- Rename `GitDateTask` → `GitLastModifiedDateTask`, keep `Output/JsValue = Option<DateTime<Utc>>`;
  it backs the new `get_file_last_modified_date_async` → `Promise<Date | null>`.
- **new** `GitLatestModifiedDateTask`, `Output/JsValue = i64`; `run()` = get_file_modification →
  `committer_time.timestamp_millis()` + `expect_not_null`; backs
  `get_file_latest_modified_date_async` → `Promise<number>`.
- `GitCreatedDateTask` → `Output/JsValue = i64`; `run()` = get_file_created_date →
  `timestamp_millis()` + `expect_not_null`; backs `get_file_created_date_async` → `Promise<number>`.
- The async `reject()` bodies (`coded_error(self.code, …)`) are unchanged; cancellation
  is handled by napi upstream and does not flow through `reject()` (verified earlier this session).

**Codegen:** run the real napi build (`yarn build:debug`) so `index.js` /
`index.d.ts` regenerate from the macros; commit the generated output verbatim
(never hand-edit — the loader/codegen is deterministic).

## Docs ripple

- `FileModification.committer_time` doc (and any method doc that says
  "Identical to `getFileLatestModifiedDate`") → point at `getFileLastModifiedDate`
  (the `Date` one; the old name is now `number`).
- `0.x-1.0-MIGRATION.md` §1: `getFileLatestModifiedDate` / `getFileCreatedDate`
  are no longer breaking (drop from the breaking table / mark "unchanged from 0.1.x");
  add `getFileLastModifiedDate[Async]` to "New in 1.0 (additive)"; move the
  `new Date(null) → 1970` hazard note onto `getFileLastModifiedDate`.
- `README`: point robustness-seekers to `getFileLastModifiedDate` / `getFileLatestModified`.

## Testing (`__test__/modification.spec.mjs`, `__test__/repo.spec.mjs`)

Existing cases assert `Date | null` for the two getters — retarget them:
- `getFileLatestModifiedDate` / `getFileCreatedDate`: `typeof === 'number'`;
  equals the committer/creation `Date.getTime()`; **throws** on a never-committed path.
- `getFileLastModifiedDate`: returns a `Date` for a committed file, `null`
  (no throw) for a never-committed path; `.getTime() === getFileLatestModifiedDate(f)`.
- Async variants mirror: `Promise<number>` throws on missing; `Promise<Date | null>` resolves null.

## Risks

- **Generated-file drift** — must run real napi codegen, not hand-edit `index.*`.
- **Existing tests** currently lock in the 1.0 `Date | null` shape; they must be
  updated in the same change or they will fail.
- **Naming confusability** — `getFileLatestModifiedDate` (number) vs
  `getFileLastModifiedDate` (Date|null) differ by one word; mitigated by distinct
  TS return types (`number` vs `Date | null`) and clear JSDoc. Accepted (user's chosen name).
```
