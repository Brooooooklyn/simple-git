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

### Repository state & inspection

Read-only accessors for HEAD, on-disk layout and repository state, plus the workdir, namespace and merge-message mutators.

#### head

```ts
head(): Reference
```

Retrieve and resolve the reference pointed at by HEAD, returning it as a `Reference`.

#### path

```ts
path(): string
```

Returns the path to the `.git` folder for normal repositories, or the repository itself for bare repositories.

#### state

```ts
state(): RepositoryState
```

Returns the current state of this repository as a `RepositoryState` discriminant (e.g. clean, merge, rebase).

#### isShallow

```ts
isShallow(): boolean
```

Tests whether this repository is a shallow clone.

#### isEmpty

```ts
isEmpty(): boolean
```

Tests whether this repository is empty.

#### isWorktree

```ts
isWorktree(): boolean
```

Tests whether this repository is a worktree.

#### workdir

```ts
workdir(): string | null
```

Get the path of the working directory for this repository. Returns `null` when the repository is bare (per the `string | null` return). This is one of the `Option`-returning accessors that returns `null` rather than throwing after `dispose()`.

#### setWorkdir

```ts
setWorkdir(path: string, updateGitlink: boolean): void
```

Set the path to the working directory for this repository. When `updateGitlink` is `true`, create/update the gitlink file in the workdir and set the `core.worktree` config (when the workdir is not the parent of the `.git` directory).

#### namespace

```ts
namespace(): string | null
```

Get the currently active namespace for this repository. Returns `null` when there is no namespace, or when the namespace is not valid UTF-8 (per the `string | null` return). This is one of the `Option`-returning accessors that returns `null` rather than throwing after `dispose()`.

#### setNamespace

```ts
setNamespace(namespace: string): void
```

Set the active namespace for this repository.

#### removeNamespace

```ts
removeNamespace(): void
```

Remove the active namespace for this repository.

#### mergeMessage

```ts
mergeMessage(): string
```

Retrieves the Git merge message (the contents of `.git/MERGE_MSG`). Remember to remove the message when finished, via `removeMergeMessage`.

#### removeMergeMessage

```ts
removeMergeMessage(): void
```

Remove the Git merge message (`.git/MERGE_MSG`).

### Config & signature

#### config

```ts
config(): Config
```

Get the configuration file for this repository as a `Config` view. If a configuration file has not been set, the default config for the repository is returned, including its global and system configurations.

#### signature

```ts
signature(): Signature
```

Create a new action `Signature` with the default user and a now timestamp. This looks up `user.name` and `user.email` from the configuration and uses the current time as the timestamp; it throws when either `user.name` or `user.email` is not set.

### Remotes

Repository-level accessors that list, create, rename, delete and reconfigure remotes. The fetch and push operations themselves live on the `Remote` handle these methods return (see the `Remote` class).

#### remotes

```ts
remotes(): Array<string>
```

List all remote names configured for this repository.

#### findRemote

```ts
findRemote(name: string): Remote | null
```

Get the information for a particular remote as a `Remote`, or `null` when no remote with that name exists (per the `Remote | null` return). This is one of the `Option`-returning accessors that returns `null` rather than throwing after `dispose()`.

#### remote

```ts
remote(name: string, url: string): Remote
```

Add a remote with the default fetch refspec to the repository's configuration, returning the new `Remote`.

#### remoteWithFetch

```ts
remoteWithFetch(name: string, url: string, refspec: string): Remote
```

Add a remote with the provided fetch `refspec` to the repository's configuration, returning the new `Remote`.

#### remoteAnonymous

```ts
remoteAnonymous(url: string): Remote
```

Create an anonymous remote with the given URL and refspec in memory. Use this when you have a URL instead of a remote's name. Note that anonymous remotes cannot be converted to persisted remotes.

#### remoteRename

```ts
remoteRename(name: string, newName: string): Array<string>
```

Give a remote a new name. All remote-tracking branches and configuration settings for the remote are updated. A temporary in-memory remote cannot be given a name with this method, and no already-loaded instances of the remote change their name or refspecs. The returned array is the list of non-default refspecs which could not be renamed and are handed back for further processing by the caller.

#### remoteDelete

```ts
remoteDelete(name: string): this
```

Delete an existing persisted remote. All remote-tracking branches and configuration settings for the remote are removed. Returns the same `Repository` for chaining.

#### remoteAddFetch

```ts
remoteAddFetch(name: string, refspec: string): this
```

Add the given fetch `refspec` to the fetch list in the configuration for the named remote, without loading it. No already-loaded remote instances are affected. Returns the same `Repository` for chaining.

#### remoteAddPush

```ts
remoteAddPush(name: string, refspec: string): this
```

Add the given push `refspec` to the push list in the configuration for the named remote. No already-loaded remote instances are affected. Returns the same `Repository` for chaining.

#### remoteSetUrl

```ts
remoteSetUrl(name: string, url: string): this
```

Set the URL of a remote in the repository's configuration, updating the configured fetch URL for the named remote. No already-loaded remote instances are affected. Returns the same `Repository` for chaining.

#### remoteSetPushUrl

```ts
remoteSetPushUrl(name: string, url?: string | undefined | null): this
```

Set the remote's push URL in the configuration. Remote objects already in memory are not affected. This assumes the common case of a single-URL remote and otherwise returns an error. Passing `null` (or omitting `url`) clears the push URL. Returns the same `Repository` for chaining.

### Branches, checkout & references

#### branches

```ts
branches(filter?: BranchType | undefined | null): Array<Branch>
```

List the branches in the repository. Pass `filter` to restrict the listing to local or remote branches; omit it to list both. Branches whose names are not valid UTF-8 are skipped (they cannot be re-resolved by name).

#### findBranch

```ts
findBranch(name: string, branchType: BranchType): Branch | null
```

Look up a branch by its name and type, returning the `Branch`, or `null` when no branch with that name and type exists (per the `Branch | null` return).

#### branch

```ts
branch(branchName: string, target: Commit, force: boolean): Branch
```

Create a new branch pointing at a target commit. A new direct reference is created pointing to `target`. If `force` is `true` and a branch already exists with the given name, it is replaced.

#### checkoutTree

```ts
checkoutTree(treeish: GitObject, options?: CheckoutOptions | undefined | null): void
```

Check out the tree pointed to by `treeish` (a commit, tag or tree object), updating the working directory to match. This does **not** update HEAD; pair it with `setHead` to switch branches. The checkout is **safe** by default — pass `options.force = true` to overwrite local modifications.

#### checkoutHead

```ts
checkoutHead(options?: CheckoutOptions | undefined | null): void
```

Update files in the index and the working tree to match the content of the tree pointed at by HEAD. The checkout is **safe** by default — pass `options.force = true` to overwrite local modifications.

#### checkoutIndex

```ts
checkoutIndex(options?: CheckoutOptions | undefined | null): void
```

Update files in the working tree to match the content of the repository's index. The checkout is **safe** by default — pass `options.force = true` to overwrite local modifications.

#### setHead

```ts
setHead(refname: string): void
```

Make HEAD point to the reference named `refname`. If `refname` names an existing branch, HEAD becomes a symbolic reference to that branch; otherwise it points to a not-yet-existing branch. This does not touch the working directory — check out separately.

#### setHeadDetached

```ts
setHeadDetached(oid: string): void
```

Make HEAD point directly at the commit with the given OID, detaching it from any branch.

#### reference

```ts
reference(name: string, oid: string, force: boolean, logMessage: string): Reference
```

Create a new direct reference named `name` pointing at the object `oid`. If `force` is `true` and a reference already exists with the given name, it is overwritten; otherwise the call fails. `logMessage` is recorded in the reflog.

#### referenceSymbolic

```ts
referenceSymbolic(name: string, target: string, force: boolean, logMessage: string): Reference
```

Create a new symbolic reference named `name` pointing at the reference named `target` (e.g. `refs/heads/main`). If `force` is `true` and a reference already exists with the given name, it is overwritten; otherwise the call fails. `logMessage` is recorded in the reflog.

### Tags

#### tag

```ts
tag(name: string, target: GitObject, tagger: Signature, message: string, force: boolean): string
```

Create a new annotated tag in the repository from an object, returning the new tag object's OID hex string. A new reference is also created pointing to this tag object; if `force` is `true` and a reference already exists with the given name, it is replaced. The `message` is not cleaned up. The tag `name` is checked for validity — avoid the characters `~ ^ : \ ? [ *` and the sequences `..` and `@{`, which have special meaning to revparse.

#### tagAnnotation

```ts
tagAnnotation(name: string, target: GitObject, tagger: Signature, message: string): string
```

Create a new annotated tag object from an object **without** creating a reference, returning its OID hex string. The `message` is not cleaned up, and the tag `name` is validated with the same rules as `tag`.

#### tagLightweight

```ts
tagLightweight(name: string, target: GitObject, force: boolean): string
```

Create a new lightweight tag pointing at a target object, returning its OID hex string. A new direct reference is created pointing to `target`; if `force` is `true` and a reference already exists with the given name, it is replaced.

#### findTag

```ts
findTag(oid: string): Tag | null
```

Look up a tag object from the repository by OID, returning the `Tag`, or `null` when no tag object with that OID exists (per the `Tag | null` return). This is one of the `Option`-returning accessors that returns `null` rather than throwing after `dispose()`.

#### findTagByPrefix

```ts
findTagByPrefix(prefixHash: string): Tag | null
```

Look up a tag object by hash prefix, returning the `Tag`, or `null` when no tag object matches the prefix (per the `Tag | null` return). This is one of the `Option`-returning accessors that returns `null` rather than throwing after `dispose()`.

#### tagDelete

```ts
tagDelete(name: string): void
```

Delete an existing tag reference. The tag `name` is checked for validity (see `tag` for the naming rules).

#### tagNames

```ts
tagNames(pattern?: string | undefined | null): Array<string>
```

Get a list of all the tag names in the repository. An optional fnmatch `pattern` can be specified to filter the results.

#### tagForeach

```ts
tagForeach(cb: (arg: TagForeachItem) => boolean): void
```

Iterate over all tags, calling `cb` on each. The callback receives a single `TagForeachItem` carrying the tag's OID (`id`, a 40-char hex string) and its raw reference name (`nameBytes`, a `Buffer`). Return `true` to continue iteration, `false` to stop.

### Diffs

#### diffTreeToWorkdir

```ts
diffTreeToWorkdir(oldTree?: Tree | undefined | null, options?: DiffOptions | undefined | null): Diff
```

Create a `Diff` between a tree and the working directory: `oldTree` is used for the "old_file" side of each delta and the working directory for the "new_file" side. This is **not** the same as `git diff <treeish>` or `git diff-index <treeish>` — those use information from the index, whereas this strictly returns the differences between the tree and the working-directory files regardless of the state of the index. Use `diffTreeToWorkdirWithIndex` to emulate those commands. When `null` is passed for `oldTree`, an empty tree is used.

#### diffTreeToWorkdirWithIndex

```ts
diffTreeToWorkdirWithIndex(oldTree?: Tree | undefined | null, options?: DiffOptions | undefined | null): Diff
```

Create a `Diff` between a tree and the working directory using index data to account for staged deletes, tracked files, etc. This emulates `git diff <tree>` by diffing the tree to the index and the index to the working directory, then blending the results into a single diff that includes staged deletions and the like.

### Revision walking

#### revWalk

```ts
revWalk(): RevWalk
```

Create a `RevWalk` that can be used to traverse the commit graph.

### Resource cleanup

`Symbol.dispose` cannot be generated by napi, so `using` support is opt-in via a single line at startup:

```js
Repository.prototype[Symbol.dispose] ??= Repository.prototype.dispose
```

#### dispose

```ts
dispose(): void
```

Eagerly release the underlying git2 repository handle (`git_repository_free`), closing any open packfile file descriptors and memory-mapped indexes without waiting for JavaScript garbage collection. It is idempotent: calling it more than once (or calling `free()` afterwards) is a no-op.

After disposal, every throwing method throws `"Repository has been disposed"`, while the `Option`-returning methods (`workdir()`, `namespace()`, `findRemote()`, `findTree()`, `findCommit()`, `findTag()`, `findTagByPrefix()`) return `null` instead. Any handle previously derived from this repository — `Remote`, `Reference`, `Tree`, `TreeEntry`, `Commit`, `Tag`, `Branch`, `GitObject`, `Diff`, `RevWalk` and their descendants — throws the same `"Repository has been disposed"` error on use, whether it is the receiver or an argument passed to another method. This is machine-enforced (mirroring better-sqlite3's `db.close()`), not merely a documented contract.

Disposal does **not** cancel `*Async` operations already in flight: a worker scheduled before `dispose()` reopens the repository from its path on its own thread and runs to completion (its promise still resolves and refs or objects may change on disk), because it never touches this freed handle. New `*Async` calls made after disposal throw synchronously. To cancel a pending async operation, pass an `AbortSignal` to the `*Async` method rather than relying on `dispose()`.

#### free

```ts
free(): void
```

Alias for `dispose()`. Eagerly releases the underlying git2 repository handle; idempotent. See `dispose()` for the full disposal contract.

## Git objects & handles

The handle types a `Repository` hands back: commits, trees, references, remotes, the index, blame results and their kin.

### Commit

A commit object, obtained from `Repository.findCommit(oid)` or another commit's `parent(i)`.

#### id

```ts
id(): string
```

Get the id (SHA1) of this repository object.

#### treeId

```ts
treeId(): string
```

Get the id of the tree pointed to by this commit. No attempts are made to fetch an object from the ODB.

#### tree

```ts
tree(): Tree
```

Get the tree pointed to by this commit.

#### message

```ts
message(): string | null
```

Get the full message of a commit. The returned message is slightly prettified by removing any potential leading newlines. Returns `null` if the message is not valid UTF-8 (per the `string | null` return).

#### messageBytes

```ts
messageBytes(): Buffer
```

Get the full message of a commit as a byte slice. The returned message is slightly prettified by removing any potential leading newlines.

#### messageEncoding

```ts
messageEncoding(): string | null
```

Get the encoding for the message of a commit, as a string representing a standard encoding name. Returns `null` if the encoding is not known (per the `string | null` return).

#### messageRaw

```ts
messageRaw(): string | null
```

Get the full raw message of a commit. Returns `null` if the message is not valid UTF-8 (per the `string | null` return).

#### messageRawBytes

```ts
messageRawBytes(): Buffer
```

Get the full raw message of a commit.

#### rawHeader

```ts
rawHeader(): string | null
```

Get the full raw text of the commit header. Returns `null` if the message is not valid UTF-8 (per the `string | null` return).

#### headerFieldBytes

```ts
headerFieldBytes(field: string): Buffer
```

Get an arbitrary header `field`.

#### rawHeaderBytes

```ts
rawHeaderBytes(): Buffer
```

Get the full raw text of the commit header.

#### summary

```ts
summary(): string | null
```

Get the short "summary" of the git commit message — the first paragraph of the message with whitespace trimmed and squashed. Returns `null` if an error occurs or if the summary is not valid UTF-8 (per the `string | null` return).

#### summaryBytes

```ts
summaryBytes(): Buffer | null
```

Get the short "summary" of the git commit message — the first paragraph of the message with whitespace trimmed and squashed. Returns `null` if an error occurs (per the `Buffer | null` return).

#### body

```ts
body(): string | null
```

Get the long "body" of the git commit message — everything but the first paragraph of the message, with leading and trailing whitespace trimmed. Returns `null` if an error occurs or if the summary is not valid UTF-8 (per the `string | null` return).

#### bodyBytes

```ts
bodyBytes(): Buffer | null
```

Get the long "body" of the git commit message — everything but the first paragraph of the message, with leading and trailing whitespace trimmed. Returns `null` if an error occurs (per the `Buffer | null` return).

#### time

```ts
time(): Date
```

Get the commit time (i.e. committer time) of a commit. Returns the committer time as a UTC `Date`; the committer's timezone offset is not preserved (the value is normalized to UTC).

#### author

```ts
author(): Signature
```

Get the author of this commit.

#### committer

```ts
committer(): Signature
```

Get the committer of this commit.

#### amend

```ts
amend(updateRef?: string | undefined | null, author?: Signature | undefined | null, committer?: Signature | undefined | null, messageEncoding?: string | undefined | null, message?: string | undefined | null, tree?: Tree | undefined | null): string
```

Amend this existing commit with all non-`null` values, returning the new commit's OID hex string. This creates a new commit that is exactly the same as the old commit, except that any non-`null` values are updated. The new commit has the same parents as the old commit. For information about `updateRef`, see `Repository.commit`.

#### parentCount

```ts
parentCount(): number
```

Get the number of parents of this commit. Use `parent`/`parentId` to read a specific parent.

#### parent

```ts
parent(i: number): Commit
```

Get the parent of the commit at index `i`. This attempts to load the parent commit from the ODB.

#### parentId

```ts
parentId(i: number): string
```

Get the id of the parent of the commit at index `i`. This is different from `parent`, which attempts to load the parent commit from the ODB.

#### asObject

```ts
asObject(): GitObject
```

Casts this `Commit` to be usable as a `GitObject`.

### Tree

A tree object, obtained from `Repository.findTree(oid)`, `Commit.tree()` or `Reference.peelToTree()`; iterate its entries with `entries()`.

#### id

```ts
id(): string
```

Get the id (SHA1) of this repository object.

#### size

```ts
size(): number
```

Get the number of entries listed in a tree.

#### isEmpty

```ts
isEmpty(): boolean
```

Return `true` if there is no entry.

#### entries

```ts
entries(): TreeIter
```

Returns a `TreeIter` iterator over the entries in this tree.

#### getId

```ts
getId(id: string): TreeEntry | null
```

Look up a tree entry by SHA value, returning the `TreeEntry`, or `null` when no entry matches (per the `TreeEntry | null` return).

#### get

```ts
get(index: number): TreeEntry | null
```

Look up a tree entry by its position in the tree, returning the `TreeEntry`, or `null` when the index is out of range (per the `TreeEntry | null` return).

#### getName

```ts
getName(name: string): TreeEntry | null
```

Look up a direct child entry of this tree by its `name`, returning the `TreeEntry`, or `null` when no such child exists (per the `TreeEntry | null` return). `name` is a single path component (a filename), not a multi-component path; this does not descend into subtrees. To follow a relative path through nested subtrees, use `getPath`.

#### getPath

```ts
getPath(name: string): TreeEntry | null
```

Look up a tree entry by a relative path, descending through subtrees, returning the `TreeEntry`, or `null` when the path does not resolve (per the `TreeEntry | null` return). `name` is a path relative to this tree and may contain multiple components (e.g. `src/lib.rs`); each component is resolved in turn, walking into nested subtrees. To look up a direct child by its name, use `getName`.

### TreeEntry

An entry in a tree, obtained from `Tree.get`/`getId`/`getName`/`getPath` or by iterating `Tree.entries()`.

#### id

```ts
id(): string
```

Get the id of the object pointed to by the entry.

#### name

```ts
name(): string
```

Get the name of a tree entry.

#### nameBytes

```ts
nameBytes(): Buffer
```

Get the filename of a tree entry.

#### toObject

```ts
toObject(repo: Repository): GitObject
```

Convert a tree entry to the `GitObject` it points to, looked up in `repo`.

### TreeIter

Iterator over a tree's entries, returned by `Tree.entries()`.

```ts
export declare class TreeIter extends Iterator<TreeEntry, void, void> {

  next(value?: void): IteratorResult<TreeEntry, void>
}
```

This type extends JavaScript's `Iterator`, and so has the iterator helper methods. It may extend the upcoming TypeScript `Iterator` class in the future. (See the [MDN iterator helper methods](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Iterator#iterator_helper_methods) and the [TypeScript 5.6 notes](https://www.typescriptlang.org/docs/handbook/release-notes/typescript-5-6.html#iterator-helper-methods).)

#### next

```ts
next(value?: void): IteratorResult<TreeEntry, void>
```

Advance the iterator, returning the next `TreeEntry` as an `IteratorResult`.

### Blob

A blob object, obtained by peeling a `GitObject` to a blob with `GitObject.peelToBlob()`.

#### id

```ts
id(): string
```

Get the id (SHA1) of a repository blob.

#### isBinary

```ts
isBinary(): boolean
```

Determine if the blob content is most certainly binary or not.

#### content

```ts
content(): Buffer
```

Get the content of this blob.

#### size

```ts
size(): number
```

Get the size in bytes of the contents of this blob.

### GitObject

A generic git object of any kind, obtained from `Commit.asObject()`, `Tag.peel()`, `TreeEntry.toObject(repo)` or `GitObject.peel(kind)`; inspect its type with `kind()`.

#### id

```ts
id(): string
```

Get the id (SHA1) of this repository object.

#### kind

```ts
kind(): ObjectType | null
```

Get the type of the object as an `ObjectType`, or `null` when the type is unknown (per the `ObjectType | null` return).

#### peel

```ts
peel(kind: ObjectType): GitObject
```

Recursively peel an object until an object of the specified `kind` is met. If you pass `Any` as the target type, then the object is peeled until the type changes (e.g. a tag is chased until the referenced object is no longer a tag).

#### peelToBlob

```ts
peelToBlob(): Blob
```

Recursively peel an object until a blob is found.

### Tag

A tag object, obtained from `Repository.findTag(oid)` or `Repository.findTagByPrefix(prefixHash)`.

#### Tag.isValidName

```ts
static isValidName(name: string): boolean
```

Determine whether a tag `name` is valid, meaning that (when prefixed with `refs/tags/`) it is a valid reference name, and that any additional tag name restrictions are imposed (e.g. it cannot start with a `-`).

#### id

```ts
id(): string
```

Get the id (SHA1) of this repository object.

#### message

```ts
message(): string | null
```

Get the message of a tag. Returns `null` if there is no message or if it is not valid UTF-8 (per the `string | null` return).

#### messageBytes

```ts
messageBytes(): Buffer | null
```

Get the message of a tag. Returns `null` if there is no message (per the `Buffer | null` return).

#### name

```ts
name(): string | null
```

Get the name of a tag. Returns `null` if it is not valid UTF-8 (per the `string | null` return).

#### nameBytes

```ts
nameBytes(): Buffer
```

Get the name of a tag.

#### peel

```ts
peel(): GitObject
```

Recursively peel a tag until a non-tag `GitObject` is found.

### Reference

A git reference (branch, tag, note or symbolic ref), obtained from `Repository.head()`, `Repository.reference`/`referenceSymbolic`, `Branch.get()` or `Reference.resolve()`.

#### Reference.isValidName

```ts
static isValidName(name: string): boolean
```

Ensure the reference `name` is well-formed. Validation is performed as if `ReferenceFormat::ALLOW_ONELEVEL` was given to `Reference.normalize_name`. No normalization is performed, however.

```ts
import { Reference } from '@napi-rs/simple-git'

console.assert(Reference.isValidName("HEAD"));
console.assert(Reference.isValidName("refs/heads/main"));

// But:
console.assert(!Reference.isValidName("main"));
console.assert(!Reference.isValidName("refs/heads/*"));
console.assert(!Reference.isValidName("foo//bar"));
```

#### isBranch

```ts
isBranch(): boolean
```

Check if a reference is a local branch.

#### isNote

```ts
isNote(): boolean
```

Check if a reference is a note.

#### isRemote

```ts
isRemote(): boolean
```

Check if a reference is a remote tracking branch.

#### isTag

```ts
isTag(): boolean
```

Check if a reference is a tag.

#### kind

```ts
kind(): ReferenceType
```

Get the type of the reference as a `ReferenceType`.

#### name

```ts
name(): string | null
```

Get the full name of a reference. Returns `null` if the name is not valid UTF-8 (per the `string | null` return).

#### shorthand

```ts
shorthand(): string | null
```

Get the full shorthand of a reference. This transforms the reference name into a "human-readable" version; if no shortname is appropriate, it returns the full name. Returns `null` if the shorthand is not valid UTF-8 (per the `string | null` return).

#### target

```ts
target(): string | null
```

Get the OID pointed to by a direct reference. Only available if the reference is direct (i.e. an object id reference, not a symbolic one); returns `null` otherwise (per the `string | null` return).

#### targetPeel

```ts
targetPeel(): string | null
```

Return the peeled OID target of this reference. This peeled OID only applies to direct references that point to a hard Tag object: it is the result of peeling such a Tag; otherwise `null` (per the `string | null` return).

#### peelToTree

```ts
peelToTree(): Tree
```

Peel a reference to a tree. This method recursively peels the reference until it reaches a `Tree`.

#### symbolicTarget

```ts
symbolicTarget(): string | null
```

Get the full name of the reference pointed to by a symbolic reference. Returns `null` if the reference is either not symbolic or not a valid UTF-8 string (per the `string | null` return).

#### resolve

```ts
resolve(): Reference
```

Resolve a symbolic reference to a direct reference. This method iteratively peels a symbolic reference until it resolves to a direct reference to an OID. If a direct reference is passed as an argument, a copy of that reference is returned.

#### rename

```ts
rename(newName: string, force: boolean, msg: string): Reference
```

Rename an existing reference to `newName`. This works for both direct and symbolic references. If `force` is not enabled and there is already a reference with the given name, the renaming fails.

### Signature

An author/committer signature — a name, email and timestamp. Construct one with `new Signature(...)` or `Signature.now(...)`, or read one from `Repository.signature()`, `Commit.author()` or `Commit.committer()`.

#### Signature.now

```ts
static now(name: string, email: string): Signature
```

Create a new action signature with a timestamp of 'now'. See the constructor for more information.

#### new Signature(name, email, time)

```ts
constructor(name: string, email: string, time: Date)
```

Create a new action signature. The `time` is a JS `Date`; it is recorded at whole-second resolution with a zero time-zone offset (UTC). Returns an error if either `name` or `email` contain angle brackets.

#### name

```ts
name(): string | null
```

Get the name on the signature. Returns `null` if the name is not valid UTF-8 (per the `string | null` return).

#### email

```ts
email(): string | null
```

Get the email on the signature. Returns `null` if the email is not valid UTF-8 (per the `string | null` return).

#### when

```ts
when(): Date
```

Return the time the signature was recorded, as a `Date`.

### Branch

A git branch — a thin wrapper around an underlying reference; the full reference name is available via `referenceName()`. Obtained from `Repository.branches()`, `Repository.findBranch()`, `Repository.branch()` or `Branch.upstream()`.

#### name

```ts
name(): string | null
```

Return the name of the given local or remote branch. Returns `null` if the name is not valid UTF-8 (per the `string | null` return).

#### isHead

```ts
isHead(): boolean
```

Determine if the current local branch is pointed at by HEAD.

#### referenceName

```ts
referenceName(): string | null
```

Get the full name of the reference backing this branch (e.g. `refs/heads/main`). Returns `null` if the reference name is not valid UTF-8 (per the `string | null` return).

#### delete

```ts
delete(): void
```

Delete an existing branch reference.

#### upstream

```ts
upstream(): Branch | null
```

Return the reference supporting the remote tracking branch, given a local branch reference. Returns `null` when the branch has no configured upstream (per the `Branch | null` return).

#### get

```ts
get(): Reference
```

Return the reference backing this branch as a live `Reference`. Branches are direct references, so the resolved direct reference is returned (e.g. `refs/heads/main`).

## Options & result types

The plain-object option bags passed into methods and the result shapes they return.

## Enums

The bitflag and discriminant enums used across statuses, resets, revision-walk sorts and error codes.

## Functions

The standalone functions exported alongside `Repository`, including the `isGitError` type guard.

## Error handling

How Git-layer failures surface as typed `Error`s carrying a `GitErrorCode`, and how to narrow them safely inside a `catch`.
