import CodeBlock from './CodeBlock'
import SectionHeader from './SectionHeader'
import Reveal from './_Reveal'
import { docSites } from '../_data/docSites'
import { docSiteSample } from '../_data/samples'

export default function DocSiteShowcase({ codeHtml }: { codeHtml: string }) {
  return (
    <section className="border-t border-(--color-border)">
      <div className="container-page py-20 md:py-28">
        <SectionHeader
          index="02"
          label="WHY"
          title={
            <>
              Powers <span className="text-(--color-accent)">&lsquo;Last updated on&rsquo;</span> for your docs
            </>
          }
          subhead="Doc generators compute 'last updated' per page from git history. @napi-rs/simple-git does it in-process — fast, with no child_process per file."
        />

        <Reveal className="mt-12">
          <div className="grid gap-8 lg:grid-cols-2 lg:items-center lg:gap-12">
            {/* Mock doc-page footer — what the stamp looks like on a rendered docs page. */}
            <div className="rounded-xl border border-(--color-border) bg-(--color-surface-1) p-6 md:p-8">
              <div className="space-y-2.5" aria-hidden="true">
                <div className="h-3 w-2/5 rounded bg-(--color-surface-2)" />
                <div className="mt-5 h-2.5 w-full rounded bg-(--color-surface-2)" />
                <div className="h-2.5 w-11/12 rounded bg-(--color-surface-2)" />
                <div className="h-2.5 w-4/5 rounded bg-(--color-surface-2)" />
              </div>
              <div className="mt-6 flex items-center gap-2 border-t border-(--color-border) pt-4 text-(--color-faint)">
                <svg className="h-3.5 w-3.5 text-(--color-accent)" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" aria-hidden="true">
                  <circle cx="12" cy="12" r="9" />
                  <path d="M12 7v5l3 2" />
                </svg>
                <span className="font-mono text-xs">Last updated on Jul 6, 2026 by LongYinan</span>
              </div>
            </div>

            <CodeBlock html={codeHtml} filename="last-updated.ts" copyText={docSiteSample} />
          </div>

          {/* Used-by row — names + text-glyph monograms, linked to official homepages.
              No remote logos or external requests (a binding constraint of this build). */}
          <div className="mt-10">
            <p className="eyebrow text-(--color-faint)">Wire it into</p>
            <div className="mt-4 flex flex-wrap items-center gap-3">
              {docSites.map((site) => (
                <a
                  key={site.name}
                  href={site.url}
                  target="_blank"
                  rel="noreferrer"
                  className="group inline-flex items-center gap-2.5 rounded-lg border border-(--color-border) bg-(--color-surface-1) px-3.5 py-2 transition-colors hover:border-(--color-border-strong)"
                >
                  <span className="inline-flex h-6 w-6 items-center justify-center rounded-md bg-(--color-accent-muted) font-mono text-xs font-medium text-(--color-accent-fg)">
                    {site.initial}
                  </span>
                  <span className="text-sm text-(--color-muted) transition-colors group-hover:text-(--color-fg)">
                    {site.name}
                  </span>
                </a>
              ))}
            </div>
          </div>
        </Reveal>
      </div>
    </section>
  )
}
