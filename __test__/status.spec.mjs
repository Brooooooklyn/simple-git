import { execSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import test from "ava";

import { Repository } from "../index.js";

const __dirname = join(fileURLToPath(import.meta.url), "..");
const workDir = join(__dirname, "..");

// Spin up an isolated repo under os.tmpdir() so mutating cases never touch the
// project's own working tree. Returns the dir; caller cleans up.
function makeTempRepo() {
  const dir = mkdtempSync(join(tmpdir(), "simple-git-status-"));
  const run = (args) => execSync(`git ${args}`, { cwd: dir });
  run("init -q");
  run("config user.name tester");
  run("config user.email tester@example.com");
  run("config commit.gpgsign false");
  writeFileSync(join(dir, "committed.txt"), "v1\n");
  run("add committed.txt");
  run("commit -q -m initial");
  return dir;
}

const find = (statuses, path) => statuses.find((s) => s.path === path);

function assertFileStatusShape(t, entry) {
  t.is(typeof entry.bits, "number");
  for (const key of [
    "isIndexNew",
    "isIndexModified",
    "isIndexDeleted",
    "isIndexRenamed",
    "isIndexTypechange",
    "isWtNew",
    "isWtModified",
    "isWtDeleted",
    "isWtTypechange",
    "isWtRenamed",
    "isIgnored",
    "isConflicted",
  ]) {
    t.is(typeof entry[key], "boolean", `${key} should be boolean`);
  }
  t.true(entry.path === null || typeof entry.path === "string");
}

// Read-only against the project repo: statuses() returns an array of well-typed
// FileStatus objects.
test("statuses returns a typed array", (t) => {
  const repo = new Repository(workDir);
  const statuses = repo.statuses();
  t.true(Array.isArray(statuses));
  for (const entry of statuses) {
    assertFileStatusShape(t, entry);
  }
});

// Untracked -> isWtNew; default includeUntracked is true (git-CLI default).
test("untracked file surfaces as isWtNew by default", (t) => {
  const dir = makeTempRepo();
  try {
    writeFileSync(join(dir, "untracked.txt"), "hello\n");
    const repo = new Repository(dir);
    const statuses = repo.statuses();
    const entry = find(statuses, "untracked.txt");
    t.truthy(entry, "untracked file should appear by default");
    t.true(entry.isWtNew);
    t.false(entry.isIndexNew);
    t.true(entry.bits !== 0);
    assertFileStatusShape(t, entry);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// includeUntracked: false hides untracked files (proves option mapping).
test("includeUntracked false hides untracked files", (t) => {
  const dir = makeTempRepo();
  try {
    writeFileSync(join(dir, "untracked.txt"), "hello\n");
    const repo = new Repository(dir);
    const statuses = repo.statuses({ includeUntracked: false });
    t.is(find(statuses, "untracked.txt"), undefined);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// Staged new file -> isIndexNew.
test("staged new file surfaces as isIndexNew", (t) => {
  const dir = makeTempRepo();
  try {
    writeFileSync(join(dir, "staged.txt"), "new\n");
    execSync("git add staged.txt", { cwd: dir });
    const repo = new Repository(dir);
    const entry = find(repo.statuses(), "staged.txt");
    t.truthy(entry);
    t.true(entry.isIndexNew);
    t.false(entry.isWtNew);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// Staged modification + further unstaged edit -> distinguishes index vs worktree.
test("index vs worktree modification are reported independently", (t) => {
  const dir = makeTempRepo();
  try {
    // stage a modification...
    writeFileSync(join(dir, "committed.txt"), "v2\n");
    execSync("git add committed.txt", { cwd: dir });
    // ...then edit the working copy again without staging.
    writeFileSync(join(dir, "committed.txt"), "v3\n");
    const repo = new Repository(dir);
    const entry = find(repo.statuses(), "committed.txt");
    t.truthy(entry);
    t.true(entry.isIndexModified, "staged change -> isIndexModified");
    t.true(entry.isWtModified, "unstaged change -> isWtModified");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// statusFile on a known tracked path returns a FileStatus echoing the path.
test("statusFile returns a FileStatus for a known path", (t) => {
  const repo = new Repository(workDir);
  const entry = repo.statusFile("Cargo.toml");
  t.truthy(entry);
  t.is(entry.path, "Cargo.toml");
  assertFileStatusShape(t, entry);
});

// Async mirrors sync on the same state.
test("statusesAsync matches sync length", async (t) => {
  const dir = makeTempRepo();
  try {
    writeFileSync(join(dir, "untracked.txt"), "hello\n");
    const repo = new Repository(dir);
    const sync = repo.statuses();
    const asyncResult = await repo.statusesAsync();
    t.true(Array.isArray(asyncResult));
    t.is(asyncResult.length, sync.length);
    t.deepEqual(
      asyncResult.map((s) => s.path).sort(),
      sync.map((s) => s.path).sort(),
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
