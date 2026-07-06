// Code samples for the landing page, copied VERBATIM from the repo README.md.
// Do NOT hand-edit the strings — regenerate from README.md if the docs change so
// they stay byte-identical. Each sample notes the README.md line ranges it was
// sliced from (1-indexed, inclusive). Stored as raw TypeScript strings for Shiki
// to highlight server-side (see pages/index.server.ts).

// README.md lines 12, 16, 18-21, 27-30
export const heroSample = `import { Repository, BranchType, RemoteCallbacks, PushOptions } from '@napi-rs/simple-git'

const repo = new Repository('/path/to/repo') // Open an existed repo

// Last-modified commit time of \`build.rs\` in milliseconds since the Unix epoch.
// Returns a \`number\`; throws when no commit in history touched the path.
const lastModified = repo.getFileLatestModifiedDate('build.rs')
console.log(new Date(lastModified)) // 2022-03-13T12:47:47.920Z

// Null-safe alternative: a \`Date\`, or \`null\` (never throws) when no commit ever
// touched the path.
const lastModifiedDate = repo.getFileLastModifiedDate('build.rs')
if (lastModifiedDate) console.log(lastModifiedDate) // 2022-03-13T12:47:47.920Z`

// README.md lines 49-55
export const statusSample = `// ---- Working-tree status (like \`git status\`) ----
const changes = repo.statuses() // => FileStatus[]
for (const file of changes) {
  console.log(file.path, file.isWtModified, file.isIndexNew)
}
console.log(repo.statusFile('README.md').isWtModified) // status of a single path
const scanned = await repo.statusesAsync({ includeIgnored: true }) // off-thread scan`

// README.md lines 57-63, 65-73
export const commitSample = `// ---- Config + default signature ----
const config = repo.config() // => Config (system + global + repo, prioritized)
config.setString('user.name', 'LongYinan')
console.log(config.getString('user.name')) // 'LongYinan'
console.log(config.getBoolean('core.bare')) // false
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
console.log(commitId) // 40-char hex OID`

// README.md lines 91-95
export const blameSample = `// ---- Blame ----
for (const hunk of repo.blameFile('build.rs')) {
  console.log(hunk.finalStartLine, hunk.linesInHunk, hunk.finalCommitId, hunk.finalAuthorName)
}
console.log(repo.blameLine('build.rs', 10)?.finalAuthorName) // hunk for line 10, or null`

// README.md lines 97-108
export const pushSample = `// ---- Push ----
const remote = repo.findRemote('origin')!
const callbacks = new RemoteCallbacks()
  // Per-ref result: one object per updated reference.
  .pushUpdateReference(({ refname, status }) => {
    console.log(refname, status) // 'refs/heads/main' null   (null === accepted)
  })
  // Pack-transfer progress: a single PushTransferProgress object.
  .pushTransferProgress(({ current, total, bytes }) => {
    console.log(\`\${current}/\${total} objects, \${bytes} bytes\`)
  })
remote.push(['refs/heads/main'], new PushOptions().remoteCallback(callbacks))`

// README.md lines 664-672
export const errorsSample = `import { isGitError, GitErrorCode } from '@napi-rs/simple-git'

try {
  // …some git operation…
} catch (e) {
  if (isGitError(e) && e.code === GitErrorCode.NotFound) {
    // handle the missing object/reference/config entry
  }
}`

