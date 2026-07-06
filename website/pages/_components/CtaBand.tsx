import SectionHeader from './SectionHeader'
import Button from './Button'
import InstallSwitcher from './_InstallSwitcher'
import Reveal from './_Reveal'

// GET STARTED / call-to-action band. A centered install switcher (SSR renders the npm
// command, so a working install line shows with JS off — hydration only adds the
// package-manager tabs + copy) plus the docs / GitHub buttons (plain <a>, crawlable).
export default function CtaBand() {
  return (
    <section className="relative overflow-hidden border-t border-(--color-border)">
      <div className="accent-glow" />
      <div className="container-page py-24 md:py-32">
        <Reveal className="flex flex-col items-center">
          <SectionHeader label="GET STARTED" title="Add it in one line" align="center" />

          <div className="mt-10 flex w-full justify-center">
            <InstallSwitcher />
          </div>

          <div className="mt-8 flex flex-wrap items-center justify-center gap-3">
            <Button variant="primary" href="/docs">
              Read the docs
            </Button>
            <Button variant="secondary" href="https://github.com/Brooooooklyn/simple-git" target="_blank" rel="noreferrer">
              GitHub
            </Button>
          </div>
        </Reveal>
      </div>
    </section>
  )
}
