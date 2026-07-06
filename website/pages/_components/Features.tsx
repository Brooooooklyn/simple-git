import SectionHeader from './SectionHeader'
import Reveal from './_Reveal'
import { features } from '../_data/features'

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
                <p className="mt-2 text-sm leading-relaxed text-(--color-muted)">{f.desc}</p>
              </div>
            ))}
          </div>
        </Reveal>
      </div>
    </section>
  )
}
