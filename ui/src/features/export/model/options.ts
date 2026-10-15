import type {
  ExportContainer,
  ExportDepth,
  ExportSpace,
  ExportTone,
  SaveAsRequest,
} from '../../../ipc/types'

export interface ExportOptions {
  container: ExportContainer
  depth: ExportDepth
  /** `null` keeps the colors the result has. */
  space: ExportSpace | null
  tone: ExportTone
  quality: number
}

export const CONTAINERS: ExportContainer[] = ['tiff', 'png', 'jpeg', 'fits']
export const SPACES: ExportSpace[] = ['srgb', 'display_p3', 'adobe_rgb']
export const TONES: ExportTone[] = ['linear', 'display', 'stretch']

export const DEFAULT_QUALITY = 92

const DEPTHS: Record<ExportContainer, ExportDepth[]> = {
  tiff: ['eight', 'sixteen', 'float'],
  png: ['eight', 'sixteen'],
  jpeg: ['eight'],
  fits: ['sixteen', 'float'],
}

const EXTENSIONS: Record<ExportContainer, string> = {
  tiff: 'tif',
  png: 'png',
  jpeg: 'jpg',
  fits: 'fits',
}

export const depthsOf = (container: ExportContainer): ExportDepth[] => DEPTHS[container]

export const extensionOf = (container: ExportContainer): string => EXTENSIONS[container]

/** FITS keeps the result as it is: its colors and its linear values. */
export const keepsColors = (container: ExportContainer): boolean => container === 'fits'

/** Floating point values are saved linear: there is nothing to encode them for. */
export const choosesTone = (options: Pick<ExportOptions, 'container' | 'depth'>): boolean =>
  options.depth !== 'float' && !keepsColors(options.container)

/** Whether the file holds less than the result: fewer bits or a lossy format. */
export const losesDetail = (options: Pick<ExportOptions, 'container' | 'depth'>): boolean =>
  options.depth !== 'float' || options.container === 'jpeg'

/** What the options must be for the file to be possible, changing as little as it can. */
export function settled(options: ExportOptions): ExportOptions {
  const depths = depthsOf(options.container)
  const depth = depths.includes(options.depth)
    ? options.depth
    : (depths.includes('sixteen') ? 'sixteen' : depths[0]!)
  const fixed = { ...options, depth }
  if (keepsColors(fixed.container)) return { ...fixed, space: null, tone: 'linear' }
  if (!choosesTone(fixed)) return { ...fixed, tone: 'linear' }
  return fixed
}

/** A result of a stack is a linear image of a deep sky: it needs stretching to look right. */
const STACK_RESULT = /_stack_\d+f\.[^.]+$/

export function defaultOptions(sourceName: string): ExportOptions {
  return {
    container: 'tiff',
    depth: 'sixteen',
    space: 'srgb',
    tone: STACK_RESULT.test(sourceName) ? 'stretch' : 'display',
    quality: DEFAULT_QUALITY,
  }
}

const SPACE_SHORT: Record<ExportSpace, string> = {
  srgb: 'srgb',
  display_p3: 'p3',
  adobe_rgb: 'adobe',
}

const DEPTH_SHORT: Record<ExportDepth, string> = { eight: '8', sixteen: '16', float: '32' }

/** The name suggested for a file made from `sourceName`, without the folder. */
export function suggestedName(sourceName: string, options: ExportOptions): string {
  const stem = sourceName.replace(/\.[^.]+$/, '')
  const colors = options.space ? SPACE_SHORT[options.space] : 'src'
  return `${stem}_${colors}${DEPTH_SHORT[options.depth]}.${extensionOf(options.container)}`
}

export const requestFor = (source: string, target: string, options: ExportOptions): SaveAsRequest => ({
  source,
  target,
  container: options.container,
  depth: options.depth,
  space: options.space,
  tone: options.tone,
  quality: options.quality,
})
