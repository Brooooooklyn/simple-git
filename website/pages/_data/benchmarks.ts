// Benchmark data, grounded in the README "Performance" section + performance.mjs.
// Task: read `getFileLatestModifiedDate` 1000 times.
//   git CLI via child_process.exec .......... 1.9 s
//   @napi-rs/simple-git ..................... 65 ms
// The only baseline is spawning the real `git` CLI as a child process. Do NOT
// invent other baselines or numbers.

export type BenchBar = {
  /** Human label for the approach being measured. */
  label: string
  /** Wall-clock time in milliseconds (for proportional bar widths). */
  ms: number
  /** Pre-formatted duration exactly as shown in the README. */
  display: string
  /** The native library — highlighted as the winner. */
  highlight?: boolean
}

/** Number of `getFileLatestModifiedDate` calls the benchmark performs. */
export const benchIterations = 1000

export const benchBars: BenchBar[] = [
  { label: 'git CLI (child_process)', ms: 1900, display: '1.9 s' },
  { label: '@napi-rs/simple-git', ms: 65, display: '65 ms', highlight: true },
]

/** 1900 ms / 65 ms ≈ 29× — derived, never fabricated. */
export const benchSpeedup = Math.round(benchBars[0].ms / benchBars[1].ms)

export const benchCaption =
  'Reading getFileLatestModifiedDate 1000× vs spawning the git CLI as a child process (source: README Performance · performance.mjs).'
