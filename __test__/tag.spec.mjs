import { execSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import test from "ava";

import { Repository, Signature } from "../index.js";

// All tests here MUTATE a repository, so each one operates on a throwaway repo
// under os.tmpdir() and never touches the project's own repo. Caller cleans up
// `root` in a finally block.
function makeRepo() {
  const root = mkdtempSync(join(tmpdir(), "simple-git-tag-"));
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

const sig = () => Signature.now("tester", "tester@example.com");

// Build a single root commit and return its OID so tags have a target object.
function rootCommit(work, repo) {
  const author = sig();
  writeFileSync(join(work, "a.txt"), "alpha\n");
  const index = repo.index();
  index.addAll();
  index.write();
  const tree = repo.findTree(index.writeTree());
  return repo.commit("HEAD", author, author, "root", tree);
}

// A real annotated tag (created with `tag`) is resolvable by its OID through
// `findTag`, and the resolved Tag carries the name and message it was created
// with.
test("findTag resolves a real annotated tag", (t) => {
  const { root, work, repo } = makeRepo();
  try {
    const target = repo.findCommit(rootCommit(work, repo)).asObject();
    const tagOid = repo.tag("v1.0.0", target, sig(), "release one\n", false);
    t.is(tagOid.length, 40);

    const tag = repo.findTag(tagOid);
    t.truthy(tag);
    t.is(tag.name(), "v1.0.0");
    t.is(tag.message(), "release one\n");
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// A missing OID must map to `null` (libgit2 NotFound), not throw. The all-zero
// OID is a valid 40-char hash that can never name a real object.
test("findTag returns null for a missing oid", (t) => {
  const { root, work, repo } = makeRepo();
  try {
    rootCommit(work, repo);
    t.is(repo.findTag("0".repeat(40)), null);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// findTagByPrefix resolves a real tag by a hash prefix, and returns `null`
// (not a throw) when no tag matches the prefix.
test("findTagByPrefix resolves by prefix and is null when absent", (t) => {
  const { root, work, repo } = makeRepo();
  try {
    const target = repo.findCommit(rootCommit(work, repo)).asObject();
    const tagOid = repo.tag("v2.0.0", target, sig(), "release two\n", false);

    const tag = repo.findTagByPrefix(tagOid.slice(0, 8));
    t.truthy(tag);
    t.is(tag.name(), "v2.0.0");

    // A prefix that cannot match any object resolves to null.
    t.is(repo.findTagByPrefix("0".repeat(40)), null);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// After dispose(), findTag/findTagByPrefix return `null` like the other
// nullable find* helpers (findRemote/findTree/findCommit) rather than throwing
// "Repository has been disposed" — a disposed repo simply has no tags to
// resolve. (A genuine libgit2 error other than not-found still throws.)
test("findTag/findTagByPrefix return null after dispose (not throw)", (t) => {
  const { root, work, repo } = makeRepo();
  try {
    const target = repo.findCommit(rootCommit(work, repo)).asObject();
    const tagOid = repo.tag("v3.0.0", target, sig(), "release three\n", false);
    t.truthy(repo.findTag(tagOid), "sanity: resolves while the repo is live");

    repo.dispose();
    t.is(repo.findTag(tagOid), null);
    t.is(repo.findTagByPrefix(tagOid.slice(0, 8)), null);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// tagForeach hands the callback a single object { id, nameBytes } per tag:
// `id` is the 40-char OID hex string, `nameBytes` is the raw reference name as
// a Buffer (e.g. "refs/tags/v1.0.0").
test("tagForeach yields a { id, nameBytes } object per tag", (t) => {
  const { root, work, repo } = makeRepo();
  try {
    const target = repo.findCommit(rootCommit(work, repo)).asObject();
    repo.tag("v1.0.0", target, sig(), "release one\n", false);
    repo.tag("v2.0.0", target, sig(), "release two\n", false);

    const seen = [];
    repo.tagForeach((tag) => {
      seen.push(tag);
      return true;
    });

    // One callback invocation per tag, each receiving a single object arg.
    t.is(seen.length, 2);
    for (const tag of seen) {
      t.is(typeof tag, "object");
      t.is(typeof tag.id, "string");
      t.is(tag.id.length, 40);
      t.true(Buffer.isBuffer(tag.nameBytes));
    }

    const names = seen.map((tag) => tag.nameBytes.toString("utf8")).sort();
    t.deepEqual(names, ["refs/tags/v1.0.0", "refs/tags/v2.0.0"]);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// tagAnnotation (renamed from tagAnnotationCreate) creates an annotated tag
// object WITHOUT a reference; the returned OID is still resolvable via findTag.
test("tagAnnotation creates a refless annotated tag resolvable by findTag", (t) => {
  const { root, work, repo } = makeRepo();
  try {
    const target = repo.findCommit(rootCommit(work, repo)).asObject();
    const oid = repo.tagAnnotation("annot", target, sig(), "annotation body\n");
    t.is(oid.length, 40);

    const tag = repo.findTag(oid);
    t.truthy(tag);
    t.is(tag.message(), "annotation body\n");

    // No reference was created for it.
    t.false(repo.tagNames().includes("annot"));
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
