import { execSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import test from "ava";

import { Repository, Signature } from "../index.js";

// All tests here MUTATE a repository (staging, writing trees, committing), so
// every one operates on a throwaway repo under os.tmpdir() and never touches
// the project's own repo. Caller cleans up `root` in a finally block.
function makeRepo() {
  const root = mkdtempSync(join(tmpdir(), "simple-git-index-"));
  const work = join(root, "work");
  execSync(`git init -q -b main "${work}"`);
  const run = (args) => execSync(`git ${args}`, { cwd: work });
  run("config user.name tester");
  run("config user.email tester@example.com");
  // Keep the temp repo hermetic: never inherit a global commit.gpgsign that
  // would make `git commit` fail (or block on a signing agent).
  run("config commit.gpgsign false");
  const repo = new Repository(work);
  return { root, work, repo };
}

const sig = () => Signature.now("tester", "tester@example.com");

// Staging a file then writing the index out as a tree yields a real tree OID
// that the repository can resolve.
test("addAll + write + writeTree returns a resolvable tree OID", (t) => {
  const { root, work, repo } = makeRepo();
  try {
    writeFileSync(join(work, "a.txt"), "alpha\n");
    const index = repo.index();
    index.addAll();
    index.write();
    t.true(index.count() >= 1);
    const treeOid = index.writeTree();
    t.is(treeOid.length, 40);
    t.truthy(repo.findTree(treeOid));
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// End-to-end: a root commit, then a second commit whose parent is the first.
// `parents: [firstOid]` must wire up real history (verified through git).
test("commit with parents builds a two-commit history", (t) => {
  const { root, work, repo } = makeRepo();
  try {
    const author = sig();

    writeFileSync(join(work, "a.txt"), "alpha\n");
    let index = repo.index();
    index.addAll();
    index.write();
    const firstTree = repo.findTree(index.writeTree());
    const firstOid = repo.commit("HEAD", author, author, "first", firstTree);
    t.is(firstOid.length, 40);

    writeFileSync(join(work, "b.txt"), "beta\n");
    index = repo.index();
    index.addAll();
    index.write();
    const secondTree = repo.findTree(index.writeTree());
    const secondOid = repo.commit("HEAD", author, author, "second", secondTree, [
      firstOid,
    ]);

    const log = execSync("git log --oneline", { cwd: work })
      .toString()
      .trim()
      .split("\n");
    t.is(log.length, 2);

    // `git rev-list --parents -n 1 <oid>` prints "<oid> <parent>...": no shell
    // metacharacters (unlike `<oid>^`, whose caret cmd.exe eats on Windows).
    const lineage = execSync(`git rev-list --parents -n 1 ${secondOid}`, {
      cwd: work,
    })
      .toString()
      .trim()
      .split(/\s+/);
    t.deepEqual(lineage, [secondOid, firstOid]);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// blob() writes the raw bytes into the object database and returns git's own
// content hash (cross-checked against `git hash-object`).
test("blob returns the git hash-object id of the bytes", (t) => {
  const { root, repo } = makeRepo();
  try {
    const oid = repo.blob(Buffer.from("hello"));
    const expected = execSync("git hash-object --stdin", { input: "hello" })
      .toString()
      .trim();
    t.is(oid, expected);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// Regression for the commit() signature change: omitting `parents` (the
// original 5-argument call) must still produce a parent-less root commit.
test("commit without parents still makes a root commit", (t) => {
  const { root, work, repo } = makeRepo();
  try {
    const author = sig();
    writeFileSync(join(work, "a.txt"), "alpha\n");
    const index = repo.index();
    index.addAll();
    index.write();
    const tree = repo.findTree(index.writeTree());
    const oid = repo.commit("HEAD", author, author, "root", tree);
    t.is(oid.length, 40);

    const log = execSync("git log --oneline", { cwd: work })
      .toString()
      .trim()
      .split("\n");
    t.is(log.length, 1);
    // A root commit has no parent: `git rev-list --parents` lists only the
    // commit itself, with no parent OIDs following it.
    const lineage = execSync(`git rev-list --parents -n 1 ${oid}`, { cwd: work })
      .toString()
      .trim()
      .split(/\s+/);
    t.deepEqual(lineage, [oid]);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// Time round-trip: a `new Signature(name, email, Date)` records the instant at
// whole-second resolution and reads back identically as a `Date` through the
// commit's author()/committer()/time().
test("Signature Date round-trips through commit author/committer time", (t) => {
  const { root, work, repo } = makeRepo();
  try {
    // Whole-second instant (no sub-second ms): git stores epoch seconds.
    const when = new Date("2021-06-15T12:34:56.000Z");
    const author = new Signature("Round Trip", "round@example.com", when);

    writeFileSync(join(work, "a.txt"), "alpha\n");
    const index = repo.index();
    index.addAll();
    index.write();
    const tree = repo.findTree(index.writeTree());
    const oid = repo.commit("HEAD", author, author, "with known time", tree);

    const commit = repo.findCommit(oid);
    t.truthy(commit);

    const authorWhen = commit.author().when();
    const committerWhen = commit.committer().when();
    const commitTime = commit.time();
    t.true(authorWhen instanceof Date);
    t.true(committerWhen instanceof Date);
    t.true(commitTime instanceof Date);

    t.is(authorWhen.getTime(), when.getTime());
    t.is(committerWhen.getTime(), when.getTime());
    t.is(commitTime.getTime(), when.getTime());

    // The standalone Signature also reads its own time back unchanged.
    t.is(author.when().getTime(), when.getTime());
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
