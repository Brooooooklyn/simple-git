import { execSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import test from "ava";

import { BranchType, Repository, Signature } from "../index.js";

const __dirname = join(fileURLToPath(import.meta.url), "..");
const workDir = join(__dirname, "..");

// Reading branches is non-mutating, so the project's own repo is used (like
// blame.spec.mjs). It always has at least the current local branch.
function projectRepo() {
  return new Repository(workDir);
}

// Mutating cases (branch create / delete) MUST use a throwaway repo so the
// project's own refs are never touched. Caller cleans up `root`.
function makeRepo() {
  const root = mkdtempSync(join(tmpdir(), "simple-git-branch-"));
  const work = join(root, "work");
  execSync(`git init -q -b main "${work}"`);
  const run = (args) => execSync(`git ${args}`, { cwd: work });
  run("config user.name tester");
  run("config user.email tester@example.com");
  run("config commit.gpgsign false");
  const repo = new Repository(work);
  return { root, work, repo };
}

const sig = () => Signature.now("tester", "tester@example.com");

// Create one root commit in a temp repo and return the head commit's OID.
function seedCommit(repo, work) {
  writeFileSync(join(work, "a.txt"), "alpha\n");
  const index = repo.index();
  index.addAll();
  index.write();
  const tree = repo.findTree(index.writeTree());
  const author = sig();
  return repo.commit("HEAD", author, author, "root", tree);
}

test("branches returns a non-empty array of named Branch objects", (t) => {
  const repo = projectRepo();
  const branches = repo.branches();
  t.true(Array.isArray(branches));
  t.true(branches.length > 0, "project repo has at least one branch");
  const named = branches.filter((b) => b.name() != null);
  t.true(named.length > 0, "at least one branch has a non-null name");
});

test("exactly one local branch is HEAD when HEAD is on a branch", (t) => {
  const repo = projectRepo();
  // Only meaningful when HEAD points at a branch (not detached).
  if (!repo.head().isBranch()) {
    t.pass("HEAD is detached; skipping isHead count");
    return;
  }
  const locals = repo.branches(BranchType.Local);
  const heads = locals.filter((b) => b.isHead());
  t.is(heads.length, 1, "exactly one local branch is HEAD");
});

test("findBranch finds the current branch and returns null for missing", (t) => {
  const repo = projectRepo();
  if (!repo.head().isBranch()) {
    t.pass("HEAD is detached; skipping findBranch current");
    return;
  }
  const current = repo.head().shorthand();
  t.truthy(current);
  const found = repo.findBranch(current, BranchType.Local);
  t.truthy(found, "current branch is found");
  t.is(found.name(), current);

  const missing = repo.findBranch("definitely-missing-xyz", BranchType.Local);
  t.is(missing, null, "missing branch returns null");
});

test("get() returns the resolved Reference for the current branch", (t) => {
  const repo = projectRepo();
  if (!repo.head().isBranch()) {
    t.pass("HEAD is detached; skipping get() reference check");
    return;
  }
  const current = repo.head().shorthand();
  t.truthy(current);
  const branch = repo.findBranch(current, BranchType.Local);
  t.truthy(branch, "current branch is found");
  const ref = branch.get();
  t.is(ref.shorthand(), repo.head().shorthand());
});

test("branch() creates a branch and delete() removes it", (t) => {
  const { root, work, repo } = makeRepo();
  try {
    const headOid = seedCommit(repo, work);
    const headCommit = repo.findCommit(headOid);
    t.truthy(headCommit);

    const branch = repo.branch("feature", headCommit, false);
    t.is(branch.name(), "feature");
    t.false(branch.isHead(), "newly created branch is not HEAD");

    const listed = execSync("git branch --list feature", { cwd: work })
      .toString()
      .trim();
    t.not(listed, "", "git sees the created branch");

    // It is now findable through the API too.
    t.truthy(repo.findBranch("feature", BranchType.Local));

    branch.delete();
    const afterDelete = execSync("git branch --list feature", { cwd: work })
      .toString()
      .trim();
    t.is(afterDelete, "", "branch is gone after delete()");
    t.is(repo.findBranch("feature", BranchType.Local), null);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// A branch with no tracking config has no upstream: upstream() returns null.
// This pins the "not found => null" path after the error-handling fix, which
// now only collapses the genuine NotFound case to null and propagates real
// libgit2 errors (corrupt refs, unreadable config) as thrown errors instead.
test("upstream returns null when the branch has no configured upstream", (t) => {
  const { root, work, repo } = makeRepo();
  try {
    seedCommit(repo, work);
    const main = repo.findBranch("main", BranchType.Local);
    t.truthy(main, "the seeded main branch is found");
    t.is(main.upstream(), null, "no tracking branch => null, not a throw");
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
