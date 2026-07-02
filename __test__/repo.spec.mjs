import { readFile } from "node:fs/promises";
import { execSync, execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import test from "ava";

const __dirname = join(fileURLToPath(import.meta.url), "..");

import { Repository } from "../index.js";

const workDir = join(__dirname, "..");

test.beforeEach((t) => {
  t.context.repo = new Repository(workDir);
});

test("Date should be equal with cli", (t) => {
  const { repo } = t.context;
  if (process.env.CI) {
    t.notThrows(() => repo.getFileLatestModifiedDate(join("src", "lib.rs")));
  } else {
    const actual = repo.getFileLatestModifiedDate(join("src", "lib.rs"));
    t.is(typeof actual, "number");
    t.is(
      new Date(
        execSync("git log -1 --format=%cd --date=iso src/lib.rs", {
          cwd: workDir,
        })
          .toString("utf8")
          .trim(),
      ).valueOf(),
      actual,
    );
  }
});

test("Created date should be equal with cli", (t) => {
  const { repo } = t.context;
  if (process.env.CI) {
    t.notThrows(() => repo.getFileCreatedDate(join("src", "lib.rs")));
  } else {
    const actual = repo.getFileCreatedDate(join("src", "lib.rs"));
    t.is(typeof actual, "number");
    t.is(
      new Date(
        execSync("git log --reverse --format=%cd --date=iso src/lib.rs", {
          cwd: workDir,
        })
          .toString("utf8")
          .split('\n')[0]
          .trim(),
      ).valueOf(),
      actual,
    );
  }
});

test("Created date async should work", async (t) => {
  const { repo } = t.context;
  if (process.env.CI) {
    await t.notThrowsAsync(() => repo.getFileCreatedDateAsync(join("src", "lib.rs")));
  } else {
    const expectedDate = new Date(
      execSync("git log --reverse --format=%cd --date=iso src/lib.rs", {
        cwd: workDir,
      })
        .toString("utf8")
        .split('\n')[0]
        .trim(),
    ).valueOf();

    const actualDate = await repo.getFileCreatedDateAsync(join("src", "lib.rs"));
    t.is(typeof actualDate, "number");
    t.is(expectedDate, actualDate);
  }
});

// A path that no commit ever touched is "no matching commit". The legacy
// `number` accessors THROW for this case (0.1.x behavior); only the
// `getFileLastModifiedDate` Date twin returns `null` (covered in modification.spec).
test("Created date THROWS for non-existent file", (t) => {
  const { repo } = t.context;
  const err = t.throws(() => repo.getFileCreatedDate("non-existent-file.txt"));
  t.is(err.code, "GenericError");
  t.is(err.message, "Failed to get created date for [non-existent-file.txt]");
});

test("Created date async THROWS for non-existent file", async (t) => {
  const { repo } = t.context;
  const err = await t.throwsAsync(() => repo.getFileCreatedDateAsync("non-existent-file.txt"));
  t.is(err.code, "GenericError");
  t.is(err.message, "Failed to get created date for [non-existent-file.txt]");
});

test("Latest modified date THROWS for non-existent file", (t) => {
  const { repo } = t.context;
  const err = t.throws(() => repo.getFileLatestModifiedDate("does-not-exist-xyz.txt"));
  t.is(err.code, "GenericError");
  t.is(err.message, "Failed to get commit for [does-not-exist-xyz.txt]");
});

test("Latest modified date async THROWS for non-existent file", async (t) => {
  const { repo } = t.context;
  const err = await t.throwsAsync(() => repo.getFileLatestModifiedDateAsync("does-not-exist-xyz.txt"));
  t.is(err.code, "GenericError");
  t.is(err.message, "Failed to get commit for [does-not-exist-xyz.txt]");
});

// Guard the null-vs-throw boundary: a fresh repo with an unborn HEAD (no commit)
// cannot walk history at all. That is a real error and MUST still throw -- it must
// NOT be swallowed into `null` like the "no matching commit" case above.
test("File-date accessors THROW on an unborn HEAD (no commit), not null", async (t) => {
  const root = mkdtempSync(join(tmpdir(), "simple-git-unborn-"));
  const work = join(root, "work");
  execSync(`git init -q -b main "${work}"`);
  const run = (args) => execSync(`git ${args}`, { cwd: work });
  run("config user.name tester");
  run("config user.email tester@example.com");
  run("config commit.gpgsign false");
  run("config core.autocrlf false");
  try {
    const repo = new Repository(work);
    t.throws(() => repo.getFileCreatedDate("anything.txt"));
    t.throws(() => repo.getFileLatestModifiedDate("anything.txt"));
    await t.throwsAsync(() => repo.getFileCreatedDateAsync("anything.txt"));
    // Unborn HEAD is a REAL error, so even the null-safe Date twin throws
    // (both sync and async), and the number async rejects.
    t.throws(() => repo.getFileLastModifiedDate("anything.txt"));
    await t.throwsAsync(() => repo.getFileLatestModifiedDateAsync("anything.txt"));
    await t.throwsAsync(() => repo.getFileLastModifiedDateAsync("anything.txt"));
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// A corrupt/missing object hit MID-WALK is a real libgit2 read failure, NOT the
// "no matching commit" case: the file-date walkers must PROPAGATE it (throw),
// never swallow it into `null` or a silently-wrong date. Repro: commit a.txt in
// a root commit + a second commit, then delete the ROOT (creation) commit's tree
// object. That breaks the mid-walk tree read for every family:
//   - modification walkers: HEAD's `parent.tree()` (parent == root) fails;
//   - getFileCreatedDate: the creation commit's own `commit.tree()` fails.
// Pre-fix this returned null / the wrong date (RED); post-fix it throws (GREEN).
test("File-date walkers THROW on a corrupt/missing object mid-walk, not null/wrong", async (t) => {
  const root = mkdtempSync(join(tmpdir(), "simple-git-corrupt-"));
  const work = join(root, "work");
  execSync(`git init -q -b main "${work}"`);
  const run = (args) => execSync(`git ${args}`, { cwd: work });
  run("config user.name tester");
  run("config user.email tester@example.com");
  run("config commit.gpgsign false");
  run("config core.autocrlf false");
  run("config gc.auto 0");
  try {
    writeFileSync(join(work, "a.txt"), "v1\n");
    run("add a.txt");
    run("commit -q -m first");
    writeFileSync(join(work, "a.txt"), "v2\n");
    run("add a.txt");
    run("commit -q -m second");

    // Root (creation) commit's tree; deleting its loose object breaks the
    // mid-walk tree read that every date family depends on. Resolve it via
    // execFileSync (no shell) so the `^{tree}` peel syntax survives Windows
    // cmd.exe, which otherwise eats the `^` and mangles the quotes.
    const rootTree = execFileSync("git", ["rev-parse", "HEAD~1^{tree}"], {
      cwd: work,
    })
      .toString()
      .trim();
    const objPath = join(
      work,
      ".git",
      "objects",
      rootTree.slice(0, 2),
      rootTree.slice(2),
    );
    t.true(existsSync(objPath), "root tree must be a loose object to corrupt");
    rmSync(objPath, { force: true });

    const repo = new Repository(work);
    t.throws(() => repo.getFileLatestModifiedDate("a.txt"));
    t.throws(() => repo.getFileLatestModified("a.txt"));
    t.throws(() => repo.getFileCreatedDate("a.txt"));
    t.throws(() => repo.getFilesLatestModified(["a.txt"]));
    await t.throwsAsync(() => repo.getFileLatestModifiedDateAsync("a.txt"));
    await t.throwsAsync(() => repo.getFileLatestModifiedAsync("a.txt"));
    await t.throwsAsync(() => repo.getFileCreatedDateAsync("a.txt"));
    await t.throwsAsync(() => repo.getFilesLatestModifiedAsync(["a.txt"]));
    // A corrupt object mid-walk is a REAL error, so the Date twin throws too.
    t.throws(() => repo.getFileLastModifiedDate("a.txt"));
    await t.throwsAsync(() => repo.getFileLastModifiedDateAsync("a.txt"));
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// Error propagation must NOT break the legitimate root-commit branch
// (parent_count() == 0): a file whose ONLY commit is the root still resolves to
// a Date, and a never-committed path in the same repo still resolves to null.
test("File-date accessors resolve a root-only file and keep no-match -> null", async (t) => {
  const root = mkdtempSync(join(tmpdir(), "simple-git-root-only-"));
  const work = join(root, "work");
  execSync(`git init -q -b main "${work}"`);
  const run = (args) => execSync(`git ${args}`, { cwd: work });
  run("config user.name tester");
  run("config user.email tester@example.com");
  run("config commit.gpgsign false");
  run("config core.autocrlf false");
  try {
    writeFileSync(join(work, "only.txt"), "hello\n");
    run("add only.txt");
    run("commit -q -m root");

    const repo = new Repository(work);
    t.is(typeof repo.getFileCreatedDate("only.txt"), "number");
    t.is(typeof repo.getFileLatestModifiedDate("only.txt"), "number");
    t.true(repo.getFileLastModifiedDate("only.txt") instanceof Date);
    t.truthy(repo.getFileLatestModified("only.txt"));
    t.is(typeof (await repo.getFileCreatedDateAsync("only.txt")), "number");

    // Same repo, never-committed path: the legacy number getters THROW, while
    // getFileLatestModified/getFileLastModifiedDate stay null (no throw).
    t.throws(() => repo.getFileCreatedDate("missing.txt"));
    t.throws(() => repo.getFileLatestModifiedDate("missing.txt"));
    t.is(repo.getFileLatestModified("missing.txt"), null);
    t.is(repo.getFileLastModifiedDate("missing.txt"), null);
    await t.throwsAsync(() => repo.getFileCreatedDateAsync("missing.txt"));
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("Should be able to resolve head", (t) => {
  const { repo } = t.context;
  t.is(
    repo.head().target(),
    process.env.CI
      ? process.env.GITHUB_SHA
      : execSync("git rev-parse HEAD", {
          cwd: workDir,
        })
          .toString("utf8")
          .trim(),
  );
});

test("Should be able to get blob content", async (t) => {
  if (process.platform === "win32") {
    t.pass("Skip test on windows");
    return;
  }
  const { repo } = t.context;
  const blob = repo
    .head()
    .peelToTree()
    .getPath("__test__/repo.spec.mjs")
    .toObject(repo)
    .peelToBlob();
  t.deepEqual(
    await readFile(join(__dirname, "repo.spec.mjs"), "utf8"),
    Buffer.from(blob.content()).toString("utf8"),
  );
});
