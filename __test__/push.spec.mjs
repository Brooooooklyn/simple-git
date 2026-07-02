import { execSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import test from "ava";

import { PushOptions, RemoteCallbacks, Repository } from "../index.js";

// Build a local bare "remote" repo and a working repo wired to it via a remote.
// Fully local: the remote is a bare repo on disk, so push never touches the
// network. Returns the dirs + the HEAD commit; caller cleans up `root`.
function makePushSetup() {
  const root = mkdtempSync(join(tmpdir(), "simple-git-push-"));
  const bare = join(root, "remote.git");
  const work = join(root, "work");
  // Bare repo acts as the push target.
  execSync(`git init -q --bare -b main "${bare}"`);
  // Working repo on a deterministic `main` branch with one commit.
  execSync(`git init -q -b main "${work}"`);
  const run = (args) => execSync(`git ${args}`, { cwd: work });
  run("config user.name tester");
  run("config user.email tester@example.com");
  // Keep the temp repo hermetic: never inherit a global commit.gpgsign that
  // would make `git commit` fail (or block on a signing agent) during setup.
  run("config commit.gpgsign false");
  writeFileSync(join(work, "file.txt"), "hello\n");
  run("add file.txt");
  run('commit -q -m "initial commit"');
  run(`remote add origin "${bare}"`);
  const head = execSync("git rev-parse HEAD", { cwd: work }).toString().trim();
  return { root, bare, work, head };
}

const bareRev = (bare, ref) =>
  execSync(`git --git-dir="${bare}" rev-parse ${ref}`).toString().trim();

// Core path: pushing a refspec lands the commit on the bare remote.
test("push updates the bare remote ref", (t) => {
  const { root, bare, work, head } = makePushSetup();
  try {
    const repo = new Repository(work);
    const remote = repo.findRemote("origin");
    t.truthy(remote);
    remote.push(["refs/heads/main:refs/heads/main"], null);
    t.is(bareRev(bare, "refs/heads/main"), head);
    // Free the git2 handle before rmSync deletes the temp dir (Windows EBUSY).
    repo.dispose();
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// Options path: a constructed PushOptions flows through push successfully.
test("push accepts a PushOptions instance", (t) => {
  const { root, bare, work, head } = makePushSetup();
  try {
    const repo = new Repository(work);
    const remote = repo.findRemote("origin");
    const options = new PushOptions();
    remote.push(["refs/heads/main:refs/heads/main"], options);
    t.is(bareRev(bare, "refs/heads/main"), head);
    repo.dispose();
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// A push option / header containing an interior NUL byte must surface as a
// thrown JS error, NOT a panic: git2 builds a CString from each string with an
// internal unwrap that would otherwise abort the whole Node process (this crate
// does not opt into catch_unwind).
test("remotePushOptions rejects an interior NUL byte instead of crashing", (t) => {
  const options = new PushOptions();
  const err = t.throws(() => options.remotePushOptions(["ok", "bad\0opt"]));
  t.regex(err.message, /NUL byte/);
  // Valid options still flow through, and the setter chains (returns `this`).
  t.is(options.remotePushOptions(["ci.skip"]), options);
});

test("customHeaders rejects an interior NUL byte instead of crashing", (t) => {
  const options = new PushOptions();
  const err = t.throws(() => options.customHeaders(["X-Ok: 1", "X-Bad: a\0b"]));
  t.regex(err.message, /NUL byte/);
  t.is(options.customHeaders(["X-Ok: 1"]), options);
});

// Callback path: pushUpdateReference fires once per ref with a single object
// argument carrying { refname, status }. status is null on a successful
// (non-rejected) push.
test("pushUpdateReference fires with a single { refname, status } object on success", (t) => {
  const { root, bare, work, head } = makePushSetup();
  try {
    const repo = new Repository(work);
    const remote = repo.findRemote("origin");
    const updates = [];
    const callbacks = new RemoteCallbacks().pushUpdateReference((update) => {
      updates.push(update);
    });
    const options = new PushOptions().remoteCallback(callbacks);
    remote.push(["refs/heads/main:refs/heads/main"], options);
    t.is(updates.length, 1);
    // Exactly one object argument with the documented fields.
    t.is(typeof updates[0], "object");
    t.is(updates[0].refname, "refs/heads/main");
    t.is(updates[0].status, null);
    t.is(bareRev(bare, "refs/heads/main"), head);
    repo.dispose();
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
