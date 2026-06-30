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

// Test #2 — async matches sync.
test("getFileLatestModificationAsync matches sync result", async (t) => {
  const { repo } = t.context;
  const sync = repo.getFileLatestModification("build.rs");
  const asyncResult = await repo.getFileLatestModificationAsync("build.rs");
  t.deepEqual(asyncResult, sync);

  // Async missing path resolves to null (mirrors sync null-on-missing, no throw).
  t.is(
    await repo.getFileLatestModificationAsync("does-not-exist-xyz.nope"),
    null,
  );
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
