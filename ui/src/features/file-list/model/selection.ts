import { ROLES } from '../../../ipc/types'
import { type Workspace, slotItems } from './workspace'

export interface Selection {
  paths: string[]
  primary: string | null
  anchor: string | null
}

export interface ClickModifiers {
  toggle: boolean
  range: boolean
}

export const emptySelection: Selection = { paths: [], primary: null, anchor: null }

export const orderedPaths = (workspace: Workspace): string[] => [
  ...workspace.sessions.flatMap((session) =>
    ROLES.flatMap((role) => slotItems(session.slots[role]).map((item) => item.path)),
  ),
  ...workspace.results.map((item) => item.path),
]

const single = (path: string): Selection => ({ paths: [path], primary: path, anchor: path })

function rangeBetween(order: string[], from: string, to: string): string[] {
  const start = order.indexOf(from)
  const end = order.indexOf(to)
  if (start < 0 || end < 0) return [to]
  return order.slice(Math.min(start, end), Math.max(start, end) + 1)
}

export function clickSelect(
  selection: Selection,
  order: string[],
  path: string,
  { toggle, range }: ClickModifiers,
): Selection {
  if (range && selection.anchor) {
    return { ...selection, paths: rangeBetween(order, selection.anchor, path), primary: path }
  }
  if (toggle) {
    return selection.paths.includes(path)
      ? removeFrom(selection, path)
      : { paths: [...selection.paths, path], primary: path, anchor: path }
  }
  return single(path)
}

function removeFrom(selection: Selection, path: string): Selection {
  const paths = selection.paths.filter((candidate) => candidate !== path)
  return { paths, primary: paths.at(-1) ?? null, anchor: paths.at(-1) ?? null }
}

export function pruneSelection(selection: Selection, workspace: Workspace): Selection {
  const present = new Set(orderedPaths(workspace))
  const paths = selection.paths.filter((path) => present.has(path))
  const primary =
    selection.primary && present.has(selection.primary) ? selection.primary : (paths.at(-1) ?? null)
  const anchor = selection.anchor && present.has(selection.anchor) ? selection.anchor : primary
  return { paths, primary, anchor }
}

export const selectAll = (workspace: Workspace): Selection => {
  const paths = orderedPaths(workspace)
  const last = paths.at(-1) ?? null
  return { paths, primary: last, anchor: paths[0] ?? null }
}
