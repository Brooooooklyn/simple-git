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
test("getFileLatestModified returns enriched metadata", (t) => {
  const { repo } = t.context;
  const mod = repo.getFileLatestModified("build.rs");
  t.truthy(mod);

  // Delegation guard (runs unconditionally; two native methods). committerTime
  // is the same instant getFileLatestModifiedDate returns -- both now Date.
  t.true(mod.committerTime instanceof Date);
  t.is(
    mod.committerTime.getTime(),
    repo.getFileLatestModifiedDate("build.rs").getTime(),
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

// Test #2b — getFileLatestModifiedDateAsync (GitDateTask) matches its sync
// sibling. Covers the previously-untested async date path; both return a Date
// for the same instant.
test("getFileLatestModifiedDateAsync matches sync result", async (t) => {
  const { repo } = t.context;
  const sync = repo.getFileLatestModifiedDate("build.rs");
  const asyncResult = await repo.getFileLatestModifiedDateAsync("build.rs");
  t.true(asyncResult instanceof Date);
  t.is(asyncResult.getTime(), sync.getTime());
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
    repo.getFileLatestModifiedDate("LICENSE").getTime(),
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
