import type { ProbeInfo } from '../../../ipc/types'

export const info = (overrides: Partial<ProbeInfo> & { path: string }): ProbeInfo => ({
  fingerprint: { size_bytes: 1000, modified_unix_ms: 0 },
  format: 'fits',
  width: 100,
  height: 80,
  layout: { kind: 'mono' },
  bit_depth: 16,
  camera: null,
  exposure_s: 30,
  iso: 100,
  captured_at: null,
  stary_kind: null,
  stack_count: null,
  calibrated: false,
  ...overrides,
})
