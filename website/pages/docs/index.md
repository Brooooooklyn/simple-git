---
title: 'Getting Started'
description: 'Fast, native Git for Node.js — read the last-modified commit time of any file, run status, stage and commit in-process, with no child git process.'
---

# Getting Started

`@napi-rs/simple-git` is a fast, native Git library for **Node.js**, built on [libgit2](https://libgit2.org/) via [git2-rs](https://github.com/rust-lang/git2-rs) and shipped as a prebuilt binary through [napi-rs](https://napi.rs/). It works with your repository **in-process** — no child `git` process, no shelling out — so reading history, computing status, staging and committing are plain function calls. The heavy operations also have `*Async` twins that run off the main thread.

Its headline job: computing a doc-site's per-page **"last updated"** timestamp from Git history in milliseconds, instead of spawning a `git log` for every page.

[[toc]]

## Install

Install from npm with your package manager of choice:

```bash
npm install @napi-rs/simple-git
```

```bash
yarn add @napi-rs/simple-git
```

```bash
pnpm add @napi-rs/simple-git
```

```bash
bun add @napi-rs/simple-git
```

## Platform support

Prebuilt native binaries ship for **15 targets**, resolved automatically at `import` time — **no toolchain, no compiler, no `node-gyp`**. Install the package, `import` it, done.

| OS | Targets |
| --- | --- |
| macOS | `x86_64-apple-darwin`, `aarch64-apple-darwin` |
| Windows | `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc`, `i686-pc-windows-msvc` |
| Linux · glibc | `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `armv7-unknown-linux-gnueabihf`, `powerpc64le-unknown-linux-gnu`, `s390x-unknown-linux-gnu` |
| Linux · musl | `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl` |
| Android | `aarch64-linux-android`, `armv7-linux-androideabi` |
| FreeBSD | `x86_64-unknown-freebsd` |

Requires **Node.js >= 10**.

## 60-second example

Open a repository, read a file's last-modified commit time, check the working-tree status, then stage and commit:

```ts
import { Repository } from '@napi-rs/simple-git'

// Open an existing repository.
const repo = new Repository('/path/to/repo')

// When did a commit last touch this file? Two flavours:
// `getFileLatestModifiedDate` returns milliseconds since the Unix epoch, and
// throws if no commit in history ever touched the path.
const ms = repo.getFileLatestModifiedDate('README.md')
console.log(new Date(ms)) // 2022-03-13T12:47:47.920Z

// `getFileLastModifiedDate` is the null-safe twin: a `Date`, or `null` (never
// throws) when the path has no commit history.
const when = repo.getFileLastModifiedDate('README.md')
if (when) console.log(when.toISOString())

// Working-tree status, like `git status`.
for (const file of repo.statuses()) {
  console.log(file.path, file.isWtModified, file.isIndexNew)
}

// Stage a file and commit it.
const sig = repo.signature() // built from user.name / user.email
const index = repo.index() // the staging area
index.addPath('README.md')
index.write()
const tree = repo.findTree(index.writeTree())! // OID of the staged tree
const parent = repo.head().target()! // current tip OID
const commitId = repo.commit('HEAD', sig, sig, 'docs: update readme', tree, [parent])
console.log(commitId) // 40-char hex OID
```

## Doc-site last-updated

The flagship use case — a per-page **"last updated"** timestamp. `getFilesLatestModified` resolves the last commit that touched **each** of many files in a **single** history walk — it early-exits once every path is found, so a whole docs tree costs one pass instead of one `git log` per page:

```ts
import { Repository } from '@napi-rs/simple-git'

const repo = new Repository(process.cwd())

const mods = repo.getFilesLatestModified([
  'docs/getting-started.md',
  'docs/api.md',
  'docs/guides/deploy.md',
])

// Every input path is present as a key; a never-committed file maps to `null`.
for (const [path, mod] of Object.entries(mods)) {
  if (mod) {
    // `committerTime` is a `Date`; identity fields carry who last changed it.
    console.log(path, '→', mod.committerTime.toISOString(), 'by', mod.authorName)
  }
}
```

This is the pattern behind every documentation site's per-page **"Last updated on …"** line. Frameworks such as [Nextra](https://nextra.site/), [Docusaurus](https://docusaurus.io/), [Starlight](https://starlight.astro.build/), [Fumadocs](https://fumadocs.dev/) and [Rspress](https://rspress.rs/) derive that timestamp from Git history — traditionally by spawning a `git log` per page at build time. `getFilesLatestModified` folds all of them into one in-process walk, which is dramatically faster on a large docs tree.

## Async and AbortSignal

The expensive operations have `*Async` variants — `cloneAsync`, `commitAsync`, `statusesAsync`, `getFilesLatestModifiedAsync`, `blameFileAsync`, `fetchAsync`, `pushAsync`, and the rest — that run their Git work on a worker thread and return a `Promise`, keeping the event loop free. Each one accepts an optional `AbortSignal` as its last argument:

```ts
import { Repository } from '@napi-rs/simple-git'

// Clone off the main thread; resolves with a ready-to-use Repository.
const repo = await Repository.cloneAsync('https://example.com/repo.git', '/tmp/clone')

// Off-thread status scan.
const changed = await repo.statusesAsync({ includeIgnored: true })

// Cancel a slow call with an AbortSignal.
const ac = new AbortController()
setTimeout(() => ac.abort(), 1_000)
try {
  const mods = await repo.getFilesLatestModifiedAsync(['docs/api.md'], ac.signal)
  console.log(mods)
} catch (e) {
  // An aborted call rejects with napi's AbortError, whose `code === 'Cancelled'`.
  if ((e as { code?: string }).code === 'Cancelled') console.log('aborted')
}
```

> `fetchAsync` / `pushAsync` accept **data-only** options; they do not take `RemoteCallbacks` (JS callbacks can't run on the worker thread). Use the synchronous `fetch()` / `push()` when you need credential or progress callbacks.

## Typed errors

Every Git-layer error this library throws — from a synchronous method or a rejected `*Async` promise — is a standard `Error` carrying a `code` from the **`GitErrorCode`** const enum (the one exception is abort cancellation, covered below). Narrow any caught value with the total **`isGitError`** type guard:

```ts
import { isGitError, GitErrorCode } from '@napi-rs/simple-git'

try {
  repo.findRemote('does-not-exist')
} catch (e) {
  if (isGitError(e) && e.code === GitErrorCode.NotFound) {
    // handle the missing remote
  }
}
```

`isGitError(e)` returns `true` only when `e` is a genuine `Error` whose `code` is a real member of `GitErrorCode` (e.g. `NotFound`, `Exists`, `Auth`, `NotFastForward`, `Conflict`), and narrows it to `Error & { code: GitErrorCode }` in TypeScript. It is total — it never throws for any input — so it is always safe inside a `catch`. Note that an **aborted** `*Async` call rejects with napi's `AbortError` (`code === 'Cancelled'`), which is *not* a `GitErrorCode` member, so `isGitError` returns `false` for it — detect cancellation via the `AbortSignal` or the `'Cancelled'` code instead.

## Resource cleanup

`dispose()` — and its alias `free()` — eagerly releases the native git2 handle without waiting for garbage collection; both are idempotent. After disposal every throwing method (and every handle derived from the repo — `Remote`, `Tree`, `Commit`, `Reference`, …) throws `"Repository has been disposed"`:

```ts
const repo = new Repository('/path/to/repo')
try {
  // …use the repo…
} finally {
  repo.dispose() // or repo.free()
}
```

napi cannot emit `Symbol.dispose`, so `using` support is opt-in with a single line at startup — after that, a `using` binding disposes the repo automatically at the end of its block:

```ts
import { Repository } from '@napi-rs/simple-git'

Repository.prototype[Symbol.dispose] ??= Repository.prototype.dispose

{
  using repo = new Repository('/path/to/repo')
  console.log(repo.getFileLastModifiedDate('README.md'))
} // repo.dispose() runs automatically here
```

Disposal does **not** cancel `*Async` operations already in flight — a worker scheduled before `dispose()` reopens the repository on its own thread and runs to completion. To cancel a pending async op, pass an `AbortSignal` rather than relying on `dispose()`.

## Full API

This page is the quick tour. The complete surface — every `Repository` method, plus `Config`, `Index`, `Branch`, blame, push callbacks, diff options and the full `GitErrorCode` table — lives in the project README, and the bundled `index.d.ts` documents each method and option inline so your editor surfaces it as you type.

- [**GitHub README →**](https://github.com/Brooooooklyn/simple-git#readme) — full API reference and usage.
- [**Type definitions (`index.d.ts`) →**](https://github.com/Brooooooklyn/simple-git/blob/main/index.d.ts) — every method and option, documented inline.
- [**npm package →**](https://www.npmjs.com/package/@napi-rs/simple-git) — install and version info.
