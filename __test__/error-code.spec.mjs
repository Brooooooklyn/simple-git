import { execSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import test from "ava";

import {
  FetchOptions,
  GitErrorCode,
  isGitError,
  RepoBuilder,
  Repository,
  Signature,
} from "../index.js";

// Every test operates on a throwaway repo under os.tmpdir() and never touches
// the project's own repo. Each repo is hermetic: commit.gpgsign=false so a
// global signing config cannot make commits fail/hang, and core.autocrlf=false
// so line-ending rewrites do not perturb blobs. Caller cleans up `root`.
function makeRepo() {
  const root = mkdtempSync(join(tmpdir(), "simple-git-error-code-"));
  const work = join(root, "work");
  execSync(`git init -q -b main "${work}"`);
  const run = (args) => execSync(`git ${args}`, { cwd: work });
  run("config user.name tester");
  run("config user.email tester@example.com");
  run("config commit.gpgsign false");
  run("config core.autocrlf false");
  const repo = new Repository(work);
  return { root, work, repo };
}

const sig = () => Signature.now("tester", "tester@example.com");

// Stage `a.txt`, write the index as a tree, and make a root commit. Returns the
// commit OID and its tree so callers have a real OID/tree to work with.
function seedCommit(repo, work) {
  writeFileSync(join(work, "a.txt"), "alpha\n");
  const index = repo.index();
  index.addAll();
  index.write();
  const tree = repo.findTree(index.writeTree());
  const oid = repo.commit("HEAD", sig(), sig(), "first", tree);
  return { oid, tree };
}

// -------- SYNC: every thrown error carries a git2-derived `error.code` --------

test("opening a missing repository throws code NotFound", (t) => {
  const root = mkdtempSync(join(tmpdir(), "simple-git-error-code-"));
  try {
    t.throws(() => new Repository(join(root, "does-not-exist")), {
      code: "NotFound",
    });
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("creating a reference that already exists throws code Exists", (t) => {
  const { root, work, repo } = makeRepo();
  try {
    const { oid } = seedCommit(repo, work);
    repo.reference("refs/heads/dup", oid, false, "create");
    t.throws(() => repo.reference("refs/heads/dup", oid, false, "again"), {
      code: "Exists",
    });
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("creating a tag that already exists throws code Exists", (t) => {
  const { root, work, repo } = makeRepo();
  try {
    const { oid } = seedCommit(repo, work);
    const target = repo.findCommit(oid).asObject();
    repo.tagLightweight("dup-tag", target, false);
    t.throws(() => repo.tagLightweight("dup-tag", target, false), {
      code: "Exists",
    });
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// Exercises the `Result<&Self>` chaining path (revWalk.pushRange), which carries
// its git2 code via `coded_error`/`code_into` rather than the plain throw path.
test("pushing an unparsable revision range throws code InvalidSpec", (t) => {
  const { root, work, repo } = makeRepo();
  try {
    seedCommit(repo, work);
    const revWalk = repo.revWalk();
    t.throws(() => revWalk.pushRange("this is not a range"), {
      code: "InvalidSpec",
    });
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("a custom header with an interior NUL byte throws code InvalidArg", (t) => {
  t.throws(() => new FetchOptions().customHeaders(["a\0b"]), {
    code: "InvalidArg",
  });
});

test("reusing a single FetchOptions twice throws code InvalidArg", (t) => {
  const options = new FetchOptions();
  const builder = new RepoBuilder();
  builder.fetchOptions(options);
  t.throws(() => builder.fetchOptions(options), { code: "InvalidArg" });
});

// -------- ASYNC: rejected promises carry `error.code` via `coded_error` -------

test("commitAsync with a nonexistent parent OID rejects with code NotFound", async (t) => {
  const { root, work, repo } = makeRepo();
  try {
    const { tree } = seedCommit(repo, work);
    // The all-zero OID is well-formed, so it parses and then fails in
    // `find_commit`, which libgit2 reports as GIT_ENOTFOUND -> `NotFound`.
    await t.throwsAsync(
      () =>
        repo.commitAsync("HEAD", sig(), sig(), "orphan", tree, [
          "0000000000000000000000000000000000000000",
        ]),
      { code: "NotFound" },
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("blameFileAsync on a missing path rejects with code NotFound", async (t) => {
  const { root, work, repo } = makeRepo();
  try {
    seedCommit(repo, work);
    // The path is absent from the tree, so libgit2 reports GIT_ENOTFOUND.
    await t.throwsAsync(() => repo.blameFileAsync("does-not-exist.txt"), {
      code: "NotFound",
    });
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("statusesAsync on a bare repository rejects with code BareRepo", async (t) => {
  const root = mkdtempSync(join(tmpdir(), "simple-git-error-code-"));
  try {
    const bare = join(root, "bare.git");
    execSync(`git init -q --bare "${bare}"`);
    const repo = new Repository(bare);
    // Status is disallowed on a bare repo -> GIT_EBAREREPO -> `BareRepo`.
    await t.throwsAsync(() => repo.statusesAsync(), { code: "BareRepo" });
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// Fast-failing bad URL: a nonexistent local path resolves via the local
// transport and fails immediately, so no pack is downloaded (no Windows .pack
// EBUSY is introduced ahead of the Task-3 dispose fix).
test("fetchAsync against a nonexistent local remote rejects with a git2 code", async (t) => {
  const { root, work, repo } = makeRepo();
  try {
    seedCommit(repo, work);
    const remote = repo.remote("badorigin", join(root, "no-such-remote.git"));
    const err = await t.throwsAsync(() => remote.fetchAsync([]));
    // The exact network/URL classification varies by platform (on this host it
    // is `GenericError`), but the flip must always surface a real GitCode token,
    // never the raw napi Status placeholder `GenericFailure` that the pre-flip
    // async reject path produced.
    t.is(typeof err.code, "string");
    t.true(err.code.length > 0);
    t.not(err.code, "GenericFailure");
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// -------- GitErrorCode enum + isGitError type guard (published TS surface) ----

// Piece 1: the enum is a real runtime export whose members equal their string
// tokens (the `string_enum` macro emits the variant identifier verbatim).
test("GitErrorCode members equal their string tokens", (t) => {
  t.is(GitErrorCode.GenericError, "GenericError");
  t.is(GitErrorCode.NotFound, "NotFound");
  t.is(GitErrorCode.Exists, "Exists");
  t.is(GitErrorCode.InvalidSpec, "InvalidSpec");
  t.is(GitErrorCode.InvalidArg, "InvalidArg");
});

// Piece 2 + end-to-end proof of the identifier==AsRef invariant on the SYNC
// throw path: the runtime `.code` equals the generated enum member for the
// same variant, so `e.code === GitErrorCode.NotFound` holds.
test("isGitError narrows a real thrown git error to its GitErrorCode", (t) => {
  const root = mkdtempSync(join(tmpdir(), "simple-git-error-code-"));
  try {
    const err = t.throws(() => new Repository(join(root, "does-not-exist")));
    t.true(isGitError(err));
    t.is(typeof err.code, "string");
    t.is(err.code, GitErrorCode.NotFound);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

// Same invariant on the ASYNC (coded_error) reject path.
test("isGitError narrows an async-rejected git error to its GitErrorCode", async (t) => {
  const { root, work, repo } = makeRepo();
  try {
    const { tree } = seedCommit(repo, work);
    const err = await t.throwsAsync(() =>
      repo.commitAsync("HEAD", sig(), sig(), "orphan", tree, [
        "0000000000000000000000000000000000000000",
      ]),
    );
    t.true(isGitError(err));
    t.is(err.code, GitErrorCode.NotFound);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("isGitError rejects a plain Error without a code", (t) => {
  t.is(isGitError(new Error("x")), false);
});

test("isGitError rejects an Error whose code is not a string", (t) => {
  const e = new Error("x");
  e.code = 42;
  t.is(isGitError(e), false);
});

test("isGitError rejects non-errors, plain objects, and null/undefined", (t) => {
  t.is(isGitError(null), false);
  t.is(isGitError(undefined), false);
  t.is(isGitError({}), false);
  t.is(isGitError("NotFound"), false);
  t.is(isGitError(123), false);
  // A PLAIN object carrying a string `code` is NOT an `Error` instance, so the
  // `instanceof Error` check we shipped rejects it (a bare shape check would not).
  t.is(isGitError({ code: "NotFound" }), false);
});

test("isGitError rejects a non-git Error whose string code is not a GitErrorCode member (ENOENT)", (t) => {
  // The guard is SOUND: membership is validated against the generated
  // `GitErrorCode` enum, so a Node system error like ENOENT — an `Error` with a
  // non-member string `.code` — does NOT narrow to a git error.
  const e = new Error("boom");
  e.code = "ENOENT";
  t.is(isGitError(e), false);
});

test("isGitError rejects an aborted-call code that is not a GitErrorCode member (Cancelled)", (t) => {
  // napi's `AbortSignal` cancellation surfaces `code: 'Cancelled'`, a napi-level
  // token that is NOT a `GitErrorCode` member, so the guard rejects it.
  const e = new Error("The operation was aborted");
  e.code = "Cancelled";
  t.is(isGitError(e), false);
});

test("isGitError is total: an Error with a throwing `code` getter returns false and does not throw", (t) => {
  const e = new Error("boom");
  Object.defineProperty(e, "code", {
    get() {
      throw new Error("code getter blew up");
    },
    enumerable: true,
    configurable: true,
  });
  let out;
  t.notThrows(() => {
    out = isGitError(e);
  });
  t.is(out, false);
  // The cleared pending exception must not leak into the next call.
  t.is(isGitError(new Error("x")), false);
  const member = new Error("m");
  member.code = GitErrorCode.NotFound;
  t.is(isGitError(member), true);
});

test("isGitError is total: a hostile Proxy with a throwing getPrototypeOf trap returns false and does not throw", (t) => {
  // A JS-level `instanceof` invokes `[[GetPrototypeOf]]`; the native
  // `napi_is_error` check does NOT, so this proxy is classified as a non-error
  // without ever running the trap. Proxies are not native errors -> `false`.
  const hostile = new Proxy(Object.create(null), {
    getPrototypeOf() {
      throw new Error("getPrototypeOf trap blew up");
    },
  });
  let out;
  t.notThrows(() => {
    out = isGitError(hostile);
  });
  t.is(out, false);
});

test("isGitError is total: a throwing Error[Symbol.hasInstance] cannot hijack the guard", (t) => {
  // A JS-level `instanceof Error` consults `Error[Symbol.hasInstance]`; the
  // native `napi_is_error` check never does. Mutate -> call -> restore
  // SYNCHRONOUSLY (no `await` between them) so the override is atomic with
  // respect to ava's in-file test concurrency and cannot leak into other tests.
  // `Error[Symbol.hasInstance]` is inherited from `Function.prototype`, so there
  // is no OWN descriptor to capture; deleting the own shadow we install restores
  // the inherited behaviour.
  const ownDescriptor = Object.getOwnPropertyDescriptor(
    Error,
    Symbol.hasInstance,
  );
  try {
    Object.defineProperty(Error, Symbol.hasInstance, {
      value() {
        throw new Error("hasInstance blew up");
      },
      configurable: true,
    });
    let out;
    t.notThrows(() => {
      out = isGitError(new Error("x"));
    });
    t.is(out, false);
  } finally {
    // Always restore, even if an assertion above threw.
    if (ownDescriptor) {
      Object.defineProperty(Error, Symbol.hasInstance, ownDescriptor);
    } else {
      delete Error[Symbol.hasInstance];
    }
  }
});
