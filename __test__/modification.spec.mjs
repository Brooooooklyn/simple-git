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
  t.deepEqual(result["build.rs"], repo.getFileLatestModified("build.rs"));
  t.deepEqual(result["Cargo.toml"], repo.getFileLatestModified("Cargo.toml"));
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
