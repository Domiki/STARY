import { useCallback, useRef, useState } from 'react'

interface Options {
  storageKey: string
  initial: number
  min: number
  max: number
  edge: 'left' | 'right'
}

const readStored = (key: string): number | null => {
  try {
    const value = Number(localStorage.getItem(key))
    return Number.isFinite(value) && value > 0 ? value : null
  } catch {
    return null
  }
}

const store = (key: string, width: number) => {
  try {
    localStorage.setItem(key, String(width))
  } catch {
    // width is simply not remembered
  }
}

export function usePanelWidth({ storageKey, initial, min, max, edge }: Options) {
  const [width, setWidth] = useState(() => readStored(storageKey) ?? initial)
  const [collapsed, setCollapsed] = useState(false)
  const start = useRef<{ x: number; width: number } | null>(null)

  const onPointerDown = useCallback(
    (event: React.PointerEvent) => {
      event.currentTarget.setPointerCapture(event.pointerId)
      start.current = { x: event.clientX, width }
    },
    [width],
  )

  const onPointerMove = useCallback(
    (event: React.PointerEvent) => {
      if (!start.current) return
      const delta = (event.clientX - start.current.x) * (edge === 'left' ? 1 : -1)
      setWidth(Math.min(max, Math.max(min, start.current.width + delta)))
    },
    [edge, max, min],
  )

  const onPointerUp = useCallback(() => {
    start.current = null
    store(storageKey, width)
  }, [storageKey, width])

  return {
    width: collapsed ? 0 : width,
    collapsed,
    toggleCollapsed: () => setCollapsed((current) => !current),
    handleProps: { onPointerDown, onPointerMove, onPointerUp },
  }
}
