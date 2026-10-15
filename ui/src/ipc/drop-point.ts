export interface Point {
  x: number
  y: number
}

export type Platform = 'windows' | 'macos' | 'linux'

// Tauri labels every drop position "physical", but the real unit depends on the platform:
// - Windows (WebView2): physical pixels inside the webview, so the pixel ratio applies.
// - macOS (WKWebView): logical points, but measured from the top of the window frame, so the
//   title bar height has to be subtracted to get viewport coordinates.
// - Linux (GTK): logical widget coordinates, used as they are.
export function dropPointToCss(
  point: Point,
  platform: Platform,
  pixelRatio: number,
  titlebarHeight: number,
): Point {
  switch (platform) {
    case 'windows':
      return { x: point.x / pixelRatio, y: point.y / pixelRatio }
    case 'macos':
      return { x: point.x, y: point.y - titlebarHeight }
    case 'linux':
      return point
  }
}

export function detectPlatform(userAgent: string = navigator.userAgent): Platform {
  if (userAgent.includes('Windows')) return 'windows'
  return userAgent.includes('Macintosh') ? 'macos' : 'linux'
}
