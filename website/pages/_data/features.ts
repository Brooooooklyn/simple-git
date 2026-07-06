// The 9 feature cards for the FEATURES section (spec §5, grounded in the repo).
// Titles + one-line descriptions are copied from the design spec; every API name
// referenced here exists in README.md / index.d.ts.

export type Feature = {
  /** Card heading. */
  title: string
  /** One-line supporting description. */
  desc: string
}

export const features: Feature[] = [
  {
    title: 'Native libgit2 speed',
    desc: 'Talks to Git through compiled Rust/libgit2, not a child-process `git` shell-out.',
  },
  {
    title: '"Last updated" dates for docs',
    desc: '`getFileLatestModifiedDate` / `getFileLastModifiedDate` / `getFilesLatestModified` power "last updated on" stamps in Nextra, Docusaurus, Starlight, Fumadocs, Rspress.',
  },
  {
    title: 'Full repository toolbox',
    desc: 'Init, open, discover, clone; status, stage, commit, blame, diff, branch, checkout, tag, refs, remotes, revwalk — one cohesive `Repository` API.',
  },
  {
    title: 'Off-main-thread async',
    desc: '`cloneAsync`, `commitAsync`, `fetchAsync`, `pushAsync`, plus async status/blame/file-date variants, each accepting an `AbortSignal`.',
  },
  {
    title: 'First-class TypeScript',
    desc: 'Hand-annotated `index.d.ts` with JS-native types: `Date`, `number`, `Buffer` instead of raw libgit2 primitives.',
  },
  {
    title: 'Typed, catchable errors',
    desc: 'Git-layer failures carry a stable `GitErrorCode`, narrowable with the total, never-throwing `isGitError()` guard; an aborted async call rejects with napi\'s `AbortError` instead.',
  },
  {
    title: 'Prebuilt for 15 platforms',
    desc: 'Windows (x64/x86/ARM64), macOS (Intel + Apple Silicon), Linux glibc & musl, Android, FreeBSD — no compiler needed.',
  },
  {
    title: 'Push/fetch with real creds & progress',
    desc: 'SSH-agent, SSH-key, userpass via `Cred`; transfer + per-ref update callbacks via `RemoteCallbacks`.',
  },
  {
    title: 'Deterministic cleanup',
    desc: '`dispose()` / `free()` release native handles eagerly (frees Windows packfile fds); `using` supported.',
  },
]
