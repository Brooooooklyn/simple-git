import type { Props } from './index.server'
import Hero from './_components/Hero'
import Benchmark from './_components/Benchmark'
import DocSiteShowcase from './_components/DocSiteShowcase'

// Landing page. `index.server.ts` pre-highlights the code samples server-side and
// passes the HTML strings as props (typed via InferProps → Props). Sections render
// top-to-bottom inside the auto-applied layout; the remaining sections land in Task 5.
export default function Home({ heroHtml, docSiteHtml }: Props) {
  return (
    <>
      <Hero codeHtml={heroHtml} />
      <Benchmark />
      <DocSiteShowcase codeHtml={docSiteHtml} />
    </>
  )
}
