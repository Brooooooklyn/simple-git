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

// index.html must never be served from cache — keep it pure SSR (no prerender, plus
// revalidate:0 in void.json) so a deploy is live on the very next request, with no
// edge/ISR copy that could go stale. The Shiki highlighter is a module-level
// singleton (lib/highlight.ts, pure-JS regex engine — no runtime WASM), so
// per-request rendering stays cheap.
export const prerender = false

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
