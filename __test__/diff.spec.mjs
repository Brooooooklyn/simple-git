import { execSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import test from "ava";

import { Repository } from "../index.js";

// Mutating setup, so each test runs against a throwaway repo under os.tmpdir().
// The committed file is left UNTOUCHED in the workdir, so the workdir matches
// HEAD exactly (no real changes).
function makeRepoWithUnmodifiedFile() {
  const root = mkdtempSync(join(tmpdir(), "simple-git-diff-"));
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
  const repo = new Repository(work);
  return { root, work, repo };
}

const deltaCount = (diff) => [...diff.deltas()].length;

// DiffOptions.showUnmodified threads through to git2's `show_unmodified`: when a
// tracked file is unmodified in the workdir, the default diff skips it (0
// deltas), but `{ showUnmodified: true }` surfaces it as a delta. The flag must
// therefore strictly increase the delta count.
test("diffTreeToWorkdir respects DiffOptions.showUnmodified", (t) => {
  const { root, repo } = makeRepoWithUnmodifiedFile();
  try {
    const headTree = repo.head().peelToTree();

    const withoutFlag = deltaCount(repo.diffTreeToWorkdir(headTree));
    const withFalse = deltaCount(
      repo.diffTreeToWorkdir(headTree, { showUnmodified: false }),
    );
    const withTrue = deltaCount(
      repo.diffTreeToWorkdir(headTree, { showUnmodified: true }),
    );

    // The clean workdir yields no deltas by default.
    t.is(withoutFlag, 0);
    t.is(withFalse, 0);
    // Enabling the flag includes the unmodified tracked file.
    t.true(withTrue > withoutFlag);
    t.true(withTrue >= 1);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// Same wiring for the index-aware variant.
test("diffTreeToWorkdirWithIndex respects DiffOptions.showUnmodified", (t) => {
  const { root, repo } = makeRepoWithUnmodifiedFile();
  try {
    const headTree = repo.head().peelToTree();

    const withoutFlag = deltaCount(repo.diffTreeToWorkdirWithIndex(headTree));
    const withTrue = deltaCount(
      repo.diffTreeToWorkdirWithIndex(headTree, { showUnmodified: true }),
    );

    t.is(withoutFlag, 0);
    t.true(withTrue > withoutFlag);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
