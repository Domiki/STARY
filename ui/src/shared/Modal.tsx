import { type ReactNode, useEffect } from 'react'

import { IconButton } from './IconButton'
import { CloseIcon } from './icons'

interface Props {
  title: string
  closeLabel: string
  onClose: () => void
  /** Tailwind width of the window. */
  width?: string
  children: ReactNode
}

/** A window over the program: closes with Escape, with a click outside or with its button. */
export function Modal({ title, closeLabel, onClose, width = 'w-[30rem]', children }: Props) {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => event.key === 'Escape' && onClose()
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [onClose])

  return (
    <div className="fixed inset-0 z-40 flex items-center justify-center bg-black/55" onClick={onClose}>
      <div
        role="dialog"
        aria-modal
        aria-label={title}
        className={`max-h-[88vh] ${width} max-w-[94vw] space-y-4 overflow-auto rounded-lg border border-twilight bg-midnight p-5 shadow-xl`}
        onClick={(event) => event.stopPropagation()}
      >
        <div className="flex items-center justify-between">
          <h2 className="text-base">{title}</h2>
          <IconButton label={closeLabel} onClick={onClose}>
            <CloseIcon />
          </IconButton>
        </div>
        {children}
      </div>
    </div>
  )
}
