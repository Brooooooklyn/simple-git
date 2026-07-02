import { execSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import test from "ava";

import { Repository } from "../index.js";

// These tests MUTATE `.git/MERGE_MSG`, so each uses a throwaway repo under
// os.tmpdir(). Caller cleans up `root` in a finally block.
function makeRepo() {
  const root = mkdtempSync(join(tmpdir(), "simple-git-merge-msg-"));
  const work = join(root, "work");
  execSync(`git init -q -b main "${work}"`);
  const run = (args) => execSync(`git ${args}`, { cwd: work });
  run("config user.name tester");
  run("config user.email tester@example.com");
  run("config commit.gpgsign false");
  run("config core.autocrlf false");
  const repo = new Repository(work);
  return { root, work, repo };
}

// mergeMessage() reads libgit2's MERGE_MSG (`.git/MERGE_MSG`). Seeding that file
// (as a merge would) must round-trip through the binding unchanged.
test("mergeMessage round-trips the MERGE_MSG file", (t) => {
  const { root, work, repo } = makeRepo();
  try {
    const body = "Merge branch 'feature' into main\n";
    writeFileSync(join(work, ".git", "MERGE_MSG"), body);
    t.is(repo.mergeMessage(), body);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// removeMergeMessage() deletes MERGE_MSG; afterwards reading it is a NotFound
// error (there is no merge message to retrieve).
test("removeMergeMessage clears the MERGE_MSG file", (t) => {
  const { root, work, repo } = makeRepo();
  try {
    writeFileSync(join(work, ".git", "MERGE_MSG"), "to be removed\n");
    t.is(repo.mergeMessage(), "to be removed\n");

    repo.removeMergeMessage();
    t.throws(() => repo.mergeMessage());
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
