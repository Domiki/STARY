export const CHART_WIDTH = 256
export const CHART_HEIGHT = 96

const inner = (bins: readonly number[]) => bins.slice(1, -1)

const scaled = (value: number, peak: number, log: boolean) => {
  if (peak <= 0) return 0
  const ratio = log ? Math.log1p(value) / Math.log1p(peak) : value / peak
  return Math.min(1, ratio)
}

export function histogramPeak(channels: readonly (readonly number[])[]): number {
  return Math.max(0, ...channels.flatMap((bins) => inner(bins)))
}

export function areaPath(bins: readonly number[], peak: number, log: boolean): string {
  const points = bins.map((count, index) => {
    const y = CHART_HEIGHT - scaled(count, peak, log) * CHART_HEIGHT
    return `${index},${y.toFixed(1)}`
  })
  return `M0,${CHART_HEIGHT} L${points.join(' L')} L${bins.length - 1},${CHART_HEIGHT} Z`
}
