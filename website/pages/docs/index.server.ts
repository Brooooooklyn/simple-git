import { defineHead } from 'void'

// /docs is fully static markdown, so prerender it at deploy time (served from the edge
// cache, no per-request render). `prerender = true` implies a 1-year revalidate TTL;
// the ISR cache is cleared each deploy so new content ships immediately. Islands still
// hydrate client-side. NOTE: void skips prerender when a path's revalidate TTL is 0, so
// void.json must NOT force `routing.revalidate: {"*": 0}`.
export const prerender = true

// Per-route head for /docs. Void's markdown frontmatter carries ONLY title +
// description — any head/link/og frontmatter keys are silently dropped — so the
// docs-specific canonical + og:url MUST live here, in a co-located server head.
//
// A server `head` export REPLACES the frontmatter-derived head, so the title and
// description are RE-DECLARED below (kept in sync with docs/index.md frontmatter).
// The base head in void.json still applies via Void's merge: `meta` entries are
// overridden by matching name/property and otherwise appended, `link`s are
// concatenated, and the `title` is wrapped by the `titleTemplate`
// ("%s | @napi-rs/simple-git"). So /docs keeps every route-invariant social tag
// (og:type, og:site_name, twitter:card, og:image, theme-color, charset) and only
// adds its own canonical, og:url and a right-length description.
//
// DESCRIPTION is trimmed to <=160 chars (dropped the trailing "Powered by libgit2
// and Rust." clause from the 175-char frontmatter copy) so it renders in full in
// search results instead of getting clipped.
const DESCRIPTION =
  'Fast, native Git for Node.js — read the last-modified commit time of any file, run status, stage and commit in-process, with no child git process.'

// Non-trailing-slash canonical form; void.json 308-redirects /docs/ -> /docs.
const CANONICAL = 'https://simple-git.napi.rs/docs'

export const head = defineHead(() => ({
  title: 'Getting Started',
  link: [{ rel: 'canonical', href: CANONICAL }],
  meta: [
    { name: 'description', content: DESCRIPTION },
    { property: 'og:title', content: 'Getting Started' },
    { property: 'og:description', content: DESCRIPTION },
    { property: 'og:url', content: CANONICAL },
  ],
}))
