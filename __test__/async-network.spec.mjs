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
// `content` defaults to a fixed string; callers that need two distinct
// commits (e.g. from two calls in the same wall-clock second, which would
// otherwise produce byte-identical — and therefore identically-hashed —
// commits) can pass different content to guarantee distinct OIDs.
function makeBareWithCommit(root, content = "hello\n") {
  const bare = join(root, "remote.git");
  const seed = join(root, "seed");
  execSync(`git init -q --bare -b main "${bare}"`);
  execSync(`git init -q -b main "${seed}"`);
  const run = (args) => execSync(`git ${args}`, { cwd: seed });
  run("config user.name tester");
  run("config user.email tester@example.com");
  run("config commit.gpgsign false");
  run("config core.autocrlf false");
  writeFileSync(join(seed, "file.txt"), content);
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

// REGRESSION GUARD (Fix A): `Remote.namespace` must not be frozen at
// `findRemote()`/construction time — it has to be queried LIVE from the
// owning `Repository` at `pushAsync` call time. `setNamespace` is called on
// the SAME `repo` variable AFTER `findRemote()` already loaded `remote`
// (order matters: `Remote.inner` shares ownership with that exact `repo`
// object, so this mutates the live state the resulting `Remote` observes —
// a second `new Repository(work)` would NOT reproduce this). Before Fix A the
// worker used the stale (pre-`setNamespace`) namespace captured at
// `findRemote()` time (`None`) and pushed the non-namespaced commit; after
// Fix A it observes "tenant" live and pushes the namespaced commit.
test("pushAsync observes a setNamespace call made after findRemote but before pushAsync", async (t) => {
  const root = mkdtempSync(join(tmpdir(), "simple-git-push-async-ns-live-"));
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

    // Second commit C2 on top, then rewind non-namespaced main back to C1 so
    // the two ref spaces diverge: plain main = C1, namespaced main = C2.
    writeFileSync(join(work, "file.txt"), "hello again\n");
    run("add file.txt");
    run('commit -q -m "second"');
    const c2 = workRev(work, "HEAD");
    run(`update-ref refs/heads/main ${c1}`);
    run(`update-ref refs/namespaces/tenant/refs/heads/main ${c2}`);

    run(`remote add origin "${bare}"`);

    const repo = new Repository(work);
    const remote = repo.findRemote("origin"); // load BEFORE setNamespace
    repo.setNamespace("tenant"); // flip AFTER remote was loaded, SAME repo
    await remote.pushAsync(["refs/heads/main:refs/heads/main"], null);

    // The remote received the NAMESPACED source commit (C2), proving the
    // setNamespace call made after findRemote was observed live.
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

// REGRESSION GUARD: FetchOptions is a one-shot object (its inner git2 state
// is drained via mem::swap on first use). Reusing it must throw SYNCHRONOUSLY
// (the guard runs before AsyncTask::with_optional_signal is constructed, so
// the rejection is not a Promise rejection but a thrown error at call time,
// mirroring "pushAsync rejects PushOptions carrying RemoteCallbacks" above).
// This test MUST fail before the reuse-guard fix and pass after.
test("fetchAsync rejects a reused FetchOptions", async (t) => {
  const root = mkdtempSync(join(tmpdir(), "simple-git-fetch-async-reuse-"));
  try {
    const { bare } = makeBareWithCommit(root);
    const consumer = join(root, "consumer");
    execSync(`git init -q -b main "${consumer}"`);
    execSync(`git remote add origin "${bare}"`, { cwd: consumer });

    const remote = new Repository(consumer).findRemote("origin");
    const options = new FetchOptions();
    await remote.fetchAsync(
      ["refs/heads/main:refs/remotes/origin/main"],
      options,
    );
    const err = t.throws(() =>
      remote.fetchAsync(
        ["refs/heads/main:refs/remotes/origin/main"],
        options,
      ),
    );
    t.regex(err.message, /FetchOptions can only be used once/);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// REGRESSION GUARD (Fix B revert): fetchAsync resolves the remote by NAME
// against the repository's CURRENT on-disk configuration at compute time
// (restored `find_remote(name)` resolution — see the round-3 brief), not
// against a URL snapshot captured when the JS `Remote` was loaded. Two
// distinct bare "remotes" (bareA, bareB) with different HEAD commits; the
// consumer's `origin` starts out pointing at bareA. `remote` is loaded while
// that is still true. The on-disk config is then mutated to point `origin` at
// bareB AFTER `remote` was loaded (without going through `remote` itself).
// This is the documented live-config caveat reintroduced by reverting the
// round-2 `remote_anonymous` snapshot resolution (round 2 traded this for an
// unbounded set of dropped name-keyed config, e.g. `tagOpt`, that has no safe
// git2-rs getter to snapshot — see the round-3 brief for the full rationale).
// Must fail if some future change reintroduces snapshot isolation without
// updating the docs on `fetchAsync`.
test("fetchAsync observes a remoteSetUrl made after the Remote was loaded (documented live-config caveat)", async (t) => {
  const root = mkdtempSync(join(tmpdir(), "simple-git-fetch-async-live-config-"));
  try {
    const { bare: bareA, head: headA } = makeBareWithCommit(
      join(root, "a"),
      "hello from A\n",
    );
    const { bare: bareB, head: headB } = makeBareWithCommit(
      join(root, "b"),
      "hello from B\n",
    );
    t.not(headA, headB);

    const consumer = join(root, "consumer");
    execSync(`git init -q -b main "${consumer}"`);
    execSync(`git remote add origin "${bareA}"`, { cwd: consumer });

    const remote = new Repository(consumer).findRemote("origin");

    // Mutate on-disk config AFTER `remote` was loaded, not through `remote`.
    execSync(`git remote set-url origin "${bareB}"`, { cwd: consumer });

    await remote.fetchAsync(
      ["refs/heads/main:refs/remotes/origin/main"],
      null,
    );

    // The LIVE config (bareB) wins, not the snapshot captured at load time
    // (bareA) — proving the documented behavior on fetchAsync is accurate.
    t.is(workRev(consumer, "refs/remotes/origin/main"), headB);
    t.not(workRev(consumer, "refs/remotes/origin/main"), headA);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// REGRESSION GUARD for the intentional fetchAsync/pushAsync asymmetry:
// pushAsync deliberately stays on the round-2 URL-snapshot mechanism
// (`remote_anonymous` from a captured effective push URL) instead of
// reverting to `find_remote(name)` the way fetchAsync did (see the round-3
// brief/report). The reason is a confirmed libgit2 bug, not a style choice:
// the local transport's `local_push()` re-derives the push destination
// directly from the remote's fetch `url`, ignoring a configured `pushurl`,
// unless the remote's SOLE url is already the pushurl-resolved value —
// verified by reproducing the same silent-wrong-destination failure through
// the synchronous `push()` method (`find_remote` + `.push()`) directly.
// `origin`'s fetch `url` here points at a SECOND real (but unrelated, unseeded)
// bare repo; a separate `pushurl` points at the real target bare repo. Using a
// real-but-wrong bare repo for the fetch url (rather than a nonexistent path)
// matters: pushAsync must land on the effective push target (pushurl) and
// must NOT silently write to the bogus fetch-url repo. A nonexistent path
// would only prove "an error was thrown", not "the wrong destination was
// avoided" — the bogus repo lets us assert the latter directly. If
// `push_async` is ever "simplified" to match `fetch_async`'s
// `find_remote(name)` design, this test must catch the resulting silent push
// to the wrong (bogus) destination.
test("pushAsync uses the remote's configured pushurl, not its fetch url", async (t) => {
  const root = mkdtempSync(join(tmpdir(), "simple-git-push-async-pushurl-"));
  try {
    const bare = join(root, "remote.git");
    // Second, real bare repo standing in for the (wrong) fetch url. Left
    // unseeded/empty so `refs/heads/main` never exists on it — a silent
    // regression to `find_remote(name)`-based push resolution would create
    // it there instead of on `bare`.
    const bogusBare = join(root, "bogus-remote.git");
    const work = join(root, "work");
    execSync(`git init -q --bare -b main "${bare}"`);
    execSync(`git init -q --bare -b main "${bogusBare}"`);
    execSync(`git init -q -b main "${work}"`);
    const run = (args) => execSync(`git ${args}`, { cwd: work });
    run("config user.name tester");
    run("config user.email tester@example.com");
    run("config commit.gpgsign false");
    run("config core.autocrlf false");
    writeFileSync(join(work, "file.txt"), "hello\n");
    run("add file.txt");
    run('commit -q -m "initial commit"');
    const head = workRev(work, "HEAD");

    // Fetch url is a real-but-wrong bare repo; pushurl is the real target.
    execSync(`git remote add origin "${bogusBare}"`, {
      cwd: work,
    });
    execSync(`git remote set-url --push origin "${bare}"`, { cwd: work });

    const remote = new Repository(work).findRemote("origin");
    await remote.pushAsync(["refs/heads/main:refs/heads/main"], null);

    // The target (pushurl) bare repo received the push...
    t.is(bareRev(bare, "refs/heads/main"), head);
    // ...and the bogus (fetch-url) bare repo was never touched.
    t.false(existsSync(join(bogusBare, "refs/heads/main")));
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// Baseline coverage of `find_remote`'s natural refspec inheritance: with an
// EMPTY refspecs array, fetchAsync must fall through to the remote's
// configured fetch refspecs (the default `+refs/heads/*:refs/remotes/origin/*`
// set up automatically by `git remote add`), not just skip the fetch.
// `git2::Remote::fetch` supplies this automatically for a `find_remote`-loaded
// remote given an empty refspec slice — no manual capture/substitution needed
// (round 2 briefly added one; the round-3 revert deleted it again, see the
// round-3 brief).
test("fetchAsync falls back to the remote's configured refspecs when refspecs is empty", async (t) => {
  const root = mkdtempSync(join(tmpdir(), "simple-git-fetch-async-fallback-"));
  try {
    const { bare, head } = makeBareWithCommit(root);
    const consumer = join(root, "consumer");
    execSync(`git init -q -b main "${consumer}"`);
    execSync(`git remote add origin "${bare}"`, { cwd: consumer });

    const remote = new Repository(consumer).findRemote("origin");
    await remote.fetchAsync([], null);

    t.is(workRev(consumer, "refs/remotes/origin/main"), head);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
