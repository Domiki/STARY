import { type PointerEvent as ReactPointerEvent, type RefObject, useEffect, useRef, useState } from 'react'

import type { SkyStroke } from '../../../ipc/types'
import { type Rect, addDab, pointsBetween, unionOf } from '../model/brush'
import { thinned, toPicturePoint } from '../model/geometry'
import { tintGround } from '../model/overlay'
import { paintInto } from '../model/paint'
import { maskOf, useMaskStore } from '../store'

interface Props {
  sessionId: string
  path: string
  width: number
  height: number
  viewport: { scale: number; panX: number; panY: number }
  containerRef: RefObject<HTMLDivElement | null>
}

interface Stroking {
  kind: 'sky' | 'ground' | 'refine'
  base: Uint8Array | null
  work: Uint8Array | null
  coverage: Float32Array | null
  dirty: Rect | null
  points: [number, number][]
}

const GUIDE_COLORS = { sky: '#7fb2ff', ground: '#e0707c', refine: '#e8d070' } as const
const GUIDE_OPACITY = 0.35

const pathOf = (contours: [number, number][][]) =>
  contours.map((line) => `M${line.map(([x, y]) => `${x} ${y}`).join('L')}`).join('')

export function MaskOverlay({ sessionId, path, width, height, viewport, containerRef }: Props) {
  const entries = useMaskStore((store) => store.entries)
  const { tool, brushRadius, hardness } = useMaskStore()
  const state = maskOf(entries, sessionId)
  const shown = state.phase === 'applied' ? state.mask : state.phase === 'contour' ? state.shaped : null
  const canvas = useRef<HTMLCanvasElement>(null)
  const pixels = useRef<ImageData | null>(null)
  const stroking = useRef<Stroking | null>(null)
  const [cursor, setCursor] = useState<{ x: number; y: number } | null>(null)
  const [live, setLive] = useState<SkyStroke | null>(null)

  useEffect(() => {
    const context = canvas.current?.getContext('2d')
    if (!context) return
    if (!shown || shown.length !== width * height) {
      context.clearRect(0, 0, width, height)
      pixels.current = null
      return
    }
    const image = new ImageData(width, height)
    tintGround(shown, width, image.data, { left: 0, top: 0, right: width, bottom: height })
    context.putImageData(image, 0, 0)
    pixels.current = image
  }, [shown, width, height])

  const painting = tool !== 'hand' && state.phase !== 'none'
  const pointOf = (event: ReactPointerEvent) => {
    const box = containerRef.current?.getBoundingClientRect()
    if (!box) return null
    return toPicturePoint(event.clientX, event.clientY, box, viewport, { width, height })
  }

  const paintDab = (point: [number, number], run: Stroking) => {
    if (!run.base || !run.work || !run.coverage) return
    const rect = addDab(run.coverage, width, height, point[0], point[1], {
      radius: brushRadius,
      hardness,
    })
    run.dirty = unionOf(run.dirty, rect)
    if (!run.dirty) return
    paintInto(run.base, run.coverage, run.kind as 'sky' | 'ground', run.dirty, width, run.work)
    const image = pixels.current
    const context = canvas.current?.getContext('2d')
    if (!image || !context) return
    tintGround(run.work, width, image.data, run.dirty)
    const { left, top, right, bottom } = run.dirty
    context.putImageData(image, 0, 0, left, top, right - left, bottom - top)
  }

  const onPointerDown = (event: ReactPointerEvent) => {
    event.stopPropagation()
    const point = pointOf(event)
    if (!point || tool === 'hand') return
    event.currentTarget.setPointerCapture?.(event.pointerId)
    const run: Stroking = {
      kind: tool,
      base: null,
      work: null,
      coverage: null,
      dirty: null,
      points: [point],
    }
    if (state.phase === 'applied' && state.mask && tool !== 'refine') {
      run.base = state.mask
      run.work = state.mask.slice()
      run.coverage = new Float32Array(width * height)
      paintDab(point, run)
    } else {
      setLive({ paint: tool, radius: brushRadius, points: [point] })
    }
    stroking.current = run
  }

  const onPointerMove = (event: ReactPointerEvent) => {
    event.stopPropagation()
    const box = containerRef.current?.getBoundingClientRect()
    if (box) setCursor({ x: event.clientX - box.left, y: event.clientY - box.top })
    const run = stroking.current
    const point = pointOf(event)
    if (!run || !point) return
    const last = run.points[run.points.length - 1] ?? point
    for (const step of pointsBetween(last, point, brushRadius)) {
      run.points.push(step)
      paintDab(step, run)
    }
    if (!run.work) setLive({ paint: run.kind, radius: brushRadius, points: [...run.points] })
  }

  const onPointerUp = (event: ReactPointerEvent) => {
    event.stopPropagation()
    const run = stroking.current
    stroking.current = null
    if (!run) return
    const store = useMaskStore.getState()
    if (run.work) {
      store.commitPaint(sessionId, run.work)
      return
    }
    setLive(null)
    store.guide(
      { sessionId, path },
      { paint: run.kind, radius: brushRadius, points: thinned(run.points, Math.max(1, brushRadius * 0.3)) },
    )
  }

  const transform = `translate(${viewport.panX}px, ${viewport.panY}px) scale(${viewport.scale})`
  const guides = [...(state.phase === 'contour' ? state.strokes : []), ...(live ? [live] : [])]
  return (
    <>
      <div
        className="pointer-events-none absolute top-1/2 left-1/2"
        style={{ width, height, marginLeft: -width / 2, marginTop: -height / 2, transform }}
      >
        <canvas ref={canvas} width={width} height={height} className="absolute inset-0" />
        <svg viewBox={`0 0 ${width} ${height}`} width={width} height={height} className="absolute inset-0">
          {guides.map((stroke, at) => (
            <polyline
              key={at}
              points={stroke.points.map(([x, y]) => `${x},${y}`).join(' ')}
              fill="none"
              stroke={GUIDE_COLORS[stroke.paint]}
              strokeOpacity={GUIDE_OPACITY}
              strokeWidth={stroke.radius * 2}
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          ))}
          {state.phase === 'contour' && (
            <path
              d={pathOf(state.contours)}
              fill="none"
              className="stroke-moon"
              strokeWidth={1.5}
              vectorEffect="non-scaling-stroke"
            />
          )}
        </svg>
      </div>
      {painting && (
        <div
          data-testid="mask-brush-layer"
          className="absolute inset-0 cursor-none"
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={onPointerUp}
          onPointerLeave={() => setCursor(null)}
          onDoubleClick={(event) => event.stopPropagation()}
        >
          {cursor && (
            <span
              className="pointer-events-none absolute rounded-full border border-ember"
              style={{
                left: cursor.x - brushRadius * viewport.scale,
                top: cursor.y - brushRadius * viewport.scale,
                width: brushRadius * 2 * viewport.scale,
                height: brushRadius * 2 * viewport.scale,
              }}
            />
          )}
        </div>
      )}
    </>
  )
}
