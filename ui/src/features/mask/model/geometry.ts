export interface ViewTransform {
  /** Screen pixels per pixel of the picture. */
  scale: number
  panX: number
  panY: number
}

/**
 * Where a position on the screen falls on the picture. `box` is the viewer's rectangle on the
 * screen; the picture is centred in it, then moved by the pan and scaled.
 */
export function toPicturePoint(
  clientX: number,
  clientY: number,
  box: { left: number; top: number; width: number; height: number },
  view: ViewTransform,
  picture: { width: number; height: number },
): [number, number] {
  const centreX = box.left + box.width / 2 + view.panX
  const centreY = box.top + box.height / 2 + view.panY
  return [
    (clientX - centreX) / view.scale + picture.width / 2,
    (clientY - centreY) / view.scale + picture.height / 2,
  ]
}

/** Drops the points that are closer than `gap` to the one kept before them (the last one stays). */
export function thinned(points: [number, number][], gap: number): [number, number][] {
  const kept: [number, number][] = []
  for (const point of points) {
    const last = kept[kept.length - 1]
    if (!last || Math.hypot(point[0] - last[0], point[1] - last[1]) >= gap) kept.push(point)
  }
  const final = points[points.length - 1]
  if (final && kept[kept.length - 1] !== final) kept.push(final)
  return kept
}
