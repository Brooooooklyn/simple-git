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

// Arg-side dispose UAF: methods that accept a DERIVED HANDLE as an argument must
// guard the ARGUMENT's own liveness, not just the receiver's. Passing a handle
// obtained from a DIFFERENT repository that was then disposed must THROW
// "Repository has been disposed" instead of dereferencing freed git2 state —
// newly reachable because dispose() can free repoB while repoA stays live. A
// missed arg-guard SEGFAULTS this worker (loud crash), which is the safety net.
test("disposed-repo handle passed as an argument throws /disposed/", (t) => {
  const dirA = makeTempRepo();
  const dirB = makeTempRepo();
  try {
    const oidA = execSync("git rev-parse HEAD", { cwd: dirA })
      .toString()
      .trim();
    const oidB = execSync("git rev-parse HEAD", { cwd: dirB })
      .toString()
      .trim();

    const repoA = new Repository(dirA);
    const repoB = new Repository(dirB);

    // Live handles from repoA (the receiver stays alive throughout).
    const commitA = repoA.findCommit(oidA);
    const treeA = commitA.tree();
    const authorA = commitA.author();
    const committerA = commitA.author();
    const objA = commitA.asObject();

    // Repo-tied handles from repoB — disposing repoB flips their liveness flag.
    const commitB = repoB.findCommit(oidB);
    const treeB = commitB.tree(); // Tree
    const authorB = commitB.author(); // Signature (FromCommit)
    const committerB = commitB.author();
    const objB = commitB.asObject(); // GitObject

    // Standalone signature — its liveness flag is never flipped, so it lets us
    // isolate the OTHER argument as the sole disposed handle in a call.
    const taggerLive = Signature.now("tester", "tester@example.com");

    // --- Positive control (all-live handles): the added arg-guards must not
    //     break the live path through the newly guarded methods. ---
    t.notThrows(() => repoA.diffTreeToWorkdir(treeA), "live diffTreeToWorkdir");
    t.is(
      typeof repoA.commit(null, authorA, committerA, "live", treeA, null),
      "string",
      "live commit returns an oid",
    );
    t.notThrows(() => repoA.branch("live-branch", commitA, true), "live branch");
    t.notThrows(
      () => commitA.amend(null, authorA, committerA, null, "amended", treeA),
      "live amend",
    );

    // --- Free repoB; every repoB-derived handle is now a dangling argument. ---
    repoB.dispose();

    // commit: each derived-handle arg guarded independently (author/committer/tree).
    t.throws(
      () => repoA.commit(null, authorB, committerA, "x", treeA, null),
      DISPOSED,
      "commit(author from disposed repo)",
    );
    t.throws(
      () => repoA.commit(null, authorA, committerB, "x", treeA, null),
      DISPOSED,
      "commit(committer from disposed repo)",
    );
    t.throws(
      () => repoA.commit(null, authorA, committerA, "x", treeB, null),
      DISPOSED,
      "commit(tree from disposed repo)",
    );

    // commitAsync: the guard is a SYNCHRONOUS pre-throw on the JS thread (the
    // call throws, it does not return a rejected promise).
    t.throws(
      () => repoA.commitAsync(null, authorB, committerA, "x", treeA, null),
      DISPOSED,
      "commitAsync(author from disposed repo)",
    );

    // tag / tagAnnotation / tagLightweight: target (GitObject) + tagger (Signature).
    t.throws(
      () => repoA.tag("t", objB, taggerLive, "m", false),
      DISPOSED,
      "tag(target from disposed repo)",
    );
    t.throws(
      () => repoA.tag("t", objA, authorB, "m", false),
      DISPOSED,
      "tag(tagger from disposed repo)",
    );
    t.throws(
      () => repoA.tagAnnotation("t", objB, taggerLive, "m"),
      DISPOSED,
      "tagAnnotation(target from disposed repo)",
    );
    t.throws(
      () => repoA.tagLightweight("t", objB, false),
      DISPOSED,
      "tagLightweight(target from disposed repo)",
    );

    // branch: target Commit.
    t.throws(
      () => repoA.branch("b", commitB, false),
      DISPOSED,
      "branch(target from disposed repo)",
    );

    // checkoutTree: treeish GitObject.
    t.throws(
      () => repoA.checkoutTree(objB),
      DISPOSED,
      "checkoutTree(treeish from disposed repo)",
    );

    // diffTreeToWorkdir / …WithIndex: Option<&Tree>.
    t.throws(
      () => repoA.diffTreeToWorkdir(treeB),
      DISPOSED,
      "diffTreeToWorkdir(oldTree from disposed repo)",
    );
    t.throws(
      () => repoA.diffTreeToWorkdirWithIndex(treeB),
      DISPOSED,
      "diffTreeToWorkdirWithIndex(oldTree from disposed repo)",
    );

    // Commit.amend on a LIVE repoA commit: Option author/committer/tree.
    t.throws(
      () => commitA.amend(null, authorB, null, null, "x", null),
      DISPOSED,
      "amend(author from disposed repo)",
    );
    t.throws(
      () => commitA.amend(null, null, null, null, "x", treeB),
      DISPOSED,
      "amend(tree from disposed repo)",
    );

    // Sanity: the repoA receiver stayed live through every guarded call.
    t.is(commitA.id(), oidA, "repoA receiver stayed live throughout");
  } finally {
    rmSync(dirA, { recursive: true, force: true });
    rmSync(dirB, { recursive: true, force: true });
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
