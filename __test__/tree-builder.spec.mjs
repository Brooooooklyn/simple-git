import { execSync } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import test from "ava";

import { Delta, Repository } from "../index.js";

// No commits needed: blobs/trees are written straight into the ODB, so a
// freshly-initialized throwaway repo under os.tmpdir() is enough.
function makeRepo() {
  const dir = mkdtempSync(join(tmpdir(), "simple-git-tree-builder-"));
  execSync(`git init -q -b main "${dir}"`);
  return { dir, repo: new Repository(dir) };
}

// Raw git file modes, passed as numbers to TreeBuilder.insert().
const BLOB_MODE = 0o100644;

const deltaCount = (diff) => [...diff.deltas()].length;

test("treebuilder().insert().write() creates a real tree; findBlob round-trips content", (t) => {
  const { dir, repo } = makeRepo();
  try {
    const content = "hello tree builder\n";
    const blobOid = repo.blob(Buffer.from(content));

    const builder = repo.treebuilder();
    t.true(builder.isEmpty());
    t.is(builder.len(), 0);
    builder.insert("greeting.txt", blobOid, BLOB_MODE);
    t.is(builder.len(), 1);
    t.false(builder.isEmpty());

    const entry = builder.get("greeting.txt");
    t.is(entry.oid, blobOid);
    t.is(entry.filemode, BLOB_MODE);
    t.is(builder.get("missing.txt"), null);

    // write() materializes the builder into a real tree object in the ODB.
    const treeOid = builder.write();
    const tree = repo.findTree(treeOid);
    t.truthy(tree);
    t.is(tree.getName("greeting.txt").id(), blobOid);

    // findBlob reads the written blob back out of the ODB.
    t.is(repo.findBlob(blobOid).content().toString(), content);

    // Mutators still work after write().
    builder.remove("greeting.txt");
    t.is(builder.len(), 0);
    builder.insert("other.txt", blobOid, BLOB_MODE);
    builder.clear();
    t.true(builder.isEmpty());
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// Issue #167: diff two arbitrary strings like `git diff --no-index`. Each
// string becomes a blob, each blob a single-entry tree via TreeBuilder, and
// diffTreeToTree compares them without touching any workdir.
test("diffTreeToTree diffs two TreeBuilder-built single-entry trees (issue #167)", (t) => {
  const { dir, repo } = makeRepo();
  try {
    const contentA = "line one\nline two\n";
    const contentB = "line one\nline changed\n";
    const blobA = repo.blob(Buffer.from(contentA));
    const blobB = repo.blob(Buffer.from(contentB));

    const builderA = repo.treebuilder();
    builderA.insert("file.txt", blobA, BLOB_MODE);
    const treeA = repo.findTree(builderA.write());

    const builderB = repo.treebuilder();
    builderB.insert("file.txt", blobB, BLOB_MODE);
    const treeB = repo.findTree(builderB.write());

    const deltas = [...repo.diffTreeToTree(treeA, treeB).deltas()];
    t.is(deltas.length, 1);
    t.is(deltas[0].status(), Delta.Modified);
    t.is(deltas[0].oldFile().id(), blobA);
    t.is(deltas[0].newFile().id(), blobB);

    // The blob written by repo.blob() is readable back through findBlob.
    t.is(repo.findBlob(blobA).content().toString(), contentA);
    t.is(repo.findBlob(blobB).content().toString(), contentB);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("treebuilder(source) seeds the builder from an existing tree", (t) => {
  const { dir, repo } = makeRepo();
  try {
    const blobOid = repo.blob(Buffer.from("seed\n"));

    const builder = repo.treebuilder();
    builder.insert("seed.txt", blobOid, BLOB_MODE);
    const tree = repo.findTree(builder.write());

    const seeded = repo.treebuilder(tree);
    t.is(seeded.len(), 1);
    t.is(seeded.get("seed.txt").oid, blobOid);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("diffTreeToTree treats a null side as the empty tree", (t) => {
  const { dir, repo } = makeRepo();
  try {
    const blobOid = repo.blob(Buffer.from("gone\n"));
    const builder = repo.treebuilder();
    builder.insert("gone.txt", blobOid, BLOB_MODE);
    const tree = repo.findTree(builder.write());

    const deleted = [...repo.diffTreeToTree(tree, null).deltas()];
    t.is(deleted.length, 1);
    t.is(deleted[0].status(), Delta.Deleted);
    t.is(deleted[0].oldFile().id(), blobOid);

    const added = [...repo.diffTreeToTree(null, tree).deltas()];
    t.is(added.length, 1);
    t.is(added[0].status(), Delta.Added);
    t.is(added[0].newFile().id(), blobOid);

    // Identical trees produce no deltas.
    t.is(deltaCount(repo.diffTreeToTree(tree, tree)), 0);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
