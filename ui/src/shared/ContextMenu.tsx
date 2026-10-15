import { useEffect } from 'react'

export interface MenuItem {
  label: string
  hint?: string
  disabled?: boolean
  danger?: boolean
  onSelect: () => void
}

interface Props {
  x: number
  y: number
  items: MenuItem[]
  onClose: () => void
}

const MENU_WIDTH = 260
const ROW_HEIGHT = 48

export function ContextMenu({ x, y, items, onClose }: Props) {
  useEffect(() => {
    const close = () => onClose()
    const onKeyDown = (event: KeyboardEvent) => event.key === 'Escape' && onClose()
    window.addEventListener('click', close)
    window.addEventListener('blur', close)
    window.addEventListener('keydown', onKeyDown)
    return () => {
      window.removeEventListener('click', close)
      window.removeEventListener('blur', close)
      window.removeEventListener('keydown', onKeyDown)
    }
  }, [onClose])

  const left = Math.min(x, window.innerWidth - MENU_WIDTH - 8)
  const top = Math.min(y, window.innerHeight - items.length * ROW_HEIGHT - 8)

  return (
    <ul
      role="menu"
      className="fixed z-50 rounded-md border border-twilight bg-midnight py-1 shadow-xl"
      style={{ left, top, width: MENU_WIDTH }}
      onContextMenu={(event) => event.preventDefault()}
    >
      {items.map((item) => (
        <li key={item.label} role="none">
          <button
            type="button"
            role="menuitem"
            disabled={item.disabled}
            onClick={(event) => {
              event.stopPropagation()
              item.onSelect()
            }}
            className={`block w-full px-3 py-1.5 text-left hover:bg-twilight/60 disabled:cursor-default disabled:opacity-45 disabled:hover:bg-transparent ${item.danger ? 'text-danger' : ''}`}
          >
            <span className="block">{item.label}</span>
            {item.hint && <span className="block text-xs text-mist">{item.hint}</span>}
          </button>
        </li>
      ))}
    </ul>
  )
}
