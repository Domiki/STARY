import { getVersion } from '@tauri-apps/api/app'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { open, save } from '@tauri-apps/plugin-dialog'
import { openUrl } from '@tauri-apps/plugin-opener'

import { type Point, detectPlatform, dropPointToCss } from './drop-point'
import { parseImagePacket, parsePacket } from './image-packet'
import { ALL_EXTENSIONS } from './supported-extensions'
import type {
  GpuInfo,
  ImageView,
  JobFinished,
  JobProgress,
  JobRequest,
  LaunchContext,
  NewerRelease,
  ProbeOutcome,
  SaveAsRequest,
  SkyDetection,
  SkyStroke,
} from './types'

export const MAX_PREVIEW_EDGE = 1600

export const expandPaths = (paths: string[]) => invoke<string[]>('expand_paths', { paths })

export const probeFiles = (paths: string[]) => invoke<ProbeOutcome[]>('probe_files', { paths })

export const launchContext = () => invoke<LaunchContext>('launch_context')

export interface MenuLabels {
  help: string
  about: string
  report_issue: string
  documentation: string
}

/** Puts the words of the menu bar of the window in the language of the screen. */
export const setMenuText = (labels: MenuLabels) => invoke<void>('set_menu_text', { labels })

export type MenuAction = 'about' | 'report_issue' | 'documentation'

/** Calls `handler` when an entry of the menu bar of the window is chosen. */
export const listenForMenu = (handler: (action: MenuAction) => void) =>
  listen<MenuAction>('menu-action', (event) => handler(event.payload))

export const appVersion = () => getVersion()

/** Opens an address in the browser of the system. */
export const openExternal = (url: string) => openUrl(url)

/** The newest release on GitHub that is newer than `current`; `null` when there is none. */
export const checkUpdate = (current: string) =>
  invoke<NewerRelease | null>('check_update', { current })

export const gpuInfo = () => invoke<GpuInfo | null>('gpu_info')

export async function openImage(path: string): Promise<ImageView> {
  const buffer = await invoke<ArrayBuffer>('open_image', { path, maxEdge: MAX_PREVIEW_EDGE })
  return parseImagePacket(buffer)
}

export async function pickFiles(title: string): Promise<string[]> {
  const picked = await open({
    multiple: true,
    title,
    filters: [{ name: 'Images', extensions: ALL_EXTENSIONS }],
  })
  return picked ?? []
}

export async function pickFolder(title: string): Promise<string | null> {
  const picked = await open({ directory: true, multiple: false, title })
  return typeof picked === 'string' ? picked : null
}

export interface DropHandlers {
  onMove: (point: Point) => void
  onLeave: () => void
  onDrop: (paths: string[], point: Point) => void
}

let nativeInnerHeight = 0

// On macOS the native "inner" height still includes the title bar area that sits above the
// page, while the page viewport does not; their difference is what drop positions are offset by.
const currentTitlebarHeight = () => Math.max(0, nativeInnerHeight - window.innerHeight)

async function refreshNativeHeight() {
  const window_ = getCurrentWindow()
  const [inner, scale] = await Promise.all([window_.innerSize(), window_.scaleFactor()])
  nativeInnerHeight = inner.height / scale
}

const toCssPixels = (point: Point): Point =>
  dropPointToCss(point, detectPlatform(), window.devicePixelRatio, currentTitlebarHeight())

export async function listenForDrops({
  onMove,
  onLeave,
  onDrop,
}: DropHandlers): Promise<() => void> {
  await refreshNativeHeight().catch((error) => console.error('window size lookup failed', error))
  const stopResize = await getCurrentWindow().onResized(() => void refreshNativeHeight().catch(() => {}))
  const stopDrops = await getCurrentWebview().onDragDropEvent(({ payload }) => {
    if (payload.type === 'leave') onLeave()
    else if (payload.type === 'drop') onDrop(payload.paths, toCssPixels(payload.position))
    else onMove(toCssPixels(payload.position))
  })
  return () => {
    stopResize()
    stopDrops()
  }
}

export const saveResultAs = (request: SaveAsRequest) =>
  invoke<void>('save_result_as', { request })

/** Asks where to save: the folder and the name. `null` when the user gave up. */
export async function pickSavePath(
  title: string,
  defaultPath: string,
  extension: string,
): Promise<string | null> {
  return save({ title, defaultPath, filters: [{ name: extension.toUpperCase(), extensions: [extension] }] })
}

export const quitApp = () => invoke<void>('quit_app')

export const listenForCloseRequest = (onRequest: () => void) =>
  listen('close-requested', onRequest)

export const startJob = (request: JobRequest) => invoke<void>('start_job', { request })

export const cancelJob = () => invoke<void>('cancel_job')

export interface JobHandlers {
  onProgress: (progress: JobProgress) => void
  onFinished: (finished: JobFinished) => void
}

export async function listenForJobs({ onProgress, onFinished }: JobHandlers): Promise<() => void> {
  const stopProgress = await listen<JobProgress>('job-progress', (event) => onProgress(event.payload))
  const stopFinished = await listen<JobFinished>('job-finished', (event) => onFinished(event.payload))
  return () => {
    stopProgress()
    stopFinished()
  }
}

interface SkyHeader {
  width: number
  height: number
  sky_fraction: number
  contours: SkyDetection['contours']
}

export async function detectSky(path: string, strokes: SkyStroke[]): Promise<SkyDetection> {
  const buffer = await invoke<ArrayBuffer>('detect_sky', { path, maxEdge: MAX_PREVIEW_EDGE, strokes })
  const { header, body } = parsePacket<SkyHeader>(buffer)
  return {
    width: header.width,
    height: header.height,
    contours: header.contours,
    skyFraction: header.sky_fraction,
    region: body,
  }
}

export interface Mask {
  width: number
  height: number
  /** 255 for sky, 0 for ground, row by row. */
  bytes: Uint8Array
}

export async function shapeSky(
  region: Mask,
  offset: number,
  feather: number,
): Promise<Uint8Array> {
  const shaped = await invoke<ArrayBuffer>('shape_sky', region.bytes, {
    headers: {
      'x-width': String(region.width),
      'x-height': String(region.height),
      'x-offset': String(offset),
      'x-feather': String(feather),
    },
  })
  return new Uint8Array(shaped)
}

export const setMask = (session: string, mask: Mask) =>
  invoke<void>('set_mask', mask.bytes, {
    headers: {
      'x-session': session,
      'x-width': String(mask.width),
      'x-height': String(mask.height),
    },
  })

export const clearMask = (session: string) => invoke<void>('clear_mask', { session })

export const skyStarCount = (session: string, path: string) =>
  invoke<number>('sky_star_count', { session, path, maxEdge: MAX_PREVIEW_EDGE })

export const isTauri = () => '__TAURI_INTERNALS__' in window
