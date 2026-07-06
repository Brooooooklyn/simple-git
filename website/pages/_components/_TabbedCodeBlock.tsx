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
// Progressive enhancement: every panel is rendered into the SSR HTML and NONE carry
// the `hidden` attribute, so the SSR/no-JS/crawler HTML keeps all five samples fully
// visible. app.css does the show/hide, gated on `html.js` (set before first paint):
//   - no JS  → tab bar hidden; every panel shown stacked, each with a visible label,
//   - JS on  → only the active panel shows (data-active), the per-panel labels drop
//              out, and clicking a tab swaps panels instantly (no refetch).
// Because `html.js` is present pre-paint, JS-on clients hide the inactive panels
// immediately — no flash of all five before hydration.
export default function TabbedCodeBlock({
  tabs,
  className,
}: {
  tabs: CodeTab[]
  className?: string
}) {
  const [active, setActive] = useState(0)

  return (
    <div className={cx('code-tabs', className)}>
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
            data-active={i === active ? 'true' : 'false'}
            className="code-tab-panel"
          >
            {/* No-JS affordance: names the sample when the (JS-only) tab bar is hidden.
                Dropped once `html.js` is set — the tab bar then names it instead. */}
            <div className="code-tab-label mb-2 font-mono text-xs font-medium text-(--color-accent)">{tab.label}</div>
            <CodeBlock html={tab.html} copyText={tab.copyText} filename={tab.filename} />
          </div>
        ))}
      </div>
    </div>
  )
}
