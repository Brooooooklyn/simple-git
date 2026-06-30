import { execSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import test from "ava";

import {
  Cred,
  CredentialType,
  DiffFlags,
  RemoteUpdateFlags,
  Repository,
  RepositoryOpenFlags,
  Sort,
  credTypeContains,
  diffFlagsContains,
} from "../index.js";

// Spin up an isolated repo under os.tmpdir() with one committed text file, then
// dirty it so a tree-to-workdir diff produces a real delta. Caller cleans up.
function makeTempRepo() {
  const dir = mkdtempSync(join(tmpdir(), "simple-git-bitflags-"));
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

// Two linear commits (initial, then second). Returns the dir plus both OID
// hex strings, oldest-first, so order assertions on a walk are deterministic.
function makeTwoCommitRepo() {
  const dir = mkdtempSync(join(tmpdir(), "simple-git-revwalk-"));
  const run = (args) => execSync(`git ${args}`, { cwd: dir });
  run("init -q");
  run("config user.name tester");
  run("config user.email tester@example.com");
  run("config commit.gpgsign false");
  run("config core.autocrlf false");
  writeFileSync(join(dir, "f.txt"), "v1\n");
  run("add f.txt");
  run("commit -q -m initial");
  writeFileSync(join(dir, "f.txt"), "v2\n");
  run("add f.txt");
  run("commit -q -m second");
  const first = execSync("git rev-parse HEAD~1", { cwd: dir }).toString().trim();
  const second = execSync("git rev-parse HEAD", { cwd: dir }).toString().trim();
  return { dir, first, second };
}

// A freshly-created bare repository directory (the dir itself is the gitdir).
function makeBareRepo() {
  const dir = mkdtempSync(join(tmpdir(), "simple-git-bare-"));
  execSync("git init --bare -q", { cwd: dir });
  return dir;
}

// --- DiffDelta.flags(): the live bug ---------------------------------------

// A real delta's flags() must be the raw libgit2 `git_diff_delta.flags` bitset
// returned as a `number`. The old `_ => DiffFlags::Binary` arm collapsed every
// value that wasn't exactly one single-bit constant (including the common empty
// bitset, 0) to Binary (1) — so flags() reported Binary for ~every delta. This
// proves the collapse is gone: the real bitset is returned, and it is NOT Binary
// for a plainly-modified, existing text file.
//
// Note: the per-file EXISTS bit lives on `git_diff_file.flags` (surfaced via
// DiffFile.exists()), not on the delta-level `git_diff_delta.flags` that
// flags() reads; libgit2 leaves the delta-level public bits empty during plain
// diff iteration, so we assert at the correct layer.
test("DiffDelta.flags() returns the raw delta bitset as a number (not Binary)", (t) => {
  const dir = makeTempRepo();
  try {
    writeFileSync(join(dir, "committed.txt"), "v2\n");
    const repo = new Repository(dir);
    const tree = repo.head().peelToTree();
    const diff = repo.diffTreeToWorkdir(tree);
    const deltas = [...diff.deltas()];
    const delta = deltas.find(
      (d) => d.newFile().path() === "committed.txt" || d.oldFile().path() === "committed.txt",
    );
    t.truthy(delta, "expected a delta for committed.txt");

    const flags = delta.flags();
    t.is(typeof flags, "number", "flags() must be a raw number, not an enum");
    // Old code returned DiffFlags.Binary (1) here via the `_ => Binary` collapse.
    t.not(flags, DiffFlags.Binary, "old _ => Binary collapse must be gone");
    t.false(diffFlagsContains(flags, DiffFlags.Binary), "Binary bit must not be set");
    // The file plainly exists on both sides (per-file EXISTS), which is exactly
    // why the old "Binary for everything" result was a bug.
    t.true(delta.newFile().exists());
    t.true(delta.oldFile().exists());
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// --- diffFlagsContains: pure bit math --------------------------------------

test("diffFlagsContains is (flags & flag) === flag", (t) => {
  const combined = DiffFlags.Exists | DiffFlags.ValidId;
  t.true(diffFlagsContains(combined, DiffFlags.Exists));
  t.true(diffFlagsContains(combined, DiffFlags.ValidId));
  t.false(diffFlagsContains(combined, DiffFlags.Binary));
  t.false(diffFlagsContains(DiffFlags.NotBinary, DiffFlags.Binary));
});

// --- Cred.credtype(): returns raw bits as a number -------------------------

test("Cred.credtype() returns the raw CredentialType bits as a number", (t) => {
  const username = Cred.username("x").credtype();
  t.is(typeof username, "number");
  t.is(username, CredentialType.Username);

  const userpass = Cred.userpassPlaintext("u", "p").credtype();
  t.is(typeof userpass, "number");
  t.is(userpass, CredentialType.UserPassPlaintext);
});

// --- credTypeContains: pure bit math on raw bits ---------------------------

test("credTypeContains operates on raw bits", (t) => {
  const mask = CredentialType.Username | CredentialType.SshKey;
  t.true(credTypeContains(mask, CredentialType.Username));
  t.true(credTypeContains(mask, CredentialType.SshKey));
  t.false(credTypeContains(mask, CredentialType.UserPassPlaintext));
  t.false(credTypeContains(CredentialType.Username, CredentialType.SshKey));
});

// --- RevWalk.setSorting(number): OR-able sort flags now take effect ---------

// Sort.Time | Sort.Reverse === 6, which is NOT a single Sort discriminant. The
// old `setSorting(sorting: Sort)` signature ran the raw value through napi enum
// coercion, which threw on 6 (no matching variant) — combined sort modes were
// unreachable. Now the param is a raw `number` fed through `from_bits_truncate`,
// so the combination applies: REVERSE flips the default newest-first walk into
// oldest-first order.
test("setSorting(Sort.Time | Sort.Reverse) walks oldest-first", (t) => {
  const { dir, first, second } = makeTwoCommitRepo();
  try {
    const repo = new Repository(dir);
    const walk = repo.revWalk();
    walk.setSorting(Sort.Time | Sort.Reverse);
    walk.pushHead();
    const oids = [...walk];
    t.deepEqual(oids, [first, second], "REVERSE must yield oldest-first");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("setSorting(Sort.Time) (single flag) still walks newest-first", (t) => {
  const { dir, first, second } = makeTwoCommitRepo();
  try {
    const repo = new Repository(dir);
    const walk = repo.revWalk();
    walk.setSorting(Sort.Time);
    walk.pushHead();
    const oids = [...walk];
    t.deepEqual(oids, [second, first], "TIME alone is newest-first");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// --- Repository.openExt(number): OR-able open flags, real libgit2 bits -------

// The const-enum discriminants must equal the real libgit2 GIT_REPOSITORY_OPEN_*
// bits (1, 2, 4, 8, 16). The old enum used sequential 0..4, so any combination
// silently miscoerced (e.g. Bare carried 2 instead of 4, NoSearch carried 0).
test("RepositoryOpenFlags discriminants equal the real libgit2 bits", (t) => {
  t.is(RepositoryOpenFlags.NoSearch, 1);
  t.is(RepositoryOpenFlags.CrossFS, 2);
  t.is(RepositoryOpenFlags.Bare, 4);
  t.is(RepositoryOpenFlags.NoDotGit, 8);
  t.is(RepositoryOpenFlags.FromEnv, 16);
});

test("openExt(path, NoSearch | Bare, []) opens a bare repo", (t) => {
  const dir = makeBareRepo();
  try {
    const flags = RepositoryOpenFlags.NoSearch | RepositoryOpenFlags.Bare;
    const repo = Repository.openExt(dir, flags, []);
    // A bare repository has no working directory.
    t.is(repo.workdir(), null);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("openExt(path, NoSearch, []) (single flag) opens the repo", (t) => {
  const dir = makeBareRepo();
  try {
    const repo = Repository.openExt(dir, RepositoryOpenFlags.NoSearch, []);
    t.is(repo.workdir(), null);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// --- RemoteUpdateFlags: discriminants are real libgit2 bits and OR-able ------

test("RemoteUpdateFlags discriminants equal the real libgit2 bits", (t) => {
  t.is(RemoteUpdateFlags.UpdateFetchHead, 1);
  t.is(RemoteUpdateFlags.ReportUnchanged, 2);
  t.is(RemoteUpdateFlags.UpdateFetchHead | RemoteUpdateFlags.ReportUnchanged, 3);
});
