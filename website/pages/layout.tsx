import type { ReactNode } from 'react'
import '../app.css'
import Footer from './_components/Footer'

// Single source of truth for the header nav — rendered both inline (desktop) and
// inside the mobile drawer.
const NAV: { href: string; label: string; external?: boolean }[] = [
  { href: '/docs', label: 'Docs' },
  { href: 'https://github.com/Brooooooklyn/simple-git', label: 'GitHub', external: true },
  { href: 'https://npmx.dev/package/@napi-rs/simple-git', label: 'npm', external: true },
]

// schema.org JSON-LD structured data, server-rendered on EVERY route: this is the
// root layout that wraps both `/` and `/docs`, so the blocks emit on both and the
// library becomes eligible for rich results. Kept as plain objects and serialized
// with JSON.stringify below — which correctly escapes each value for embedding in a
// <script type="application/ld+json"> (do NOT hand-build the JSON string). No client
// JS: JSON-LD is inert markup.
const SITE_URL = 'https://simple-git.napi.rs'

// Mirrors the landing page's meta description (pages/index.server.ts). Keep in sync.
const DESCRIPTION =
  "Native Git for Node.js via libgit2 — no git shell-out. Read a file's last-updated commit date, run status, blame, stage, commit, branch and push, all in-process."

const SOFTWARE_APPLICATION_LD = {
  '@context': 'https://schema.org',
  '@type': 'SoftwareApplication',
  name: '@napi-rs/simple-git',
  description: DESCRIPTION,
  applicationCategory: 'DeveloperApplication',
  // Distilled from the 15 napi build targets in the root package.json.
  operatingSystem: 'Windows, macOS, Linux, Android, FreeBSD',
  programmingLanguage: ['JavaScript', 'TypeScript', 'Rust'],
  // Hand-maintained: bump to match the published package version in the root package.json.
  softwareVersion: '1.0.0',
  license: 'https://opensource.org/licenses/MIT',
  codeRepository: 'https://github.com/Brooooooklyn/simple-git',
  downloadUrl: 'https://npmx.dev/package/@napi-rs/simple-git',
  author: { '@type': 'Person', name: 'LongYinan', url: 'https://github.com/Brooooooklyn' },
  offers: { '@type': 'Offer', price: '0', priceCurrency: 'USD' },
  url: SITE_URL,
}

const WEBSITE_LD = {
  '@context': 'https://schema.org',
  '@type': 'WebSite',
  name: '@napi-rs/simple-git',
  url: SITE_URL,
}

export default function Layout({ children }: { children: ReactNode }) {
  return (
    <>
      {/* Skip link (WCAG A): first focusable element; sr-only until focused, then it
          surfaces top-left so keyboard users can jump straight to <main>. */}
      <a
        href="#main-content"
        className="sr-only focus:not-sr-only focus:absolute focus:left-3 focus:top-3 focus:z-[100] focus:rounded-lg focus:bg-(--color-accent) focus:px-4 focus:py-2 focus:text-(--color-accent-fg)"
      >
        Skip to content
      </a>
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
      {/* Server-rendered JSON-LD structured data (SoftwareApplication + WebSite). */}
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(SOFTWARE_APPLICATION_LD) }}
      />
      <script type="application/ld+json" dangerouslySetInnerHTML={{ __html: JSON.stringify(WEBSITE_LD) }} />
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
            <nav aria-label="Primary" className="hidden items-center gap-6 text-sm text-(--color-muted) md:flex">
              {NAV.map((item) => (
                <a
                  key={item.href}
                  href={item.href}
                  className="transition-colors hover:text-(--color-fg)"
                  {...(item.external ? { target: '_blank', rel: 'noreferrer' } : {})}
                >
                  {item.label}
                  {item.external && <span className="sr-only"> (opens in a new tab)</span>}
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
          <nav
            id="nav-menu"
            aria-label="Primary"
            className="nav-drawer hidden border-t border-(--color-border) bg-(--color-bg) md:!hidden"
          >
            <div className="container-page flex flex-col py-2">
              {NAV.map((item) => (
                <a
                  key={item.href}
                  href={item.href}
                  className="flex min-h-11 items-center text-(--color-muted) transition-colors hover:text-(--color-fg)"
                  {...(item.external ? { target: '_blank', rel: 'noreferrer' } : {})}
                >
                  {item.label}
                  {item.external && <span className="sr-only"> (opens in a new tab)</span>}
                </a>
              ))}
            </div>
          </nav>
        </header>
        <main id="main-content">{children}</main>
        <Footer />
      </div>
    </>
  )
}
