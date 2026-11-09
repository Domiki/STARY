export const ROLES = ['light', 'dark', 'flat', 'dark_flat', 'bias'] as const
export type Role = (typeof ROLES)[number]

export type SourceFormat = 'raw' | 'fits' | 'jpeg' | 'png' | 'tiff'
export type BayerPattern = 'RGGB' | 'BGGR' | 'GRBG' | 'GBRG'
export type CfaPattern =
  | { kind: 'bayer'; value: BayerPattern }
  | { kind: 'x_trans'; value: number[][] }
export type PixelLayout =
  | { kind: 'cfa'; value: CfaPattern }
  | { kind: 'rgb' }
  | { kind: 'mono' }

export type StaryKind =
  | 'master_bias'
  | 'master_dark'
  | 'master_flat'
  | 'master_dark_flat'
  | 'stack_result'

export interface ProbeInfo {
  path: string
  fingerprint: { size_bytes: number; modified_unix_ms: number | null }
  format: SourceFormat
  width: number
  height: number
  layout: PixelLayout
  bit_depth: number | null
  camera: string | null
  exposure_s: number | null
  iso: number | null
  captured_at: string | null
  stary_kind: StaryKind | null
  stack_count: number | null
  calibrated: boolean
}

export type ProbeOutcome =
  | { status: 'ok'; info: ProbeInfo }
  | { status: 'error'; path: string; code: string; message: string }

export interface Histogram {
  red: number[]
  green: number[]
  blue: number[]
  clipped_low: number
  clipped_high: number
}

export interface ImageView {
  width: number
  height: number
  histogram: Histogram
  url: string
}

export interface LaunchGroup {
  role: string | null
  paths: string[]
}

export interface LaunchContext {
  groups: LaunchGroup[]
  language: string | null
}

export interface CommandError {
  code: string
  message: string
}

export type JobKind = 'calibration' | 'trails' | 'stacking'
export type DemosaicQuality = 'fast' | 'high'
export type TrailsMethodChoice = 'lighten' | 'comet' | 'gap_filling'
export type ComputeDevice = 'cpu' | 'gpu'
/** How often the frames of a master are read: twice clips outliers exactly, once is faster on slow drives. */
export type ReadMode = 'exact' | 'fast'
export type GroundHandling = 'with_sky' | 'separate'

export type ExportContainer = 'tiff' | 'png' | 'jpeg' | 'fits'
export type ExportDepth = 'eight' | 'sixteen' | 'float'
export type ExportSpace = 'srgb' | 'display_p3' | 'adobe_rgb'
export type ExportTone = 'linear' | 'display' | 'stretch'

/** Which file to make from which saved result, and how. */
export interface SaveAsRequest {
  source: string
  target: string
  container: ExportContainer
  depth: ExportDepth
  /** `null` keeps the colors the result has. */
  space: ExportSpace | null
  tone: ExportTone
  /** For JPEG, 1 to 100. */
  quality: number
}

/** A stroke of a round brush over the preview of the representative image, in its pixels. */
export interface SkyStroke {
  paint: 'sky' | 'ground' | 'refine'
  radius: number
  points: [number, number][]
}

/** A line along the border between sky and ground, on the corners of the pixels. */
export type Contour = [number, number][]

export interface SkyDetection {
  width: number
  height: number
  /** 255 where the sky is, 0 for the ground, row by row. */
  region: Uint8Array
  contours: Contour[]
  skyFraction: number
}

/** The sky painted for a session, which the job reads from the backend. */
export interface MaskingRequest {
  session_id: string
  ground: GroundHandling
}

export interface GpuInfo {
  name: string
  backend: string
  /** The GPU works in the memory of the computer (Apple silicon, integrated GPUs). */
  shares_memory: boolean
}

/** What laid the frames of a finished job on the picture. */
export type UsedDevice = { kind: 'cpu' } | { kind: 'gpu'; name: string }

export interface TrailsRequest {
  method: TrailsMethodChoice
  /** For a comet: how many frames its tail takes to fade. */
  tail_frames: number
  /** For a comet: read the lights from the last to the first. */
  reverse_order: boolean
}

export type AlignModel = 'similarity' | 'affine' | 'homography'
export type Resampler = 'bilinear' | 'bicubic' | 'lanczos3'
export type CombineMethod = 'average' | 'sigma' | 'winsorized'

export interface StackingChoices {
  model: AlignModel
  resampler: Resampler
  combine: CombineMethod
  low: number
  high: number
  /** Leave out frames that are much worse than the rest (clouds, blur, trailing). */
  skip_poor_frames: boolean
}

export interface StackingRequest extends StackingChoices {
  /** Position of the reference frame in the list of lights: the session's representative image. */
  reference_index: number
}

export type JobSlot =
  | { kind: 'none' }
  | { kind: 'frames'; paths: string[] }
  | { kind: 'master'; path: string }

export interface JobRequest {
  kind: JobKind
  lights: string[]
  bias: JobSlot
  dark: JobSlot
  dark_flat: JobSlot
  flat: JobSlot
  /** The other sessions stacked with this one, which is the reference session. */
  more_sessions: SessionRequest[]
  apply_calibration: boolean
  masters_folder: string | null
  calibrated_folder: string | null
  result_folder: string | null
  quality: DemosaicQuality
  trails: TrailsRequest
  stacking: StackingRequest
  memory_percent: number
  device: ComputeDevice
  read: ReadMode
  hdr: boolean
  masking: MaskingRequest | null
}

export interface SessionRequest {
  lights: string[]
  bias: JobSlot
  dark: JobSlot
  dark_flat: JobSlot
  flat: JobSlot
}

export type JobStage =
  | 'bias'
  | 'dark_flat'
  | 'dark'
  | 'flat'
  | 'lights'
  | 'align'
  | 'stack'
  | 'demosaic'
  | 'saving'

export interface JobProgress {
  stage: JobStage
  done: number
  total: number
}

export interface JobOutput {
  kind: 'result' | 'master' | 'calibrated'
  path: string
}

export interface LeftOut {
  path: string
  reason: string
}

export interface FrameCounts {
  used: number
  total: number
  misaligned: number
  poor: number
  left_out: readonly LeftOut[]
}

export type JobFinished =
  | {
      status: 'done'
      outputs: JobOutput[]
      frames: FrameCounts | null
      device: UsedDevice | null
      /** Thousandths of the pixels of a master made in one read whose room for spoiled values was full. */
      one_read_full: number | null
      seconds: number
    }
  | { status: 'cancelled' }
  | { status: 'error'; failure: { code: string; message: string } }
