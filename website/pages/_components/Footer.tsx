const links: { label: string; href: string; external?: boolean }[] = [
  { label: 'Docs', href: '/docs' },
  { label: 'GitHub', href: 'https://github.com/Brooooooklyn/simple-git', external: true },
  { label: 'npm', href: 'https://www.npmjs.com/package/@napi-rs/simple-git', external: true },
  { label: 'napi.rs', href: 'https://napi.rs', external: true },
]

export default function Footer() {
  return (
    <footer className="border-t border-(--color-border)">
      <div className="container-page py-12">
        <div className="flex flex-col gap-8 md:flex-row md:items-start md:justify-between">
          <div className="flex flex-col gap-3">
            <span className="font-mono text-sm font-medium tracking-tight text-(--color-fg)">@napi-rs/simple-git</span>
            <p className="max-w-xs text-sm text-(--color-muted)">
              Native Git for Node.js — libgit2 bindings, built with napi-rs.
            </p>
          </div>

          <nav className="flex flex-wrap gap-x-8 gap-y-2">
            {links.map((l) => (
              <a
                key={l.label}
                href={l.href}
                className="text-sm text-(--color-muted) transition-colors hover:text-(--color-fg)"
                {...(l.external ? { target: '_blank', rel: 'noreferrer' } : {})}
              >
                {l.label}
              </a>
            ))}
          </nav>
        </div>

        <div className="mt-10 border-t border-(--color-border) pt-6">
          <p className="font-mono text-xs text-(--color-faint) tabular-nums">Built with napi-rs · MIT licensed</p>
        </div>
      </div>
    </footer>
  )
}
