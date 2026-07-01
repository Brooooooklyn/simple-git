import { execSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import test from "ava";

import { Repository } from "../index.js";

const __dirname = join(fileURLToPath(import.meta.url), "..");

// Spin up an isolated repo under os.tmpdir() so nothing touches the project's
// own working tree. Returns the dir; caller cleans up.
function makeTempRepo() {
  const dir = mkdtempSync(join(tmpdir(), "simple-git-dispose-"));
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

// After dispose(), guarded methods throw "Repository has been disposed".
test("dispose() makes repo methods throw a disposed error", (t) => {
  const dir = makeTempRepo();
  try {
    const repo = new Repository(dir);
    // Sanity: works before disposal.
    t.truthy(repo.head());
    repo.dispose();
    t.throws(() => repo.head(), { message: /Repository has been disposed/ });
    t.throws(() => repo.config(), { message: /Repository has been disposed/ });
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// dispose() and free() are idempotent no-ops after the first call.
test("dispose()/free() are idempotent no-ops", (t) => {
  const dir = makeTempRepo();
  try {
    const repo = new Repository(dir);
    repo.dispose();
    t.notThrows(() => repo.dispose());
    t.notThrows(() => repo.free());
    // Still disposed after the extra calls.
    t.throws(() => repo.head(), { message: /Repository has been disposed/ });
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// free() is an alias for dispose().
test("free() disposes the repository", (t) => {
  const dir = makeTempRepo();
  try {
    const repo = new Repository(dir);
    repo.free();
    t.throws(() => repo.head(), { message: /Repository has been disposed/ });
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// The disposed-repo contract for the Option-returning methods: workdir() and
// namespace() return null instead of throwing.
test("workdir()/namespace() return null (not throw) after dispose", (t) => {
  const dir = makeTempRepo();
  try {
    const repo = new Repository(dir);
    t.truthy(repo.workdir());
    repo.dispose();
    t.is(repo.workdir(), null);
    t.is(repo.namespace(), null);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
