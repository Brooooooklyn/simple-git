import { execSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import test from "ava";

import { Repository, Signature } from "../index.js";

// Spin up an isolated repo under os.tmpdir() with one commit that stages a file,
// so every derived handle can be materialized. Caller cleans up.
function makeTempRepo() {
  const dir = mkdtempSync(join(tmpdir(), "simple-git-dispose-hardening-"));
  const run = (args) => execSync(`git ${args}`, { cwd: dir });
  run("init -q");
  run("config user.name tester");
  run("config user.email tester@example.com");
  run("config commit.gpgsign false");
  run("config core.autocrlf false");
  writeFileSync(join(dir, "committed.txt"), "v1\n");
  run("add committed.txt");
  run("commit -q -m initial");
  return dir;
}

const DISPOSED = { message: /disposed/ };

// The completeness gate: obtain a LIVE handle of every one of the 16 derived
// types, dispose the repo (freeing the underlying git2::Repository), then prove
// each guarded method THROWS "Repository has been disposed" instead of
// dereferencing the freed object. A missed guard segfaults this worker (loud
// crash) rather than failing softly — that is the safety net.
test("every derived handle throws /disposed/ after repo.dispose()", (t) => {
  const dir = makeTempRepo();
  try {
    const headOid = execSync("git rev-parse HEAD", { cwd: dir })
      .toString()
      .trim();
    const repo = new Repository(dir);

    // --- Obtain a live handle of every derived type BEFORE disposal. ---
    const reference = repo.head(); // Reference
    const branch = repo.branches()[0]; // Branch
    t.truthy(branch, "sanity: a local branch exists");
    const commit = repo.findCommit(headOid); // Commit
    const tree = commit.tree(); // Tree (rooted at Commit)
    const treeEntry = tree.getName("committed.txt"); // TreeEntry (Ref variant)
    t.truthy(treeEntry, "sanity: committed.txt entry exists");
    const commitObj = commit.asObject(); // GitObject (owned)
    const signature = commit.author(); // Signature (repo-tied, FromCommit)
    const blobObj = treeEntry.toObject(repo); // GitObject (Repository variant)
    const blob = blobObj.peelToBlob(); // Blob
    const remote = repo.remote("origin", "https://example.com/repo.git"); // Remote

    const tagger = Signature.now("tester", "tester@example.com");
    const tagOid = repo.tag("v1", commitObj, tagger, "annotated", false);
    const tag = repo.findTag(tagOid); // Tag
    t.truthy(tag, "sanity: annotated tag resolves");

    // Diff / Deltas / DiffDelta / DiffFile — make the workdir differ from the
    // committed tree so a delta actually exists.
    writeFileSync(join(dir, "committed.txt"), "v2\n");
    const diff = repo.diffTreeToWorkdir(tree); // Diff
    const deltaList = [...diff.deltas()]; // consumes one Deltas iterator
    t.true(deltaList.length > 0, "sanity: a delta exists before disposal");
    const diffDelta = deltaList[0]; // DiffDelta
    const diffFile = diffDelta.oldFile(); // DiffFile

    // The three iterators: obtained live, iterated AFTER disposal.
    const revWalk = repo.revWalk(); // RevWalk
    revWalk.pushHead();
    const treeIter = tree.iter(); // TreeIter
    const deltasIter = diff.deltas(); // Deltas (a fresh, unconsumed one)

    // --- Free the underlying git2::Repository. ---
    repo.dispose();

    // 13 throwing derived handles (representative method each).
    t.throws(() => reference.isBranch(), DISPOSED, "Reference");
    t.throws(() => branch.name(), DISPOSED, "Branch");
    t.throws(() => commit.id(), DISPOSED, "Commit");
    t.throws(() => tree.id(), DISPOSED, "Tree");
    t.throws(() => treeEntry.id(), DISPOSED, "TreeEntry");
    t.throws(() => commitObj.id(), DISPOSED, "GitObject");
    t.throws(() => signature.name(), DISPOSED, "Signature (from commit)");
    t.throws(() => blob.id(), DISPOSED, "Blob");
    t.throws(() => remote.name(), DISPOSED, "Remote");
    // Remote async: the guard is a SYNCHRONOUS pre-throw (the call throws, it
    // does not return a rejected promise).
    t.throws(() => remote.fetchAsync([]), DISPOSED, "Remote.fetchAsync");
    t.throws(() => tag.id(), DISPOSED, "Tag");
    t.throws(() => diff.isSortedIcase(), DISPOSED, "Diff");
    t.throws(() => diffDelta.status(), DISPOSED, "DiffDelta");
    t.throws(() => diffFile.id(), DISPOSED, "DiffFile");

    // 3 iterators can't throw (Generator::next returns Option), so on disposal
    // they terminate safely (yield nothing) instead — proving no UAF deref.
    t.deepEqual([...revWalk], [], "RevWalk terminates safely");
    t.deepEqual([...treeIter], [], "TreeIter terminates safely");
    t.deepEqual([...deltasIter], [], "Deltas terminates safely");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// Positive control: the guard must not break the live path. Handles used before
// disposal return real values, and a standalone (non-repo-tied) Signature stays
// valid even after the repo is disposed.
test("handles work before dispose; standalone signature survives dispose", (t) => {
  const dir = makeTempRepo();
  try {
    const headOid = execSync("git rev-parse HEAD", { cwd: dir })
      .toString()
      .trim();
    const repo = new Repository(dir);

    const commit = repo.findCommit(headOid);
    t.is(commit.id(), headOid, "Commit.id() returns the real oid before dispose");
    const reference = repo.head();
    t.is(typeof reference.isBranch(), "boolean");
    const tree = commit.tree();
    t.true(tree.size() >= 1);

    // A standalone signature borrows no repository — its liveness flag is never
    // flipped, so it must keep working after the repo is disposed.
    const sig = Signature.now("tester", "tester@example.com");
    repo.dispose();
    t.is(sig.name(), "tester", "standalone Signature unaffected by dispose");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
