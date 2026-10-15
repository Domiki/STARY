import { allItems, type Workspace } from '../file-list/model/workspace'

export type OutputKind = 'master' | 'calibrated' | 'result'

const OUTPUT_ROOT = 'stary-output'
const SUBFOLDER: Record<OutputKind, string | null> = {
  master: 'masters',
  calibrated: 'calibrated',
  result: null,
}

const separatorOf = (path: string) => (path.includes('\\') && !path.includes('/') ? '\\' : '/')

export const parentFolder = (path: string): string => {
  const index = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'))
  return index > 0 ? path.slice(0, index) : path
}

export function defaultOutputFolder(workspace: Workspace, kind: OutputKind): string | null {
  const first = allItems(workspace)[0]
  if (!first) return null
  const separator = separatorOf(first.path)
  return [parentFolder(first.path), OUTPUT_ROOT, SUBFOLDER[kind]]
    .filter((part): part is string => part !== null)
    .join(separator)
}
