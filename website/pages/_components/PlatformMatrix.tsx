import SectionHeader from './SectionHeader'
import Chip from './Chip'
import Reveal from './_Reveal'
import { platformGroups, platformCount, platformsCaption, nodeEngine } from '../_data/platforms'

// PLATFORMS section — the 15 prebuilt napi target triples grouped by OS. All content
// is sourced from _data/platforms.ts (ground truth: the repo-root package.json
// `napi.targets` + `engines.node`); the count and Node version are derived, not
// hard-coded. Every triple is in the SSR HTML (no-JS safe); <Reveal> only adds the
// scroll fade once JS hydrates.
export default function PlatformMatrix() {
  return (
    <section className="border-t border-(--color-border)">
      <div className="container-page py-20 md:py-28">
        <SectionHeader index="05" label="PLATFORMS" title="Prebuilt everywhere" subhead={platformsCaption} />

        <Reveal className="mt-12">
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {platformGroups.map((group) => (
              <div
                key={group.os}
                className="flex flex-col rounded-xl border border-(--color-border) bg-(--color-surface-1) p-5 transition-colors hover:border-(--color-border-strong)"
              >
                <div className="flex items-baseline justify-between">
                  <h3 className="font-display text-base font-medium text-(--color-fg)">{group.os}</h3>
                  <span className="font-mono text-xs tabular-nums text-(--color-faint)">
                    {group.platforms.length}
                  </span>
                </div>
                <ul className="mt-4 flex flex-col gap-3">
                  {group.platforms.map((p) => (
                    <li key={p.triple} className="flex flex-col gap-1.5">
                      <Chip tone="muted">{p.label}</Chip>
                      <code className="block overflow-x-auto font-mono text-xs text-(--color-muted)">
                        {p.triple}
                      </code>
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </div>

          <p className="mt-6 font-mono text-xs tabular-nums text-(--color-faint)">
            {platformCount} prebuilt triples · Node {nodeEngine}
          </p>
        </Reveal>
      </div>
    </section>
  )
}
