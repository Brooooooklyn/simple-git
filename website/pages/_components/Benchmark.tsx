import type { ReactNode } from 'react'
import { useEffect, useRef, useState } from 'react'
import { benchBars, benchIterations, benchSpeedup, benchCaption } from '../_data/benchmarks'
import SectionHeader from './SectionHeader'
import Reveal from './_Reveal'
import CountUp from './_CountUp'

// Widths are proportional to wall-clock time, so the git-CLI bar fills the track and
// the native bar is a deliberate sliver — the visual point of the section. Scaled to
// the slowest run (git CLI, 1.9 s) so the ratio is honest, not exaggerated.
const maxMs = Math.max(...benchBars.map((b) => b.ms))
const widthFor = (ms: number) => `${(ms / maxMs) * 100}%`

type Widths = Record<string, string>

// Bars render at full target width for SSR / no-JS, then grow from 0 once scrolled
// into view (skipped under prefers-reduced-motion). Mirrors the reference mechanism.
function BarTrack({ children }: { children: (widths: Widths) => ReactNode }) {
  const finalWidths: Widths = Object.fromEntries(benchBars.map((b) => [b.label, widthFor(b.ms)]))
  const [widths, setWidths] = useState<Widths>(finalWidths)
  const ref = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (typeof window === 'undefined' || typeof IntersectionObserver === 'undefined') return
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return

    const el = ref.current
    if (!el) return

    setWidths(Object.fromEntries(benchBars.map((b) => [b.label, '0%'])))

    const obs = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (!entry.isIntersecting) continue
          obs.disconnect()
          // Next frame so the browser registers the 0% start before transitioning.
          requestAnimationFrame(() => setWidths(finalWidths))
        }
      },
      { threshold: 0.3 },
    )
    obs.observe(el)
    return () => obs.disconnect()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  return <div ref={ref}>{children(widths)}</div>
}

export default function Benchmark() {
  return (
    <section className="border-t border-(--color-border)">
      <div className="container-page py-20 md:py-28">
        <SectionHeader
          index="01"
          label="BENCHMARK"
          title={
            <>
              <span className="text-(--color-accent)">~30×</span> faster than shelling out
            </>
          }
          subhead={`Reading getFileLatestModifiedDate ${benchIterations.toLocaleString()}× — the native call versus spawning the git CLI as a child process.`}
        />

        <Reveal className="mt-12">
          <div className="grid gap-10 lg:grid-cols-[minmax(0,16rem)_minmax(0,1fr)] lg:items-center lg:gap-12">
            <div className="relative">
              <div className="accent-glow" aria-hidden />
              <p className="eyebrow">speedup</p>
              <p className="mt-3 font-display text-display-lg leading-none text-(--color-fg)">
                <CountUp to={benchSpeedup} suffix="×" />
              </p>
              <p className="mt-4 max-w-xs text-sm text-(--color-muted)">
                {benchIterations.toLocaleString()} reads drop from{' '}
                <span className="text-(--color-fg)">{benchBars[0].display}</span> of child processes to{' '}
                <span className="text-(--color-accent)">{benchBars[1].display}</span> in-process.
              </p>
            </div>

            <div className="rounded-xl border border-(--color-border) bg-(--color-surface-1) p-6 md:p-8">
              <BarTrack>
                {(widths) => (
                  <div className="space-y-7">
                    {benchBars.map((bar) => (
                      <div key={bar.label} className="space-y-2.5">
                        <div className="flex items-baseline justify-between gap-3">
                          <span className="min-w-0 truncate font-mono text-xs text-(--color-muted)">{bar.label}</span>
                          <span
                            className={
                              'shrink-0 font-mono text-sm tabular-nums ' +
                              (bar.highlight ? 'text-(--color-accent)' : 'text-(--color-faint)')
                            }
                          >
                            {bar.display}
                          </span>
                        </div>
                        <div className="h-8 overflow-hidden rounded-md bg-(--color-bg)">
                          <div
                            className={
                              'h-full rounded-md transition-[width] duration-1000 ease-out ' +
                              (bar.highlight ? 'bg-(--color-accent)' : 'bg-(--color-faint)')
                            }
                            style={{ width: widths[bar.label] }}
                          />
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </BarTrack>

              <p className="mt-7 font-mono text-xs leading-relaxed text-(--color-faint)">{benchCaption}</p>
            </div>
          </div>
        </Reveal>
      </div>
    </section>
  )
}
