import { execSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

import test from "ava";

import {
  FetchOptions,
  PushOptions,
  RemoteCallbacks,
  Repository,
  Signature,
} from "../index.js";

// Every test here is fully local: bare repos on disk act as "remotes", so the
// network is never touched. Async variants run their git work off the main
// thread; we `await` the returned Promises. Caller cleans up `root`.

const sig = () => Signature.now("tester", "tester@example.com");

const bareRev = (bare, ref) =>
  execSync(`git --git-dir="${bare}" rev-parse ${ref}`).toString().trim();

const workRev = (work, ref) =>
  execSync(`git rev-parse ${ref}`, { cwd: work }).toString().trim();

// A bare "remote" with a single commit on `main`, produced by a throwaway work
// repo that pushes into it. Returns the bare path + that commit's OID.
function makeBareWithCommit(root) {
  const bare = join(root, "remote.git");
  const seed = join(root, "seed");
  execSync(`git init -q --bare -b main "${bare}"`);
  execSync(`git init -q -b main "${seed}"`);
  const run = (args) => execSync(`git ${args}`, { cwd: seed });
  run("config user.name tester");
  run("config user.email tester@example.com");
  run("config commit.gpgsign false");
  run("config core.autocrlf false");
  writeFileSync(join(seed, "file.txt"), "hello\n");
  run("add file.txt");
  run('commit -q -m "initial commit"');
  run(`remote add origin "${bare}"`);
  run("push -q origin main");
  return { bare, head: bareRev(bare, "refs/heads/main") };
}

// A working repo wired to a fresh bare remote, with one local commit on `main`
// that has NOT been pushed yet. Mirrors push.spec's setup.
function makePushSetup(root) {
  const bare = join(root, "remote.git");
  const work = join(root, "work");
  execSync(`git init -q --bare -b main "${bare}"`);
  execSync(`git init -q -b main "${work}"`);
  const run = (args) => execSync(`git ${args}`, { cwd: work });
  run("config user.name tester");
  run("config user.email tester@example.com");
  run("config commit.gpgsign false");
  run("config core.autocrlf false");
  writeFileSync(join(work, "file.txt"), "hello\n");
  run("add file.txt");
  run('commit -q -m "initial commit"');
  run(`remote add origin "${bare}"`);
  return { bare, work, head: workRev(work, "HEAD") };
}

// cloneAsync of a local bare repo resolves to a usable Repository whose HEAD
// points at the bare remote's commit.
test("cloneAsync clones a local bare repo into a usable Repository", async (t) => {
  const root = mkdtempSync(join(tmpdir(), "simple-git-clone-async-"));
  try {
    const { bare, head } = makeBareWithCommit(root);
    const dest = join(root, "cloned");
    const repo = await Repository.cloneAsync(bare, dest);
    t.true(repo instanceof Repository);
    // A usable repo: HEAD resolves and points at the remote's commit.
    t.is(repo.head().target(), head);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// commitAsync returns a 40-hex OID that findCommit can resolve, off-thread.
test("commitAsync returns a resolvable commit OID", async (t) => {
  const root = mkdtempSync(join(tmpdir(), "simple-git-commit-async-"));
  const work = join(root, "work");
  try {
    execSync(`git init -q -b main "${work}"`);
    const run = (args) => execSync(`git ${args}`, { cwd: work });
    run("config user.name tester");
    run("config user.email tester@example.com");
    run("config commit.gpgsign false");
    run("config core.autocrlf false");
    const repo = new Repository(work);

    const author = sig();
    writeFileSync(join(work, "a.txt"), "alpha\n");
    const index = repo.index();
    index.addAll();
    index.write();
    const tree = repo.findTree(index.writeTree());

    const oid = await repo.commitAsync("HEAD", author, author, "async root", tree);
    t.is(typeof oid, "string");
    t.is(oid.length, 40);
    t.regex(oid, /^[0-9a-f]{40}$/);

    // The OID resolves and HEAD advanced to it.
    t.truthy(repo.findCommit(oid));
    t.is(workRev(work, "HEAD"), oid);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// pushAsync advances the bare remote ref, like the sync push.
test("pushAsync updates the bare remote ref", async (t) => {
  const root = mkdtempSync(join(tmpdir(), "simple-git-push-async-"));
  try {
    const { bare, work, head } = makePushSetup(root);
    const remote = new Repository(work).findRemote("origin");
    await remote.pushAsync(["refs/heads/main:refs/heads/main"], null);
    t.is(bareRev(bare, "refs/heads/main"), head);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// pushAsync flows a (callback-free) PushOptions through successfully.
test("pushAsync accepts a callback-free PushOptions", async (t) => {
  const root = mkdtempSync(join(tmpdir(), "simple-git-push-async-opts-"));
  try {
    const { bare, work, head } = makePushSetup(root);
    const remote = new Repository(work).findRemote("origin");
    const options = new PushOptions().packbuilderParallelism(1);
    await remote.pushAsync(["refs/heads/main:refs/heads/main"], options);
    t.is(bareRev(bare, "refs/heads/main"), head);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// Documented limitation: JS-backed RemoteCallbacks cannot run off the main
// thread, so pushAsync rejects options that carry them up front (the argument
// is validated synchronously, before any work is scheduled). Use sync push.
test("pushAsync rejects PushOptions carrying RemoteCallbacks", (t) => {
  const root = mkdtempSync(join(tmpdir(), "simple-git-push-async-cb-"));
  try {
    const { work } = makePushSetup(root);
    const remote = new Repository(work).findRemote("origin");
    const callbacks = new RemoteCallbacks().pushUpdateReference(() => {});
    const options = new PushOptions().remoteCallback(callbacks);
    const err = t.throws(() =>
      remote.pushAsync(["refs/heads/main:refs/heads/main"], options),
    );
    t.regex(err.message, /RemoteCallbacks/);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// A fetch custom header containing an interior NUL byte must surface as a
// thrown JS error, NOT a panic: git2 builds a CString from each header with an
// internal unwrap that would otherwise abort the whole Node process (this crate
// does not opt into catch_unwind). Mirrors PushOptions.customHeaders.
test("FetchOptions.customHeaders rejects an interior NUL byte instead of crashing", (t) => {
  const options = new FetchOptions();
  const err = t.throws(() => options.customHeaders(["X-Ok: 1", "X-Bad: a\0b"]));
  t.regex(err.message, /NUL byte/);
  // Valid headers still flow through, and the setter chains (returns `this`).
  t.is(options.customHeaders(["X-Ok: 1"]), options);
});

// libgit2 stores namespaced refs under `<gitdir>/refs/namespaces/<ns>/…`
// (including a per-namespace `HEAD`). Git's own `GIT_NAMESPACE` is
// transport-oriented and does NOT resolve that layout for local plumbing, so
// the helper seeds the on-disk files directly and reads them back by their full
// ref path. `ref` is relative to the namespace root (e.g. `refs/heads/main`).
const nsRefPath = (work, ns, ref) =>
  join(work, ".git", "refs", "namespaces", ns, ref);
const writeNsRef = (work, ns, ref, content) => {
  const p = nsRefPath(work, ns, ref);
  mkdirSync(dirname(p), { recursive: true });
  writeFileSync(p, content);
};

// REGRESSION GUARD: an async worker reopens a fresh git2 handle that does NOT
// inherit the JS handle's in-memory namespace. Before the fix, commitAsync
// wrote the NON-namespaced ref; sync commit() respects the namespace. With an
// active namespace, commitAsync must write under refs/namespaces/<ns>/…
// (FAILS pre-fix, PASSES post-fix).
test("commitAsync respects an active namespace", async (t) => {
  const root = mkdtempSync(join(tmpdir(), "simple-git-commit-async-ns-"));
  const work = join(root, "work");
  try {
    execSync(`git init -q -b main "${work}"`);
    const run = (args) => execSync(`git ${args}`, { cwd: work });
    run("config user.name tester");
    run("config user.email tester@example.com");
    run("config commit.gpgsign false");
    run("config core.autocrlf false");
    writeFileSync(join(work, "file.txt"), "hello\n");
    run("add file.txt");
    run('commit -q -m "initial commit"');
    const base = workRev(work, "HEAD");

    // Give the namespace its own HEAD + branch so libgit2 can resolve HEAD when
    // committing inside it (a commit always resolves HEAD for its reflog).
    writeNsRef(work, "tenant", "HEAD", "ref: refs/heads/main\n");
    writeNsRef(work, "tenant", "refs/heads/main", `${base}\n`);

    const repo = new Repository(work);
    repo.setNamespace("tenant");

    const author = sig();
    writeFileSync(join(work, "b.txt"), "beta\n");
    const index = repo.index();
    index.addAll();
    index.write();
    const tree = repo.findTree(index.writeTree());

    const oid = await repo.commitAsync(
      "refs/heads/feature",
      author,
      author,
      "namespaced async commit",
      tree,
      [base],
    );

    // The ref landed inside the namespace, resolving to the new commit...
    t.is(
      workRev(work, "refs/namespaces/tenant/refs/heads/feature"),
      oid,
    );
    // ...and NOT at the non-namespaced path (the pre-fix behaviour).
    t.false(existsSync(join(work, ".git/refs/heads/feature")));
    t.throws(() =>
      execSync('git rev-parse --verify --quiet "refs/heads/feature"', {
        cwd: work,
      }),
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// REGRESSION GUARD (Remote path): pushAsync reopens a worker handle that must
// re-apply the parent repo's namespace, so the refspec SOURCE resolves through
// the namespace just like sync push. Here the namespaced `refs/heads/main` and
// the non-namespaced one point at DIFFERENT commits; pushing must send the
// namespaced commit. Before the fix the worker dropped the namespace and pushed
// the non-namespaced commit instead.
test("pushAsync resolves the refspec source through an active namespace", async (t) => {
  const root = mkdtempSync(join(tmpdir(), "simple-git-push-async-ns-"));
  try {
    const bare = join(root, "remote.git");
    const work = join(root, "work");
    execSync(`git init -q --bare -b main "${bare}"`);
    execSync(`git init -q -b main "${work}"`);
    const run = (args) => execSync(`git ${args}`, { cwd: work });
    run("config user.name tester");
    run("config user.email tester@example.com");
    run("config commit.gpgsign false");
    run("config core.autocrlf false");

    // First commit C1 on the non-namespaced main.
    writeFileSync(join(work, "file.txt"), "hello\n");
    run("add file.txt");
    run('commit -q -m "first"');
    const c1 = workRev(work, "HEAD");

    // Second commit C2 on top, then rewind non-namespaced main back to C1 so the
    // two ref spaces diverge: plain main = C1, namespaced main = C2.
    writeFileSync(join(work, "file.txt"), "hello again\n");
    run("add file.txt");
    run('commit -q -m "second"');
    const c2 = workRev(work, "HEAD");
    run(`update-ref refs/heads/main ${c1}`);
    run(`update-ref refs/namespaces/tenant/refs/heads/main ${c2}`);

    run(`remote add origin "${bare}"`);

    const repo = new Repository(work);
    repo.setNamespace("tenant");
    const remote = repo.findRemote("origin");
    await remote.pushAsync(["refs/heads/main:refs/heads/main"], null);

    // The remote received the NAMESPACED source commit, not the plain one.
    t.is(bareRev(bare, "refs/heads/main"), c2);
    t.not(c1, c2);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// fetchAsync updates a remote-tracking ref, like the sync fetch.
test("fetchAsync updates a remote-tracking ref", async (t) => {
  const root = mkdtempSync(join(tmpdir(), "simple-git-fetch-async-"));
  try {
    const { bare, head } = makeBareWithCommit(root);
    const consumer = join(root, "consumer");
    execSync(`git init -q -b main "${consumer}"`);
    execSync(`git remote add origin "${bare}"`, { cwd: consumer });

    const remote = new Repository(consumer).findRemote("origin");
    await remote.fetchAsync(
      ["refs/heads/main:refs/remotes/origin/main"],
      null,
    );
    t.is(workRev(consumer, "refs/remotes/origin/main"), head);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
