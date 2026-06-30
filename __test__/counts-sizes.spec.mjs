import { execSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import test from "ava";

import { Repository } from "../index.js";

// All count/size getters must return a plain JS `number` (not `bigint`), so that
// ordinary arithmetic (`x + 1`) works without a "Cannot mix BigInt" TypeError.
// Each test runs on a throwaway repo under os.tmpdir(); caller cleans up `root`.
// The repo has two commits on `main`: `file.txt` is "v1\n" then "v2\n" (HEAD).
function makeRepo() {
  const root = mkdtempSync(join(tmpdir(), "simple-git-counts-"));
  const work = join(root, "work");
  execSync(`git init -q -b main "${work}"`);
  const run = (args) => execSync(`git ${args}`, { cwd: work });
  run("config user.name tester");
  run("config user.email tester@example.com");
  run("config commit.gpgsign false");
  run("config core.autocrlf false");

  writeFileSync(join(work, "file.txt"), "v1\n");
  run("add file.txt");
  run("commit -q -m first");
  const firstOid = run("rev-parse HEAD").toString().trim();

  writeFileSync(join(work, "file.txt"), "v2\n");
  run("add file.txt");
  run("commit -q -m second");
  const headOid = run("rev-parse HEAD").toString().trim();

  const repo = new Repository(work);
  return { root, repo, firstOid, headOid };
}

test("Tree.size() is a plain number and supports arithmetic", (t) => {
  const { root, repo } = makeRepo();
  try {
    const tree = repo.head().peelToTree();
    const size = tree.size();
    t.is(typeof size, "number");
    t.is(size, 1); // single entry: file.txt
    t.is(size + 1, 2); // no BigInt TypeError
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("Commit.parentCount() is a plain number and supports arithmetic", (t) => {
  const { root, repo, firstOid, headOid } = makeRepo();
  try {
    const head = repo.findCommit(headOid);
    const headParents = head.parentCount();
    t.is(typeof headParents, "number");
    t.is(headParents, 1);
    t.is(headParents + 1, 2); // no BigInt TypeError

    const root_ = repo.findCommit(firstOid);
    t.is(typeof root_.parentCount(), "number");
    t.is(root_.parentCount(), 0); // root commit
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("Blob.size() is a plain number", (t) => {
  const { root, repo } = makeRepo();
  try {
    const blob = repo
      .head()
      .peelToTree()
      .getPath("file.txt")
      .toObject(repo)
      .peelToBlob();
    const size = blob.size();
    t.is(typeof size, "number");
    t.is(size, 3); // "v2\n"
    t.is(size + 1, 4); // no BigInt TypeError
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("DiffFile.size() is a plain number", (t) => {
  const { root, repo } = makeRepo();
  try {
    // Modify the tracked file, then diff the HEAD tree against the workdir so
    // file.txt is guaranteed to show up as a "modified" delta.
    writeFileSync(join(root, "work", "file.txt"), "a longer third version\n");
    const diff = repo.diffTreeToWorkdir(repo.head().peelToTree());
    let checked = 0;
    for (const delta of diff.deltas()) {
      const size = delta.newFile().size();
      t.is(typeof size, "number");
      t.is(typeof delta.oldFile().size(), "number");
      checked += 1;
    }
    t.true(checked >= 1);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
