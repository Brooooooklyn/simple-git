import { useState } from 'react'

// Interactive island (uses hooks). On a regular Void route the whole page hydrates,
// so this component becomes live automatically — no `with { island }` needed. The
// `_` filename prefix marks it as a client/interactive component (author convention,
// mirrors the reference toolkit).
export default function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false)
  return (
    <button
      type="button"
      onClick={() => {
        navigator.clipboard.writeText(text).then(() => {
          setCopied(true)
          setTimeout(() => setCopied(false), 1500)
        })
      }}
      className="rounded-md border border-(--color-border) px-2 py-1 font-mono text-xs text-(--color-muted) transition-colors hover:border-(--color-border-strong) hover:text-(--color-fg)"
      aria-label="Copy to clipboard"
    >
      {copied ? 'Copied' : 'Copy'}
    </button>
  )
}
