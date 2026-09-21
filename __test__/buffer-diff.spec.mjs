import { execSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import test from "ava";

import { Repository, diffBuffers } from "../index.js";

const enc = (s) => new TextEncoder().encode(s);

// Mutating setup, so each test runs against a throwaway repo under os.tmpdir().
function makeRepoWithModifiedFile() {
  const root = mkdtempSync(join(tmpdir(), "simple-git-buffer-diff-"));
  const work = join(root, "work");
  execSync(`git init -q -b main "${work}"`);
  const run = (args) => execSync(`git ${args}`, { cwd: work });
  run("config user.name tester");
  run("config user.email tester@example.com");
  run("config commit.gpgsign false");
  run("config core.autocrlf false");
  writeFileSync(join(work, "tracked.txt"), "stable\n");
  run("add tracked.txt");
  run('commit -q -m "initial commit"');
  // Unstaged workdir modification for diffTreeToWorkdir to pick up.
  writeFileSync(join(work, "tracked.txt"), "stable\nchanged\n");
  const repo = new Repository(work);
  return { root, repo };
}

test("diffBuffers: identical buffers produce an empty patch", (t) => {
  const result = diffBuffers(enc("same\n"), "a.txt", enc("same\n"), "a.txt");

  t.is(result.hunks.length, 0);
  t.is(result.stats.additions, 0);
  t.is(result.stats.deletions, 0);
  t.is(result.stats.filesChanged, 0);
  // No content line may start with a single +/- sigil (the `---`/`+++` file
  // headers are excluded by requiring a non-+/- character next).
  t.false(/^[+-][^+-]/m.test(result.patch));
});

test("diffBuffers: both sides null produce an empty patch", (t) => {
  const result = diffBuffers(null, null, null, null);

  t.is(result.hunks.length, 0);
  t.is(result.stats.additions, 0);
  t.is(result.stats.deletions, 0);
  t.is(result.stats.filesChanged, 0);
  t.is(result.patch, "");
});

test("diffBuffers: modified content yields expected +/- lines", (t) => {
  const result = diffBuffers(
    enc("one\ntwo\nthree\n"),
    "old.txt",
    enc("one\nTWO\nthree\n"),
    "new.txt",
  );

  t.is(result.hunks.length, 1);

  const hunk = result.hunks[0];
  t.true(hunk.header.includes("@@"));
  t.is(hunk.oldStart, 1);
  t.is(hunk.newStart, 1);

  const origins = hunk.lines.map((l) => l.origin);
  t.true(origins.includes("-"));
  t.true(origins.includes("+"));

  const deleted = hunk.lines.find((l) => l.origin === "-");
  const added = hunk.lines.find((l) => l.origin === "+");
  t.is(deleted.newLineno, null);
  t.is(added.oldLineno, null);
  t.is(deleted.content.toString(), "two\n");
  t.is(added.content.toString(), "TWO\n");

  t.true(result.patch.includes("-two"));
  t.true(result.patch.includes("+TWO"));
  t.true(result.patch.includes("@@"));
  t.is(result.stats.additions, 1);
  t.is(result.stats.deletions, 1);
  t.is(result.stats.filesChanged, 1);
});

test("diffBuffers: null old side reports every line as added", (t) => {
  const result = diffBuffers(null, null, enc("one\ntwo\n"), "new.txt");

  t.is(result.hunks.length, 1);
  const origins = result.hunks[0].lines.map((l) => l.origin);
  t.true(origins.length > 0);
  t.true(origins.every((o) => o === "+"));
  t.is(result.stats.additions, 2);
  t.is(result.stats.deletions, 0);
  t.is(result.stats.filesChanged, 1);
});

test("diffBuffers: null new side reports every line as deleted", (t) => {
  const result = diffBuffers(enc("one\ntwo\n"), "old.txt", null, null);

  t.is(result.hunks.length, 1);
  const origins = result.hunks[0].lines.map((l) => l.origin);
  t.true(origins.length > 0);
  t.true(origins.every((o) => o === "-"));
  t.is(result.stats.additions, 0);
  t.is(result.stats.deletions, 2);
  t.is(result.stats.filesChanged, 1);
});

test("Diff.toPatch returns unified diff text for a workdir change", (t) => {
  const { root, repo } = makeRepoWithModifiedFile();
  try {
    const headTree = repo.head().peelToTree();
    const patch = repo.diffTreeToWorkdir(headTree).toPatch();

    t.true(patch.length > 0);
    t.true(patch.includes("diff --git"));
    t.true(patch.includes("@@"));
    t.true(patch.includes("+changed"));
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
