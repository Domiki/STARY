import type { ProbeInfo, ProbeOutcome } from '../../../ipc/types'
import { type Placement, type Rejection, type RejectionReason, type Target, placeInfo } from './placement'
import type { Workspace } from './workspace'

export interface AppliedOutcomes {
  workspace: Workspace
  placed: string[]
  rejections: Rejection[]
}

const byPath = (a: ProbeInfo, b: ProbeInfo) =>
  a.path.localeCompare(b.path, undefined, { numeric: true })

const PROBE_ERROR_REASONS: RejectionReason[] = [
  'read_failed',
  'unsupported_file',
  'decode_failed',
  'internal',
]

const probeErrorReason = (code: string): RejectionReason =>
  PROBE_ERROR_REASONS.find((reason) => reason === code) ?? 'internal'

export function applyOutcomes(
  workspace: Workspace,
  outcomes: ProbeOutcome[],
  explicit?: Target,
): AppliedOutcomes {
  const infos = outcomes.flatMap((outcome) => (outcome.status === 'ok' ? [outcome.info] : []))
  const probeRejections: Rejection[] = outcomes.flatMap((outcome) =>
    outcome.status === 'error'
      ? [{ path: outcome.path, reason: probeErrorReason(outcome.code) }]
      : [],
  )
  const initial: AppliedOutcomes = { workspace, placed: [], rejections: probeRejections }
  return [...infos].sort(byPath).reduce<AppliedOutcomes>((state, info) => {
    const placement: Placement = placeInfo(state.workspace, info, explicit)
    return {
      workspace: placement.workspace,
      placed: placement.rejection ? state.placed : [...state.placed, info.path],
      rejections: placement.rejection ? [...state.rejections, placement.rejection] : state.rejections,
    }
  }, initial)
}
