export interface ZoomPan {
  zoom: number
  panX: number
  panY: number
}

export const FIT: ZoomPan = { zoom: 1, panX: 0, panY: 0 }

const MIN_ZOOM = 0.5
const MAX_ZOOM = 32

export const clampZoom = (zoom: number) => Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, zoom))

export const fitScale = (
  container: { width: number; height: number },
  image: { width: number; height: number },
) =>
  image.width > 0 && image.height > 0
    ? Math.min(container.width / image.width, container.height / image.height)
    : 1

// Keeps the image point under (pointerX, pointerY) fixed while the zoom changes.
// Coordinates are relative to the viewport centre.
export function zoomAround(
  state: ZoomPan,
  nextZoom: number,
  baseScale: number,
  pointerX: number,
  pointerY: number,
): ZoomPan {
  const zoom = clampZoom(nextZoom)
  const ratio = (baseScale * zoom) / (baseScale * state.zoom)
  return {
    zoom,
    panX: pointerX - (pointerX - state.panX) * ratio,
    panY: pointerY - (pointerY - state.panY) * ratio,
  }
}
