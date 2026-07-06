// The 15 prebuilt napi target triples, grouped by OS for the PLATFORMS section.
// Ground truth: the `napi.targets` array in the repo-root package.json (verified
// against it — all 15 present). Node engine is `>= 10` (package.json `engines`).

export type Platform = {
  /** The Rust target triple, exactly as published. */
  triple: string
  /** Short human label for the arch/variant within its OS group. */
  label: string
}

export type PlatformGroup = {
  /** OS family heading. */
  os: string
  platforms: Platform[]
}

export const platformGroups: PlatformGroup[] = [
  {
    os: 'Windows',
    platforms: [
      { triple: 'x86_64-pc-windows-msvc', label: 'x64' },
      { triple: 'i686-pc-windows-msvc', label: 'x86' },
      { triple: 'aarch64-pc-windows-msvc', label: 'ARM64' },
    ],
  },
  {
    os: 'macOS',
    platforms: [
      { triple: 'x86_64-apple-darwin', label: 'Intel' },
      { triple: 'aarch64-apple-darwin', label: 'Apple Silicon' },
    ],
  },
  {
    os: 'Linux (glibc)',
    platforms: [
      { triple: 'x86_64-unknown-linux-gnu', label: 'x64' },
      { triple: 'aarch64-unknown-linux-gnu', label: 'ARM64' },
      { triple: 'armv7-unknown-linux-gnueabihf', label: 'ARMv7' },
      { triple: 'powerpc64le-unknown-linux-gnu', label: 'ppc64le' },
      { triple: 's390x-unknown-linux-gnu', label: 's390x' },
    ],
  },
  {
    os: 'Linux (musl)',
    platforms: [
      { triple: 'x86_64-unknown-linux-musl', label: 'x64' },
      { triple: 'aarch64-unknown-linux-musl', label: 'ARM64' },
    ],
  },
  {
    os: 'Android',
    platforms: [
      { triple: 'aarch64-linux-android', label: 'ARM64' },
      { triple: 'armv7-linux-androideabi', label: 'ARMv7' },
    ],
  },
  {
    os: 'FreeBSD',
    platforms: [{ triple: 'x86_64-unknown-freebsd', label: 'x64' }],
  },
]

/** Total prebuilt platforms (15) — derived from the groups, not hard-coded. */
export const platformCount = platformGroups.reduce((n, g) => n + g.platforms.length, 0)

/** Minimum supported Node version (package.json `engines.node`). */
export const nodeEngine = '>= 10'

export const platformsCaption = 'npm install pulls a ready binary — no compiler, no node-gyp.'
