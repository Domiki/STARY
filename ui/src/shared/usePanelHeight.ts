import { useRef, useState } from 'react'

interface Options {
  initial: number
  min: number
  max: number
}

export function usePanelHeight({ initial, min, max }: Options) {
  const [height, setHeight] = useState(initial)
  const start = useRef<{ y: number; height: number } | null>(null)

  return {
    height,
    handleProps: {
      onPointerDown: (event: React.PointerEvent) => {
        event.currentTarget.setPointerCapture(event.pointerId)
        start.current = { y: event.clientY, height }
      },
      onPointerMove: (event: React.PointerEvent) => {
        if (!start.current) return
        const next = start.current.height - (event.clientY - start.current.y)
        setHeight(Math.min(max, Math.max(min, next)))
      },
      onPointerUp: () => {
        start.current = null
      },
    },
  }
}
