import { defineHandler, defineHead, type InferProps } from 'void'
import { highlight } from '../lib/highlight'
import {
  heroSample,
  statusSample,
  commitSample,
  blameSample,
  pushSample,
  errorsSample,
  docSiteSample,
} from './_data/samples'

// This page is fully static (no request- or time-varying data), so prerender it at
// deploy time and serve it from the edge cache instead of re-running Shiki on every
// request. `prerender = true` implies a 1-year revalidate TTL, but the ISR cache is
// cleared on each deploy, so a new build still goes live immediately. The Shiki
// highlighter (lib/highlight.ts, module-level singleton, pure-JS regex engine — no
// runtime WASM) runs at BUILD time; its HTML is baked into the prerendered document,
// so code stays fully highlighted with JS off. Islands (CountUp/InstallSwitcher/tabs)
// still hydrate client-side. NOTE: void skips prerender for any path whose revalidate
// TTL resolves to 0, so void.json must NOT force `routing.revalidate: {"*": 0}`.
export const prerender = true

// Pre-highlight every code sample server-side and hand the resulting HTML strings
// to the page as props. Section components render them with `dangerouslySetInnerHTML`
// via <CodeBlock> — so the code is fully styled in the SSR HTML and visible with JS
// off. Task 4/5 wire these props into the landing sections.
export const loader = defineHandler(async () => ({
  heroHtml: await highlight(heroSample),
  statusHtml: await highlight(statusSample),
  commitHtml: await highlight(commitSample),
  blameHtml: await highlight(blameSample),
  pushHtml: await highlight(pushSample),
  errorsHtml: await highlight(errorsSample),
  docSiteHtml: await highlight(docSiteSample),
}))

export type Props = InferProps<typeof loader>

const DESCRIPTION =
  "Native Git for Node.js via libgit2 — no git shell-out. Read a file's last-updated commit date, run status, blame, stage, commit, branch and push, all in-process."

export const head = defineHead<Props>(() => ({
  title: 'Git for Node, at native speed',
  link: [{ rel: 'canonical', href: 'https://simple-git.napi.rs/' }],
  meta: [
    { name: 'description', content: DESCRIPTION },
    { property: 'og:title', content: 'Git for Node, at native speed' },
    { property: 'og:description', content: DESCRIPTION },
    { property: 'og:url', content: 'https://simple-git.napi.rs/' },
  ],
}))
