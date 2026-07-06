import { execSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
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
    writeFileSync(join(work, name), `${name}\n`);
  }
  run("add -A");
  run("commit -q -m seed");
  return { root, repo: new Repository(work) };
}

// Build a hermetic throwaway repo with a controlled MULTI-commit history so the
// creation walk (`created`, oldest-first) can diverge from the modification walk
// (newest-first). Each `step` is `{ write?: {name: content}, del?: [names],
// message }`: it writes/deletes the named files, stages everything (`add -A`),
// then commits. Same init/config pattern as `makeRepo`. Returns `{ root, repo,
// git, commits }` where `commits[i]` is the 40-hex OID of step `i`'s commit
// (captured via `git rev-parse HEAD` right after it lands, oldest first) and
// `git` runs a command in the work tree for optional CLI cross-checks. Caller
// removes `root`.
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
    for (const [name, content] of Object.entries(step.write ?? {})) {
      writeFileSync(join(work, name), content);
    }
    for (const name of step.del ?? []) {
      rmSync(join(work, name), { force: true });
    }
    run("add -A");
    run(`commit -q -m "${step.message}"`);
    commits.push(capture("rev-parse HEAD"));
  }
  return { root, repo: new Repository(work), git: capture, commits };
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
