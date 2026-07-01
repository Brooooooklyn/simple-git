import { readFile } from "node:fs/promises";
import { execSync } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
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
    t.true(actual instanceof Date);
    t.is(
      new Date(
        execSync("git log -1 --format=%cd --date=iso src/lib.rs", {
          cwd: workDir,
        })
          .toString("utf8")
          .trim(),
      ).valueOf(),
      actual.getTime(),
    );
  }
});

test("Created date should be equal with cli", (t) => {
  const { repo } = t.context;
  if (process.env.CI) {
    t.notThrows(() => repo.getFileCreatedDate(join("src", "lib.rs")));
  } else {
    const actual = repo.getFileCreatedDate(join("src", "lib.rs"));
    t.true(actual instanceof Date);
    t.is(
      new Date(
        execSync("git log --reverse --format=%cd --date=iso src/lib.rs", {
          cwd: workDir,
        })
          .toString("utf8")
          .split('\n')[0]
          .trim(),
      ).valueOf(),
      actual.getTime(),
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
    t.true(actualDate instanceof Date);
    t.is(expectedDate, actualDate.getTime());
  }
});

// A path that no commit ever touched is "no matching commit", NOT an error:
// the accessors now resolve to `null` instead of throwing.
test("Created date returns null for non-existent file", (t) => {
  const { repo } = t.context;
  t.is(repo.getFileCreatedDate("non-existent-file.txt"), null);
});

test("Created date async returns null for non-existent file", async (t) => {
  const { repo } = t.context;
  t.is(await repo.getFileCreatedDateAsync("non-existent-file.txt"), null);
});

test("Latest modified date returns null for non-existent file", (t) => {
  const { repo } = t.context;
  t.is(repo.getFileLatestModifiedDate("does-not-exist-xyz.txt"), null);
});

test("Latest modified date async returns null for non-existent file", async (t) => {
  const { repo } = t.context;
  t.is(await repo.getFileLatestModifiedDateAsync("does-not-exist-xyz.txt"), null);
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
