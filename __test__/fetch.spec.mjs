import { execSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import test from "ava";

import { AutotagOption, FetchOptions, RemoteCallbacks, Repository } from "../index.js";

// Build a local bare "remote" repo seeded with a commit, and a fresh empty
// work repo wired to it via `git remote add origin`. Fully local: the remote
// is a bare repo on disk, so fetch never touches the network. Returns the
// dirs + the bare's HEAD commit; caller cleans up `root`.
function makeFetchSetup() {
  const root = mkdtempSync(join(tmpdir(), "simple-git-fetch-"));
  const bare = join(root, "remote.git");
  const seed = join(root, "seed");
  const work = join(root, "work");
  // Bare repo acts as the fetch source, seeded via a throwaway work repo.
  execSync(`git init -q --bare -b main "${bare}"`);
  execSync(`git init -q -b main "${seed}"`);
  const runSeed = (args) => execSync(`git ${args}`, { cwd: seed });
  runSeed("config user.name tester");
  runSeed("config user.email tester@example.com");
  // Keep the temp repo hermetic: never inherit a global commit.gpgsign that
  // would make `git commit` fail (or block on a signing agent) during setup.
  runSeed("config commit.gpgsign false");
  runSeed("config core.autocrlf false");
  writeFileSync(join(seed, "file.txt"), "hello\n");
  runSeed("add file.txt");
  runSeed('commit -q -m "initial commit"');
  runSeed(`remote add origin "${bare}"`);
  runSeed("push -q origin main");
  const head = execSync(`git --git-dir="${bare}" rev-parse refs/heads/main`)
    .toString()
    .trim();
  // Fresh empty work repo with a remote pointing at the bare repo.
  execSync(`git init -q -b main "${work}"`);
  const runWork = (args) => execSync(`git ${args}`, { cwd: work });
  runWork("config user.name tester");
  runWork("config user.email tester@example.com");
  runWork("config commit.gpgsign false");
  runWork("config core.autocrlf false");
  runWork(`remote add origin "${bare}"`);
  return { root, bare, work, head };
}

const bareRev = (bare, ref) =>
  execSync(`git --git-dir="${bare}" rev-parse ${ref}`).toString().trim();

const workRev = (work, ref) =>
  execSync(`git rev-parse ${ref}`, { cwd: work }).toString().trim();

// Core path: fetching a refspec lands the bare's commit in the local
// tracking ref.
test("fetch updates a local tracking ref", (t) => {
  const { root, bare, work, head } = makeFetchSetup();
  try {
    const repo = new Repository(work);
    const remote = repo.findRemote("origin");
    t.truthy(remote);
    remote.fetch(["refs/heads/main:refs/remotes/origin/main"], null);
    t.is(workRev(work, "refs/remotes/origin/main"), head);
    t.is(workRev(work, "refs/remotes/origin/main"), bareRev(bare, "refs/heads/main"));
    // Free the git2 handle (and its held-open fetched .pack fd) before rmSync
    // deletes the temp dir, so Windows can unlink it (fixes CI EBUSY).
    repo.dispose();
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// Options path: a constructed (unconfigured) FetchOptions flows through
// fetch successfully.
test("fetch accepts a FetchOptions instance", (t) => {
  const { root, work, head } = makeFetchSetup();
  try {
    const repo = new Repository(work);
    const remote = repo.findRemote("origin");
    const options = new FetchOptions();
    remote.fetch(["refs/heads/main:refs/remotes/origin/main"], options);
    t.is(workRev(work, "refs/remotes/origin/main"), head);
    repo.dispose();
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// REGRESSION GUARD: FetchOptions is a one-shot object (its inner git2 state
// is drained via mem::swap on first use). Reusing it must throw instead of
// silently fetching with blank (default) options a 2nd time. This test MUST
// fail before the reuse-guard fix (no error currently thrown) and pass after.
test("FetchOptions can only be used once", (t) => {
  const { root, work } = makeFetchSetup();
  try {
    const repo = new Repository(work);
    const remote = repo.findRemote("origin");
    const options = new FetchOptions();
    remote.fetch(["refs/heads/main:refs/remotes/origin/main"], options);
    const err = t.throws(() =>
      remote.fetch(["refs/heads/main:refs/remotes/origin/main"], options),
    );
    t.regex(err.message, /FetchOptions can only be used once/);
    repo.dispose();
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// REJECT GUARD: updateTips borrows a RemoteCallbacks; handing it one whose
// `used` flag is already set must throw instead of silently running with a
// drained (empty) callback set. `used` is not JS-readable, so we flip it the
// only way JS can: feed the callbacks to a FetchOptions, which consumes it
// (mem::swap) and marks it used. This MUST NOT throw before the guard lands
// (updateTips would just run with the emptied callbacks) and MUST throw after.
test("updateTips rejects an already-used RemoteCallbacks", (t) => {
  const { root, work } = makeFetchSetup();
  try {
    const cbs = new RemoteCallbacks();
    // Consuming the callbacks into a FetchOptions flips its `used` flag.
    new FetchOptions().remoteCallback(cbs);
    const repo = new Repository(work);
    const remote = repo.findRemote("origin");
    t.throws(
      () => remote.updateTips(0, AutotagOption.Unspecified, cbs, null),
      { message: /already been used/ },
    );
    repo.dispose();
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// CHECK-ONLY GUARD: the reject guard must READ `used` without SETTING it,
// because updateTips borrows the callbacks (no consume). A FRESH callbacks
// object must therefore survive being passed to updateTips more than once.
// Both calls succeeding proves updateTips never marks the callbacks used.
test("updateTips does not consume a fresh RemoteCallbacks", (t) => {
  const { root, work } = makeFetchSetup();
  try {
    const repo = new Repository(work);
    const remote = repo.findRemote("origin");
    remote.fetch(["refs/heads/main:refs/remotes/origin/main"], null);
    const cbs = new RemoteCallbacks();
    t.notThrows(() =>
      remote.updateTips(0, AutotagOption.Unspecified, cbs, null),
    );
    t.notThrows(() =>
      remote.updateTips(0, AutotagOption.Unspecified, cbs, null),
    );
    repo.dispose();
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
