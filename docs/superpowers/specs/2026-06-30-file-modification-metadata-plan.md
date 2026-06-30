# Implementation Plan — `FileModification` (author + bulk metadata) for `@napi-rs/simple-git`

## Overview

Add enriched file-modification metadata to the napi-rs binding over git2 0.21. Today `getFileLatestModifiedDate(path)` returns only an `i64` committer timestamp. We add:

- A flat `#[napi(object)] FileModification` struct (owned `String`/`i64`, no lifetimes/`Reference`), built from the **raw** `git2::Commit`.
- A new module `src/file_modification.rs` holding the struct, builder, single-file walk, and bulk walk.
- Four new `#[napi] Repository` methods: `getFileLatestModification(+Async)`, `getFilesLatestModification(+Async)`.
- Behavior-preserving delegation: the legacy `getFileLatestModifiedDate(+Async)` is refactored to call the new walk and return `.timestamp` (byte-identical). `getFileCreatedDate` is **untouched**.

**Return shape (confirmed feasible):** single-file → `FileModification | null`; bulk → `Record<string, FileModification | null>` (Rust `HashMap<String, Option<FileModification>>`). Async tasks use `Option<FileModification>` / `HashMap<...>` as `Task::JsValue` directly.

### Resolved blocking issues (verified against source)

1. **git2 0.21 accessor return types** (VERIFIED at `git2-0.21.0/src/commit.rs:130,203,225` and `signature.rs:65,75,85`):
   - `Commit::summary() -> Result<Option<&str>, Error>` → use `.ok().flatten()`
   - `Signature::name()/email() -> Result<&str, Error>` → use `.ok()`
   - `Commit::author()/committer() -> Signature` (NOT Result, direct)
   - `Commit::time() -> Time`, `Signature::when() -> Time`; `Time::seconds() -> i64`
   Mirrors existing repo code (`src/commit.rs:140` uses `.ok().flatten()`; `src/signature.rs:72,80` use `.ok()`).
2. **Module imports** (VERIFIED `src/diff.rs:3-4`): `#[napi(object)]` needs BOTH `use napi::bindgen_prelude::*;` and `use napi_derive::napi;`. Both go at the top of `src/file_modification.rs`.
3. **Sort flag quirk** (VERIFIED `repo.rs:951`): keep `git2::Sort::TIME & git2::Sort::TOPOLOGICAL` verbatim in BOTH walks — the legacy single-file walk uses it and `repo.spec.mjs` passes against `git log -1`, so this exact (bitwise-AND) value is load-bearing for byte-identical parity. The bulk-walk comment must NOT claim "newest first"; label it "default revwalk order (matches legacy single-file walk)".

### Byte-identical legacy guarantee

Legacy value = `commit.time().seconds() * 1000` (`repo.rs:966,974`). `FileModification.timestamp` and `.committer_time` are BOTH set to `commit.time().seconds() * 1000` (NOT `committer.when()`), so delegation cannot drift.

### Definition of done

- All 6 new tests in `__test__/modification.spec.mjs` green.
- Existing `__test__/repo.spec.mjs` green (back-compat guard — do NOT edit it).
- `index.d.ts` shows `interface FileModification` + all 4 new methods with correct camelCase types.
- `cargo clippy --all-targets -- -D warnings` clean (lib.rs has `#![deny(clippy::all)]`).
- `yarn build` (release) + `yarn test` pass; `git status` shows only intended files changed.

### Build/test loop (CRITICAL)

Tests load `../index.js` → the native `*.node`. You MUST rebuild the addon before every ava run, or you test stale code:

```
cd /Users/brooklyn/workspace/github/simple-git && yarn build:debug && yarn ava __test__/modification.spec.mjs
```

`yarn build:debug` = `napi build --platform` (fast, regenerates `index.js`/`index.d.ts`/`*.node`). Use it for the TDD loop. Use `yarn build` (release) only in the final verification task.

### Ground-truth anchors (verified this session)

- `repo.rs:890` `get_file_latest_modified_date`; `:898` `_async`; `:913`/`:929` created-date methods (untouched). The `impl Repository` block closes at `:937`.
- `repo.rs:944` free fn `get_file_modified_date` (lines ~944–986); `:988+` `get_file_created_date` (untouched).
- `GitDateTask` struct `repo.rs:104-108`; `unsafe impl Send` `:110`; `Task` impl `:118-141`. `GitCreatedDateTask` `:112-116` / `:143-170` (untouched).
- `repo.rs` top imports `:1-17`; no `std::collections::HashMap` yet — must add.
- `lib.rs:3-17` modules (`pub mod ...;`).
- Repo opens via `new Repository(workDir)` where `workDir` = repo root (`repo.spec.mjs:12-16`).
- Fixture facts: `build.rs` author `LongYinan`, committer `GitHub` (author ≠ committer — catches field swaps). `LICENSE` has exactly 1 commit = the root commit (`parent_count()==0`). `src/lib.rs` is a nested forward-slash path.

### Dependency graph

```
Task1 (module: struct + build + BOTH walk fns + lib reg)
  └─> Task2 (refactor legacy delegation, guard repo.spec.mjs)
        ├─> Task3 (sync single + tests #1,#3,#root,#summary)
        │     └─> Task4 (async single + test #2)
        └─> Task5 (sync bulk + tests #4,#empty,#nested,#root)
              └─> Task6 (async bulk + test #5)
Task3..6 ─> Task7 (README) ─> Task8 (release build + clippy + full suite)
```

**IMPORTANT:** Land Task 1 as one edit (struct + builder + BOTH walk functions together). A partial commit with only the struct would trip `-D warnings` on the unused `HashMap`/`HashSet`/`Path`/`PathBuf` imports.

---

## Task 1 — New module: struct + builder + both walk functions + lib registration

**Goal:** Compile `src/file_modification.rs` with the `FileModification` object, `build_modification`, `get_file_modification`, and `get_files_modification`; register the module in `lib.rs`.

**Files:** create `src/file_modification.rs`; edit `src/lib.rs`.

**Changes — `src/lib.rs`:** add this line after `pub mod error;`/in the module list (alphabetical-ish, after `mod error;` line):

```rust
pub mod file_modification;
```

**Changes — create `src/file_modification.rs` with EXACTLY this content:**

```rust
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use napi::bindgen_prelude::*;
use napi_derive::napi;

/// Last commit that modified a file, with author/committer identity.
/// All times are ms since epoch (UTC; timezone offset ignored).
#[napi(object)]
pub struct FileModification {
  /// Committer time, ms since epoch. Identical to `getFileLatestModifiedDate`. Equals `committerTime`.
  pub timestamp: i64,
  /// 40-char lowercase hex OID of the last commit that modified the file.
  pub commit_id: String,
  /// Commit summary (first line). Undefined if absent or not valid UTF-8.
  pub summary: Option<String>,
  /// Author name. Undefined if not valid UTF-8.
  pub author_name: Option<String>,
  /// Author email. Undefined if not valid UTF-8.
  pub author_email: Option<String>,
  /// Author time, ms since epoch.
  pub author_time: i64,
  /// Committer name. Undefined if not valid UTF-8.
  pub committer_name: Option<String>,
  /// Committer email. Undefined if not valid UTF-8.
  pub committer_email: Option<String>,
  /// Committer time, ms since epoch. Equals `timestamp`.
  pub committer_time: i64,
}

pub(crate) fn build_modification(commit: &git2::Commit) -> FileModification {
  let author = commit.author();
  let committer = commit.committer();
  // Byte-identical to the legacy value (repo.rs get_file_modified_date): commit.time(), NOT committer.when().
  let committer_time = commit.time().seconds() * 1000;
  FileModification {
    timestamp: committer_time,
    commit_id: commit.id().to_string(),
    summary: commit.summary().ok().flatten().map(|s| s.to_owned()),
    author_name: author.name().ok().map(|s| s.to_owned()),
    author_email: author.email().ok().map(|s| s.to_owned()),
    author_time: author.when().seconds() * 1000,
    committer_name: committer.name().ok().map(|s| s.to_owned()),
    committer_email: committer.email().ok().map(|s| s.to_owned()),
    committer_time,
  }
}

/// Single-file walk. Mirrors the legacy repo.rs get_file_modified_date EXACTLY
/// (same revwalk, sort flag, pathspec, diff direction, merge-skip, root-commit
/// handling); only the returned value differs (struct instead of i64).
pub(crate) fn get_file_modification(
  repo: &git2::Repository,
  filepath: &str,
) -> std::result::Result<Option<FileModification>, git2::Error> {
  let mut diff_options = git2::DiffOptions::new();
  diff_options.disable_pathspec_match(false);
  diff_options.pathspec(filepath);
  let mut rev_walk = repo.revwalk()?;
  rev_walk.push_head()?;
  rev_walk.set_sorting(git2::Sort::TIME & git2::Sort::TOPOLOGICAL)?;
  let path = PathBuf::from(filepath);
  Ok(
    rev_walk
      .by_ref()
      .filter_map(|oid| oid.ok())
      .find_map(|oid| {
        let commit = repo.find_commit(oid).ok()?;
        match commit.parent_count() {
          // commit with parent
          1 => {
            let tree = commit.tree().ok()?;
            if let Ok(parent) = commit.parent(0) {
              let parent_tree = parent.tree().ok()?;
              if let Ok(diff) =
                repo.diff_tree_to_tree(Some(&tree), Some(&parent_tree), Some(&mut diff_options))
                && diff.deltas().len() > 0
              {
                return Some(build_modification(&commit));
              }
            }
          }
          // root commit
          0 => {
            let tree = commit.tree().ok()?;
            if tree.get_path(&path).is_ok() {
              return Some(build_modification(&commit));
            }
          }
          // ignore merge commits
          _ => {}
        };
        None
      }),
  )
}

/// Bulk walk: resolve the last commit that modified each of `filepaths` in a
/// SINGLE history walk. Every input path is a key; never-committed paths map to
/// `None`. Exact-string match against an `unresolved` set (NOT glob/pathspec
/// semantics); first (newest, since revwalk yields newest commits first under
/// the default order) hit wins; early-exit when `unresolved` empties.
pub(crate) fn get_files_modification(
  repo: &git2::Repository,
  filepaths: &[String],
) -> std::result::Result<HashMap<String, Option<FileModification>>, git2::Error> {
  let mut result: HashMap<String, Option<FileModification>> =
    filepaths.iter().map(|p| (p.clone(), None)).collect();
  let mut unresolved: HashSet<String> = filepaths.iter().cloned().collect();

  if unresolved.is_empty() {
    return Ok(result);
  }

  let mut diff_options = git2::DiffOptions::new();
  diff_options.disable_pathspec_match(false);
  for p in &unresolved {
    diff_options.pathspec(p);
  }

  let mut rev_walk = repo.revwalk()?;
  rev_walk.push_head()?;
  // default revwalk order (matches legacy single-file walk's sort flag)
  rev_walk.set_sorting(git2::Sort::TIME & git2::Sort::TOPOLOGICAL)?;

  for oid in rev_walk.by_ref().filter_map(|oid| oid.ok()) {
    if unresolved.is_empty() {
      break; // early-exit: nothing left to resolve
    }
    let commit = match repo.find_commit(oid) {
      Ok(c) => c,
      Err(_) => continue,
    };
    match commit.parent_count() {
      // commit with parent: diff (parent=old, commit=new) so added/modified
      // paths surface as new_file().path(); fall back to old_file() for deletes.
      1 => {
        let tree = match commit.tree() {
          Ok(t) => t,
          Err(_) => continue,
        };
        let parent = match commit.parent(0) {
          Ok(p) => p,
          Err(_) => continue,
        };
        let parent_tree = match parent.tree() {
          Ok(t) => t,
          Err(_) => continue,
        };
        if let Ok(diff) =
          repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), Some(&mut diff_options))
        {
          for delta in diff.deltas() {
            let path = delta
              .new_file()
              .path()
              .or_else(|| delta.old_file().path())
              .and_then(|p| p.to_str());
            if let Some(p) = path
              && unresolved.contains(p)
            {
              let key = p.to_owned();
              result.insert(key.clone(), Some(build_modification(&commit)));
              unresolved.remove(&key);
            }
          }
        }
      }
      // root commit: probe each still-unresolved path in the tree
      0 => {
        if let Ok(tree) = commit.tree() {
          for p in unresolved.clone() {
            if tree.get_path(Path::new(&p)).is_ok() {
              result.insert(p.clone(), Some(build_modification(&commit)));
              unresolved.remove(&p);
            }
          }
        }
      }
      // ignore merge commits
      _ => {}
    }
  }
  Ok(result)
}
```

**Test (write first):** none for this task — it is pure-Rust scaffolding proven by the build + downstream tests.

**Verify:**

```
cd /Users/brooklyn/workspace/github/simple-git && yarn build:debug && grep -n "FileModification" index.d.ts
```

Expect compile success and an `index.d.ts` `export interface FileModification { timestamp: number; commitId: string; summary?: string ... }` with camelCase fields. (The two walk fns are not yet wired to any `#[napi]` method, so clippy may warn they are unused — that is fine at this intermediate step; they get wired in Tasks 2/3/5 before the clippy gate in Task 8. If you want a clean intermediate, proceed straight to Task 2 which consumes `get_file_modification`.)

---

## Task 2 — Refactor legacy delegation (byte-identical back-compat)

**Goal:** Delete the old `get_file_modified_date` free fn; make `getFileLatestModifiedDate` and `GitDateTask` delegate to `get_file_modification(...).map(|m| m.timestamp)`. Prove zero behavior drift via the existing `repo.spec.mjs`.

**Depends on:** Task 1.

**Files:** edit `src/repo.rs`.

**Changes — `src/repo.rs`:**

1. Add imports near top (after `use crate::error::...;` line ~9). Add `HashMap` and the module symbols:

```rust
use std::collections::HashMap;
```

```rust
use crate::file_modification::{
  get_file_modification, get_files_modification, FileModification,
};
```

(`get_files_modification` is consumed in Task 5; importing it now is fine because the fn already exists from Task 1 — no unused-import warning since it is used by Task 5 before the clippy gate. If executing Task 2 in isolation, temporarily import only `{get_file_modification, FileModification}` and add `get_files_modification` in Task 5.)

2. **Delete** the entire free fn `get_file_modified_date` (the `fn get_file_modified_date(...) -> std::result::Result<Option<i64>, git2::Error> { ... }` block, ~`repo.rs:944-986`). Leave `get_file_created_date` intact.

3. Replace `get_file_latest_modified_date` (`repo.rs:890-894`) with:

```rust
  #[napi]
  pub fn get_file_latest_modified_date(&self, filepath: String) -> Result<i64> {
    get_file_modification(&self.inner, &filepath)
      .convert_without_message()
      .and_then(|value| {
        value
          .map(|m| m.timestamp)
          .expect_not_null(format!("Failed to get commit for [{filepath}]"))
      })
  }
```

4. Replace `GitDateTask::compute` body (`repo.rs:123-136`) with:

```rust
  fn compute(&mut self) -> napi::Result<Self::Output> {
    get_file_modification(
      &self
        .repo
        .read()
        .map_err(|err| napi::Error::new(Status::GenericFailure, format!("{err}")))?
        .inner,
      &self.filepath,
    )
    .convert_without_message()
    .and_then(|value| {
      value
        .map(|m| m.timestamp)
        .expect_not_null(format!("Failed to get commit for [{}]", &self.filepath))
    })
  }
```

`GitDateTask::Output`/`JsValue` stay `i64`; `resolve` unchanged. Do NOT touch `GitCreatedDateTask` or `get_file_created_date`.

**Test (write first):** No new test — the existing `__test__/repo.spec.mjs` "Date should be equal with cli" IS the regression guard (test #6). Confirm it is green BEFORE editing, and stays green after.

**Verify:**

```
cd /Users/brooklyn/workspace/github/simple-git && yarn build:debug && yarn ava __test__/repo.spec.mjs
```

Expect: all existing tests pass (timestamp byte-identical → drift = 0). Do NOT edit `repo.spec.mjs`.

---

## Task 3 — `getFileLatestModification` (sync) + tests #1, #3, root-commit, summary

**Goal:** Expose the enriched single-file method; assert real identity values against the git CLI (not just existence).

**Depends on:** Task 2.

**Files:** create `__test__/modification.spec.mjs`; edit `src/repo.rs`.

**Test (write first) — create `__test__/modification.spec.mjs`:**

```js
import { execSync } from "node:child_process";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import test from "ava";

import { Repository } from "../index.js";

const __dirname = join(fileURLToPath(import.meta.url), "..");
const workDir = join(__dirname, "..");

const git = (args) =>
  execSync(`git ${args}`, { cwd: workDir }).toString().trim();

test.beforeEach((t) => {
  t.context.repo = new Repository(workDir);
});

// Test #1 — enriched metadata, value-asserted against git CLI.
// build.rs author (LongYinan) != committer (GitHub): catches author/committer swaps.
test("getFileLatestModification returns enriched metadata", (t) => {
  const { repo } = t.context;
  const mod = repo.getFileLatestModification("build.rs");
  t.truthy(mod);

  // Byte-identical delegation guard (runs unconditionally; two native methods).
  t.is(mod.timestamp, repo.getFileLatestModifiedDate("build.rs"));
  t.is(mod.committerTime, mod.timestamp);
  t.regex(mod.commitId, /^[0-9a-f]{40}$/);

  // Value parity with git CLI; skip on CI where the checkout may be shallow/squashed.
  if (!process.env.CI) {
    t.is(mod.commitId, git("log -1 --format=%H -- build.rs"));
    t.is(mod.authorName, git("log -1 --format=%an -- build.rs"));
    t.is(mod.authorEmail, git("log -1 --format=%ae -- build.rs"));
    t.is(mod.committerName, git("log -1 --format=%cn -- build.rs"));
    t.is(mod.committerEmail, git("log -1 --format=%ce -- build.rs"));
    t.is(mod.summary, git("log -1 --format=%s -- build.rs"));
    // author != committer for this file
    t.not(mod.authorName, mod.committerName);
  } else {
    t.truthy(mod.authorName);
    t.truthy(mod.authorEmail);
    t.is(typeof mod.summary, "string");
  }
  t.is(typeof mod.authorTime, "number");
});

// Test #3 — null for a path that was never committed.
test("getFileLatestModification returns null for missing path", (t) => {
  const { repo } = t.context;
  t.is(repo.getFileLatestModification("does-not-exist-xyz.txt"), null);
});

// Root-commit branch (parent_count()==0): LICENSE's only commit is the root.
test("getFileLatestModification resolves a file whose only commit is the root", (t) => {
  const { repo } = t.context;
  const mod = repo.getFileLatestModification("LICENSE");
  t.truthy(mod);
  t.regex(mod.commitId, /^[0-9a-f]{40}$/);
  t.is(mod.timestamp, repo.getFileLatestModifiedDate("LICENSE"));
});
```

**Run (RED):**

```
cd /Users/brooklyn/workspace/github/simple-git && yarn build:debug && yarn ava __test__/modification.spec.mjs
```

Expect failure: `getFileLatestModification is not a function`.

**Changes — `src/repo.rs`:** add INSIDE the existing `#[napi] impl Repository { ... }` block, right after `get_file_latest_modified_date_async` (~`repo.rs:911`):

```rust
  #[napi]
  /// Last commit that modified `filepath`, with author/committer identity.
  /// Returns `null` when no commit in history touched the path.
  pub fn get_file_latest_modification(
    &self,
    filepath: String,
  ) -> Result<Option<FileModification>> {
    get_file_modification(&self.inner, &filepath).convert_without_message()
  }
```

**Run (GREEN) + Verify d.ts:**

```
cd /Users/brooklyn/workspace/github/simple-git && yarn build:debug && yarn ava __test__/modification.spec.mjs && grep -n "getFileLatestModification" index.d.ts
```

Expect `getFileLatestModification(filepath: string): FileModification | null`.

---

## Task 4 — `GitModificationTask` async + `getFileLatestModificationAsync` + test #2

**Goal:** Async single-file variant; proves `Option<FileModification>` works as `Task::JsValue`.

**Depends on:** Task 3.

**Files:** edit `__test__/modification.spec.mjs`; edit `src/repo.rs`.

**Test (write first) — append to `modification.spec.mjs`:**

```js
// Test #2 — async matches sync.
test("getFileLatestModificationAsync matches sync result", async (t) => {
  const { repo } = t.context;
  const sync = repo.getFileLatestModification("build.rs");
  const asyncResult = await repo.getFileLatestModificationAsync("build.rs");
  t.deepEqual(asyncResult, sync);
});
```

**Run (RED):**

```
cd /Users/brooklyn/workspace/github/simple-git && yarn build:debug && yarn ava __test__/modification.spec.mjs
```

**Changes — `src/repo.rs`:**

1. Add as FREE top-level items next to `GitDateTask` (after `unsafe impl Send for GitCreatedDateTask {}` ~`repo.rs:116`):

```rust
pub struct GitModificationTask {
  repo: RwLock<napi::bindgen_prelude::Reference<Repository>>,
  filepath: String,
}

unsafe impl Send for GitModificationTask {}
```

2. Add the `Task` impl as a FREE top-level item next to the other `Task` impls (after `GitCreatedDateTask`'s `Task` impl ~`repo.rs:170`):

```rust
#[napi]
impl Task for GitModificationTask {
  type Output = Option<FileModification>;
  type JsValue = Option<FileModification>;

  fn compute(&mut self) -> napi::Result<Self::Output> {
    get_file_modification(
      &self
        .repo
        .read()
        .map_err(|err| napi::Error::new(Status::GenericFailure, format!("{err}")))?
        .inner,
      &self.filepath,
    )
    .convert_without_message()
  }

  fn resolve(&mut self, _env: napi::Env, output: Self::Output) -> napi::Result<Self::JsValue> {
    Ok(output)
  }
}
```

3. Add the method INSIDE `#[napi] impl Repository`, after `get_file_latest_modification`:

```rust
  #[napi]
  pub fn get_file_latest_modification_async(
    &self,
    self_ref: Reference<Repository>,
    filepath: String,
    signal: Option<AbortSignal>,
  ) -> Result<AsyncTask<GitModificationTask>> {
    Ok(AsyncTask::with_optional_signal(
      GitModificationTask {
        repo: RwLock::new(self_ref),
        filepath,
      },
      signal,
    ))
  }
```

**Run (GREEN):**

```
cd /Users/brooklyn/workspace/github/simple-git && yarn build:debug && yarn ava __test__/modification.spec.mjs
```

---

## Task 5 — `getFilesLatestModification` (sync bulk) + tests #4, empty, nested, root

**Goal:** Expose one-pass bulk resolution → `Record<string, FileModification | null>`. Cross-validate against the single-file path (opposite diff directions), plus empty-input, nested forward-slash, and root-commit coverage.

**Depends on:** Task 2 (walk fn exists from Task 1) and Task 3 (single-file method, used as the parity oracle).

**Files:** edit `__test__/modification.spec.mjs`; edit `src/repo.rs`.

**Test (write first) — append to `modification.spec.mjs`:**

```js
// Test #4 — bulk resolves many paths in one pass; cross-validate vs single-file.
test("getFilesLatestModification resolves many paths in one pass", (t) => {
  const { repo } = t.context;
  const result = repo.getFilesLatestModification([
    "build.rs",
    "Cargo.toml",
    "bogus-zzz.txt",
  ]);
  t.deepEqual(
    Object.keys(result).sort(),
    ["Cargo.toml", "bogus-zzz.txt", "build.rs"],
  );
  t.deepEqual(result["build.rs"], repo.getFileLatestModification("build.rs"));
  t.deepEqual(result["Cargo.toml"], repo.getFileLatestModification("Cargo.toml"));
  t.is(result["bogus-zzz.txt"], null);
});

// Empty input -> {} (exercises the early-return branch + empty-Record serialization).
test("getFilesLatestModification returns {} for empty input", (t) => {
  const { repo } = t.context;
  t.deepEqual(repo.getFilesLatestModification([]), {});
});

// Nested forward-slash path: exact-string match against git's forward-slash delta path.
// Use a literal "src/lib.rs" (NOT path.join, which yields backslashes on Windows).
test("getFilesLatestModification matches a nested forward-slash path", (t) => {
  const { repo } = t.context;
  const result = repo.getFilesLatestModification(["src/lib.rs"]);
  t.deepEqual(result["src/lib.rs"], repo.getFileLatestModification("src/lib.rs"));
  t.truthy(result["src/lib.rs"]);
});

// Root-commit branch in the bulk walk, cross-validated vs single-file.
test("getFilesLatestModification resolves a root-only file (LICENSE)", (t) => {
  const { repo } = t.context;
  const result = repo.getFilesLatestModification(["LICENSE"]);
  t.deepEqual(result["LICENSE"], repo.getFileLatestModification("LICENSE"));
  t.truthy(result["LICENSE"]);
});
```

**Run (RED):**

```
cd /Users/brooklyn/workspace/github/simple-git && yarn build:debug && yarn ava __test__/modification.spec.mjs
```

**Changes — `src/repo.rs`:** ensure `get_files_modification` is imported (added in Task 2). Add the method INSIDE `#[napi] impl Repository`, after `get_file_latest_modification_async`:

```rust
  #[napi]
  /// Resolve the last commit that modified each of `filepaths` in a single
  /// history walk. Every input path is a key; never-committed paths map to `null`.
  pub fn get_files_latest_modification(
    &self,
    filepaths: Vec<String>,
  ) -> Result<HashMap<String, Option<FileModification>>> {
    get_files_modification(&self.inner, &filepaths).convert_without_message()
  }
```

**Run (GREEN) + Verify d.ts:**

```
cd /Users/brooklyn/workspace/github/simple-git && yarn build:debug && yarn ava __test__/modification.spec.mjs && grep -n "getFilesLatestModification" index.d.ts
```

Expect `getFilesLatestModification(filepaths: Array<string>): Record<string, FileModification | null>`.

> Debug note: if the `deepEqual(result['build.rs'], getFileLatestModification('build.rs'))` parity ever fails, the bulk delta-path matching (`new_file()/old_file()` fallback, diff direction `(parent_tree, tree)`) is the place to debug — the single-file path uses the opposite direction `(tree, parent_tree)` + `deltas().len()>0`. They must resolve to the same newest non-merge commit.

---

## Task 6 — `GitBulkModificationTask` async + `getFilesLatestModificationAsync` + test #5

**Goal:** Async bulk variant.

**Depends on:** Task 5.

**Files:** edit `__test__/modification.spec.mjs`; edit `src/repo.rs`.

**Test (write first) — append to `modification.spec.mjs`:**

```js
// Test #5 — async bulk matches sync bulk.
test("getFilesLatestModificationAsync matches sync bulk result", async (t) => {
  const { repo } = t.context;
  const paths = ["build.rs", "Cargo.toml", "bogus-zzz.txt"];
  const sync = repo.getFilesLatestModification(paths);
  const bulkAsync = await repo.getFilesLatestModificationAsync(paths);
  t.deepEqual(bulkAsync, sync);
});
```

**Run (RED):**

```
cd /Users/brooklyn/workspace/github/simple-git && yarn build:debug && yarn ava __test__/modification.spec.mjs
```

**Changes — `src/repo.rs`:**

1. FREE top-level struct + Send next to the other tasks:

```rust
pub struct GitBulkModificationTask {
  repo: RwLock<napi::bindgen_prelude::Reference<Repository>>,
  filepaths: Vec<String>,
}

unsafe impl Send for GitBulkModificationTask {}
```

2. FREE top-level `Task` impl:

```rust
#[napi]
impl Task for GitBulkModificationTask {
  type Output = HashMap<String, Option<FileModification>>;
  type JsValue = HashMap<String, Option<FileModification>>;

  fn compute(&mut self) -> napi::Result<Self::Output> {
    get_files_modification(
      &self
        .repo
        .read()
        .map_err(|err| napi::Error::new(Status::GenericFailure, format!("{err}")))?
        .inner,
      &self.filepaths,
    )
    .convert_without_message()
  }

  fn resolve(&mut self, _env: napi::Env, output: Self::Output) -> napi::Result<Self::JsValue> {
    Ok(output)
  }
}
```

3. Method INSIDE `#[napi] impl Repository`, after `get_files_latest_modification`:

```rust
  #[napi]
  pub fn get_files_latest_modification_async(
    &self,
    self_ref: Reference<Repository>,
    filepaths: Vec<String>,
    signal: Option<AbortSignal>,
  ) -> Result<AsyncTask<GitBulkModificationTask>> {
    Ok(AsyncTask::with_optional_signal(
      GitBulkModificationTask {
        repo: RwLock::new(self_ref),
        filepaths,
      },
      signal,
    ))
  }
```

**Run (GREEN):**

```
cd /Users/brooklyn/workspace/github/simple-git && yarn build:debug && yarn ava __test__/modification.spec.mjs
```

Expect all new tests green.

---

## Task 7 — README update

**Goal:** Document the new API. Copy signatures VERBATIM from the regenerated `index.d.ts` (the generator emits `Promise<unknown>` and `signal?: AbortSignal | undefined | null`, NOT hand-written `Promise<T>` — do not invent types).

**Depends on:** Tasks 3–6.

**Files:** edit `README.md`.

**Changes:**

1. In the Usage code block, add:

```ts
// enriched single-file result (null if the file has no history)
const mod = repo.getFileLatestModification('build.rs')
console.log(mod?.authorName, mod?.authorEmail, new Date(mod?.timestamp ?? 0))

// bulk: one history walk for many files
const many = repo.getFilesLatestModification(['build.rs', 'Cargo.toml'])
console.log(many['build.rs']?.committerName)
```

2. In the API block, add the `FileModification` interface and the four new method signatures to the `Repository` class. After the final `yarn build` (Task 8), run `grep -n "getFile\(s\)\?LatestModification\|interface FileModification" index.d.ts` and paste those exact lines so README matches generated output.

**Verify:**

```
cd /Users/brooklyn/workspace/github/simple-git && grep -n "getFilesLatestModification" README.md
```

---

## Task 8 — Final verification (release build + clippy + full suite)

**Goal:** Prove non-breaking + lint-clean against real release artifacts.

**Depends on:** all prior tasks.

**Files:** none (regenerates `index.js`, `index.d.ts`, `*.node`).

**Commands (run in order; all must pass):**

```
cd /Users/brooklyn/workspace/github/simple-git
cargo clippy --all-targets -- -D warnings
yarn build
yarn test
git status --short
grep -n "interface FileModification\|getFileLatestModification\|getFilesLatestModification" index.d.ts
```

**Expect:**
- clippy clean (no unused imports — all of `HashMap`/`HashSet`/`Path`/`PathBuf` in `file_modification.rs` are live once both walk fns exist; `HashMap` in `repo.rs` is live via the bulk method/task).
- `index.d.ts` contains `interface FileModification` + all 4 new methods.
- Full ava suite green: existing `repo.spec.mjs` (back-compat guard) + all new `modification.spec.mjs` tests.
- `git status` shows only: `src/file_modification.rs` (new), `src/lib.rs`, `src/repo.rs`, `__test__/modification.spec.mjs` (new), `README.md`, and regenerated `index.js`/`index.d.ts`/`*.node`. `__test__/repo.spec.mjs` MUST be unchanged.

If any pre-existing test newly fails, it was introduced in this session — debug it via systematic-debugging; do not dismiss as flaky.