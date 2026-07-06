---
title: 'API Reference'
description: 'The complete @napi-rs/simple-git surface — Repository, git object handles, options, enums, functions and typed error handling, all from the package root.'
---

# API Reference

Everything is exported from the package root:

```ts
import { Repository, isGitError, GitErrorCode } from '@napi-rs/simple-git'
```

This page is organized by kind: the `Repository` class first, then the object handles it returns, the option and result shapes those methods use, the enums, the standalone functions, and finally error handling.

[[toc]]

## Repository

The primary entry point — open, clone, inspect and mutate a repository in-process, with an `*Async` twin for each expensive operation.

Every `*Async` method runs its Git work on a worker thread and returns a `Promise`, taking an optional `AbortSignal` as its final argument. Aborting rejects that promise with napi's `AbortError`, whose `code === 'Cancelled'` — which is **not** a `GitErrorCode`, so `isGitError` returns `false` for it; detect cancellation via the signal or the `'Cancelled'` code. The `*Async` entries below describe only their own result and error cases; the `AbortSignal` contract is this one.

### Construction & opening

#### new Repository(gitDir)

```ts
constructor(gitDir: string)
```

Attempt to open an already-existing repository at `gitDir`. The path can point to either a normal or a bare repository. This is the primary open primitive — pass a repository (or `.git`) directory and get a handle back.

#### Repository.openExt

```ts
static openExt(path: string, flags: number, ceilingDirs: Array<string>): Repository
```

Find and open an existing repository, with additional options. `flags` is a raw bitset of `RepositoryOpenFlags` OR-ed together (e.g. `RepositoryOpenFlags.NoSearch | RepositoryOpenFlags.CrossFS`); unknown bits are ignored:

- `NoSearch` — only open the repository at `path`; do not walk upward through parent directories searching for one.
- `CrossFS` — when searching upward, allow crossing filesystem boundaries.
- `Bare` — force opening as a bare repository (ignore any working directory) and defer loading its config.
- `NoDotGit` — don't try appending `/.git` to `path`.
- `FromEnv` — resolve the repository from the same environment variables git honors (ignores the other flags and `ceilingDirs`).

`ceilingDirs` is a list of absolute paths at which the upward search stops (ignored when `FromEnv` is set).

A `FromEnv` handle re-consults the environment when an `*Async` method reopens it on a worker thread: the git directory, working directory and namespace are re-pinned to this handle's resolved values, but environment-derived index/object inputs (notably `GIT_INDEX_FILE`, `GIT_OBJECT_DIRECTORY` and `GIT_ALTERNATE_OBJECT_DIRECTORIES`) are re-read from the *current* process environment at reopen time. Mutating those variables between a synchronous call and a later `*Async` call on the same handle can make the two observe different index/object state — for stable results, don't change them mid-flight, or open without `FromEnv`.

#### Repository.discover

```ts
static discover(path: string): Repository
```

Attempt to open an already-existing repository at or above `path`. This starts at `path` and looks up the filesystem hierarchy until it finds a repository.

#### Repository.init

```ts
static init(p: string): Repository
```

Initialize a new Git repository at path `p`, returning a handle to it.

#### Repository.initBare

```ts
static initBare(path: string): Repository
```

Creates a new `--bare` repository in the specified folder. The folder must exist prior to invoking this function.

#### Repository.clone

```ts
static clone(url: string, path: string): Repository
```

Clone a remote repository from `url` into `path`. Delegates to a fresh `RepoBuilder` internally.

#### Repository.cloneAsync

```ts
static cloneAsync(url: string, path: string, signal?: AbortSignal | undefined | null): Promise<Repository>
```

Asynchronous variant of `clone`, performed off the main thread. The network/clone work runs on a worker thread and the resulting `Repository` is constructed on the main thread once the clone completes; the returned handle only exists after the promise resolves and, because the underlying git2 handle is not `Sync`, must be used only from the main thread.

#### Repository.cloneRecurse

```ts
static cloneRecurse(url: string, path: string): Repository
```

Clone a remote repository, then initialize and update its submodules recursively — similar to `git clone --recursive`.

### File "last updated" dates

The headline family: read from Git history *when* (and *by whom*) a file was last touched. Two axes vary — the return shape (epoch milliseconds `number`, a `Date`, or a full `FileModification`) and the missing-path behavior (throw vs. `null`). Each has an off-thread `*Async` twin.

#### getFileLatestModifiedDate

```ts
getFileLatestModifiedDate(filepath: string): number
```

Last-modified commit time of `filepath` in **milliseconds since the Unix epoch**. Throws when no commit in history touched the path. For a `null`-on-missing `Date` instead, use `getFileLastModifiedDate`.

#### getFileLatestModifiedDateAsync

```ts
getFileLatestModifiedDateAsync(filepath: string, signal?: AbortSignal | undefined | null): Promise<number>
```

Off-the-main-thread variant of `getFileLatestModifiedDate`. Rejects when no commit in history touched `filepath`.

#### getFileLastModifiedDate

```ts
getFileLastModifiedDate(filepath: string): Date | null
```

Last-modified commit time of `filepath` as a `Date`, or `null` when no commit in history touched the path (never throws for the missing case). Equals `FileModification.committerTime` from `getFileLatestModified`. Only real errors throw (unborn/empty HEAD, corrupt object, out-of-range timestamp). For milliseconds-since-epoch, use `getFileLatestModifiedDate`.

#### getFileLastModifiedDateAsync

```ts
getFileLastModifiedDateAsync(filepath: string, signal?: AbortSignal | undefined | null): Promise<Date | null>
```

Off-the-main-thread variant of `getFileLastModifiedDate`. Resolves to `null` when no commit in history touched `filepath`.

#### getFileLatestModified

```ts
getFileLatestModified(filepath: string): FileModification | null
```

The last commit that modified `filepath` — author/committer identity, summary and OID — or `null` when no commit in history touched the path. Walks history from HEAD newest-first (`Sort::TIME | Sort::TOPOLOGICAL`), diffing each non-merge commit against its parent under a libgit2 pathspec (so `filepath` may be a directory or glob that matches a file); merge commits are skipped. Its `committerTime` equals `getFileLastModifiedDate`. Only real errors throw (unborn/empty HEAD, corrupt object, out-of-range timestamp).

#### getFileLatestModifiedAsync

```ts
getFileLatestModifiedAsync(filepath: string, signal?: AbortSignal | undefined | null): Promise<FileModification | null>
```

Off-the-main-thread variant of `getFileLatestModified`. Resolves to `null` when no commit in history touched `filepath`.

#### getFilesLatestModified

```ts
getFilesLatestModified(filepaths: Array<string>): Record<string, FileModification | null>
```

Resolve the last commit that modified **each** of `filepaths` in a single history walk (early-exits once every path is resolved) — the pattern behind a doc-site's per-page "last updated" line. Unlike the single-file methods, each input is matched by **exact** repo-root-relative file-path string, **not** libgit2 pathspec/glob semantics: inputs must be file paths (a directory or glob will not match). Every input path is present as a key in the result; a never-committed path maps to `null`. Merge commits are skipped; only real errors throw.

#### getFilesLatestModifiedAsync

```ts
getFilesLatestModifiedAsync(filepaths: Array<string>, signal?: AbortSignal | undefined | null): Promise<Record<string, FileModification | null>>
```

Off-the-main-thread variant of `getFilesLatestModified`. Every input path is a key; never-committed paths map to `null`.

#### getFileCreatedDate

```ts
getFileCreatedDate(filepath: string): number
```

Committer time in **milliseconds since the Unix epoch** of the earliest commit whose tree contains `filepath`. Throws when no commit in history contains the path. Walks all history from HEAD newest-first (`Sort::TIME | Sort::TOPOLOGICAL`) and keeps the last-visited commit whose tree contains `filepath` (matched by exact tree path, not pathspec) — i.e. the oldest containing commit; merge commits are included in this walk. Rename detection is **not** performed (no `git log --follow`), so history is not traced across renames. Also throws on real errors (unborn/empty HEAD, corrupt object, out-of-range timestamp).

#### getFileCreatedDateAsync

```ts
getFileCreatedDateAsync(filepath: string, signal?: AbortSignal | undefined | null): Promise<number>
```

Off-the-main-thread variant of `getFileCreatedDate`. Rejects when no commit in history contains `filepath`.

### Status

#### statuses

```ts
statuses(options?: StatusOptions | undefined | null): Array<FileStatus>
```

List the working-tree and index status of files in the repository — mirrors `git status`. By default untracked files are included and ignored files are not; pass `options` to tune the scan. Each returned `FileStatus` decodes the `git2::Status` flags into booleans plus the raw `bits`.

#### statusFile

```ts
statusFile(path: string): FileStatus
```

Get the status of a single file by its workdir-relative `path`. More efficient than scanning the whole tree when only one path is of interest. Errors (e.g. an ambiguous path) surface as a napi error.

#### statusesAsync

```ts
statusesAsync(options?: StatusOptions | undefined | null, signal?: AbortSignal | undefined | null): Promise<Array<FileStatus>>
```

Off-the-main-thread variant of `statuses`.

### Index & commit

#### index()

```ts
index(): Index
```

Get the index (staging area) for this repository. If a custom index has not been set, the default index for the repository is returned (the one at `.git/index`).

#### commit

```ts
commit(updateRef: string | undefined | null, author: Signature, committer: Signature, message: string, tree: Tree, parents?: Array<string> | undefined | null): string
```

Create a new commit in the repository, returning its OID hex string. When `updateRef` is not `null`, it names the reference to update to point at this commit; if the reference is not direct, it is resolved to a direct one. Use `"HEAD"` to move the current branch's HEAD to this commit — the ref is created if it doesn't exist, and if it does exist the first parent must be its current tip. `parents` is an optional list of parent commit OID hex strings: when `null` or empty a parent-less root commit is created; otherwise each OID is resolved to a commit and used as a parent (the first parent must be the current tip of `updateRef`).

#### commitAsync

```ts
commitAsync(updateRef: string | undefined | null, author: Signature, committer: Signature, message: string, tree: Tree, parents?: Array<string> | undefined | null, signal?: AbortSignal | undefined | null): Promise<string>
```

Off-the-main-thread variant of `commit`. Resolves with the new commit's OID hex string. Arguments mirror `commit`: the `author`/`committer` signatures are copied and the `tree` is captured by OID, so the work can move to a worker thread safely. Do not use the same `Repository` from the main thread while this operation is pending — the underlying git2 handle is not `Sync`.

#### blob

```ts
blob(data: Uint8Array): string
```

Write an in-memory buffer to the object database as a blob and return its OID hex string.

#### blobPath

```ts
blobPath(path: string): string
```

Read a file from the filesystem and write its content to the object database as a blob, returning its OID hex string.

#### findTree

```ts
findTree(oid: string): Tree | null
```

Look up the tree object with id `oid`, returning the `Tree`, or `null` if it is not found (per the `Tree | null` return).

#### findCommit

```ts
findCommit(oid: string): Commit | null
```

Look up the commit object with id `oid`, returning the `Commit`, or `null` if it is not found (per the `Commit | null` return).

### Blame

#### blameFile

```ts
blameFile(path: string, options?: BlameOptions | undefined | null): Array<BlameHunk>
```

Compute the blame for `path`: who last changed each line, as an ordered list of hunks (contiguous runs of lines sharing one final commit). `path` is workdir-relative. Pass `options` to restrict the line/commit range or enable copy tracking. Each `BlameHunk` is eagerly materialized so it outlives the underlying libgit2 blame.

#### blameLine

```ts
blameLine(path: string, lineNo: number, options?: BlameOptions | undefined | null): BlameHunk | null
```

Blame `path` and return only the hunk covering `lineNo` (1-based), or `null` when the line is out of range.

#### blameFileAsync

```ts
blameFileAsync(path: string, options?: BlameOptions | undefined | null, signal?: AbortSignal | undefined | null): Promise<Array<BlameHunk>>
```

Off-the-main-thread variant of `blameFile`.

## Git objects & handles

The handle types a `Repository` hands back: commits, trees, references, remotes, the index, blame results and their kin.

## Options & result types

The plain-object option bags passed into methods and the result shapes they return.

## Enums

The bitflag and discriminant enums used across statuses, resets, revision-walk sorts and error codes.

## Functions

The standalone functions exported alongside `Repository`, including the `isGitError` type guard.

## Error handling

How Git-layer failures surface as typed `Error`s carrying a `GitErrorCode`, and how to narrow them safely inside a `catch`.
