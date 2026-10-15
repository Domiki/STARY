import { useEffect } from 'react'

interface Props {
  title: string
  message: string
  note?: string
  confirmLabel: string
  cancelLabel: string
  tone?: 'danger' | 'neutral'
  onConfirm: () => void
  onCancel: () => void
}

export function ConfirmDialog({
  title,
  message,
  note,
  confirmLabel,
  cancelLabel,
  tone = 'danger',
  onConfirm,
  onCancel,
}: Props) {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => event.key === 'Escape' && onCancel()
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [onCancel])

  return (
    <div
      className="fixed inset-0 z-40 flex items-center justify-center bg-black/55"
      onClick={onCancel}
    >
      <div
        role="alertdialog"
        aria-modal
        aria-label={title}
        className="w-96 rounded-lg border border-twilight bg-midnight p-5 shadow-xl"
        onClick={(event) => event.stopPropagation()}
      >
        <h2 className="mb-2 text-base">{title}</h2>
        <p className={tone === 'danger' ? 'text-danger' : ''}>{message}</p>
        {note && <p className="mt-2 text-xs text-mist">{note}</p>}
        <div className="mt-5 flex justify-end gap-2">
          <button
            type="button"
            autoFocus
            onClick={onCancel}
            className="rounded border border-twilight px-3 py-1 hover:bg-twilight/50"
          >
            {cancelLabel}
          </button>
          <button
            type="button"
            onClick={onConfirm}
            className={`rounded border px-3 py-1 ${tone === 'danger' ? 'border-danger/60 bg-danger/15 text-danger hover:bg-danger/25' : 'border-ember/70 bg-ember/15 text-ember hover:bg-ember/25'}`}
          >
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  )
}
