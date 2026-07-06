import type { ReactNode } from 'react'
import SectionHeader from './SectionHeader'
import Reveal from './_Reveal'
import { features } from '../_data/features'

// The feature `desc` strings carry markdown-style backticks around API names. Render
// backtick-delimited spans as styled inline <code> instead of literal backticks. This
// is a tiny, safe split — no markdown library, no data changes: split on the backtick,
// so odd-indexed segments are the code spans. Pure render (no client JS) → server-
// rendered and no-JS safe; React escapes each segment, so it's XSS-safe too.
function renderDesc(desc: string): ReactNode[] {
  return desc.split('`').map((segment, i) =>
    i % 2 === 1 ? (
      <code
        key={i}
        className="rounded bg-(--color-surface-2) px-1 py-0.5 font-mono text-[0.85em] text-(--color-fg)"
      >
        {segment}
      </code>
    ) : (
      segment
    ),
  )
}

// FEATURES section — a responsive 3-column card grid (3×3 on desktop) of the nine
// feature cards. Content is sourced from _data/features.ts (grounded in README.md /
// index.d.ts); nothing is hard-coded here. Wrapped in <Reveal> so it fades in on
// scroll once JS hydrates — but the cards are fully in the SSR HTML (no-JS safe).
export default function Features() {
  return (
    <section className="border-t border-(--color-border)">
      <div className="container-page py-20 md:py-28">
        <SectionHeader index="03" label="FEATURES" title="A full Git toolbox" />

        <Reveal className="mt-12">
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {features.map((f) => (
              <div
                key={f.title}
                className="flex flex-col rounded-xl border border-(--color-border) bg-(--color-surface-1) p-5 transition-colors hover:border-(--color-border-strong) md:p-6"
              >
                <h3 className="font-display text-base font-medium text-(--color-fg)">{f.title}</h3>
                <p className="mt-2 text-sm leading-relaxed text-(--color-muted)">{renderDesc(f.desc)}</p>
              </div>
            ))}
          </div>
        </Reveal>
      </div>
    </section>
  )
}
