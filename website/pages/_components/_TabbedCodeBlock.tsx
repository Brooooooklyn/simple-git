import { useState } from 'react'
import CodeBlock from './CodeBlock'

const cx = (...c: (string | false | undefined)[]) => c.filter(Boolean).join(' ')

export type CodeTab = {
  /** Stable id, also used for ARIA wiring. */
  id: string
  /** Tab button label. */
  label: string
  /** Shiki-highlighted HTML for this tab's sample (from index.server.ts). */
  html: string
  /** Raw source for the copy button. */
  copyText: string
  /** Optional filename shown in the block header. */
  filename?: string
}

// Tabbed wrapper over several <CodeBlock>s (island — holds the active-tab state).
// Every panel is rendered into the SSR HTML; inactive panels carry `hidden`, so:
//   - all snippets are in the HTML (crawler/SEO friendly),
//   - with JS off the first tab's highlighted code is still visible,
//   - after hydration, clicking a tab swaps which panel is shown (instant, no refetch).
export default function TabbedCodeBlock({
  tabs,
  className,
}: {
  tabs: CodeTab[]
  className?: string
}) {
  const [active, setActive] = useState(0)

  return (
    <div className={className}>
      <div role="tablist" aria-label="Code examples" className="flex flex-wrap items-center gap-2">
        {tabs.map((tab, i) => {
          const isActive = i === active
          return (
            <button
              key={tab.id}
              type="button"
              role="tab"
              id={`tab-${tab.id}`}
              aria-selected={isActive}
              aria-controls={`panel-${tab.id}`}
              onClick={() => setActive(i)}
              className={cx(
                'min-h-9 rounded-lg border px-3 py-1.5 font-mono text-xs transition-colors',
                isActive
                  ? 'border-(--color-accent) text-(--color-accent)'
                  : 'border-(--color-border) text-(--color-muted) hover:border-(--color-border-strong) hover:text-(--color-fg)',
              )}
            >
              {tab.label}
            </button>
          )
        })}
      </div>
      <div className="mt-4">
        {tabs.map((tab, i) => (
          <div
            key={tab.id}
            role="tabpanel"
            id={`panel-${tab.id}`}
            aria-labelledby={`tab-${tab.id}`}
            hidden={i !== active}
          >
            <CodeBlock html={tab.html} copyText={tab.copyText} filename={tab.filename} />
          </div>
        ))}
      </div>
    </div>
  )
}
