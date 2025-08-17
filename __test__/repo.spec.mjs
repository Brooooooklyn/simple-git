import { readFile } from "node:fs/promises";
import { execSync } from "node:child_process";
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
    t.deepEqual(
      new Date(
        execSync("git log -1 --format=%cd --date=iso src/lib.rs", {
          cwd: workDir,
        })
          .toString("utf8")
          .trim(),
      ).valueOf(),
      repo.getFileLatestModifiedDate(join("src", "lib.rs")),
    );
  }
});

test("Created date should be equal with cli", (t) => {
  const { repo } = t.context;
  if (process.env.CI) {
    t.notThrows(() => repo.getFileCreatedDate(join("src", "lib.rs")));
  } else {
    t.deepEqual(
      new Date(
        execSync("git log --reverse --format=%cd --date=iso src/lib.rs", {
          cwd: workDir,
        })
          .toString("utf8")
          .split('\n')[0]
          .trim(),
      ).valueOf(),
      repo.getFileCreatedDate(join("src", "lib.rs")),
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
    t.deepEqual(expectedDate, actualDate);
  }
});

test("Created date should throw for non-existent file", (t) => {
  const { repo } = t.context;
  t.throws(() => repo.getFileCreatedDate("non-existent-file.txt"), {
    message: /Failed to get created date for/
  });
});

test("Created date async should throw for non-existent file", async (t) => {
  const { repo } = t.context;
  await t.throwsAsync(() => repo.getFileCreatedDateAsync("non-existent-file.txt"), {
    message: /Failed to get created date for/
  });
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
