import { execSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import test from "ava";

import { FetchOptions, Repository } from "../index.js";

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
    const remote = new Repository(work).findRemote("origin");
    t.truthy(remote);
    remote.fetch(["refs/heads/main:refs/remotes/origin/main"], null);
    t.is(workRev(work, "refs/remotes/origin/main"), head);
    t.is(workRev(work, "refs/remotes/origin/main"), bareRev(bare, "refs/heads/main"));
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// Options path: a constructed (unconfigured) FetchOptions flows through
// fetch successfully.
test("fetch accepts a FetchOptions instance", (t) => {
  const { root, work, head } = makeFetchSetup();
  try {
    const remote = new Repository(work).findRemote("origin");
    const options = new FetchOptions();
    remote.fetch(["refs/heads/main:refs/remotes/origin/main"], options);
    t.is(workRev(work, "refs/remotes/origin/main"), head);
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
    const remote = new Repository(work).findRemote("origin");
    const options = new FetchOptions();
    remote.fetch(["refs/heads/main:refs/remotes/origin/main"], options);
    const err = t.throws(() =>
      remote.fetch(["refs/heads/main:refs/remotes/origin/main"], options),
    );
    t.regex(err.message, /FetchOptions can only be used once/);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
