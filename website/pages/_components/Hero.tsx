import Button from './Button'
import CodeBlock from './CodeBlock'
import CountUp from './_CountUp'
import InstallSwitcher from './_InstallSwitcher'
import Reveal from './_Reveal'
import { benchSpeedup } from '../_data/benchmarks'
import { platformCount } from '../_data/platforms'
import { heroSample } from '../_data/samples'

// Stat tiles. Numbers are grounded, not hard-coded:
//   • benchSpeedup (29) is derived in _data/benchmarks.ts (1.9 s / 65 ms).
//   • platformCount (15) is derived from _data/platforms.ts.
//   • 0 = the package's runtime `dependencies` count (root package.json is `{}`).
const stats = [
  { node: <CountUp to={benchSpeedup} suffix="×" />, label: 'faster than the git CLI' },
  { node: <CountUp to={platformCount} />, label: 'prebuilt platforms' },
  { node: <CountUp to={0} />, label: 'runtime dependencies' },
]

export default function Hero({ codeHtml }: { codeHtml: string }) {
  return (
    <section className="relative overflow-hidden">
      <div className="accent-glow" />
      <div className="container-page grid items-start gap-12 pt-20 pb-16 md:pt-28 lg:grid-cols-[minmax(0,1fr)_minmax(0,540px)] lg:gap-16">
        <div className="flex min-w-0 flex-col">
          <span className="eyebrow">NATIVE NODE ADDON · POWERED BY RUST · libgit2</span>
          <h1 className="mt-5 font-display text-display-xl text-(--color-fg)">
            Git for Node, at <span className="text-(--color-accent)">native</span> speed.
          </h1>
          <p className="mt-6 max-w-xl text-lg text-(--color-muted)">
            Open, inspect, stage, commit, blame, branch and push real repositories through libgit2 — no{' '}
            <code className="font-mono text-(--color-fg)">git</code> shell-out, with JS-native Date / number / Buffer
            types.
          </p>
          <div className="mt-8 flex flex-wrap items-center gap-3">
            <Button variant="primary" href="/docs">
              Get started
            </Button>
            <Button variant="secondary" href="https://github.com/Brooooooklyn/simple-git" target="_blank" rel="noreferrer">
              GitHub
            </Button>
            <Button
              variant="ghost"
              href="https://www.npmjs.com/package/@napi-rs/simple-git"
              target="_blank"
              rel="noreferrer"
            >
              npm
            </Button>
          </div>
          <div className="mt-8">
            <InstallSwitcher />
          </div>
        </div>

        <Reveal className="flex min-w-0 flex-col gap-6" delay={120}>
          <CodeBlock html={codeHtml} filename="last-updated.ts" copyText={heroSample} />
          <div className="grid grid-cols-3 gap-2 sm:gap-3">
            {stats.map((s) => (
              <div
                key={s.label}
                className="rounded-xl border border-(--color-border) bg-(--color-surface-1) px-3 py-4 transition-colors hover:border-(--color-border-strong) sm:px-4 sm:py-5"
              >
                <div className="font-mono text-xl tabular-nums text-(--color-accent) sm:text-2xl md:text-3xl">
                  {s.node}
                </div>
                <div className="mt-2 font-mono text-[0.7rem] uppercase leading-snug tracking-wide text-(--color-faint)">
                  {s.label}
                </div>
              </div>
            ))}
          </div>
        </Reveal>
      </div>
    </section>
  )
}
