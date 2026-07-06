import SectionHeader from './SectionHeader'
import TabbedCodeBlock, { type CodeTab } from './_TabbedCodeBlock'
import Reveal from './_Reveal'
import { statusSample, commitSample, blameSample, pushSample, errorsSample } from '../_data/samples'

// API section — a tabbed code block over the five headlined samples. The Shiki HTML
// for every tab is produced server-side in index.server.ts and threaded in as props;
// the raw source (for the copy button) comes from _data/samples.ts (byte-verbatim from
// README.md). Every panel is in the SSR HTML — with JS off the first tab (Status) is
// visible; hydration only swaps which panel shows.
export default function CodeSample({
  statusHtml,
  commitHtml,
  blameHtml,
  pushHtml,
  errorsHtml,
}: {
  statusHtml: string
  commitHtml: string
  blameHtml: string
  pushHtml: string
  errorsHtml: string
}) {
  const tabs: CodeTab[] = [
    { id: 'status', label: 'Status', html: statusHtml, copyText: statusSample, filename: 'status.ts' },
    { id: 'commit', label: 'Stage & commit', html: commitHtml, copyText: commitSample, filename: 'commit.ts' },
    { id: 'blame', label: 'Blame', html: blameHtml, copyText: blameSample, filename: 'blame.ts' },
    { id: 'push', label: 'Push', html: pushHtml, copyText: pushSample, filename: 'push.ts' },
    { id: 'errors', label: 'Typed errors', html: errorsHtml, copyText: errorsSample, filename: 'errors.ts' },
  ]

  return (
    <section className="border-t border-(--color-border)">
      <div className="container-page py-20 md:py-28">
        <SectionHeader index="04" label="API" title="Show me the code" />

        <Reveal className="mt-12">
          <TabbedCodeBlock tabs={tabs} className="mx-auto max-w-3xl" />
        </Reveal>
      </div>
    </section>
  )
}
