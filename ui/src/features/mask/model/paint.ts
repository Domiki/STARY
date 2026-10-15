import type { Rect } from './brush'

export type PaintKind = 'sky' | 'ground'

const FULL = 255

/**
 * The mask with a stroke painted on it. Sky paint pushes a pixel towards 255 by the coverage and
 * ground paint pushes it towards 0, so a soft edge of the brush leaves in-between values.
 * `out` is written only inside `rect`.
 */
export function paintInto(
  base: Uint8Array,
  coverage: Float32Array,
  kind: PaintKind,
  rect: Rect,
  width: number,
  out: Uint8Array,
): void {
  for (let row = rect.top; row < rect.bottom; row++) {
    for (let column = rect.left; column < rect.right; column++) {
      const index = row * width + column
      const value = base[index]!
      const share = coverage[index]!
      out[index] =
        kind === 'sky'
          ? Math.round(value + (FULL - value) * share)
          : Math.round(value * (1 - share))
    }
  }
}

/** How much of the picture is mostly sky. */
export function skyShare(mask: Uint8Array): number {
  let sky = 0
  for (const value of mask) if (value >= FULL * 0.8) sky++
  return mask.length === 0 ? 0 : sky / mask.length
}

/** Whether any pixel of the mask is not fully sky. */
export const hasGround = (mask: Uint8Array): boolean => mask.some((value) => value < FULL)
