import { type ReactNode } from 'react'

// Docs shell. The site header + footer already come from pages/layout.tsx — Void
// composes nested layouts by directory, so this layout only frames the docs prose
// and must NOT re-render them. There is a single docs page (Getting Started), so
// there is no multi-page sidebar: a single centered prose column is enough, and the
// markdown itself carries an in-page `[[toc]]`.
//
// `.void-md` applies the @void/md prose theme (app.css imports
// `@void/md/theme-content.css`, i.e. prose + code + container styles). Fenced code
// blocks are highlighted at build time by voidMarkdown() using Shiki's pure-JS regex
// engine — there is no runtime WebAssembly, and the whole page renders fully readable
// with JavaScript disabled (it never hydrates).
export default function DocsLayout({ children }: { children: ReactNode }) {
  return (
    <div className="container-page py-12 md:py-16">
      <article className="void-md mx-auto min-w-0 max-w-3xl">{children}</article>
    </div>
  )
}
