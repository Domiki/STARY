import { type ReactNode, useState } from 'react'

import { ChevronIcon } from './icons'

interface Props {
  title: string
  defaultOpen?: boolean
  actions?: ReactNode
  children: ReactNode
}

export function CollapsibleSection({ title, defaultOpen = true, actions, children }: Props) {
  const [open, setOpen] = useState(defaultOpen)
  return (
    <section className="border-b border-twilight/60">
      <div className="flex items-center justify-between pr-2">
        <button
          type="button"
          aria-expanded={open}
          onClick={() => setOpen(!open)}
          className="flex flex-1 items-center gap-1 px-3 py-1.5 text-left text-xs tracking-wide text-mist hover:text-moon"
        >
          <ChevronIcon open={open} />
          {title}
        </button>
        {open && actions}
      </div>
      {open && children}
    </section>
  )
}
