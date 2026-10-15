import { useCallback, useEffect, useRef, useState } from 'react'

import { FIT, type ZoomPan, fitScale, zoomAround } from '../zoom-pan'

export function useViewport(imageWidth: number, imageHeight: number) {
  const containerRef = useRef<HTMLDivElement>(null)
  const [size, setSize] = useState({ width: 0, height: 0 })
  const [state, setState] = useState<ZoomPan>(FIT)
  const drag = useRef<{ x: number; y: number } | null>(null)

  useEffect(() => {
    const element = containerRef.current
    if (!element) return
    const observer = new ResizeObserver(([entry]) => {
      if (entry) setSize({ width: entry.contentRect.width, height: entry.contentRect.height })
    })
    observer.observe(element)
    return () => observer.disconnect()
  }, [])

  useEffect(() => setState(FIT), [imageWidth, imageHeight])

  const baseScale = fitScale(size, { width: imageWidth, height: imageHeight })

  const zoomBy = useCallback(
    (factor: number, pointerX = 0, pointerY = 0) =>
      setState((current) => zoomAround(current, current.zoom * factor, baseScale, pointerX, pointerY)),
    [baseScale],
  )

  const onWheel = (event: React.WheelEvent) => {
    const rect = event.currentTarget.getBoundingClientRect()
    zoomBy(
      Math.exp(-event.deltaY * 0.0015),
      event.clientX - rect.left - rect.width / 2,
      event.clientY - rect.top - rect.height / 2,
    )
  }

  const onPointerDown = (event: React.PointerEvent) => {
    event.currentTarget.setPointerCapture(event.pointerId)
    drag.current = { x: event.clientX, y: event.clientY }
  }

  const onPointerMove = (event: React.PointerEvent) => {
    const start = drag.current
    if (!start) return
    drag.current = { x: event.clientX, y: event.clientY }
    setState((current) => ({
      ...current,
      panX: current.panX + event.clientX - start.x,
      panY: current.panY + event.clientY - start.y,
    }))
  }

  return {
    containerRef,
    scale: baseScale * state.zoom,
    panX: state.panX,
    panY: state.panY,
    zoomIn: () => zoomBy(1.25),
    zoomOut: () => zoomBy(0.8),
    reset: () => setState(FIT),
    handlers: {
      onWheel,
      onPointerDown,
      onPointerMove,
      onPointerUp: () => (drag.current = null),
      onDoubleClick: () => setState(FIT),
    },
  }
}
