# `@napi-rs/simple-git`

![https://github.com/Brooooooklyn/simple-git/actions](https://github.com/Brooooooklyn/simple-git/workflows/CI/badge.svg)
![](https://img.shields.io/npm/dm/@napi-rs/simple-git.svg?sanitize=true)
[![Install size](https://packagephobia.com/badge?p=@napi-rs/simple-git)](https://packagephobia.com/result?p=@napi-rs/simple-git)

## `Repository`

### Usage

```ts
import { Repository, BranchType, RemoteCallbacks, PushOptions } from '@napi-rs/simple-git'

Repository.init('/path/to/repo') // init a git repository

const repo = new Repository('/path/to/repo') // Open an existed repo

const lastModified = repo.getFileLatestModifiedDate('build.rs') // latest modified date of `build.rs`, a `Date`
console.log(lastModified) // 2022-03-13T12:47:47.920Z

const lastModifiedAsync = await repo.getFileLatestModifiedDateAsync('build.rs') // Async version of `getFileLatestModifiedDate`, also a `Date`

console.log(lastModifiedAsync) // 2022-03-13T12:47:47.920Z

// Enriched metadata for the last commit that touched a file.
// Returns `null` (does **not** throw) when the path has no commit history.
const mod = repo.getFileLatestModification('build.rs')
if (mod) {
  console.log(mod.authorName, mod.authorEmail) // 'LongYinan' 'github@lyn.one'
  console.log(mod.committerTime) // a `Date`, identical to getFileLatestModifiedDate('build.rs')
  console.log(mod.commitId, mod.summary)
}

// Bulk: resolve many files in a single history walk (early-exits once all are found).
// Every input path is present as a key; a never-committed path maps to `null`.
const mods = repo.getFilesLatestModification(['build.rs', 'Cargo.toml'])
console.log(mods['build.rs']?.committerName)
console.log(mods['Cargo.toml']?.committerTime) // a `Date`
// Empty input returns `{}`:
console.log(repo.getFilesLatestModification([])) // {}

// ---- Working-tree status (like `git status`) ----
const changes = repo.statuses() // => FileStatus[]
for (const file of changes) {
  console.log(file.path, file.isWtModified, file.isIndexNew)
}
console.log(repo.statusFile('README.md').isWtModified) // status of a single path
const scanned = await repo.statusesAsync({ includeIgnored: true }) // off-thread scan

// ---- Config + default signature ----
const config = repo.config() // => Config (system + global + repo, prioritized)
config.setString('user.name', 'LongYinan')
console.log(config.getString('user.name')) // 'LongYinan'
console.log(config.getBool('core.bare')) // false
const sig = repo.signature() // built from user.name / user.email
console.log(sig.name(), sig.email()) // 'LongYinan' 'github@lyn.one'

// ---- Stage from the working tree and commit ----
const index = repo.index() // => Index (the staging area)
index.addPath('file.txt')
index.write()
const treeOid = index.writeTree() // OID of the staged tree
const tree = repo.findTree(treeOid)!
const parent = repo.head().target()! // current tip OID
const commitId = repo.commit('HEAD', sig, sig, 'commit from workdir', tree, [parent])
console.log(commitId) // 40-char hex OID
console.log(repo.blob(Buffer.from('hello'))) // write a blob straight to the ODB => OID

// ---- Branches ----
const tip = repo.findCommit(parent)!
const branch = repo.branch('feature', tip, false) // => Branch
console.log(branch.name(), branch.isHead()) // 'feature' false
for (const b of repo.branches(BranchType.Local)) console.log(b.name())
repo.findBranch('feature', BranchType.Local)?.delete()

// ---- Checkout / HEAD / refs ----
repo.checkoutTree(tip.asObject(), { force: true }) // update workdir to a tree-ish
repo.setHead('refs/heads/feature') // move HEAD (without touching the workdir)
repo.checkoutHead({ force: true }) // sync the workdir to HEAD
repo.setHeadDetached(tip.id()) // detach HEAD at a commit
repo.reference('refs/heads/tmp', tip.id(), true, 'create tmp') // a direct ref
repo.referenceSymbolic('refs/heads/alias', 'refs/heads/main', true, 'alias main')

// ---- Blame ----
for (const hunk of repo.blameFile('build.rs')) {
  console.log(hunk.finalStartLine, hunk.linesInHunk, hunk.finalCommitId, hunk.finalAuthorName)
}
console.log(repo.blameLine('build.rs', 10)?.finalAuthorName) // hunk for line 10, or null

// ---- Push ----
const remote = repo.findRemote('origin')!
const callbacks = new RemoteCallbacks()
  // Per-ref result: one object per updated reference.
  .pushUpdateReference(({ refname, status }) => {
    console.log(refname, status) // 'refs/heads/main' null   (null === accepted)
  })
  // Pack-transfer progress: a single PushTransferProgress object.
  .pushTransferProgress(({ current, total, bytes }) => {
    console.log(`${current}/${total} objects, ${bytes} bytes`)
  })
remote.push(['refs/heads/main'], new PushOptions().remoteCallback(callbacks))

// ---- Async (off-main-thread) network + commit ----
// Clone, commit, fetch and push also have async variants that run their git
// work on a worker thread and return a Promise. `commitAsync` mirrors `commit`.
const cloned = await Repository.cloneAsync('https://example.com/repo.git', '/tmp/clone')
const asyncOid = await repo.commitAsync('HEAD', sig, sig, 'async commit', tree, [parent])
// fetchAsync / pushAsync accept data-only Fetch/Push options. They do NOT accept
// RemoteCallbacks (JS callbacks can't run off the main thread) — use the sync
// fetch()/push() when you need credential or progress callbacks.
await remote.fetchAsync(['refs/heads/main:refs/remotes/origin/main'])
await remote.pushAsync(['refs/heads/main'], new PushOptions().packbuilderParallelism(1))

// ---- Tags & diff ----
// tagForeach hands the callback a single { id, nameBytes } object per tag.
repo.tagForeach(({ id, nameBytes }) => {
  console.log(id, nameBytes.toString('utf8')) // '<40-hex>' 'refs/tags/v1.0.0'
  return true // return false to stop iterating
})
// DiffOptions.showUnmodified pulls unmodified files into the diff so they show
// up in `deltas()` (with an Unmodified status) instead of being skipped.
const headTree = repo.head().peelToTree()
repo.diffTreeToWorkdir(headTree, { showUnmodified: true })
```

### API

```ts
export class Repository {
  static init(p: string): Repository
  constructor(gitDir: string)
  /**
   * Asynchronous variant of `clone`, performed off the main thread. Resolves
   * with a ready-to-use `Repository` once the clone completes.
   */
  static cloneAsync(url: string, path: string, signal?: AbortSignal | undefined | null): Promise<Repository>
  /** Retrieve and resolve the reference pointed at by HEAD. */
  head(): Reference
  getFileLatestModifiedDate(filepath: string): Date
  getFileLatestModifiedDateAsync(filepath: string, signal?: AbortSignal | undefined | null): Promise<Date>
  /**
   * Last commit that modified `filepath`, with author/committer identity.
   * Returns `null` when no commit in history touched the path.
   */
  getFileLatestModification(filepath: string): FileModification | null
  getFileLatestModificationAsync(filepath: string, signal?: AbortSignal | undefined | null): Promise<FileModification | null>
  /**
   * Resolve the last commit that modified each of `filepaths` in a single
   * history walk. Every input path is a key; never-committed paths map to `null`.
   */
  getFilesLatestModification(filepaths: Array<string>): Record<string, FileModification | undefined | null>
  getFilesLatestModificationAsync(filepaths: Array<string>, signal?: AbortSignal | undefined | null): Promise<Record<string, FileModification | undefined | null>>
  /** Repository config view (system + global + repo, prioritized). */
  config(): Config
  /** Default signature built from `user.name` / `user.email`. */
  signature(): Signature
  /** List the branches in the repository, optionally filtered by type. */
  branches(filter?: BranchType | undefined | null): Array<Branch>
  /** Lookup a branch by name and type; `null` when it does not exist. */
  findBranch(name: string, branchType: BranchType): Branch | null
  /** Create a new branch pointing at a target commit. */
  branch(branchName: string, target: Commit, force: boolean): Branch
  /** Check out a tree-ish into the working directory (does not move HEAD). */
  checkoutTree(treeish: GitObject, options?: CheckoutOptions | undefined | null): void
  /** Update the index and the working tree to match HEAD. */
  checkoutHead(options?: CheckoutOptions | undefined | null): void
  /** Update the working tree to match the index. */
  checkoutIndex(options?: CheckoutOptions | undefined | null): void
  /** Make HEAD point to the reference named `refname`. */
  setHead(refname: string): void
  /** Detach HEAD directly at the commit with the given OID. */
  setHeadDetached(oid: string): void
  /** Create a new direct reference named `name` pointing at `oid`. */
  reference(name: string, oid: string, force: boolean, logMessage: string): Reference
  /** Create a new symbolic reference named `name` pointing at `target`. */
  referenceSymbolic(name: string, target: string, force: boolean, logMessage: string): Reference
  /** Create a new commit; `parents` is an optional list of parent OID hex strings. */
  commit(updateRef: string | undefined | null, author: Signature, committer: Signature, message: string, tree: Tree, parents?: Array<string> | undefined | null): string
  /** Asynchronous variant of `commit`, performed off the main thread. Resolves with the new commit's OID hex. */
  commitAsync(updateRef: string | undefined | null, author: Signature, committer: Signature, message: string, tree: Tree, parents?: Array<string> | undefined | null, signal?: AbortSignal | undefined | null): Promise<string>
  /** Get the index (staging area) for this repository. */
  index(): Index
  /** Write an in-memory buffer to the object database as a blob; returns its OID hex. */
  blob(data: Uint8Array): string
  /** Read a file and write its content to the object database as a blob; returns its OID hex. */
  blobPath(path: string): string
  /** List the working-tree and index status of files (like `git status`). */
  statuses(options?: StatusOptions | undefined | null): Array<FileStatus>
  /** Status of a single file by its workdir-relative path. */
  statusFile(path: string): FileStatus
  /** Asynchronous variant of `statuses`, computed off the main thread. */
  statusesAsync(options?: StatusOptions | undefined | null, signal?: AbortSignal | undefined | null): Promise<Array<FileStatus>>
  /** Blame `path`: who last changed each line, as ordered hunks. */
  blameFile(path: string, options?: BlameOptions | undefined | null): Array<BlameHunk>
  /** Blame `path` and return only the hunk covering `lineNo` (1-based), or `null`. */
  blameLine(path: string, lineNo: number, options?: BlameOptions | undefined | null): BlameHunk | null
  /** Asynchronous variant of `blameFile`, computed off the main thread. */
  blameFileAsync(path: string, options?: BlameOptions | undefined | null, signal?: AbortSignal | undefined | null): Promise<Array<BlameHunk>>
  /** Create an annotated tag (and its ref); returns the new tag object's OID. */
  tag(name: string, target: GitObject, tagger: Signature, message: string, force: boolean): string
  /** Create an annotated tag object WITHOUT a ref; returns its OID. */
  tagAnnotation(name: string, target: GitObject, tagger: Signature, message: string): string
  /** Lookup a tag object by OID; `null` when it does not exist. */
  findTag(oid: string): Tag | null
  /** Lookup a tag object by hash prefix; `null` when no tag matches. */
  findTagByPrefix(prefixHash: string): Tag | null
  /** Read libgit2's merge message (`.git/MERGE_MSG`). */
  mergeMessage(): string
  /** Remove the merge message (`.git/MERGE_MSG`). */
  removeMergeMessage(): void
  /** Add a remote with the provided fetch refspec to the configuration. */
  remoteWithFetch(name: string, url: string, refspec: string): Remote
}

/**
 * Last commit that modified a file, with author/committer identity.
 * All times are `Date`s (UTC; timezone offset ignored).
 */
export interface FileModification {
  /** 40-char lowercase hex OID of the last commit that modified the file. */
  commitId: string
  /** Commit summary (first line). Undefined if absent or not valid UTF-8. */
  summary?: string
  /** Author name. Undefined if not valid UTF-8. */
  authorName?: string
  /** Author email. Undefined if not valid UTF-8. */
  authorEmail?: string
  /** Author time, as a `Date`. */
  authorTime: Date
  /** Committer name. Undefined if not valid UTF-8. */
  committerName?: string
  /** Committer email. Undefined if not valid UTF-8. */
  committerEmail?: string
  /** Committer time, as a `Date`. Identical to `getFileLatestModifiedDate`. */
  committerTime: Date
}

/**
 * A git configuration store. Obtain one with `repo.config()` (system + global +
 * repository, prioritized) or `Config.openDefault()` (system/global/XDG only).
 */
export class Config {
  /** Open global, XDG and system config into one prioritized object. */
  static openDefault(): Config
  /** Get a string config value (highest-priority occurrence wins). */
  getString(name: string): string
  getBool(name: string): boolean
  getNumber(name: string): number
  /** i64 value as a `bigint` (no >2^53 truncation). */
  getBigInt(name: string): bigint
  /** Set a value in the highest-level config file (usually the local one). */
  setString(name: string, value: string): void
  setBool(name: string, value: boolean): void
  setNumber(name: string, value: number): void
  /** Set an i64 value from a `bigint`; throws if it doesn't fit in i64. */
  setBigInt(name: string, value: bigint): void
  /** Delete a variable from the highest-level config file. */
  removeEntry(name: string): void
  /** Create a read-only point-in-time snapshot of this configuration. */
  snapshot(): Config
  /** List entries, optionally filtered by a glob pattern. */
  entries(glob?: string | undefined | null): Array<ConfigEntry>
}

/**
 * A single configuration entry: its fully-qualified name, value, and the
 * level (file) it was read from.
 */
export interface ConfigEntry {
  name: string
  value: string
  level: ConfigLevel
}

/**
 * The priority level a configuration entry or file applies to. Higher levels
 * take precedence; `Local` (the repository's own `.git/config`) is where
 * `set_*`/`remove_entry` write by default.
 */
export declare const enum ConfigLevel {
  /** System-wide on Windows, for compatibility with portable git */
  ProgramData = 0,
  /** System-wide configuration file, e.g. /etc/gitconfig */
  System = 1,
  /** XDG-compatible configuration file, e.g. ~/.config/git/config */
  Xdg = 2,
  /** User-specific configuration, e.g. ~/.gitconfig */
  Global = 3,
  /** Repository specific config, e.g. $PWD/.git/config */
  Local = 4,
  /** Worktree specific configuration file, e.g. $GIT_DIR/config.worktree */
  Worktree = 5,
  /** Application specific configuration file */
  App = 6,
  /** Highest level available */
  Highest = 7
}

/**
 * A git index (the staging area). Obtain one with `repo.index()`. Mutating
 * methods change memory only; call `write()` to persist it or `writeTree()` to
 * write its current state to the object database as a tree.
 */
export class Index {
  /** Add or update an index entry from a file on disk. */
  addPath(path: string): void
  /** Add or update entries matching files in the working directory. */
  addAll(pathspecs?: Array<string> | undefined | null, force?: boolean | undefined | null): void
  /** Update all index entries to match the working directory. */
  updateAll(pathspecs?: Array<string> | undefined | null): void
  /** Remove an index entry corresponding to a file on disk. */
  removePath(path: string): void
  /** Get the count of entries currently in the index. */
  count(): number
  /** Write the in-memory index back to disk using an atomic file lock. */
  write(): void
  /** Write the index as a tree to the object database and return its OID. */
  writeTree(): string
}

/**
 * A git branch — a thin wrapper around an underlying reference; the full
 * reference name is available via `referenceName`.
 */
export class Branch {
  /** Name of the local or remote branch (`null` if not valid utf-8). */
  name(): string | null
  /** Whether the current local branch is pointed at by HEAD. */
  isHead(): boolean
  /** Full reference name backing this branch (e.g. `refs/heads/main`). */
  referenceName(): string | null
  /** Delete this branch reference. */
  delete(): void
  /** The configured upstream tracking branch, or `null`. */
  upstream(): Branch | null
  /** The reference backing this branch as a live `Reference`. */
  get(): Reference
}

/** An enumeration for the possible types of branches. */
export declare const enum BranchType {
  /** A local branch not on a remote. */
  Local = 0,
  /** A branch for a remote. */
  Remote = 1
}

export class PushOptions {
  constructor()
  /** Set the callbacks to use for the push operation. */
  remoteCallback(callback: RemoteCallbacks): this
  /** Set the proxy options to use for the push operation. */
  proxyOptions(options: ProxyOptions): this
  /** Number of packbuilder worker threads (0 = auto-detect, default 1). */
  packbuilderParallelism(parallel: number): this
  /** Set remote redirection settings. */
  followRedirects(opt: RemoteRedirect): this
  /** Set extra headers for this push operation. */
  customHeaders(headers: Array<string>): this
  /** Set "push options" to deliver to the remote. */
  remotePushOptions(options: Array<string>): this
}

/**
 * Status of a single file in the working tree and/or index.
 *
 * The boolean flags mirror the `git2::Status` bits; `bits` carries the raw
 * value as a forward-compatible escape hatch for flags not surfaced here.
 */
export interface FileStatus {
  /** Workdir-relative path. Undefined if the path is not valid UTF-8. */
  path?: string
  /** Raw `git2::Status` bits — forward-compat escape hatch. */
  bits: number
  /** Staged: a new file was added to the index. */
  isIndexNew: boolean
  /** Staged: a tracked file was modified in the index. */
  isIndexModified: boolean
  /** Staged: a tracked file was deleted from the index. */
  isIndexDeleted: boolean
  /** Staged: a tracked file was renamed in the index. */
  isIndexRenamed: boolean
  /** Staged: a tracked file changed type in the index. */
  isIndexTypechange: boolean
  /** Unstaged: an untracked file (new in the working directory). */
  isWtNew: boolean
  /** Unstaged: a tracked file was modified in the working directory. */
  isWtModified: boolean
  /** Unstaged: a tracked file was deleted from the working directory. */
  isWtDeleted: boolean
  /** Unstaged: a tracked file changed type in the working directory. */
  isWtTypechange: boolean
  /** Unstaged: a tracked file was renamed in the working directory. */
  isWtRenamed: boolean
  /** The file is ignored. */
  isIgnored: boolean
  /** The file has merge conflicts. */
  isConflicted: boolean
}

/**
 * Options controlling how a working-tree status scan is performed.
 *
 * Every field is optional; omitted fields fall back to the git CLI defaults
 * (`include_untracked` is `true`, everything else `false`).
 */
export interface StatusOptions {
  /** Include untracked files in the status. Defaults to `true`. */
  includeUntracked?: boolean
  /** Include ignored files in the status. Defaults to `false`. */
  includeIgnored?: boolean
  /** Include unmodified files in the status. Defaults to `false`. */
  includeUnmodified?: boolean
  /** Skip submodules. Defaults to `false`. */
  excludeSubmodules?: boolean
  /**
   * Recurse into untracked directories instead of reporting the directory
   * itself. Defaults to `false`.
   */
  recurseUntrackedDirs?: boolean
  /** Detect renames between the HEAD tree and the index. Defaults to `false`. */
  renamesHeadToIndex?: boolean
  /** Detect renames between the index and the working directory. Defaults to `false`. */
  renamesIndexToWorkdir?: boolean
  /** Restrict the scan to the given pathspecs. */
  pathspec?: Array<string>
}

/**
 * A single blame hunk: a contiguous run of lines attributed to one commit.
 *
 * All identity fields are copied out of the borrowed `git2::BlameHunk` so the
 * value can safely outlive the underlying `git2::Blame`.
 */
export interface BlameHunk {
  /** Number of lines covered by this hunk. */
  linesInHunk: number
  /** 40-char lowercase hex OID of the commit where these lines were last changed. */
  finalCommitId: string
  /** Line number where this hunk begins in the final file (1-based). */
  finalStartLine: number
  /** Author name of the final commit. Undefined if absent or not valid UTF-8. */
  finalAuthorName?: string
  /** Author email of the final commit. Undefined if absent or not valid UTF-8. */
  finalAuthorEmail?: string
  /** Author time of the final commit, as a `Date`. The Unix epoch if no signature. */
  finalTime: Date
  /** 40-char lowercase hex OID of the commit where this hunk was found. */
  origCommitId: string
  /** Line number where this hunk begins in the original file (1-based). */
  origStartLine: number
  /** Path to the file where this hunk originated. Undefined if not valid UTF-8. */
  origPath?: string
  /** Whether this hunk was tracked to a boundary commit (root or `oldest_commit`). */
  isBoundary: boolean
}

/**
 * Options controlling how a blame is computed.
 *
 * Every field is optional; omitted fields fall back to libgit2's defaults
 * (no copy tracking, the whole file, starting from the current HEAD).
 */
export interface BlameOptions {
  /** Track lines that have moved within a file. Defaults to `false`. */
  trackCopiesSameFile?: boolean
  /** Track lines that have moved across files in the same commit. Defaults to `false`. */
  trackCopiesSameCommitMoves?: boolean
  /** 40-char hex OID of the newest commit to consider (the blame starts here). */
  newestCommit?: string
  /** 40-char hex OID of the oldest commit to consider (a boundary). */
  oldestCommit?: string
  /** Restrict the search to first-parent history only. Defaults to `false`. */
  firstParent?: boolean
  /** Map names/emails through the repository's mailmap. Defaults to `false`. */
  useMailmap?: boolean
  /** Ignore whitespace differences. Defaults to `false`. */
  ignoreWhitespace?: boolean
  /** The first line in the file to blame (1-based). */
  minLine?: number
  /** The last line in the file to blame (1-based). */
  maxLine?: number
}

/**
 * Options controlling how a checkout writes files into the working directory.
 *
 * The default is a **safe** checkout (matching `git checkout`): files with
 * local modifications are left untouched. Set `force` to overwrite them, which
 * can discard uncommitted changes — use it deliberately.
 */
export interface CheckoutOptions {
  /**
   * Force the checkout, overwriting any local changes in the working tree.
   * Defaults to a safe checkout when omitted or `false`.
   */
  force?: boolean
  /**
   * Recreate files that are missing from the working tree even in a safe
   * checkout.
   */
  recreateMissing?: boolean
  /** Allow the checkout to write files that conflict with the working tree. */
  allowConflicts?: boolean
  /**
   * Restrict the checkout to these pathspecs. When omitted, all paths are
   * checked out.
   */
  paths?: Array<string>
  /**
   * Write the checked-out files into this directory instead of the
   * repository's working directory.
   */
  targetDir?: string
}
```

## `Reference`

### Usage

```ts
import { Repository } from '@napi-rs/simple-git'

const repo = new Repository('/path/to/repo') // Open an existed repo

const headReference = repo.head()

headReference.shorthand() // 'main'
headReference.name() // 'refs/heads/main'
headReference.target() // 7a1256e2f847f395219980bc06c6dadf0148f18d
```

### API

```ts
/** An enumeration of all possible kinds of references. */
export const enum ReferenceType {
  /** A reference which points at an object id. */
  Direct = 0,
  /** A reference which points at another reference. */
  Symbolic = 1,
  Unknown = 2
}
export class Reference {
  /**
   * Ensure the reference name is well-formed.
   *
   * Validation is performed as if [`ReferenceFormat::ALLOW_ONELEVEL`]
   * was given to [`Reference.normalize_name`]. No normalization is
   * performed, however.
   *
   * ```ts
   * import { Reference } from '@napi-rs/simple-git'
   *
   * console.assert(Reference.isValidName("HEAD"));
   * console.assert(Reference.isValidName("refs/heads/main"));
   *
   * // But:
   * console.assert(!Reference.isValidName("main"));
   * console.assert(!Reference.isValidName("refs/heads/*"));
   * console.assert(!Reference.isValidName("foo//bar"));
   * ```
   */
  static isValidName(name: string): boolean
  /** Check if a reference is a local branch. */
  isBranch(): boolean
  /** Check if a reference is a note. */
  isNote(): boolean
  /** Check if a reference is a remote tracking branch */
  isRemote(): boolean
  /** Check if a reference is a tag */
  isTag(): boolean
  kind(): ReferenceType
  /**
   * Get the full name of a reference.
   *
   * Returns `None` if the name is not valid utf-8.
   */
  name(): string | null
  /**
   * Get the full shorthand of a reference.
   *
   * This will transform the reference name into a name "human-readable"
   * version. If no shortname is appropriate, it will return the full name.
   *
   * Returns `None` if the shorthand is not valid utf-8.
   */
  shorthand(): string | null
  /**
   * Get the OID pointed to by a direct reference.
   *
   * Only available if the reference is direct (i.e. an object id reference,
   * not a symbolic one).
   */
  target(): string | null
  /**
   * Return the peeled OID target of this reference.
   *
   * This peeled OID only applies to direct references that point to a hard
   * Tag object: it is the result of peeling such Tag.
   */
  targetPeel(): string | null
  /**
   * Get full name to the reference pointed to by a symbolic reference.
   *
   * May return `None` if the reference is either not symbolic or not a
   * valid utf-8 string.
   */
  symbolicTarget(): string | null
}
```

## Performance

Compared with the `exec` function, which gets the file's latest modified date by spawning a child process. Getting the latest modified date from the file 1000 times:

```
Child process took 1.9s
@napi-rs/simple-git took 65ms
```
