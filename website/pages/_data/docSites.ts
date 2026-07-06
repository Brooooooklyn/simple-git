// Doc-site generators whose "last updated on" stamps can be computed from git
// history with @napi-rs/simple-git (spec §5). Names + official homepage URLs only.
//
// NOTE: no remote logos are fetched or hotlinked. Each entry carries a plain
// monogram `initial` rendered as a text glyph in the UI — no brand image assets,
// no external requests (a binding constraint of this build).

export type DocSite = {
  /** Product name. */
  name: string
  /** Official homepage. */
  url: string
  /** Single-letter monogram used as an inline text glyph (no external logo). */
  initial: string
}

export const docSites: DocSite[] = [
  { name: 'Nextra', url: 'https://nextra.site', initial: 'N' },
  { name: 'Docusaurus', url: 'https://docusaurus.io', initial: 'D' },
  { name: 'Starlight', url: 'https://starlight.astro.build', initial: 'S' },
  { name: 'Fumadocs', url: 'https://fumadocs.dev', initial: 'F' },
  { name: 'Rspress', url: 'https://rspress.dev', initial: 'R' },
]
