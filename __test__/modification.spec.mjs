import { execSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import test from "ava";

import { Repository } from "../index.js";

const __dirname = join(fileURLToPath(import.meta.url), "..");
const workDir = join(__dirname, "..");

const git = (args) =>
  execSync(`git ${args}`, { cwd: workDir }).toString().trim();

// Build a hermetic throwaway repo committing each name in `files` (relative to
// the work tree). Caller removes `root`. Used by the `__proto__`-safety
// regression: git happily tracks a file literally named `__proto__`.
function makeRepo(files) {
  const root = mkdtempSync(join(tmpdir(), "simple-git-modification-"));
  const work = join(root, "work");
  execSync(`git init -q -b main "${work}"`);
  const run = (args) => execSync(`git ${args}`, { cwd: work });
  run("config user.name tester");
  run("config user.email tester@example.com");
  run("config commit.gpgsign false");
  run("config core.autocrlf false");
  for (const name of files) {
    const dest = join(work, name);
    // Support nested paths like "dir/a.txt": writeFileSync won't create parents.
    mkdirSync(dirname(dest), { recursive: true });
    writeFileSync(dest, `${name}\n`);
  }
  run("add -A");
  run("commit -q -m seed");
  return { root, repo: new Repository(work) };
}

// Build a hermetic throwaway repo with a controlled MULTI-commit history so the
// creation walk (`created`, oldest-first) can diverge from the modification walk
// (newest-first). Each `step` is `{ write?: {name: content}, del?: [names],
// message }`: it deletes the named paths, then writes the named files, stages
// everything (`add -A`), then commits. Deletions run BEFORE writes and `rmSync`
// is recursive so a single step can delete a FILE and then create a DIRECTORY of
// the same name (a TYPE transition, and vice-versa: delete a dir, add a file);
// writes create parent dirs so nested paths like "dir/a.txt" work. Same
// init/config pattern as `makeRepo`. Returns `{ root, repo, git, commits }` where
// `commits[i]` is the 40-hex OID of step `i`'s commit (captured via `git
// rev-parse HEAD` right after it lands, oldest first) and `git` runs a command in
// the work tree for optional CLI cross-checks. Caller removes `root`.
function makeRepoWithHistory(steps) {
  const root = mkdtempSync(join(tmpdir(), "simple-git-modification-hist-"));
  const work = join(root, "work");
  execSync(`git init -q -b main "${work}"`);
  const run = (args) => execSync(`git ${args}`, { cwd: work });
  const capture = (args) =>
    execSync(`git ${args}`, { cwd: work }).toString().trim();
  run("config user.name tester");
  run("config user.email tester@example.com");
  run("config commit.gpgsign false");
  run("config core.autocrlf false");
  const commits = [];
  for (const step of steps) {
    // Deletions first (recursive), so a step can delete a FILE named `dir` and
    // then create the DIRECTORY `dir/a.txt` in the same commit (type transition).
    for (const name of step.del ?? []) {
      rmSync(join(work, name), { recursive: true, force: true });
    }
    for (const [name, content] of Object.entries(step.write ?? {})) {
      const dest = join(work, name);
      // Create parents (idempotent) so a nested write, or a write to a name that
      // was just deleted as a file, lands even when the parent didn't exist.
      mkdirSync(dirname(dest), { recursive: true });
      writeFileSync(dest, content);
    }
    run("add -A");
    run(`commit -q -m "${step.message}"`);
    commits.push(capture("rev-parse HEAD"));
  }
  return { root, repo: new Repository(work), git: capture, commits };
}

// Build a hermetic throwaway repo containing an EVIL MERGE: a file that exists
// only in a merge commit's tree, present in NEITHER parent. History:
//   c0 root            -> root.txt
//   c1 on branch `side`-> on-branch.txt  (a normal 1-parent add)
//   c2 on `main`       -> main2.txt      (so the merge is a real non-fast-forward)
//   c3 evil merge      -> `git merge --no-commit --no-ff side`, then write a NEW
//                         `evil.txt`, `git add evil.txt`, and commit -- so the
//                         merge commit's tree carries `evil.txt` (in neither
//                         parent) plus `on-branch.txt` (from the merged branch).
// `evil.txt` is thus touched by NO non-merge commit (the modification walk skips
// merges), while `on-branch.txt` arrived via the 1-parent branch add. Returns
// `{ root, repo, git, mergeCommit, onBranchCommit }` with the merge commit OID
// and the branch-add commit OID (both via `git rev-parse HEAD`). Caller removes
// `root`. Same init/config pattern as the other helpers.
function makeRepoWithEvilMerge() {
  const root = mkdtempSync(join(tmpdir(), "simple-git-modification-evil-"));
  const work = join(root, "work");
  execSync(`git init -q -b main "${work}"`);
  const run = (args) => execSync(`git ${args}`, { cwd: work });
  const capture = (args) =>
    execSync(`git ${args}`, { cwd: work }).toString().trim();
  run("config user.name tester");
  run("config user.email tester@example.com");
  run("config commit.gpgsign false");
  run("config core.autocrlf false");

  // c0 — root commit.
  writeFileSync(join(work, "root.txt"), "root\n");
  run("add -A");
  run('commit -q -m "c0 root"');

  // c1 — a normal 1-parent add of on-branch.txt on branch `side`.
  run("checkout -q -b side");
  writeFileSync(join(work, "on-branch.txt"), "on branch\n");
  run("add on-branch.txt");
  run('commit -q -m "c1 add on-branch (1-parent)"');
  const onBranchCommit = capture("rev-parse HEAD");

  // c2 — main advances so `--no-ff` produces a genuine merge commit.
  run("checkout -q main");
  writeFileSync(join(work, "main2.txt"), "main two\n");
  run("add main2.txt");
  run('commit -q -m "c2 main advances"');

  // c3 — evil merge: merge `side` WITHOUT committing, then introduce a brand-new
  // `evil.txt` (absent from both parents) and fold it into the merge commit.
  run("merge --no-commit --no-ff side");
  writeFileSync(join(work, "evil.txt"), "evil\n");
  run("add evil.txt");
  run('commit -q -m "c3 evil merge"');
  const mergeCommit = capture("rev-parse HEAD");

  return {
    root,
    repo: new Repository(work),
    git: capture,
    mergeCommit,
    onBranchCommit,
  };
}

// Build a hermetic throwaway repo where `evil.txt` is BOTH added and deleted only
// via merge commits, so it is ABSENT at HEAD. History extends the evil-merge add:
//   c0 root                -> root.txt
//   c1 on `side`           -> on-branch.txt (a normal 1-parent add)
//   c2 on `main`           -> main2.txt     (so the add-merge is non-fast-forward)
//   c3 evil ADD merge      -> merge `side` --no-commit --no-ff, write NEW evil.txt,
//                             add + commit -> evil.txt is in the merge tree only.
//   c4 on `side2` (from c3)-> side2.txt      (side2 tip inherits evil.txt)
//   c5 on `main`           -> main3.txt      (main tip inherits evil.txt)
//   c6 evil DELETE merge   -> merge `side2` --no-commit --no-ff (BOTH parents carry
//                             evil.txt), then `git rm evil.txt`, then commit -> the
//                             merge removes evil.txt. It is thus deleted ONLY via a
//                             merge (the modification walk skips merges), and ABSENT
//                             at HEAD.
// So `get_file_modification` finds nothing (None) while `get_file_creation` still
// resolves the c3 add-merge -- the exact case the present-at-HEAD gate must catch.
// Returns `{ root, repo, git, addMergeCommit, deleteMergeCommit }`. Caller removes
// `root`. Same init/config pattern as the other helpers.
function makeRepoWithEvilMergeThenDelete() {
  const root = mkdtempSync(join(tmpdir(), "simple-git-modification-evildel-"));
  const work = join(root, "work");
  execSync(`git init -q -b main "${work}"`);
  const run = (args) => execSync(`git ${args}`, { cwd: work });
  const capture = (args) =>
    execSync(`git ${args}`, { cwd: work }).toString().trim();
  run("config user.name tester");
  run("config user.email tester@example.com");
  run("config commit.gpgsign false");
  run("config core.autocrlf false");

  // c0 — root commit.
  writeFileSync(join(work, "root.txt"), "root\n");
  run("add -A");
  run('commit -q -m "c0 root"');

  // c1 — a normal 1-parent add of on-branch.txt on branch `side`.
  run("checkout -q -b side");
  writeFileSync(join(work, "on-branch.txt"), "on branch\n");
  run("add on-branch.txt");
  run('commit -q -m "c1 add on-branch (1-parent)"');

  // c2 — main advances so `--no-ff` produces a genuine merge commit.
  run("checkout -q main");
  writeFileSync(join(work, "main2.txt"), "main two\n");
  run("add main2.txt");
  run('commit -q -m "c2 main advances"');

  // c3 — evil ADD merge: introduce evil.txt inside the merge commit's tree.
  run("merge --no-commit --no-ff side");
  writeFileSync(join(work, "evil.txt"), "evil\n");
  run("add evil.txt");
  run('commit -q -m "c3 evil add merge"');
  const addMergeCommit = capture("rev-parse HEAD");

  // c4 — side2 branches from the add-merge (so its tip carries evil.txt).
  run("checkout -q -b side2");
  writeFileSync(join(work, "side2.txt"), "side two\n");
  run("add side2.txt");
  run('commit -q -m "c4 side2 advances"');

  // c5 — main advances again so the delete-merge is a genuine --no-ff merge.
  run("checkout -q main");
  writeFileSync(join(work, "main3.txt"), "main three\n");
  run("add main3.txt");
  run('commit -q -m "c5 main advances again"');

  // c6 — evil DELETE merge: both parents carry evil.txt, but `git rm` before the
  // commit drops it, so evil.txt is removed ONLY via this merge commit.
  run("merge --no-commit --no-ff side2");
  run("rm -q evil.txt");
  run('commit -q -m "c6 evil delete merge"');
  const deleteMergeCommit = capture("rev-parse HEAD");

  return {
    root,
    repo: new Repository(work),
    git: capture,
    addMergeCommit,
    deleteMergeCommit,
  };
}

// Build a hermetic throwaway repo where `evil.txt` is BOTH added AND later
// changed ONLY via merge commits, so it is a "double evil merge": present at HEAD
// with the LATEST content, but no non-merge commit ever touched it. History:
//   c0 root                -> root.txt
//   c1 on `side`           -> on-branch.txt (a normal 1-parent add)
//   c2 on `main`           -> main2.txt     (so the add-merge is non-fast-forward)
//   M1 evil ADD merge      -> merge `side` --no-commit --no-ff, write NEW evil.txt
//                             = "v1", add + commit -> evil.txt="v1" is in the merge
//                             tree only (neither parent has it).
//   c4 on `side2` (from M1)-> side2.txt      (side2 tip inherits evil.txt="v1")
//   c5 on `main`           -> main3.txt      (main tip inherits evil.txt="v1")
//   M2 evil CHANGE merge   -> merge `side2` --no-commit --no-ff (BOTH parents carry
//                             evil.txt="v1"), overwrite evil.txt="v2", add + commit
//                             -> evil.txt is CHANGED to "v2" ONLY via this merge.
// So `get_file_modification` finds nothing (merges skipped) while HEAD content is
// "v2" from M2. The flat record must be M2 (the LATEST change), and `created` must
// be M1 (the creation) -- distinct commits. Returns `{ root, repo, git,
// addMergeCommit, changeMergeCommit }`. Caller removes `root`.
function makeRepoWithDoubleEvilMerge() {
  const root = mkdtempSync(join(tmpdir(), "simple-git-modification-double-evil-"));
  const work = join(root, "work");
  execSync(`git init -q -b main "${work}"`);
  const run = (args) => execSync(`git ${args}`, { cwd: work });
  const capture = (args) =>
    execSync(`git ${args}`, { cwd: work }).toString().trim();
  run("config user.name tester");
  run("config user.email tester@example.com");
  run("config commit.gpgsign false");
  run("config core.autocrlf false");

  // c0 — root commit.
  writeFileSync(join(work, "root.txt"), "root\n");
  run("add -A");
  run('commit -q -m "c0 root"');

  // c1 — a normal 1-parent add of on-branch.txt on branch `side`.
  run("checkout -q -b side");
  writeFileSync(join(work, "on-branch.txt"), "on branch\n");
  run("add on-branch.txt");
  run('commit -q -m "c1 add on-branch (1-parent)"');

  // c2 — main advances so `--no-ff` produces a genuine merge commit.
  run("checkout -q main");
  writeFileSync(join(work, "main2.txt"), "main two\n");
  run("add main2.txt");
  run('commit -q -m "c2 main advances"');

  // M1 — evil ADD merge: evil.txt="v1" enters inside the merge commit's tree.
  run("merge --no-commit --no-ff side");
  writeFileSync(join(work, "evil.txt"), "v1\n");
  run("add evil.txt");
  run('commit -q -m "M1 evil add merge"');
  const addMergeCommit = capture("rev-parse HEAD");

  // c4 — side2 branches from M1 (so its tip carries evil.txt="v1").
  run("checkout -q -b side2");
  writeFileSync(join(work, "side2.txt"), "side two\n");
  run("add side2.txt");
  run('commit -q -m "c4 side2 advances"');

  // c5 — main advances again so the change-merge is a genuine --no-ff merge.
  run("checkout -q main");
  writeFileSync(join(work, "main3.txt"), "main three\n");
  run("add main3.txt");
  run('commit -q -m "c5 main advances again"');

  // M2 — evil CHANGE merge: both parents carry evil.txt="v1", but overwrite to
  // "v2" before the commit, so evil.txt is CHANGED to "v2" ONLY via this merge.
  run("merge --no-commit --no-ff side2");
  writeFileSync(join(work, "evil.txt"), "v2\n");
  run("add evil.txt");
  run('commit -q -m "M2 evil change merge"');
  const changeMergeCommit = capture("rev-parse HEAD");

  return {
    root,
    repo: new Repository(work),
    git: capture,
    addMergeCommit,
    changeMergeCommit,
  };
}

test.beforeEach((t) => {
  t.context.repo = new Repository(workDir);
});

// Test #1 — enriched metadata, value-asserted against git CLI.
// build.rs author (LongYinan) != committer (GitHub): catches author/committer swaps.
test("getFileLatestModified returns enriched metadata", (t) => {
  const { repo } = t.context;
  const mod = repo.getFileLatestModified("build.rs");
  t.truthy(mod);

  // Delegation guard (runs unconditionally; two native methods). committerTime
  // is the same instant getFileLatestModifiedDate returns -- a Date whose
  // epoch ms equal the number getter.
  t.true(mod.committerTime instanceof Date);
  t.is(
    mod.committerTime.getTime(),
    repo.getFileLatestModifiedDate("build.rs"),
  );
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
  t.true(mod.authorTime instanceof Date);
});

// Test #1b — `created` = the commit that FIRST added the file (oldest-first
// creation walk), separate from the newest-first modification walk. build.rs's
// creating commit differs from its latest-modifying commit, catching a
// created/modified swap.
test("getFileLatestModified attaches the creating commit as `created`", (t) => {
  const { repo } = t.context;
  const mod = repo.getFileLatestModified("build.rs");
  t.truthy(mod);

  // Shape (runs unconditionally, incl. CI): truthy CommitInfo, 40-hex OID,
  // author/committer times are Dates.
  t.truthy(mod.created);
  t.regex(mod.created.commitId, /^[0-9a-f]{40}$/);
  t.true(mod.created.authorTime instanceof Date);
  t.true(mod.created.committerTime instanceof Date);

  // Value parity with git CLI; skip on CI where the checkout may be shallow.
  if (!process.env.CI) {
    // First commit that ADDED build.rs (oldest of --diff-filter=A --reverse).
    const firstAdd = git(
      "log --diff-filter=A --format=%H --reverse -- build.rs",
    ).split("\n")[0];
    t.is(mod.created.commitId, firstAdd);
    // Creation predates (or equals) the latest modification, and here differs.
    t.not(mod.created.commitId, mod.commitId);
  }
});

// `created` is EXACT-path only: a glob/pathspec input resolves the flat fields
// via pathspec, but leaves `created` undefined (not null) -- matching the other
// Option fields' "undefined when absent" convention.
test("getFileLatestModified leaves created undefined for a glob/pathspec input", (t) => {
  const { repo } = t.context;
  const mod = repo.getFileLatestModified("*.rs");
  t.truthy(mod);                        // flat fields resolve via pathspec
  t.regex(mod.commitId, /^[0-9a-f]{40}$/);
  t.is(mod.created, undefined);         // exact tree.get_path("*.rs") never matches
  t.false(Object.prototype.hasOwnProperty.call(mod, "created"));
});

// `created` resolves an exact FILE (blob) only: a DIRECTORY input resolves the
// flat fields via pathspec (a directory pathspec matches files under it), but
// "src" is a TREE entry (not a blob), so `created` stays undefined -- NOT a
// bogus record for the directory's own creation commit.
test("getFileLatestModified leaves created undefined for a directory input", (t) => {
  const { repo } = t.context;
  const mod = repo.getFileLatestModified("src");   // a directory, not a file
  t.truthy(mod);                                    // flat fields resolve via pathspec
  t.is(mod.created, undefined);                     // src is a tree entry, not a blob
  t.false(Object.prototype.hasOwnProperty.call(mod, "created"));
});

// Hermetic file-vs-directory: the exact FILE "dir/a.txt" is a blob -> `created`
// resolves; its parent "dir" is a TREE entry -> `created` is undefined. Guards
// the blob-only creation resolution end to end in a throwaway repo.
test("getFileLatestModified resolves created for a nested file but not its directory", (t) => {
  const { root, repo } = makeRepo(["dir/a.txt"]);
  try {
    const file = repo.getFileLatestModified("dir/a.txt");
    t.truthy(file);
    t.truthy(file.created);                         // exact file (blob) resolves
    t.regex(file.created.commitId, /^[0-9a-f]{40}$/);

    const dir = repo.getFileLatestModified("dir");
    t.truthy(dir);                                  // flat fields resolve via pathspec
    t.is(dir.created, undefined);                   // "dir" is a tree entry, not a blob
    t.false(Object.prototype.hasOwnProperty.call(dir, "created"));
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// Test #2 — async matches sync.
test("getFileLatestModifiedAsync matches sync result", async (t) => {
  const { repo } = t.context;
  const sync = repo.getFileLatestModified("build.rs");
  const asyncResult = await repo.getFileLatestModifiedAsync("build.rs");
  t.deepEqual(asyncResult, sync);

  // Async missing path resolves to null (mirrors sync null-on-missing, no throw).
  t.is(
    await repo.getFileLatestModifiedAsync("does-not-exist-xyz.nope"),
    null,
  );
});

// Test #2b — getFileLatestModifiedDateAsync (GitLatestModifiedDateTask) matches
// its sync sibling. Covers the async number-date path; both return epoch ms.
test("getFileLatestModifiedDateAsync matches sync result", async (t) => {
  const { repo } = t.context;
  const sync = repo.getFileLatestModifiedDate("build.rs");
  const asyncResult = await repo.getFileLatestModifiedDateAsync("build.rs");
  t.is(typeof asyncResult, "number");
  t.is(asyncResult, sync);
});

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

// Test #3 — null for a path that was never committed.
test("getFileLatestModified returns null for missing path", (t) => {
  const { repo } = t.context;
  t.is(repo.getFileLatestModified("does-not-exist-xyz.txt"), null);
});

// Root-commit branch (parent_count()==0): LICENSE's only commit is the root.
test("getFileLatestModified resolves a file whose only commit is the root", (t) => {
  const { repo } = t.context;
  const mod = repo.getFileLatestModified("LICENSE");
  t.truthy(mod);
  t.regex(mod.commitId, /^[0-9a-f]{40}$/);
  t.is(
    mod.committerTime.getTime(),
    repo.getFileLatestModifiedDate("LICENSE"),
  );
});

// Test #4 — bulk resolves many paths in one pass; cross-validate vs single-file.
test("getFilesLatestModified resolves many paths in one pass", (t) => {
  const { repo } = t.context;
  const result = repo.getFilesLatestModified([
    "build.rs",
    "Cargo.toml",
    "bogus-zzz.txt",
  ]);
  t.deepEqual(
    Object.keys(result).sort(),
    ["Cargo.toml", "bogus-zzz.txt", "build.rs"],
  );
  // Bulk now carries the same `created` creation record as the single-file
  // path, so each present record is byte-identical to getFileLatestModified.
  const buildSingle = repo.getFileLatestModified("build.rs");
  const cargoSingle = repo.getFileLatestModified("Cargo.toml");
  t.deepEqual(result["build.rs"], buildSingle);
  t.deepEqual(result["Cargo.toml"], cargoSingle);
  // Explicitly assert the creation record crosses the bulk boundary.
  t.truthy(result["build.rs"].created);
  t.deepEqual(result["build.rs"].created, buildSingle.created);
  t.deepEqual(result["Cargo.toml"].created, cargoSingle.created);
  t.is(result["bogus-zzz.txt"], null);
});

// Empty input -> {} (exercises the early-return branch + empty-Record serialization).
test("getFilesLatestModified returns {} for empty input", (t) => {
  const { repo } = t.context;
  t.deepEqual(repo.getFilesLatestModified([]), {});
});

// Regression: empty input on an UNBORN HEAD (a repo with no commits) must return
// {} WITHOUT throwing. The bulk merge-only fallback peels HEAD lazily -- only on a
// real fallback candidate -- so a no-op call never touches HEAD; an unconditional
// peel would throw "unborn HEAD" here. Locks that the lazy peel stays lazy.
test("getFilesLatestModified returns {} for empty input on an unborn HEAD", (t) => {
  const root = mkdtempSync(join(tmpdir(), "simple-git-modification-empty-"));
  const work = join(root, "work");
  execSync(`git init -q -b main "${work}"`);
  try {
    const repo = new Repository(work);
    t.deepEqual(repo.getFilesLatestModified([]), {}); // no commits, no HEAD peel
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// Nested forward-slash path: exact-string match against git's forward-slash delta path.
// Use a literal "src/lib.rs" (NOT path.join, which yields backslashes on Windows).
test("getFilesLatestModified matches a nested forward-slash path", (t) => {
  const { repo } = t.context;
  const result = repo.getFilesLatestModified(["src/lib.rs"]);
  t.deepEqual(result["src/lib.rs"], repo.getFileLatestModified("src/lib.rs"));
  t.truthy(result["src/lib.rs"]);
});

// Root-commit branch in the bulk walk, cross-validated vs single-file.
test("getFilesLatestModified resolves a root-only file (LICENSE)", (t) => {
  const { repo } = t.context;
  const result = repo.getFilesLatestModified(["LICENSE"]);
  t.deepEqual(result["LICENSE"], repo.getFileLatestModified("LICENSE"));
  t.truthy(result["LICENSE"]);
});

// Test #5 — async bulk matches sync bulk.
test("getFilesLatestModifiedAsync matches sync bulk result", async (t) => {
  const { repo } = t.context;
  const paths = ["build.rs", "Cargo.toml", "bogus-zzz.txt"];
  const sync = repo.getFilesLatestModified(paths);
  const bulkAsync = await repo.getFilesLatestModifiedAsync(paths);
  t.deepEqual(bulkAsync, sync);
  // `created` must survive the async boundary, not just match an empty sync.
  t.truthy(bulkAsync["build.rs"].created);
  t.regex(bulkAsync["build.rs"].created.commitId, /^[0-9a-f]{40}$/);
});

// -------- __proto__-safety regression (own-keyed result object) --------------
// The result is built with own-property DEFINE semantics, so a valid path key
// literally named `__proto__` becomes an OWN enumerable data property instead
// of triggering `Object.prototype`'s `__proto__` setter (which would corrupt
// the result object's prototype). Asserts the `Record<string, ...>` contract:
// every path is an own key, value is a FileModification or null, prototype intact.

test("getFilesLatestModified keeps a present __proto__ path as an own key (sync)", (t) => {
  const { root, repo } = makeRepo(["__proto__", "normal.txt"]);
  try {
    const result = repo.getFilesLatestModified(["__proto__", "normal.txt"]);
    t.true(Object.getOwnPropertyNames(result).includes("__proto__"));
    t.truthy(result["__proto__"]); // an own FileModification, not the prototype
    t.regex(result["__proto__"].commitId, /^[0-9a-f]{40}$/);
    // The record now also carries a `created` CommitInfo; the enrichment must
    // not corrupt the own-key define semantics (GC9). Single seed commit here,
    // so creation == modification commit.
    t.truthy(result["__proto__"].created);
    t.regex(result["__proto__"].created.commitId, /^[0-9a-f]{40}$/);
    t.true(Object.prototype.hasOwnProperty.call(result, "__proto__"));
    t.is(Object.getPrototypeOf(result), Object.prototype);
    // Normal sibling unaffected.
    t.truthy(result["normal.txt"]);
    t.true(Object.getOwnPropertyNames(result).includes("normal.txt"));
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("getFilesLatestModified keeps a missing __proto__ path as an own null key (sync)", (t) => {
  const { root, repo } = makeRepo(["normal.txt"]);
  try {
    const result = repo.getFilesLatestModified(["__proto__"]);
    t.true(Object.getOwnPropertyNames(result).includes("__proto__"));
    t.is(result["__proto__"], null); // own key, value null (never-committed)
    t.true(Object.prototype.hasOwnProperty.call(result, "__proto__"));
    t.is(Object.getPrototypeOf(result), Object.prototype);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("getFilesLatestModifiedAsync keeps __proto__ paths as own keys (async)", async (t) => {
  const present = makeRepo(["__proto__", "normal.txt"]);
  const missing = makeRepo(["normal.txt"]);
  try {
    const p = await present.repo.getFilesLatestModifiedAsync([
      "__proto__",
      "normal.txt",
    ]);
    t.true(Object.getOwnPropertyNames(p).includes("__proto__"));
    t.truthy(p["__proto__"]);
    t.true(Object.prototype.hasOwnProperty.call(p, "__proto__"));
    t.is(Object.getPrototypeOf(p), Object.prototype);

    const m = await missing.repo.getFilesLatestModifiedAsync(["__proto__"]);
    t.true(Object.getOwnPropertyNames(m).includes("__proto__"));
    t.is(m["__proto__"], null);
    t.is(Object.getPrototypeOf(m), Object.prototype);
  } finally {
    rmSync(present.root, { recursive: true, force: true });
    rmSync(missing.root, { recursive: true, force: true });
  }
});

// `constructor` and other non-`__proto__` keys were already normal shadowing
// own props; confirm the define path keeps them own + prototype intact.
test("getFilesLatestModified keeps a constructor path as an own key (sync)", (t) => {
  const { root, repo } = makeRepo(["normal.txt"]);
  try {
    const result = repo.getFilesLatestModified(["constructor"]);
    t.true(Object.getOwnPropertyNames(result).includes("constructor"));
    t.is(result["constructor"], null);
    t.true(Object.prototype.hasOwnProperty.call(result, "constructor"));
    t.is(Object.getPrototypeOf(result), Object.prototype);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// -------- multi-commit `created` semantics (the point of the feature) --------
// These build a controlled history so the oldest-first creation walk diverges
// from the newest-first modification walk. They characterize ALREADY-shipped
// behavior; each would fail if `created` regressed (e.g. collapsed onto the
// modification commit, or followed the newest add on a delete/re-add).

// Core value: `created` is the FIRST commit to add the file, distinct from the
// LAST commit to modify it. C1 adds f.txt (v1); C2 modifies it (v2). If `created`
// regressed to track the modification commit, `.created.commitId` would be C2
// and this fails.
test("created is the first-adding commit, distinct from the last modification", (t) => {
  const { root, repo, git, commits } = makeRepoWithHistory([
    { write: { "f.txt": "v1\n" }, message: "c1 add f" },
    { write: { "f.txt": "v2\n" }, message: "c2 modify f" },
  ]);
  try {
    const [c1, c2] = commits;
    t.not(c1, c2); // real modification -> distinct commits

    const mod = repo.getFileLatestModified("f.txt");
    t.truthy(mod);
    t.truthy(mod.created);
    t.is(mod.created.commitId, c1); // creation == first add (C1)
    t.is(mod.commitId, c2); // last modification == C2
    t.not(mod.created.commitId, mod.commitId); // the two genuinely differ

    // Independent CLI cross-check inside the throwaway repo (skipped under CI per
    // GC11). `--diff-filter=A --reverse` recomputes the first ADD of f.txt.
    if (!process.env.CI) {
      const firstAdd = git(
        "log --diff-filter=A --format=%H --reverse -- f.txt",
      ).split("\n")[0];
      t.is(mod.created.commitId, firstAdd);
      t.is(mod.commitId, git("log -1 --format=%H -- f.txt"));
    }
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// delete -> re-add: `created` returns the ORIGINAL add (C1), never the later
// re-add (C3). C1 adds g.txt; C2 deletes it; C3 re-adds it. The oldest-first walk
// early-exits at C1, so a regression that returned the newest add would surface.
test("created returns the ORIGINAL add across a delete-then-re-add", (t) => {
  const { root, repo, commits } = makeRepoWithHistory([
    { write: { "g.txt": "v1\n" }, message: "c1 add g" },
    { del: ["g.txt"], message: "c2 delete g" },
    { write: { "g.txt": "v3\n" }, message: "c3 re-add g" },
  ]);
  try {
    const [c1, , c3] = commits;
    t.not(c1, c3);

    const mod = repo.getFileLatestModified("g.txt");
    t.truthy(mod);
    t.truthy(mod.created);
    t.is(mod.created.commitId, c1); // the ORIGINAL add, per GC6
    t.not(mod.created.commitId, c3); // NOT the re-add
    t.is(mod.commitId, c3); // last modification is the re-add
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// Root-only file: with a single commit, the creation walk and the modification
// walk resolve the same commit, so `created.commitId === commitId`.
test("created equals the modification commit for a root-only file", (t) => {
  const { root, repo } = makeRepo(["only.txt"]);
  try {
    const mod = repo.getFileLatestModified("only.txt");
    t.truthy(mod);
    t.truthy(mod.created);
    t.regex(mod.created.commitId, /^[0-9a-f]{40}$/);
    t.is(mod.created.commitId, mod.commitId); // creation == last modification
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// Bulk parity on a multi-commit repo: the bulk creation walk must resolve the
// exact same `created` record as the single-file path (not just an empty/root
// match). Deep-equal on the whole record catches any drift in either field set.
test("bulk created matches single-file created on a multi-commit repo", (t) => {
  const { root, repo } = makeRepoWithHistory([
    { write: { "f.txt": "v1\n" }, message: "c1 add f" },
    { write: { "f.txt": "v2\n" }, message: "c2 modify f" },
  ]);
  try {
    const single = repo.getFileLatestModified("f.txt");
    const bulk = repo.getFilesLatestModified(["f.txt"])["f.txt"];
    t.truthy(single.created);
    t.deepEqual(bulk.created, single.created); // creation record crosses the bulk boundary
    t.deepEqual(bulk, single); // whole record (flat fields + created) matches
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// -------- type-transition `created` semantics (HEAD-kind gate) ---------------
// `created` is a FILE's creation; whether it resolves is decided by the queried
// path's ENTRY KIND AT HEAD, not just per-commit history. A path that is a
// TREE (directory) or GITLINK (submodule) at HEAD yields `undefined` even if a
// FILE of that name existed earlier; a DELETED file (absent at HEAD) still
// resolves its original add; a path that BECAME a file resolves that commit.

// file -> directory (THE fix): C1 adds a FILE named `dir`; C2 deletes it and
// adds `dir/a.txt`, so `dir` is a DIRECTORY (tree) at HEAD. The old FILE `dir`
// lives on in C1's tree, but because `dir` is a tree AT HEAD, `created` must be
// undefined -- NOT the bogus C1 add. (Pre-fix this returned C1.)
test("created is undefined for a path that is a file in history but a directory at HEAD", (t) => {
  const { root, repo } = makeRepoWithHistory([
    { write: { dir: "i am a file\n" }, message: "c1 add file dir" },
    {
      del: ["dir"],
      write: { "dir/a.txt": "now inside a directory\n" },
      message: "c2 dir becomes a directory",
    },
  ]);
  try {
    const dir = repo.getFileLatestModified("dir");
    t.truthy(dir); // flat fields still resolve via pathspec (the deletion delta)
    t.is(dir.created, undefined); // `dir` is a tree AT HEAD -> no creation
    t.false(Object.prototype.hasOwnProperty.call(dir, "created"));

    // The exact FILE under the directory still resolves its creation (40-hex).
    const file = repo.getFileLatestModified("dir/a.txt");
    t.truthy(file);
    t.truthy(file.created);
    t.regex(file.created.commitId, /^[0-9a-f]{40}$/);

    // Bulk form: if a record is returned for `dir` at all, its `created` must
    // also be undefined (the deletion delta gives it a flat record).
    const bulk = repo.getFilesLatestModified(["dir"]);
    const rec = bulk["dir"];
    if (rec) {
      t.is(rec.created, undefined);
      t.false(Object.prototype.hasOwnProperty.call(rec, "created"));
    }
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// deleted file stays resolved (guards against over-restriction): C1 adds
// gone.txt; C2 deletes it, so it is ABSENT at HEAD. A gate that keyed on "blob
// at HEAD" would wrongly drop this; the correct gate only rejects tree/gitlink
// AT HEAD, so the original add (C1) must still resolve.
test("created stays resolved for a file deleted at HEAD", (t) => {
  const { root, repo, commits } = makeRepoWithHistory([
    { write: { "gone.txt": "v1\n" }, message: "c1 add gone" },
    { del: ["gone.txt"], message: "c2 delete gone" },
  ]);
  try {
    const [c1] = commits;
    const mod = repo.getFileLatestModified("gone.txt");
    t.truthy(mod);
    t.truthy(mod.created); // deleted-at-HEAD file still reports its creation
    t.is(mod.created.commitId, c1); // == C1, the ORIGINAL add
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// directory -> file (compose check): C1 adds `x/a.txt` (so `x` is a DIRECTORY);
// C2 deletes `x/` and adds a FILE named `x`. At HEAD `x` is a blob, so `created`
// resolves -- to C2, the commit where `x` first became a FILE (the per-commit
// blob check skips C1's tree entry for `x`), NOT undefined.
test("created resolves to the commit where a path became a file (directory -> file)", (t) => {
  const { root, repo, commits } = makeRepoWithHistory([
    { write: { "x/a.txt": "v1\n" }, message: "c1 add x/a.txt (x is a dir)" },
    { del: ["x"], write: { x: "now a file\n" }, message: "c2 x becomes a file" },
  ]);
  try {
    const [, c2] = commits;
    const mod = repo.getFileLatestModified("x");
    t.truthy(mod);
    t.truthy(mod.created);
    t.is(mod.created.commitId, c2); // the commit where x became a FILE
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// -------- evil-merge fallback (the PR-feedback fix) --------------------------
// A file present at HEAD that ONLY merge commits ever touched (an "evil merge":
// introduced in a merge commit's tree, in neither parent) is skipped by the
// newest-first modification walk (merges are skipped), so the whole record used
// to be `null`. The fix falls back to the file's CREATING commit (the merge)
// for the entire record, so `commitId === created.commitId === the merge OID`.
// The modification walk itself is unchanged, so a normal branch-merged file
// still resolves to its 1-parent add, and a never-committed path stays `null`.

// THE bug: evil-merge-only file must resolve (was null pre-fix). Both the flat
// record and `created` are the merge commit.
test("getFileLatestModified falls back to the merge commit for an evil-merge-only file", (t) => {
  const { root, repo, mergeCommit } = makeRepoWithEvilMerge();
  try {
    const mod = repo.getFileLatestModified("evil.txt");
    t.truthy(mod); // pre-fix this was null -- the reported gap
    t.is(mod.commitId, mergeCommit); // the whole record IS the creating merge
    t.truthy(mod.created);
    t.is(mod.created.commitId, mergeCommit); // created is that same merge commit
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// Bulk mirrors single-file: the bulk fallback builds the identical record.
test("getFilesLatestModified matches the single-file result for an evil-merge-only file", (t) => {
  const { root, repo } = makeRepoWithEvilMerge();
  try {
    const single = repo.getFileLatestModified("evil.txt");
    const bulk = repo.getFilesLatestModified(["evil.txt"])["evil.txt"];
    t.truthy(bulk);
    t.deepEqual(bulk, single); // whole record (flat fields + created) matches
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// Regression guard: a normal branch-merged file (arrived via a 1-parent add on
// the merged branch) still resolves to that ADD commit, NOT the merge, with
// `created` the same add commit. The fix must not perturb the modification walk.
test("a normal branch-merged file resolves to its 1-parent add, not the merge", (t) => {
  const { root, repo, onBranchCommit, mergeCommit } = makeRepoWithEvilMerge();
  try {
    const mod = repo.getFileLatestModified("on-branch.txt");
    t.truthy(mod);
    t.is(mod.commitId, onBranchCommit); // the branch's 1-parent add
    t.not(mod.commitId, mergeCommit); // NOT the merge commit
    t.truthy(mod.created);
    t.is(mod.created.commitId, onBranchCommit); // created is that same add
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// Guard the fallback does not over-trigger: a path that no commit ever added
// stays `null` in both the single-file and bulk forms.
test("evil-merge fallback stays null for a never-committed path", (t) => {
  const { root, repo } = makeRepoWithEvilMerge();
  try {
    t.is(repo.getFileLatestModified("nope-zzz.txt"), null);
    t.is(repo.getFilesLatestModified(["nope-zzz.txt"])["nope-zzz.txt"], null);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// Documented divergence: the standalone date methods keep the raw merge-skipping
// walk and do NOT apply the merge-only fallback. So for an evil-merge-only file
// present at HEAD, getFileLatestModified RESOLVES (committerTime = the merge),
// while getFileLastModifiedDate returns null and getFileLatestModifiedDate throws.
// Locks that asymmetry: routing the date methods through the fallback would fail
// this and force a docs update (see committerTime / getFileLastModifiedDate docs).
test("date-twin methods keep raw merge-skipping semantics for an evil-merge-only file", (t) => {
  const { root, repo } = makeRepoWithEvilMerge();
  try {
    const mod = repo.getFileLatestModified("evil.txt");
    t.truthy(mod); // the rich accessor resolves via the merge-only fallback
    t.true(mod.committerTime instanceof Date);

    // The cheap date twins do NOT apply the fallback: null / throw.
    t.is(repo.getFileLastModifiedDate("evil.txt"), null);
    t.throws(() => repo.getFileLatestModifiedDate("evil.txt"));
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// THE fix: an evil-merge file that was later DELETED by a merge is ABSENT at HEAD.
// The modification walk finds nothing (both the add and the delete are merges it
// skips), but `get_file_creation` still resolves the c3 add-merge -- so the
// UNGATED fallback wrongly reported that stale add-merge as the whole record.
// Gating the fallback on the path being a blob AT HEAD makes an absent (deleted)
// path stay `null` in both the single-file and bulk forms.
test("evil-merge fallback stays null for a file deleted by a later merge", (t) => {
  const { root, repo, addMergeCommit, deleteMergeCommit } =
    makeRepoWithEvilMergeThenDelete();
  try {
    t.not(addMergeCommit, deleteMergeCommit); // sanity: two distinct merges

    // Single-file: absent at HEAD -> null, NOT the stale add-merge record.
    t.is(repo.getFileLatestModified("evil.txt"), null);

    // Bulk: same gate on the None-slot fallback.
    t.is(repo.getFilesLatestModified(["evil.txt"])["evil.txt"], null);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// -------- double-evil-merge: flat = LATEST change, created = OLDEST (the P2) ---
// A file touched ONLY by merge commits, where ONE evil merge ADDS it (M1="v1")
// and a LATER, different evil merge CHANGES it (M2="v2"), still present at HEAD.
// The merge-skipping modification walk finds nothing, so the fallback builds the
// whole record. Pre-fix it built EVERYTHING from the CREATION (M1), so `commitId`
// wrongly pointed at the original add rather than the merge that last changed the
// file. Post-fix the FLAT fields are M2 (the latest genuine change) while
// `created` stays M1 (the creation) -- distinct commits.

// THE P2 bug: flat fields must be M2 (latest change), created must be M1
// (creation), and the two must differ. Pre-fix commitId was M1 (== created).
test("getFileLatestModified flat fields are the LATEST merge, created is the creating merge", (t) => {
  const { root, repo, git, addMergeCommit, changeMergeCommit } =
    makeRepoWithDoubleEvilMerge();
  try {
    t.not(addMergeCommit, changeMergeCommit); // sanity: two distinct merges
    t.is(git("show HEAD:evil.txt"), "v2"); // HEAD content is M2's "v2"

    const mod = repo.getFileLatestModified("evil.txt");
    t.truthy(mod);
    t.is(mod.commitId, changeMergeCommit); // flat = the LATEST change (M2), NOT M1
    t.truthy(mod.created);
    t.is(mod.created.commitId, addMergeCommit); // created = the creation (M1)
    t.not(mod.commitId, mod.created.commitId); // genuinely distinct
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// Bulk mirrors single-file: the bulk fallback builds the identical split record.
test("getFilesLatestModified matches single-file for a double-evil-merge file", (t) => {
  const { root, repo, addMergeCommit, changeMergeCommit } =
    makeRepoWithDoubleEvilMerge();
  try {
    const single = repo.getFileLatestModified("evil.txt");
    const bulk = repo.getFilesLatestModified(["evil.txt"])["evil.txt"];
    t.truthy(bulk);
    t.is(bulk.commitId, changeMergeCommit); // flat = latest change (M2)
    t.is(bulk.created.commitId, addMergeCommit); // created = creation (M1)
    t.deepEqual(bulk, single); // whole record (flat fields + created) matches
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
