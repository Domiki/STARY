export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  const units = ['KB', 'MB', 'GB', 'TB']
  const exponent = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length)
  return `${(bytes / 1024 ** exponent).toFixed(exponent >= 3 ? 2 : 1)} ${units[exponent - 1]}`
}

export function formatExposure(seconds: number): string {
  if (seconds >= 1) return `${Number(seconds.toFixed(2))}s`
  return `1/${Math.round(1 / seconds)}s`
}

export const formatDimensions = (width: number, height: number) => `${width} × ${height}`

export const fileNameOf = (path: string) => path.split(/[\\/]/).pop() ?? path

export const formatCapturedAt = (iso: string) => iso.replace('T', ' ')
