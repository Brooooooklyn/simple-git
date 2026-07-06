import { type ReactNode } from 'react'

// Docs shell. The site header + footer already come from pages/layout.tsx — Void
// composes nested layouts by directory, so this layout only frames the docs prose
// and must NOT re-render them.
//
// Two-column shell: a docs sidebar (mobile <details> disclosure + desktop sticky
// <aside>) beside the markdown prose column. `.void-md` applies the @void/md prose
// theme (app.css imports `@void/md/theme-content.css`, i.e. prose + code + container
// styles). Fenced code blocks are highlighted at build time by voidMarkdown() using
// Shiki's pure-JS regex engine — there is no runtime WebAssembly, and the whole page
// renders fully readable with JavaScript disabled (it never hydrates).

// Explicit, ordered docs navigation. Kept as a hand-maintained list (rather than
// auto-discovered) so the order and labels are deterministic — docs read
// top-to-bottom, and that order is editorial, not alphabetical.
//
// No active-link highlight: these are fully server-rendered markdown pages that
// don't hydrate, and a nested layout's router only exposes its mounted base
// ('/docs'), not the leaf route — so the current page can't be resolved here
// without client JS the page never ships. The sidebar is a plain nav list.
const NAV = [
  { href: '/docs', label: 'Getting Started' },
  { href: '/docs/api', label: 'API Reference' },
]

export default function DocsLayout({ children }: { children: ReactNode }) {
  return (
    <div className="container-page py-12 md:py-16">
      {/* Mobile docs nav. Native <details> — no JS, so it works on these
          non-hydrating markdown pages. Replaced by the sidebar at md+. */}
      <details className="mb-8 rounded-lg border border-(--color-border) bg-(--color-surface-1) md:hidden">
        <summary className="flex cursor-pointer list-none items-center justify-between px-4 py-3 font-mono text-xs uppercase tracking-wider text-(--color-muted)">
          Documentation
          <span className="nav-caret text-(--color-faint)" aria-hidden="true">
            ▾
          </span>
        </summary>
        <nav aria-label="Documentation" className="flex flex-col gap-1 px-2 pb-2 text-sm">
          {NAV.map((item) => (
            <a
              key={item.href}
              href={item.href}
              className="flex min-h-11 items-center rounded-md px-2 text-(--color-muted) transition-colors hover:bg-(--color-surface-2) hover:text-(--color-fg)"
            >
              {item.label}
            </a>
          ))}
        </nav>
      </details>
      <div className="flex gap-10 lg:gap-16">
        <aside className="hidden w-48 shrink-0 md:block">
          <nav aria-label="Documentation" className="sticky top-20 flex flex-col gap-1 text-sm">
            <p className="mb-2 font-mono text-xs uppercase tracking-wider text-(--color-muted) opacity-60">
              Documentation
            </p>
            {NAV.map((item) => (
              <a
                key={item.href}
                href={item.href}
                className="border-l-2 border-transparent pl-3 text-(--color-muted) transition-colors hover:border-(--color-accent) hover:text-(--color-fg)"
              >
                {item.label}
              </a>
            ))}
          </nav>
        </aside>
        <article className="void-md min-w-0 max-w-3xl flex-1">{children}</article>
      </div>
    </div>
  )
}
