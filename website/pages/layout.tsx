import type { ReactNode } from 'react'
import '../app.css'
import Footer from './_components/Footer'

// Single source of truth for the header nav — rendered both inline (desktop) and
// inside the mobile drawer.
const NAV: { href: string; label: string; external?: boolean }[] = [
  { href: '/docs', label: 'Docs' },
  { href: 'https://github.com/Brooooooklyn/simple-git', label: 'GitHub', external: true },
  { href: 'https://www.npmjs.com/package/@napi-rs/simple-git', label: 'npm', external: true },
]

export default function Layout({ children }: { children: ReactNode }) {
  return (
    <>
      {/* Mark JS-capable clients before first paint so scroll-reveal hidden state
          (gated behind html.js in app.css) only applies when JS can reveal it —
          no-JS / crawler HTML stays fully visible. Also closes the (CSS-only)
          mobile nav drawer on link tap / Escape; the drawer itself needs no JS. */}
      <script
        dangerouslySetInnerHTML={{
          __html:
            "document.documentElement.classList.add('js');" +
            "document.addEventListener('click',function(e){var t=e.target.closest&&e.target.closest('#nav-menu a');if(t){var c=document.getElementById('nav-toggle');if(c)c.checked=false;}});" +
            "document.addEventListener('keydown',function(e){if(e.key==='Escape'){var c=document.getElementById('nav-toggle');if(c)c.checked=false;}});",
        }}
      />
      <div className="min-h-screen">
        <header className="site-header sticky top-0 z-50 border-b border-(--color-border)">
          {/* CSS-only drawer toggle: header-level so .nav-drawer / the bar's icons
              are later siblings the :checked rule (app.css) can reach. sr-only keeps
              it focusable on mobile; md:hidden drops it from the desktop tab order. */}
          <input id="nav-toggle" type="checkbox" className="sr-only md:hidden" aria-label="Toggle navigation menu" />
          <div className="container-page flex h-14 items-center justify-between gap-4">
            <a href="/" className="font-mono text-sm font-medium tracking-tight whitespace-nowrap text-(--color-fg)">
              @napi-rs/simple-git
            </a>
            <nav className="hidden items-center gap-6 text-sm text-(--color-muted) md:flex">
              {NAV.map((item) => (
                <a
                  key={item.href}
                  href={item.href}
                  className="transition-colors hover:text-(--color-fg)"
                  {...(item.external ? { target: '_blank', rel: 'noreferrer' } : {})}
                >
                  {item.label}
                </a>
              ))}
            </nav>
            <label
              htmlFor="nav-toggle"
              aria-controls="nav-menu"
              className="nav-toggle-btn -mr-2 inline-flex h-10 w-10 cursor-pointer items-center justify-center rounded-lg text-(--color-muted) transition-colors hover:text-(--color-fg) md:hidden"
            >
              <svg
                className="nav-icon-open h-5 w-5"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.8"
                aria-hidden="true"
              >
                <line x1="3" y1="6" x2="21" y2="6" />
                <line x1="3" y1="12" x2="21" y2="12" />
                <line x1="3" y1="18" x2="21" y2="18" />
              </svg>
              <svg
                className="nav-icon-close hidden h-5 w-5"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.8"
                aria-hidden="true"
              >
                <line x1="5" y1="5" x2="19" y2="19" />
                <line x1="19" y1="5" x2="5" y2="19" />
              </svg>
            </label>
          </div>
          {/* Drawer — hidden by default; the #nav-toggle:checked rule in app.css
              reveals it. id is targeted by the inline auto-close script above. */}
          <nav id="nav-menu" className="nav-drawer hidden border-t border-(--color-border) bg-(--color-bg) md:!hidden">
            <div className="container-page flex flex-col py-2">
              {NAV.map((item) => (
                <a
                  key={item.href}
                  href={item.href}
                  className="flex min-h-11 items-center text-(--color-muted) transition-colors hover:text-(--color-fg)"
                  {...(item.external ? { target: '_blank', rel: 'noreferrer' } : {})}
                >
                  {item.label}
                </a>
              ))}
            </div>
          </nav>
        </header>
        <main>{children}</main>
        <Footer />
      </div>
    </>
  )
}
