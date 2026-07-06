import { defineHead } from 'void'

// /docs/api is fully static markdown, so prerender it at deploy time (served from the
// edge cache, no per-request render). `prerender = true` implies a 1-year revalidate
// TTL; the ISR cache is cleared each deploy so new content ships immediately. Islands
// still hydrate client-side. NOTE: void skips prerender when a path's revalidate TTL
// is 0, so void.json must NOT force `routing.revalidate: {"*": 0}`.
export const prerender = true

// Per-route head for /docs/api. Void's markdown frontmatter carries ONLY title +
// description — any head/link/og frontmatter keys are silently dropped — so the
// route-specific canonical + og:url MUST live here, in a co-located server head.
//
// A server `head` export REPLACES the frontmatter-derived head, so the title and
// description are RE-DECLARED below (kept in sync with docs/api.md frontmatter).
// The base head in void.json still applies via Void's merge, so /docs/api keeps every
// route-invariant social tag (og:type, og:site_name, twitter:card, og:image,
// theme-color, charset) and the titleTemplate ("%s | @napi-rs/simple-git"), and only
// adds its own canonical, og:url and a right-length description.
const DESCRIPTION =
  'The complete @napi-rs/simple-git surface — Repository, git object handles, options, enums, functions and typed error handling, all from the package root.'

// Non-trailing-slash canonical form.
const CANONICAL = 'https://simple-git.napi.rs/docs/api'

export const head = defineHead(() => ({
  title: 'API Reference',
  link: [{ rel: 'canonical', href: CANONICAL }],
  meta: [
    { name: 'description', content: DESCRIPTION },
    { property: 'og:title', content: 'API Reference' },
    { property: 'og:description', content: DESCRIPTION },
    { property: 'og:url', content: CANONICAL },
  ],
}))
