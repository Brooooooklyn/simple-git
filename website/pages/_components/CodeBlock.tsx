import CopyButton from './_CopyButton'

const cx = (...c: (string | false | undefined)[]) => c.filter(Boolean).join(' ')

// Presentational (server-rendered) code block. `html` is Shiki output produced in
// index.server.ts and injected via dangerouslySetInnerHTML — so highlighted code is
// in the SSR HTML and fully visible with JS off. Only the optional copy button (an
// island) needs the client.
export default function CodeBlock({
  html,
  copyText,
  filename,
  className,
}: {
  html: string
  copyText?: string
  filename?: string
  className?: string
}) {
  return (
    <div
      className={cx(
        'overflow-hidden rounded-xl border border-(--color-border) bg-(--color-surface-1)',
        className,
      )}
    >
      {filename || copyText ? (
        <div className="flex items-center justify-between border-b border-(--color-border) px-4 py-2">
          {filename ? <span className="font-mono text-xs text-(--color-faint)">{filename}</span> : <span />}
          {copyText ? <CopyButton text={copyText} /> : null}
        </div>
      ) : null}
      <div
        className="overflow-x-auto p-4 text-sm [&_pre]:!m-0 [&_pre]:!bg-transparent"
        dangerouslySetInnerHTML={{ __html: html }}
      />
    </div>
  )
}
