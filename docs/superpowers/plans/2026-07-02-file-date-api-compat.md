# File-Date API Compat (preserve number + add getFileLastModifiedDate) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore the 0.1.x shape of `getFileLatestModifiedDate` / `getFileCreatedDate` (`number` ms, throws on missing) and add a null-safe `getFileLastModifiedDate` (`Date | null`) twin.

**Architecture:** In `src/repo.rs`, the current `Date | null` modified-date impl is renamed to `getFileLastModifiedDate` (+`GitDateTask` → `GitLastModifiedDateTask`); the original names are re-added returning `number` and throwing on missing (via inline `.ok_or_else`, since the 0.1.x `NotNullError` trait was removed at HEAD). `getFileCreatedDate` reverts to `number`. napi regenerates `index.js`/`index.d.ts`. Tests and docs follow.

**Tech Stack:** Rust (napi-rs 3.9.4 over git2 0.21), TypeScript `.d.ts` codegen via `napi build`, ava tests (`.mjs`).

## Global Constraints

- **Generated bindings are REAL codegen output.** Regenerate `index.js`/`index.d.ts` with `yarn build:debug` (`napi build --platform`); NEVER hand-edit them. Commit the generated output verbatim.
- **Throw-on-missing messages must match 0.1.x exactly:** `Failed to get commit for [{filepath}]` (modified) and `Failed to get created date for [{filepath}]` (created).
- **`getFileLastModifiedDate` throws ONLY on real errors** (unborn HEAD, corrupt object, out-of-range timestamp); a never-committed path returns `null`, never throws.
- **The two reverted getters throw on BOTH** real errors AND a never-committed path (0.1.x behavior).
- **Async mirrors sync:** the `number` async variants REJECT on missing; the `Date|null` async variant RESOLVES `null` on missing.
- **`crate::Result<T>` is `napi::Result<T>`;** `convert_without_message()` yields it. `Status` and `coded_error` are already imported in `src/repo.rs`.
- Execution runs on the current session model (Opus) per project policy — do not downgrade subagents.

---

### Task 1: Revert getters to `number` + add `getFileLastModifiedDate` twin (Rust + bindings + tests)

**Files:**
- Modify: `src/repo.rs` (structs ~130-146; task impls ~278-343; methods ~1959-2010 and ~2160-2190)
- Modify: `src/file_modification.rs:26` (doc comment)
- Modify (regenerated): `index.js`, `index.d.ts`
- Test: `__test__/repo.spec.mjs`, `__test__/modification.spec.mjs`

**Interfaces:**
- Produces (JS surface):
  - `getFileLatestModifiedDate(filepath: string): number` — throws on missing
  - `getFileLatestModifiedDateAsync(filepath, signal?): Promise<number>` — rejects on missing
  - `getFileLastModifiedDate(filepath: string): Date | null` — null on missing
  - `getFileLastModifiedDateAsync(filepath, signal?): Promise<Date | null>` — resolves null on missing
  - `getFileCreatedDate(filepath: string): number` — throws on missing
  - `getFileCreatedDateAsync(filepath, signal?): Promise<number>` — rejects on missing
  - Unchanged: `getFileLatestModified[Async]`, `getFilesLatestModified[Async]`

- [ ] **Step 1: Rename the modified-date async task struct**

In `src/repo.rs`, rename `struct GitDateTask` (~line 130) to `GitLastModifiedDateTask`. Fields are unchanged:

```rust
pub struct GitLastModifiedDateTask {
  path: String,
  open_flags: Option<u32>,
  namespace: Option<String>,
  workdir: Option<String>,
  filepath: String,
  code: GitErrorCode,
}
```

Rename its `impl GitDateTask { fn run … }` (~line 278) and `impl Task for GitDateTask` (~line 288) to `GitLastModifiedDateTask`. The `run()` body and `Task` impl (`Output`/`JsValue = Option<DateTime<Utc>>`) are otherwise unchanged.

- [ ] **Step 2: Add the new `number`-returning modified-date task**

Immediately after the `GitLastModifiedDateTask` `Task` impl, add:

```rust
impl GitLatestModifiedDateTask {
  fn run(&mut self) -> Result<i64> {
    let repo = reopen_worker_repo(&self.path, self.open_flags)?;
    restore_worker_handle_state(&repo, self.namespace.as_deref(), self.workdir.as_deref())?;
    get_file_modification(&repo, &self.filepath)
      .convert_without_message()?
      .map(|m| m.committer_time.timestamp_millis())
      .ok_or_else(|| {
        napi::Error::new(
          Status::GenericFailure,
          format!("Failed to get commit for [{}]", self.filepath),
        )
      })
  }
}

#[napi]
impl Task for GitLatestModifiedDateTask {
  type Output = i64;
  type JsValue = i64;

  fn compute(&mut self) -> napi::Result<Self::Output> {
    self.run().map_err(|mut e| {
      self.code = e.status;
      napi::Error::new(Status::GenericFailure, core::mem::take(&mut e.reason))
    })
  }

  fn resolve(&mut self, _env: napi::Env, output: Self::Output) -> napi::Result<Self::JsValue> {
    Ok(output)
  }

  fn reject(&mut self, env: napi::Env, mut err: Error) -> napi::Result<Self::JsValue> {
    Err(coded_error(env, self.code, core::mem::take(&mut err.reason)))
  }
}
```

Add the struct next to the others (~line 130):

```rust
pub struct GitLatestModifiedDateTask {
  path: String,
  open_flags: Option<u32>,
  namespace: Option<String>,
  workdir: Option<String>,
  filepath: String,
  code: GitErrorCode,
}
```

- [ ] **Step 3: Replace the modified-date sync + async methods**

Replace the current `get_file_latest_modified_date` sync method and its `_async` (~lines 1955-1986) with BOTH the reverted `number` methods and the new `…last…` methods:

```rust
  #[napi]
  /// Last-modified commit time of `filepath` in **milliseconds since the Unix
  /// epoch**. Throws when no commit in history touched the path. For a
  /// `null`-on-missing `Date`, use `getFileLastModifiedDate`.
  pub fn get_file_latest_modified_date(&self, filepath: String) -> Result<i64> {
    get_file_modification(self.inner()?, &filepath)
      .convert_without_message()?
      .map(|m| m.committer_time.timestamp_millis())
      .ok_or_else(|| {
        napi::Error::new(
          Status::GenericFailure,
          format!("Failed to get commit for [{filepath}]"),
        )
      })
  }

  #[napi]
  /// Asynchronous variant of `getFileLatestModifiedDate`, computed off the main
  /// thread. Rejects when no commit in history touched `filepath`.
  pub fn get_file_latest_modified_date_async(
    &self,
    filepath: String,
    signal: Option<AbortSignal>,
  ) -> Result<AsyncTask<GitLatestModifiedDateTask>> {
    let repo = self.inner()?;
    Ok(AsyncTask::with_optional_signal(
      GitLatestModifiedDateTask {
        path: repo.path().to_string_lossy().into_owned(),
        open_flags: self.open_flags,
        namespace: repo.namespace().ok().flatten().map(|s| s.to_owned()),
        workdir: repo.workdir().map(|p| p.to_string_lossy().into_owned()),
        filepath,
        code: GitErrorCode::GenericError,
      },
      signal,
    ))
  }

  #[napi]
  /// Last-modified commit time of `filepath` as a `Date`, or `null` when no
  /// commit in history touched the path (never throws for the missing case).
  /// Equals `FileModification.committerTime` from `getFileLatestModified`. Only
  /// real errors throw (unborn/empty HEAD, corrupt object, out-of-range
  /// timestamp). For milliseconds-since-epoch, use `getFileLatestModifiedDate`.
  pub fn get_file_last_modified_date(&self, filepath: String) -> Result<Option<DateTime<Utc>>> {
    get_file_modification(self.inner()?, &filepath)
      .convert_without_message()
      .map(|value| value.map(|m| m.committer_time))
  }

  #[napi]
  /// Asynchronous variant of `getFileLastModifiedDate`, computed off the main
  /// thread. Resolves to `null` when no commit in history touched `filepath`.
  pub fn get_file_last_modified_date_async(
    &self,
    filepath: String,
    signal: Option<AbortSignal>,
  ) -> Result<AsyncTask<GitLastModifiedDateTask>> {
    let repo = self.inner()?;
    Ok(AsyncTask::with_optional_signal(
      GitLastModifiedDateTask {
        path: repo.path().to_string_lossy().into_owned(),
        open_flags: self.open_flags,
        namespace: repo.namespace().ok().flatten().map(|s| s.to_owned()),
        workdir: repo.workdir().map(|p| p.to_string_lossy().into_owned()),
        filepath,
        code: GitErrorCode::GenericError,
      },
      signal,
    ))
  }
```

- [ ] **Step 4: Revert the created-date sync method + task to `number`**

Replace `get_file_created_date` sync (~line 2164) with:

```rust
  pub fn get_file_created_date(&self, filepath: String) -> Result<i64> {
    get_file_created_date(self.inner()?, &filepath)
      .convert_without_message()?
      .map(|d| d.timestamp_millis())
      .ok_or_else(|| {
        napi::Error::new(
          Status::GenericFailure,
          format!("Failed to get created date for [{filepath}]"),
        )
      })
  }
```

Change `impl GitCreatedDateTask { fn run … }` (~line 312) to return `Result<i64>`:

```rust
impl GitCreatedDateTask {
  fn run(&mut self) -> Result<i64> {
    let repo = reopen_worker_repo(&self.path, self.open_flags)?;
    restore_worker_handle_state(&repo, self.namespace.as_deref(), self.workdir.as_deref())?;
    get_file_created_date(&repo, &self.filepath)
      .convert_without_message()?
      .map(|d| d.timestamp_millis())
      .ok_or_else(|| {
        napi::Error::new(
          Status::GenericFailure,
          format!("Failed to get created date for [{}]", self.filepath),
        )
      })
  }
}
```

And change its `impl Task for GitCreatedDateTask` associated types from `Option<DateTime<Utc>>` to `i64`:

```rust
  type Output = i64;
  type JsValue = i64;
```

(The `compute`/`resolve`/`reject` bodies are unchanged.) The created-date sync doc comment above the method may keep its wording except drop any "resolves to null" phrasing; update it to say it throws on missing.

- [ ] **Step 5: Fix doc cross-references to point at the Date getter**

- `src/file_modification.rs:26` — change `/// Committer time, as a \`Date\`. Identical to \`getFileLatestModifiedDate\`.` to `... Identical to \`getFileLastModifiedDate\`.`
- `src/repo.rs` `get_file_latest_modified` doc (~line 1994) — change `\`committerTime\` equals \`getFileLatestModifiedDate\`.` to `\`committerTime\` equals \`getFileLastModifiedDate\`.`

- [ ] **Step 6: Build and verify the regenerated surface**

Run: `yarn build:debug`
Expected: compiles; `index.d.ts` regenerates. Verify:

```bash
grep -nE 'getFileLatestModifiedDate\(|getFileLastModifiedDate\(|getFileCreatedDate\(' index.d.ts
```

Expected lines:
```
getFileLatestModifiedDate(filepath: string): number
getFileLatestModifiedDateAsync(filepath: string, signal?: ...): Promise<number>
getFileLastModifiedDate(filepath: string): Date | null
getFileLastModifiedDateAsync(filepath: string, signal?: ...): Promise<Date | null>
getFileCreatedDate(filepath: string): number
getFileCreatedDateAsync(filepath: string, signal?: ...): Promise<number>
```

- [ ] **Step 7: Update existing `repo.spec.mjs` assertions to the reverted contract**

In `__test__/repo.spec.mjs`, apply these edits (open each site):

- Test "Date should be equal with cli" (~26,35): `t.true(actual instanceof Date)` → `t.is(typeof actual, "number")`; `actual.getTime()` → `actual`.
- Test "Created date should be equal with cli" (~46,56): `t.true(actual instanceof Date)` → `t.is(typeof actual, "number")`; `actual.getTime()` → `actual`.
- Test "Created date async should work" (~76,77): `t.true(actualDate instanceof Date)` → `t.is(typeof actualDate, "number")`; `actualDate.getTime()` → `actualDate`.
- Test "Created date returns null for non-existent file" (~83-86): replace body with `t.throws(() => repo.getFileCreatedDate("non-existent-file.txt"));` and rename to `"Created date THROWS for non-existent file"`.
- Test "Created date async returns null for non-existent file" (~88-91): replace with `await t.throwsAsync(() => repo.getFileCreatedDateAsync("non-existent-file.txt"));` and rename to `"... THROWS ..."`.
- Test "Latest modified date returns null for non-existent file" (~93-96): replace with `t.throws(() => repo.getFileLatestModifiedDate("does-not-exist-xyz.txt"));` and rename to `"... THROWS ..."`.
- Test "Latest modified date async returns null for non-existent file" (~98-101): replace with `await t.throwsAsync(() => repo.getFileLatestModifiedDateAsync("does-not-exist-xyz.txt"));` and rename to `"... THROWS ..."`.
- Unborn-HEAD test (~117-119): after the existing three `throws`, add `t.throws(() => repo.getFileLastModifiedDate("anything.txt"));` and `await t.throwsAsync(() => repo.getFileLatestModifiedDateAsync("anything.txt"));` (unborn HEAD is a real error → the Date getter throws too, and the number async rejects).
- Corrupt-object test (~171-178): after the existing asserts add `t.throws(() => repo.getFileLastModifiedDate("a.txt"));` and `await t.throwsAsync(() => repo.getFileLastModifiedDateAsync("a.txt"));`.
- Root-only test (~202-213):
  - `t.true(repo.getFileCreatedDate("only.txt") instanceof Date)` → `t.is(typeof repo.getFileCreatedDate("only.txt"), "number")`
  - `t.true(repo.getFileLatestModifiedDate("only.txt") instanceof Date)` → `t.is(typeof repo.getFileLatestModifiedDate("only.txt"), "number")`
  - `t.true((await repo.getFileCreatedDateAsync("only.txt")) instanceof Date)` → `t.is(typeof (await repo.getFileCreatedDateAsync("only.txt")), "number")`
  - Add `t.true(repo.getFileLastModifiedDate("only.txt") instanceof Date);`
  - `t.is(repo.getFileCreatedDate("missing.txt"), null)` → `t.throws(() => repo.getFileCreatedDate("missing.txt"))`
  - `t.is(repo.getFileLatestModifiedDate("missing.txt"), null)` → `t.throws(() => repo.getFileLatestModifiedDate("missing.txt"))`
  - Keep `t.is(repo.getFileLatestModified("missing.txt"), null);` (unchanged).
  - Add `t.is(repo.getFileLastModifiedDate("missing.txt"), null);`
  - `t.is(await repo.getFileCreatedDateAsync("missing.txt"), null)` → `await t.throwsAsync(() => repo.getFileCreatedDateAsync("missing.txt"))`

- [ ] **Step 8: Update `modification.spec.mjs` + add the new `getFileLastModifiedDate` test**

In `__test__/modification.spec.mjs`:

- Test "getFileLatestModified returns enriched metadata" (~52-54): `repo.getFileLatestModifiedDate("build.rs").getTime()` → `repo.getFileLatestModifiedDate("build.rs")` (it is now `number` ms, compared to `mod.committerTime.getTime()`).
- Test "getFileLatestModifiedDateAsync matches sync result" (~92-97): change to the `number` contract:

```js
test("getFileLatestModifiedDateAsync matches sync result", async (t) => {
  const { repo } = t.context;
  const sync = repo.getFileLatestModifiedDate("build.rs");
  const asyncResult = await repo.getFileLatestModifiedDateAsync("build.rs");
  t.is(typeof asyncResult, "number");
  t.is(asyncResult, sync);
});
```

- Root-only-file test (~114): `repo.getFileLatestModifiedDate("LICENSE").getTime()` → `repo.getFileLatestModifiedDate("LICENSE")`.
- Add a new test block for the Date twin:

```js
// getFileLastModifiedDate — the robust Date|null twin. Same instant as the
// number getter; null (not throw) for a never-committed path; async mirrors.
test("getFileLastModifiedDate returns a Date and mirrors the number getter", async (t) => {
  const { repo } = t.context;
  const date = repo.getFileLastModifiedDate("build.rs");
  t.true(date instanceof Date);
  t.is(date.getTime(), repo.getFileLatestModifiedDate("build.rs"));

  const asyncDate = await repo.getFileLastModifiedDateAsync("build.rs");
  t.true(asyncDate instanceof Date);
  t.is(asyncDate.getTime(), date.getTime());
});

test("getFileLastModifiedDate returns null (no throw) for a missing path", async (t) => {
  const { repo } = t.context;
  t.is(repo.getFileLastModifiedDate("does-not-exist-xyz.nope"), null);
  t.is(await repo.getFileLastModifiedDateAsync("does-not-exist-xyz.nope"), null);
});
```

- [ ] **Step 9: Catch any straggler call sites**

Run: `grep -rnE 'getFileLatestModifiedDate|getFileCreatedDate' __test__ examples benchmark README.md 2>/dev/null | grep -iE 'getTime|instanceof Date'`
For each hit outside `getFileLastModifiedDate`, convert the `Date` assumption to `number` (README is handled in Task 2 — leave it).

- [ ] **Step 10: Run the suite**

Run: `yarn test`
Expected: PASS (all file-date tests green).

- [ ] **Step 11: Commit**

```bash
git add src/repo.rs src/file_modification.rs index.js index.d.ts __test__/repo.spec.mjs __test__/modification.spec.mjs
git commit -m "feat!: preserve 0.1.x getFileLatestModifiedDate/getFileCreatedDate (number) + add getFileLastModifiedDate (Date|null) twin"
```

---

### Task 2: Update markdown docs (README + migration guide)

**Files:**
- Modify: `README.md`
- Modify: `0.x-1.0-MIGRATION.md`

**Interfaces:**
- Consumes: the final JS surface from Task 1 (the six method signatures).

- [ ] **Step 1: README — document the split**

In `README.md`, wherever `getFileLatestModifiedDate` / `getFileCreatedDate` are described, state they return `number` (ms since epoch) and throw when no commit touched the path; add `getFileLastModifiedDate` as the null-safe `Date | null` alternative, and point richness-seekers to `getFileLatestModified`. Verify current mentions first:

Run: `grep -n 'getFileLatestModifiedDate\|getFileCreatedDate\|getFileLastModifiedDate\|getFileLatestModified' README.md`

- [ ] **Step 2: Migration guide §1 — reclassify**

In `0.x-1.0-MIGRATION.md` §1 (file-date accessors):
- Remove `getFileLatestModifiedDate` and `getFileCreatedDate` from the breaking-changes table (they are unchanged from 0.1.x: `number`, throws on missing). Add a one-line note that they are preserved for backward compatibility.
- Add `getFileLastModifiedDate` / `getFileLastModifiedDateAsync` (`Date | null`) to the "New in 1.0 (additive)" section.
- Move the `new Date(null) → 1970-01-01` hazard note onto `getFileLastModifiedDate` (the only file-date accessor that can now return `null`).

- [ ] **Step 3: Verify no stale claims remain**

Run: `grep -nE 'getFileLatestModifiedDate|getFileCreatedDate' 0.x-1.0-MIGRATION.md README.md`
Confirm every remaining mention states `number`/throws (legacy) and that `Date | null` is attributed only to `getFileLastModifiedDate`.

- [ ] **Step 4: Commit**

```bash
git add README.md 0.x-1.0-MIGRATION.md
git commit -m "docs: document file-date number getters + getFileLastModifiedDate twin (0.1.x compat)"
```

---

## Self-Review

**Spec coverage:**
- Preserve `getFileLatestModifiedDate`/`getFileCreatedDate` as `number`+throws → Task 1 Steps 3-4. ✓
- New `getFileLastModifiedDate[Async]` `Date|null` → Task 1 Steps 1,3. ✓
- Async mirrors (number rejects / Date resolves null) → Steps 2,3, tests 7-8. ✓
- Regenerate bindings → Step 6. ✓
- Doc cross-refs (`committerTime` "Identical to…") → Step 5. ✓
- Migration §1 + README ripple → Task 2. ✓
- Tests updated + added → Steps 7-8. ✓

**Placeholder scan:** none — all code blocks are concrete; existing-test edits reference exact sites with exact old→new assertions.

**Type consistency:** `GitLastModifiedDateTask` (Option<DateTime>), `GitLatestModifiedDateTask` (i64), `GitCreatedDateTask` (i64) used consistently across struct/impl/method; JS surface names match between Task 1 Interfaces and the tests.
