export interface Brush {
  /** Pixels of the picture from the centre to the edge. */
  radius: number
  /** 0 to 1: how much of the radius is fully painted; beyond it the paint fades out. */
  hardness: number
}

export interface Rect {
  left: number
  top: number
  right: number
  bottom: number
}

const smoothStep = (t: number) => {
  const clamped = Math.min(1, Math.max(0, t))
  return clamped * clamped * (3 - 2 * clamped)
}

/** How much a dab paints at a distance from its centre: 1 inside the hard core, fading to 0 at the edge. */
export function coverageAt(distance: number, { radius, hardness }: Brush): number {
  if (distance >= radius) return 0
  const core = radius * Math.min(1, Math.max(0, hardness))
  if (distance <= core) return 1
  return smoothStep((radius - distance) / (radius - core))
}

/** The pixels a dab can touch, clipped to the picture; null when it is off the picture. */
export function dabBounds(x: number, y: number, radius: number, width: number, height: number): Rect | null {
  const rect = {
    left: Math.max(0, Math.floor(x - radius)),
    top: Math.max(0, Math.floor(y - radius)),
    right: Math.min(width, Math.ceil(x + radius)),
    bottom: Math.min(height, Math.ceil(y + radius)),
  }
  return rect.right > rect.left && rect.bottom > rect.top ? rect : null
}

export const unionOf = (a: Rect | null, b: Rect | null): Rect | null =>
  a && b
    ? {
        left: Math.min(a.left, b.left),
        top: Math.min(a.top, b.top),
        right: Math.max(a.right, b.right),
        bottom: Math.max(a.bottom, b.bottom),
      }
    : (a ?? b)

/**
 * Adds a dab to the coverage of a stroke: where dabs overlap the larger coverage stays, so a slow
 * stroke does not get darker than a fast one. Returns the pixels that were looked at.
 */
export function addDab(
  coverage: Float32Array,
  width: number,
  height: number,
  x: number,
  y: number,
  brush: Brush,
): Rect | null {
  const rect = dabBounds(x, y, brush.radius, width, height)
  if (!rect) return null
  for (let row = rect.top; row < rect.bottom; row++) {
    for (let column = rect.left; column < rect.right; column++) {
      const value = coverageAt(Math.hypot(column + 0.5 - x, row + 0.5 - y), brush)
      const index = row * width + column
      if (value > coverage[index]!) coverage[index] = value
    }
  }
  return rect
}

const SPACING_PER_RADIUS = 0.15

/** Points along the way from one position to the next, close enough for the dabs to run together. */
export function pointsBetween(
  from: [number, number],
  to: [number, number],
  radius: number,
): [number, number][] {
  const distance = Math.hypot(to[0] - from[0], to[1] - from[1])
  const steps = Math.max(1, Math.ceil(distance / Math.max(1, radius * SPACING_PER_RADIUS)))
  return Array.from({ length: steps }, (_, step) => {
    const t = (step + 1) / steps
    return [from[0] + (to[0] - from[0]) * t, from[1] + (to[1] - from[1]) * t]
  })
}
