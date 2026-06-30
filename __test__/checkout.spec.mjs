import { execSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import test from "ava";

import { Repository } from "../index.js";

// Every test here MUTATES the working tree and/or HEAD, so each one operates on
// a throwaway repo under os.tmpdir() and never touches the project's own repo.
// The repo has two commits on `main`: `file.txt` is "v1\n" at the first commit
// and "v2\n" at the second (which HEAD/index sit on). Caller cleans up `root`.
function makeRepo() {
  const root = mkdtempSync(join(tmpdir(), "simple-git-checkout-"));
  const work = join(root, "work");
  execSync(`git init -q -b main "${work}"`);
  const run = (args, opts) => execSync(`git ${args}`, { cwd: work, ...opts });
  run("config user.name tester");
  run("config user.email tester@example.com");
  // Keep the temp repo hermetic: never inherit a global commit.gpgsign that
  // would make `git commit` fail (or block on a signing agent).
  run("config commit.gpgsign false");
  // Git-for-Windows defaults core.autocrlf=true, so libgit2's checkout would
  // rewrite the workdir file with CRLF ("v1\r\n") and break the LF assertions
  // below. Pin it off so blobs round-trip byte-for-byte on every platform.
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
  return { root, work, repo, firstOid, headOid, run };
}

// setHead repoints HEAD at a branch sitting on the older commit; a forced
// checkoutHead then rewrites the working tree to that branch's content.
test("setHead + checkoutHead(force) reverts the workdir and moves HEAD", (t) => {
  const { root, work, repo, firstOid, run } = makeRepo();
  try {
    run(`branch old ${firstOid}`);
    repo.setHead("refs/heads/old");
    repo.checkoutHead({ force: true });

    t.is(readFileSync(join(work, "file.txt"), "utf8"), "v1\n");
    t.is(run("symbolic-ref HEAD").toString().trim(), "refs/heads/old");
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// A forced checkoutIndex rewrites a dirty file back to the staged (index)
// content, which still matches the second commit.
test("checkoutIndex(force) restores the workdir from the index", (t) => {
  const { root, work, repo } = makeRepo();
  try {
    writeFileSync(join(work, "file.txt"), "dirty\n");
    repo.checkoutIndex({ force: true });
    t.is(readFileSync(join(work, "file.txt"), "utf8"), "v2\n");
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// CRITICAL safe-default guarantee: without `force`, a checkout must NOT clobber
// a locally-modified tracked file (silent data loss). The dirty content stays.
test("checkoutIndex without force preserves local modifications (safe default)", (t) => {
  const { root, work, repo } = makeRepo();
  try {
    writeFileSync(join(work, "file.txt"), "dirty\n");
    repo.checkoutIndex();
    t.is(readFileSync(join(work, "file.txt"), "utf8"), "dirty\n");
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// setHeadDetached points HEAD straight at a commit OID; HEAD then resolves to
// that commit and is no longer symbolic (so `git symbolic-ref HEAD` errors).
test("setHeadDetached detaches HEAD at the given commit", (t) => {
  const { root, work, repo, firstOid } = makeRepo();
  try {
    repo.setHeadDetached(firstOid);
    t.is(
      execSync("git rev-parse HEAD", { cwd: work }).toString().trim(),
      firstOid,
    );
    t.throws(() =>
      execSync("git symbolic-ref HEAD", { cwd: work, stdio: "pipe" }),
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// reference() creates a direct ref pointing at an OID; git can resolve it and
// the returned Reference reports the same target.
test("reference creates a direct ref resolvable by git", (t) => {
  const { root, work, repo, headOid } = makeRepo();
  try {
    const ref = repo.reference(
      "refs/heads/made-by-api",
      headOid,
      false,
      "create via api",
    );
    t.is(ref.target(), headOid);
    t.is(
      execSync("git rev-parse refs/heads/made-by-api", { cwd: work })
        .toString()
        .trim(),
      headOid,
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// referenceSymbolic() creates a symbolic ref pointing at another ref; git
// resolves through it to the underlying commit.
test("referenceSymbolic creates a symbolic ref that resolves through", (t) => {
  const { root, work, repo, headOid } = makeRepo();
  try {
    const ref = repo.referenceSymbolic(
      "refs/heads/sym",
      "refs/heads/main",
      false,
      "create sym",
    );
    t.is(ref.symbolicTarget(), "refs/heads/main");
    t.is(
      execSync("git rev-parse refs/heads/sym", { cwd: work })
        .toString()
        .trim(),
      headOid,
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
