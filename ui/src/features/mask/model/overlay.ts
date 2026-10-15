import type { Rect } from './brush'

const GROUND_TINT = [196, 78, 92] as const
const MAX_TINT_ALPHA = 150
const FULL = 255

/** Paints the ground of a mask as a tint over the picture: clear where it is sky. */
export function tintGround(mask: Uint8Array, width: number, pixels: Uint8ClampedArray, rect: Rect): void {
  for (let row = rect.top; row < rect.bottom; row++) {
    for (let column = rect.left; column < rect.right; column++) {
      const index = row * width + column
      const ground = (FULL - mask[index]!) / FULL
      const at = index * 4
      pixels[at] = GROUND_TINT[0]
      pixels[at + 1] = GROUND_TINT[1]
      pixels[at + 2] = GROUND_TINT[2]
      pixels[at + 3] = Math.round(ground * MAX_TINT_ALPHA)
    }
  }
}
