import type { Props } from './index.server'
import Hero from './_components/Hero'
import Benchmark from './_components/Benchmark'
import DocSiteShowcase from './_components/DocSiteShowcase'
import Features from './_components/Features'
import CodeSample from './_components/CodeSample'
import PlatformMatrix from './_components/PlatformMatrix'
import CtaBand from './_components/CtaBand'

// Landing page. `index.server.ts` pre-highlights the code samples server-side and
// passes the HTML strings as props (typed via InferProps → Props). Sections render
// top-to-bottom inside the auto-applied layout (Footer comes from the root layout):
// Hero → Benchmark → DocSiteShowcase → Features → CodeSample → PlatformMatrix → CtaBand.
export default function Home({ heroHtml, docSiteHtml, statusHtml, commitHtml, blameHtml, pushHtml, errorsHtml }: Props) {
  return (
    <>
      <Hero codeHtml={heroHtml} />
      <Benchmark />
      <DocSiteShowcase codeHtml={docSiteHtml} />
      <Features />
      <CodeSample
        statusHtml={statusHtml}
        commitHtml={commitHtml}
        blameHtml={blameHtml}
        pushHtml={pushHtml}
        errorsHtml={errorsHtml}
      />
      <PlatformMatrix />
      <CtaBand />
    </>
  )
}
