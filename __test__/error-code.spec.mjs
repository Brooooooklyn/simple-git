import { execSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import test from "ava";

import {
  FetchOptions,
  RepoBuilder,
  Repository,
  Signature,
} from "../index.js";

// Every test operates on a throwaway repo under os.tmpdir() and never touches
// the project's own repo. Each repo is hermetic: commit.gpgsign=false so a
// global signing config cannot make commits fail/hang, and core.autocrlf=false
// so line-ending rewrites do not perturb blobs. Caller cleans up `root`.
function makeRepo() {
  const root = mkdtempSync(join(tmpdir(), "simple-git-error-code-"));
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

// Stage `a.txt`, write the index as a tree, and make a root commit. Returns the
// commit OID and its tree so callers have a real OID/tree to work with.
function seedCommit(repo, work) {
  writeFileSync(join(work, "a.txt"), "alpha\n");
  const index = repo.index();
  index.addAll();
  index.write();
  const tree = repo.findTree(index.writeTree());
  const oid = repo.commit("HEAD", sig(), sig(), "first", tree);
  return { oid, tree };
}

// -------- SYNC: every thrown error carries a git2-derived `error.code` --------

test("opening a missing repository throws code NotFound", (t) => {
  const root = mkdtempSync(join(tmpdir(), "simple-git-error-code-"));
  try {
    t.throws(() => new Repository(join(root, "does-not-exist")), {
      code: "NotFound",
    });
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("creating a reference that already exists throws code Exists", (t) => {
  const { root, work, repo } = makeRepo();
  try {
    const { oid } = seedCommit(repo, work);
    repo.reference("refs/heads/dup", oid, false, "create");
    t.throws(() => repo.reference("refs/heads/dup", oid, false, "again"), {
      code: "Exists",
    });
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("creating a tag that already exists throws code Exists", (t) => {
  const { root, work, repo } = makeRepo();
  try {
    const { oid } = seedCommit(repo, work);
    const target = repo.findCommit(oid).asObject();
    repo.tagLightweight("dup-tag", target, false);
    t.throws(() => repo.tagLightweight("dup-tag", target, false), {
      code: "Exists",
    });
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// Exercises the `Result<&Self>` chaining path (revWalk.pushRange), which carries
// its git2 code via `coded_error`/`code_into` rather than the plain throw path.
test("pushing an unparsable revision range throws code InvalidSpec", (t) => {
  const { root, work, repo } = makeRepo();
  try {
    seedCommit(repo, work);
    const revWalk = repo.revWalk();
    t.throws(() => revWalk.pushRange("this is not a range"), {
      code: "InvalidSpec",
    });
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("a custom header with an interior NUL byte throws code InvalidArg", (t) => {
  t.throws(() => new FetchOptions().customHeaders(["a\0b"]), {
    code: "InvalidArg",
  });
});

test("reusing a single FetchOptions twice throws code InvalidArg", (t) => {
  const options = new FetchOptions();
  const builder = new RepoBuilder();
  builder.fetchOptions(options);
  t.throws(() => builder.fetchOptions(options), { code: "InvalidArg" });
});

// -------- ASYNC: rejected promises carry `error.code` via `coded_error` -------

test("commitAsync with a nonexistent parent OID rejects with code NotFound", async (t) => {
  const { root, work, repo } = makeRepo();
  try {
    const { tree } = seedCommit(repo, work);
    // The all-zero OID is well-formed, so it parses and then fails in
    // `find_commit`, which libgit2 reports as GIT_ENOTFOUND -> `NotFound`.
    await t.throwsAsync(
      () =>
        repo.commitAsync("HEAD", sig(), sig(), "orphan", tree, [
          "0000000000000000000000000000000000000000",
        ]),
      { code: "NotFound" },
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("blameFileAsync on a missing path rejects with code NotFound", async (t) => {
  const { root, work, repo } = makeRepo();
  try {
    seedCommit(repo, work);
    // The path is absent from the tree, so libgit2 reports GIT_ENOTFOUND.
    await t.throwsAsync(() => repo.blameFileAsync("does-not-exist.txt"), {
      code: "NotFound",
    });
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("statusesAsync on a bare repository rejects with code BareRepo", async (t) => {
  const root = mkdtempSync(join(tmpdir(), "simple-git-error-code-"));
  try {
    const bare = join(root, "bare.git");
    execSync(`git init -q --bare "${bare}"`);
    const repo = new Repository(bare);
    // Status is disallowed on a bare repo -> GIT_EBAREREPO -> `BareRepo`.
    await t.throwsAsync(() => repo.statusesAsync(), { code: "BareRepo" });
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// Fast-failing bad URL: a nonexistent local path resolves via the local
// transport and fails immediately, so no pack is downloaded (no Windows .pack
// EBUSY is introduced ahead of the Task-3 dispose fix).
test("fetchAsync against a nonexistent local remote rejects with a git2 code", async (t) => {
  const { root, work, repo } = makeRepo();
  try {
    seedCommit(repo, work);
    const remote = repo.remote("badorigin", join(root, "no-such-remote.git"));
    const err = await t.throwsAsync(() => remote.fetchAsync([]));
    // The exact network/URL classification varies by platform (on this host it
    // is `GenericError`), but the flip must always surface a real GitCode token,
    // never the raw napi Status placeholder `GenericFailure` that the pre-flip
    // async reject path produced.
    t.is(typeof err.code, "string");
    t.true(err.code.length > 0);
    t.not(err.code, "GenericFailure");
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
