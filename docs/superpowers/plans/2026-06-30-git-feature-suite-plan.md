# Plan: Git feature suite for @napi-rs/simple-git

Add the six most-impactful missing capabilities to this napi-rs/git2 binding, ranked by a prior multi-agent gap analysis: working-tree **status**, repository **config + signature()**, remote **push**, **index/staging + commit-from-workdir**, **blame**, and **branch + checkout**. Each is an independent task; a final task documents them in the README.

## Context

- This is a napi-rs (v3) binding over `git2` 0.21 (libgit2). Rust lives in `src/*.rs`; the generated public API is `index.d.ts` + `index.js` (both tracked). Tests are AVA `.mjs` files under `__test__/` that import from `../index.js`.
- The native class is `Repository` (`src/repo.rs`), plus object/type modules (`blob.rs`, `commit.rs`, `tree.rs`, `reference.rs`, `remote.rs`, `signature.rs`, `tag.rs`, `diff.rs`, `deltas.rs`, `object.rs`, `file_modification.rs`). New modules are declared in `src/lib.rs`.
- Error mapping: `src/error.rs` provides `IntoNapiError` (`.convert_without_message()` / `.convert(msg)`) on `Result<T, git2::Error>`, and `NotNullError` (`.expect_not_null(msg)`) on `Option<T>`. **Use these — never `.unwrap()`/panic.**
- Async + `AbortSignal` pattern: see `GitModificationTask` in `src/repo.rs` (a `struct` holding `repo: RwLock<Reference<Repository>>` + args, `unsafe impl Send`, `#[napi] impl Task` with `compute`/`resolve`, and a `#[napi] pub fn ..._async(&self, self_ref: Reference<Repository>, ..., signal: Option<AbortSignal>) -> Result<AsyncTask<TheTask>>` factory using `AsyncTask::with_optional_signal`).
- `#[napi(object)]` structs are plain value objects (like `FileModification`) — type-only, **no `index.js` runtime export**. `#[napi] pub struct` are classes — they **do** need an `index.js` export line.
- Owned-vs-borrowed: `git2::Config`, `git2::Index`, `git2::PushOptions` are OWNED (no lifetime) → wrap as a plain `#[napi] struct Foo { inner: git2::Foo }`. `git2::Branch` borrows the repo → wrap with the `SharedReference<Repository, git2::X<'static>>` + `share_with` pattern used by `reference::Reference`/`Remote` (study `src/reference.rs` and `repo.rs::head`).

## Global Constraints (apply to EVERY task)

1. **TDD.** Write the AVA test(s) first, watch them fail, then implement. Tests import from `../index.js`.
2. **Read-only tests** (status, config, blame) may run against the project repo itself (`new Repository(workDir)`), like existing specs. **Mutating tests** (index/staging, branch-create, checkout, setHead) **MUST** operate on a throwaway repo created under `os.tmpdir()` (init + a couple of commits via the library or `execSync('git ...')`); they must **never** mutate the project's own repo/index/HEAD. Clean up temp dirs in the test.
3. **Build + verify loop, run in this exact order before committing:**
   - `cargo fmt` (rustfmt.toml: 2-space tabs; CI runs `cargo fmt -- --check`).
   - `cargo clippy --all-targets` (crate has `#![deny(clippy::all)]` — zero warnings allowed).
   - `yarn build:debug` (compiles Rust + regenerates `index.js`/`index.d.ts`; ~2s incremental).
   - `yarn test` (AVA — all green, including pre-existing 16).
4. **Codegen reconciliation (do AFTER `yarn build:debug`, the build pollutes the tracked files with environmental loader drift that must NOT be committed):**
   - `index.d.ts`: keep your intended new type blocks. The one known pre-existing drift line is `tagForeach` — if the build rewrote it to `tagForeach(cb: (arg: [string, Buffer]) => boolean): void`, restore it to `tagForeach(cb: (arg0: string, arg1: Buffer) => boolean): void`. Review `git diff index.d.ts` and confirm every remaining change is something you intentionally added.
   - `index.js`: `git checkout -- index.js`, then **only if you added a new `#[napi] pub struct` class or a new `#[napi] enum`**, hand-add its export line(s) `module.exports.<Name> = nativeBinding.<Name>` in the exports block at the bottom, next to the alphabetical neighbors. **New methods on existing classes (e.g. `Repository`) and `#[napi(object)]` value structs need NO `index.js` change.**
   - Never stage `*.node` (gitignored).
5. **Commit** the Rust source, the test file(s), and the reconciled `index.d.ts`/`index.js` together. One commit per task is fine. Conventional-commit message (`feat: ...`).
6. Match existing code style: doc comments (`///`) precede the `#[napi]` attribute; mirror the wording/structure of neighboring methods.

---

## Task 1: Working-tree status — `Repository.statuses` / `statusFile` / `statusesAsync`

**New file `src/status.rs`.** Declare `pub mod status;` in `src/lib.rs`.

Define two `#[napi(object)]` value structs:

```rust
#[napi(object)]
pub struct StatusOptions {
  pub include_untracked: Option<bool>,      // default true
  pub include_ignored: Option<bool>,        // default false
  pub include_unmodified: Option<bool>,     // default false
  pub exclude_submodules: Option<bool>,     // default false
  pub recurse_untracked_dirs: Option<bool>, // default false
  pub renames_head_to_index: Option<bool>,  // default false
  pub renames_index_to_workdir: Option<bool>, // default false
  pub pathspec: Option<Vec<String>>,
}

#[napi(object)]
pub struct FileStatus {
  pub path: Option<String>, // workdir-relative; None if not valid UTF-8
  pub bits: u32,            // raw git2::Status bits — forward-compat escape hatch
  pub is_index_new: bool,
  pub is_index_modified: bool,
  pub is_index_deleted: bool,
  pub is_index_renamed: bool,
  pub is_index_typechange: bool,
  pub is_wt_new: bool,      // untracked
  pub is_wt_modified: bool,
  pub is_wt_deleted: bool,
  pub is_wt_typechange: bool,
  pub is_wt_renamed: bool,
  pub is_ignored: bool,
  pub is_conflicted: bool,
}
```

Helpers in `status.rs`:
- `pub(crate) fn build_status_opts(opts: Option<StatusOptions>) -> git2::StatusOptions` — map each field onto the `git2::StatusOptions` builder. **Defaults must match the git CLI**: `include_untracked` defaults to **true** when the field is `None`; `recurse_untracked_dirs` defaults to false; rename flags opt-in; add each `pathspec` via `.pathspec(p)`.
- `pub(crate) fn status_from_bits(status: git2::Status, path: Option<String>) -> FileStatus` — decode each `git2::Status` flag (`INDEX_NEW`, `INDEX_MODIFIED`, `INDEX_DELETED`, `INDEX_RENAMED`, `INDEX_TYPECHANGE`, `WT_NEW`, `WT_MODIFIED`, `WT_DELETED`, `WT_TYPECHANGE`, `WT_RENAMED`, `IGNORED`, `CONFLICTED`) into the bools, store `status.bits()` raw.

Methods on `impl Repository` in `src/repo.rs` (place near the file-modification methods):
- `statuses(&self, options: Option<StatusOptions>) -> Result<Vec<FileStatus>>` — build opts, call `self.inner.statuses(Some(&mut opts))`, iterate `StatusEntry`s, eager-collect into `Vec<FileStatus>` **inside this fn** (the `Statuses<'repo>` borrows the repo — must not escape). Decode path via `entry.path()` → `Option<String>` (lossy/None on non-UTF-8, mirroring `file_modification.rs`). Map errors with `.convert_without_message()` so bare repos surface as a napi error, not a panic.
- `status_file(&self, path: String) -> Result<FileStatus>` — `self.inner.status_file(Path::new(&path))` returns a `git2::Status`; wrap with `status_from_bits(status, Some(path))`.
- `statuses_async(&self, self_ref: Reference<Repository>, options: Option<StatusOptions>, signal: Option<AbortSignal>) -> Result<AsyncTask<GitStatusTask>>` — mirror `get_file_latest_modification_async`. Add a `GitStatusTask { repo: RwLock<Reference<Repository>>, options: ... }` next to the other tasks. NOTE: `StatusOptions` is not `Clone` by default unless you derive it; capture the resolved primitive fields (or re-derive `Clone` on the `#[napi(object)]` struct — napi object structs can derive `Clone`). Simplest: store the `Option<StatusOptions>` and rebuild opts in `compute`.

**Tests (`__test__/status.spec.mjs`, read-only against project repo + a temp repo for untracked/staged cases):**
- clean subset: `repo.statuses()` returns an array; types are correct.
- In a temp repo: an untracked file → a `FileStatus` with `isWtNew === true`; `git add` it → `isIndexNew === true`; modify a committed-then-staged file to distinguish `isIndexModified` vs `isWtModified`.
- `statusFile` on a known path returns a `FileStatus`.
- `statusesAsync()` resolves to an array equal in length to the sync call on the same state.

---

## Task 2: Repository config + default signature — `Repository.config` / `Repository.signature`

**New file `src/config.rs`.** Declare `pub mod config;` in `src/lib.rs`.

`git2::Config` is owned → wrap as a class:

```rust
#[napi]
pub struct Config { inner: git2::Config }
```

Define a `#[napi(object)] pub struct ConfigEntry { pub name: String, pub value: String, pub level: ConfigLevel }` and a `#[napi] pub enum ConfigLevel { ProgramData, System, Xdg, Global, Local, Worktree, App, Highest }` mapping `git2::ConfigLevel` (use the variants git2 0.21 exposes; map both directions if needed).

`#[napi] impl Config` methods (all map git2 methods, `.convert_without_message()` on error):
- `#[napi(factory)] open_default() -> Result<Config>` → `git2::Config::open_default()`.
- `get_string_value(&self, name: String) -> Result<String>` → `get_string` (named to avoid clashing with JS `getString`/clarity; expose as `getStringValue`).
- `get_bool(&self, name) -> Result<bool>`, `get_i32(&self, name) -> Result<i32>`, `get_i64(&self, name) -> Result<i64>`.
- `set_str(&mut self, name, value) -> Result<()>`, `set_bool(&mut self, name, value: bool) -> Result<()>`, `set_i32`, `set_i64`. (git2 `set_*` need `&mut self`; napi-rs 3 supports `&mut self` methods.)
- `remove_entry(&mut self, name) -> Result<()>` → `remove`.
- `snapshot(&self) -> Result<Config>` → `git2::Config::snapshot` (returns a read-only point-in-time copy; document that `get_*` on a live config re-reads files).
- `entries(&self, glob: Option<String>) -> Result<Vec<ConfigEntry>>` — iterate `self.inner.entries(glob.as_deref())`, eagerly materialize each borrowed `ConfigEntry` into the owned `#[napi(object)]` (name/value via `.name()/.value()` → `String`, skip or lossy on non-UTF-8; `.level()` → `ConfigLevel`).

Methods on `impl Repository` (`src/repo.rs`):
- `config(&self) -> Result<Config>` → `self.inner.config()` wrapped.
- `signature(&self) -> Result<Signature>` → `self.inner.signature()` → wrap into the existing `Signature` (`SignatureInner::Signature(git2_sig)`). This finally implements the method the `Signature` doc-comment already references. (Check `src/signature.rs` for how to construct `Signature` from a `git2::Signature<'static>` — `git2::Repository::signature()` returns an owned `Signature<'static>`.)

`Config` and `ConfigLevel` are new exports → add `module.exports.Config` and `module.exports.ConfigLevel` to `index.js` after build.

**Tests (`__test__/config.spec.mjs`):**
- `repo.config().snapshot().getStringValue('core.bare')` (or another always-present key) returns a string; assert against `execSync('git config core.bare')` where deterministic.
- In a temp repo: `set_str('user.name', 'x')` then `getStringValue('user.name') === 'x'`; `get_bool('core.bare')`.
- `repo.signature()` — in a temp repo with `user.name`/`user.email` set, returns a `Signature` whose `.name()/.email()` match; assert it throws when identity unset (use a temp repo / config scope where it's unset, or assert `t.throws`).
- `entries('user.*')` returns entries including the ones you set.

---

## Task 3: Remote push — `Remote.push` + `PushOptions` + `RemoteCallbacks.pushUpdateReference`

All in `src/remote.rs`.

Add a `PushOptions` class mirroring the existing `FetchOptions` (which holds `inner: git2::FetchOptions<'static>` + a `used` guard and builder methods). Study `FetchOptions` (struct + `impl`) and copy its shape:

```rust
#[napi]
pub struct PushOptions { pub(crate) inner: git2::PushOptions<'static>, used: bool }
```
`#[napi] impl PushOptions` with `#[napi(constructor)] new()` and builder methods returning `&Self` (or `this`): `remote_callback(&mut self, callback: &mut RemoteCallbacks)`, `proxy_options(&mut self, options: &mut ProxyOptions)`, `packbuilder_parallelism(&mut self, parallel: u32)`, `follow_redirects(&mut self, opt: RemoteRedirect)`, `custom_headers(&mut self, headers: Vec<String>)`, `remote_push_options(&mut self, options: Vec<String>)`. Match how `FetchOptions` wires each of these to the underlying git2 builder (some take callbacks/proxy by moving out of the wrapper like fetch does).

Add `push` to `#[napi] impl Remote`, placed right after `fetch`, mirroring `fetch`'s `mem::swap` "used-once options" pattern:
```rust
/// Perform a push. If `refspecs` is empty the configured push refspecs are used.
/// Delete a remote ref with ":refs/heads/branch". To detect per-ref server
/// rejections, set a pushUpdateReference callback on the RemoteCallbacks.
pub fn push(&mut self, refspecs: Vec<String>, push_options: Option<&mut PushOptions>) -> Result<()>
```
Keep it **synchronous** (matches `fetch`/`update_tips`; no AsyncTask).

Add `push_update_reference` to `#[napi] impl RemoteCallbacks` (mirror how `transfer_progress`/`credentials` are wired with `ThreadsafeFunction` or the existing callback storage; study the existing callbacks). It fires once per ref with `(refname: String, status: Option<String>)` where `status` is `None` on success and the server's rejection reason otherwise. Wire it onto `self.inner.push_update_reference(...)`.

`PushOptions` is a new export → add `module.exports.PushOptions` to `index.js`.

**Tests (`__test__/push.spec.mjs`, fully local — no network):**
- Create two temp repos: a **bare** repo `remote.git` and a working clone/repo with the bare as a remote. Make a commit in the working repo, `remote.push(['refs/heads/main:refs/heads/main'], null)` (or the branch your temp repo uses), then assert the bare repo now has that ref (`execSync('git --git-dir=remote.git rev-parse refs/heads/main')` equals the commit). 
- Construct `new PushOptions()` and pass it to push to prove the options path compiles/runs.
- Optionally: set a `pushUpdateReference` callback and assert it fires with `status === null` on the successful push.
- Clean up temp dirs.

---

## Task 4: Index / staging + blob creation + commit parents

Enables building a commit from working-tree changes. **New file `src/index.rs`** (module name `index`; declare `pub mod index;` in `src/lib.rs`). `git2::Index` is owned → wrap as a class:

```rust
#[napi]
pub struct Index { inner: git2::Index }
```
`#[napi] impl Index` (`&mut self` where git2 needs it; `.convert_without_message()`):
- `add_path(&mut self, path: String) -> Result<()>` → `add_path(Path::new(&path))`.
- `add_all(&mut self, pathspecs: Option<Vec<String>>, force: Option<bool>) -> Result<()>` → `add_all(specs, flag, None)`; default specs `["*"]` when `None`; `force` maps to `IndexAddOption::FORCE` else `DEFAULT`.
- `update_all(&mut self, pathspecs: Option<Vec<String>>) -> Result<()>` → `update_all`, default specs `["*"]`.
- `remove_path(&mut self, path: String) -> Result<()>` → `remove_path`.
- `count(&self) -> u32` → `self.inner.len() as u32`.
- `write(&mut self) -> Result<()>` → `write`.
- `write_tree(&mut self) -> Result<String>` → `write_tree`, return OID hex.

Methods on `impl Repository` (`src/repo.rs`):
- `index(&self) -> Result<Index>` → `self.inner.index()` wrapped.
- `blob(&self, data: Uint8Array) -> Result<String>` → `self.inner.blob(&data)` → OID hex.
- `blob_path(&self, path: String) -> Result<String>` → `self.inner.blob_path(Path::new(&path))` → OID hex.

**Modify the existing `commit` method** (`src/repo.rs`) to accept optional parents so it can extend history (currently hardcodes `&[]`, only making root commits). Add a trailing optional param **`parents: Option<Vec<String>>`** (parent commit OID hex strings — unambiguous and definitely napi-supported; resolve each via `self.inner.find_commit(Oid::from_str(&p)?)`). Backward-compatible: when `None`/empty, behaviour is identical to today (`&[]`). Build `Vec<git2::Commit>`, then `commit(update_ref, author, committer, message, tree, &parent_refs)` where `parent_refs: Vec<&git2::Commit>`. Update the method's doc-comment.

`Index` is a new export → add `module.exports.Index` to `index.js`.

**Tests (`__test__/index.spec.mjs`, mutating → temp repo only):**
- Init a temp repo, write a file, `repo.index().addAll()` + `write()`, `writeTree()` returns a 40-char OID; `repo.findTree(oid)` is non-null.
- End-to-end: create an initial commit (parents omitted → root), then a second file, stage, writeTree, and `repo.commit('HEAD', sig, sig, 'second', tree, [firstCommitOid])`; verify with `execSync('git log --oneline')` that there are 2 commits and the second's parent is the first.
- `repo.blob(Buffer.from('hello'))` returns the OID of `hello` (`execSync('git hash-object')` to cross-check).
- Verify the existing root-commit path (parents omitted) still works (regression for the signature change).

---

## Task 5: Blame — `Repository.blameFile` / `blameFileAsync` / `blameLine`

**New file `src/blame.rs`.** Declare `pub mod blame;` in `src/lib.rs`. Eager-materialize hunks (the `git2::Blame<'repo>` and `BlameHunk<'blame>` both borrow — copy fields out before they drop).

```rust
#[napi(object)]
pub struct BlameOptions {
  pub track_copies_same_file: Option<bool>,
  pub track_copies_same_commit_moves: Option<bool>,
  pub newest_commit: Option<String>, // 40-char hex
  pub oldest_commit: Option<String>,
  pub first_parent: Option<bool>,
  pub use_mailmap: Option<bool>,
  pub ignore_whitespace: Option<bool>,
  pub min_line: Option<u32>, // 1-based
  pub max_line: Option<u32>,
}

#[napi(object)]
pub struct BlameHunk {
  pub lines_in_hunk: u32,
  pub final_commit_id: String,
  pub final_start_line: u32,      // 1-based
  pub final_author_name: Option<String>,
  pub final_author_email: Option<String>,
  pub final_time: i64,            // ms since epoch
  pub orig_commit_id: String,
  pub orig_start_line: u32,
  pub orig_path: Option<String>,
  pub is_boundary: bool,
}
```
Helpers: `build_blame_opts(Option<BlameOptions>) -> git2::BlameOptions` (map builder; parse `newest_commit`/`oldest_commit` hex via `Oid::from_str` and `.newest_commit`/`.oldest_commit`); `hunk_to_struct(&git2::BlameHunk) -> BlameHunk` (copy identity out of `final_signature()`/`orig_signature()`: name/email `Option<String>`, time `sig.when().seconds()*1000`; commit ids via `.final_commit_id()/.orig_commit_id()`; lines 1-based — git2 `final_start_line()` is 1-based already; `orig_path()` → `Option<String>`).

Methods on `impl Repository` (`src/repo.rs`):
- `blame_file(&self, path: String, options: Option<BlameOptions>) -> Result<Vec<BlameHunk>>` — `self.inner.blame_file(Path::new(&path), Some(&mut opts))`, iterate `blame.iter()`, eager-collect.
- `blame_line(&self, path: String, line_no: u32, options: Option<BlameOptions>) -> Result<Option<BlameHunk>>` — get the blame, `blame.get_line(line_no as usize)` → `Option<BlameHunk>`, map.
- `blame_file_async(&self, self_ref, path, options, signal) -> Result<AsyncTask<GitBlameTask>>` — mirror the modification async task. `BlameOptions` can derive `Clone` (it's a plain `#[napi(object)]`) so the task can rebuild opts in `compute`.

**Tests (`__test__/blame.spec.mjs`, read-only against project repo):**
- `repo.blameFile('Cargo.toml')` returns a non-empty array; each hunk has a 40-char `finalCommitId`, positive `linesInHunk`, 1-based `finalStartLine`, and a numeric `finalTime`.
- The sum of `linesInHunk` is ≥ 1 and hunks are contiguous (optional sanity).
- `blameLine('Cargo.toml', 1)` returns a hunk whose range covers line 1.
- `blameFileAsync('Cargo.toml')` length equals the sync call.

---

## Task 6: Branch type + enumeration + creation — `Repository.branches` / `findBranch` / `branch` + `Branch`

**New file `src/branch.rs`.** Declare `pub mod branch;` in `src/lib.rs`. `git2::Branch<'repo>` borrows the repo → use the `SharedReference<Repository, git2::Branch<'static>>` pattern (study `src/reference.rs` — `Reference` wraps `SharedReference<Repository, git2::Reference<'static>>` and is built via `this_ref.share_with(env, |repo| ...)`).

```rust
#[napi] pub enum BranchType { Local, Remote }  // map git2::BranchType both ways
#[napi] pub struct Branch { inner: SharedReference<Repository, git2::Branch<'static>> }
```
`#[napi] impl Branch`:
- `name(&self) -> Result<Option<String>>` → `self.inner.name()` (`Result<Option<&str>>` → map).
- `is_head(&self) -> bool` → `self.inner.is_head()`.
- `get(&self, ...) -> Reference` — return the underlying reference. (Branch derefs to Reference; you can build a `reference::Reference` sharing from the same parent. If this is awkward across the SharedReference boundary, expose `reference_name(&self) -> Option<String>` returning `self.inner.get().name()` instead, and note the simplification in your report.)
- `delete(&mut self) -> Result<()>` → `self.inner.delete()`.
- `upstream(&self, ...) -> Result<Option<Branch>>` — `self.inner.get().branch_upstream...`; if cross-SharedReference sharing is hard, you MAY defer `upstream` and report it as deferred (it's the least-critical accessor). Prefer to implement `name`/`is_head`/`delete` solidly.

Methods on `impl Repository` (`src/repo.rs`):
- `branches(&self, this_ref: Reference<Repository>, env: Env, filter: Option<BranchType>) -> Result<Vec<Branch>>` — `self.inner.branches(filter.map(Into))`, for each `(git2_branch, _type)` build a `Branch` via `this_ref.clone(env)?.share_with(env, |repo| repo.inner.find_branch(name, ty))` — i.e. re-find by name+type inside `share_with` (the iterator's borrowed branch can't be moved into the SharedReference; re-finding by name is the clean approach, mirroring how `head()` shares). Determine the cleanest construction by studying `head()` and `find_tree()`.
- `find_branch(&self, this_ref: Reference<Repository>, env: Env, name: String, branch_type: BranchType) -> Result<Option<Branch>>` — share via `find_branch(&name, branch_type.into())`; return `None` when not found (map the not-found git2 error to `Ok(None)`, like `find_tree`/`find_commit` return `Option`).
- `branch(&self, this_ref: Reference<Repository>, env: Env, branch_name: String, target: &Commit, force: bool) -> Result<Branch>` — `self.inner.branch(&branch_name, target_commit, force)`; build the `Branch` (re-find by name+Local after creating, inside `share_with`).

`Branch` and `BranchType` are new exports → add `module.exports.Branch` and `module.exports.BranchType` to `index.js`.

**Tests (`__test__/branch.spec.mjs`):**
- Read-only against project repo: `repo.branches()` returns a non-empty array containing a `Branch` whose `name()` is non-null; exactly one has `isHead() === true` among local branches (when HEAD is on a branch).
- `findBranch('<current>', BranchType.Local)` (derive current name from `repo.head().shorthand()`) returns a `Branch`; `findBranch('definitely-missing', ...)` returns `null`.
- In a temp repo: `repo.branch('feature', headCommit, false)` creates it (`execSync('git branch --list feature')` non-empty); `branch.delete()` removes it.

---

## Task 7: Checkout + HEAD + reference creation — `checkoutTree` / `checkoutHead` / `checkoutIndex` / `setHead` / `setHeadDetached` / `reference` / `referenceSymbolic`

All on `impl Repository` (`src/repo.rs`). Add a `#[napi(object)] pub struct CheckoutOptions` (put it in `src/repo.rs` or a small `src/checkout.rs` — your call; if a new module, declare it in `lib.rs`):

```rust
#[napi(object)]
pub struct CheckoutOptions {
  pub force: Option<bool>,            // default safe
  pub recreate_missing: Option<bool>,
  pub allow_conflicts: Option<bool>,
  pub paths: Option<Vec<String>>,
  pub target_dir: Option<String>,
}
```
Helper `build_checkout_builder(Option<CheckoutOptions>) -> git2::build::CheckoutBuilder` — `force` → `.force()`, else default (safe); `recreate_missing` → `.recreate_missing(true)`; `allow_conflicts` → `.allow_conflicts(true)`; each `paths` entry → `.path(p)`; `target_dir` → `.target_dir(Path::new(...))`. **Default must be safe checkout** (no `.force()`), to avoid silent data loss.

Methods:
- `checkout_tree(&self, treeish: &GitObject, options: Option<CheckoutOptions>) -> Result<()>` → `self.inner.checkout_tree(git2_object, Some(&mut builder))` (`GitObject` derefs to `git2::Object`; study `src/object.rs`).
- `checkout_head(&self, options: Option<CheckoutOptions>) -> Result<()>` → `checkout_head`.
- `checkout_index(&self, options: Option<CheckoutOptions>) -> Result<()>` → `checkout_index(None, Some(&mut builder))`.
- `set_head(&self, refname: String) -> Result<()>` → `set_head(&refname)`.
- `set_head_detached(&self, oid: String) -> Result<()>` → `set_head_detached(Oid::from_str(&oid)?)`.
- `reference(&self, this_ref: Reference<Repository>, env: Env, name: String, oid: String, force: bool, log_message: String) -> Result<reference::Reference>` → `self.inner.reference(&name, Oid::from_str(&oid)?, force, &log_message)`, wrap into `reference::Reference` via `share_with` (mirror `head()`).
- `reference_symbolic(&self, this_ref, env, name: String, target: String, force: bool, log_message: String) -> Result<reference::Reference>` → `self.inner.reference_symbolic(...)`, wrap.

No new classes here (`CheckoutOptions` is `#[napi(object)]`, `Reference` already exported) → **no `index.js` export changes** unless you put `CheckoutOptions` as a class (don't).

**Tests (`__test__/checkout.spec.mjs`, mutating → temp repo only):**
- Init a temp repo with two commits on `main`, create a branch at the first commit, `setHead('refs/heads/<branch>')` + `checkoutHead({ force: true })`, assert the workdir file content reverted (read the file) and `execSync('git symbolic-ref HEAD')` points at the branch.
- `setHeadDetached(firstCommitOid)` → `execSync('git rev-parse HEAD')` equals it and `git symbolic-ref HEAD` errors (detached).
- `reference('refs/heads/made-by-api', headOid, false, 'msg')` creates the ref (`git rev-parse` resolves it); `referenceSymbolic('refs/heads/sym', 'refs/heads/main', false, 'msg')` resolves through.
- Clean up temp dirs.

---

## Task 8: README documentation

Update `README.md` to document the new APIs, matching the existing style (the `## Repository` usage block + the hand-written `### API` TypeScript block). After all of Tasks 1–7 are merged on the branch:

- Add a concise usage snippet per feature area (status, config + signature, push, index/commit-from-workdir, blame, branch + checkout), mirroring the tone of the existing `getFileLatestModification` examples (short, with `// =>` result comments).
- Extend the `### API` code block so the `export class Repository { ... }` listing and the new classes (`Config`, `Index`, `Branch`, `PushOptions`) / interfaces (`FileStatus`, `StatusOptions`, `BlameHunk`, `BlameOptions`, `CheckoutOptions`, `ConfigEntry`) / enums (`BranchType`, `ConfigLevel`) are present. **Source the exact signatures from the committed `index.d.ts`** (run `yarn build:debug` first so it's current, reconcile per the codegen rule, then copy the real generated signatures 1:1 — do not hand-invent them).
- This task changes only `README.md` (and possibly a codegen-reconciled `index.d.ts` if it drifted). No Rust, no new tests; verify by confirming README signatures match `index.d.ts`.
