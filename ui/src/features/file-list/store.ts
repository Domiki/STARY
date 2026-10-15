import { create } from 'zustand'

import { expandPaths, listenForDrops, pickFiles, probeFiles } from '../../ipc/commands'
import type { Role } from '../../ipc/types'
import { applyOutcomes } from './model/apply-outcomes'
import { dropTargetAt } from './drop-target'
import { locate } from './model/locate'
import { clearMaster, setMaster } from './model/master'
import {
  type Rejection,
  type Target,
  createSession,
  ensureActiveSession,
  initialWorkspace,
  placeResult,
} from './model/placement'
import { removePaths, removeSession } from './model/removal'
import { setRepresentative } from './model/representative'
import { renameSession } from './model/rename'
import {
  type ClickModifiers,
  type Selection,
  clickSelect,
  emptySelection,
  orderedPaths,
  pruneSelection,
  selectAll,
} from './model/selection'
import { type Workspace, sessionItems } from './model/workspace'

export interface Status {
  added: number
  rejected: number
}

export type PendingRemoval =
  | { kind: 'paths'; paths: string[] }
  | { kind: 'session'; sessionId: string; fileCount: number }

export interface PendingDrop {
  paths: string[]
  target: Target
}

export interface ContextMenuState {
  x: number
  y: number
  path: string
}

interface FileListState {
  workspace: Workspace
  selection: Selection
  rejections: Rejection[]
  status: Status | null
  /** How far the files being added have been looked at; `null` when nothing is being added. */
  /** The list may not change while a job is running on it. */
  locked: boolean
  importProgress: { done: number; total: number } | null
  /** How many drops are having their folders searched for files. */
  scanning: number
  dropHover: boolean
  pendingRemoval: PendingRemoval | null
  contextMenu: ContextMenuState | null
  dropTarget: Target | null
  pendingDrop: PendingDrop | null
  addFiles: (paths: string[], target?: Target) => Promise<void>
  addFilesToRole: (paths: string[], role: Role) => Promise<void>
  addResult: (path: string) => Promise<void>
  pickAndAdd: (title: string, target: Target) => Promise<void>
  clickItem: (path: string, modifiers: ClickModifiers) => void
  selectEverything: () => void
  clearSelection: () => void
  setActiveSession: (sessionId: string) => void
  addSession: () => void
  renameSession: (sessionId: string, name: string) => void
  requestRemoval: (paths: string[]) => void
  requestSessionRemoval: (sessionId: string) => void
  confirmRemoval: () => void
  cancelRemoval: () => void
  assignMaster: (path: string) => void
  releaseMaster: (path: string) => void
  chooseRepresentative: (path: string) => void
  openContextMenu: (menu: ContextMenuState) => void
  closeContextMenu: () => void
  dismissRejections: () => void
  setDropHover: (hovering: boolean) => void
  setDropTarget: (target: Target | null) => void
  handleDrop: (paths: string[], target: Target | null) => Promise<void>
  confirmDrop: () => void
  cancelDrop: () => void
}

const afterChange = (workspace: Workspace, selection: Selection) => ({
  workspace,
  selection: pruneSelection(selection, workspace),
})

const sessionOf = (workspace: Workspace, path: string): string | null => {
  const location = locate(workspace, path)
  return location?.kind === 'session' ? location.sessionId : null
}

const PROBE_CHUNK = 24

/** Imports under way, each with how many of its files have been looked at: they may overlap. */
const imports = new Set<{ done: number; total: number }>()
/** What the imports that finished while others were still under way came to. */
let settled = { done: 0, total: 0 }

const importTotals = () =>
  imports.size === 0
    ? null
    : [...imports].reduce(
        (sum, { done, total }) => ({ done: sum.done + done, total: sum.total + total }),
        settled,
      )

const finishImport = (progress: { done: number; total: number }) => {
  imports.delete(progress)
  settled =
    imports.size === 0
      ? { done: 0, total: 0 }
      : { done: settled.done + progress.total, total: settled.total + progress.total }
}

async function probeInChunks(paths: string[], onProgress: (done: number) => void) {
  const outcomes: Awaited<ReturnType<typeof probeFiles>> = []
  for (let start = 0; start < paths.length; start += PROBE_CHUNK) {
    outcomes.push(...(await probeFiles(paths.slice(start, start + PROBE_CHUNK))))
    onProgress(outcomes.length)
  }
  return outcomes
}

export const useFileListStore = create<FileListState>((set, get) => ({
  workspace: initialWorkspace(),
  selection: emptySelection,
  rejections: [],
  status: null,
  locked: false,
  importProgress: null,
  scanning: 0,
  dropHover: false,
  pendingRemoval: null,
  contextMenu: null,
  dropTarget: null,
  pendingDrop: null,
  addFiles: async (paths, target) => {
    if (paths.length === 0 || get().locked) return
    const progress = { done: 0, total: paths.length }
    imports.add(progress)
    set({ importProgress: importTotals() })
    try {
      const outcomes = await probeInChunks(paths, (done) => {
        progress.done = done
        set({ importProgress: importTotals() })
      })
      const applied = applyOutcomes(get().workspace, outcomes, target)
      const first = applied.placed[0]
      set({
        workspace: applied.workspace,
        selection: first ? { paths: [first], primary: first, anchor: first } : get().selection,
        rejections: applied.rejections,
        status: { added: applied.placed.length, rejected: applied.rejections.length },
      })
    } catch (error) {
      console.error('probe_files failed', error)
    } finally {
      finishImport(progress)
      set({ importProgress: importTotals() })
    }
  },
  addFilesToRole: async (paths, role) => {
    const workspace = ensureActiveSession(get().workspace)
    set({ workspace })
    const sessionId = workspace.activeSessionId
    return get().addFiles(paths, sessionId ? { sessionId, role } : undefined)
  },
  addResult: async (path) => {
    const [outcome] = await probeFiles([path])
    if (outcome?.status !== 'ok') return
    const placement = placeResult(get().workspace, outcome.info)
    set({
      workspace: placement.workspace,
      selection: { paths: [path], primary: path, anchor: path },
    })
  },
  pickAndAdd: async (title, target) => get().addFiles(await pickFiles(title), target),
  clickItem: (path, modifiers) =>
    set((state) => {
      const sessionId = sessionOf(state.workspace, path)
      return {
        selection: clickSelect(state.selection, orderedPaths(state.workspace), path, modifiers),
        workspace: sessionId
          ? { ...state.workspace, activeSessionId: sessionId }
          : state.workspace,
      }
    }),
  selectEverything: () => set((state) => ({ selection: selectAll(state.workspace) })),
  clearSelection: () => set({ selection: emptySelection }),
  setActiveSession: (sessionId) =>
    set((state) => ({ workspace: { ...state.workspace, activeSessionId: sessionId } })),
  addSession: () =>
    set((state) => {
      if (state.locked) return {}
      const session = createSession()
      return {
        workspace: {
          ...state.workspace,
          sessions: [...state.workspace.sessions, session],
          activeSessionId: session.id,
        },
      }
    }),
  renameSession: (sessionId, name) =>
    set((state) => (state.locked ? {} : { workspace: renameSession(state.workspace, sessionId, name) })),
  requestRemoval: (paths) => {
    if (paths.length > 0 && !get().locked) set({ pendingRemoval: { kind: 'paths', paths }, contextMenu: null })
  },
  requestSessionRemoval: (sessionId) => {
    const session = get().workspace.sessions.find(({ id }) => id === sessionId)
    if (session && !get().locked) {
      set({
        pendingRemoval: { kind: 'session', sessionId, fileCount: sessionItems(session).length },
      })
    }
  },
  confirmRemoval: () =>
    set((state) => {
      const pending = state.pendingRemoval
      if (!pending) return {}
      const workspace =
        pending.kind === 'paths'
          ? removePaths(state.workspace, pending.paths)
          : removeSession(state.workspace, pending.sessionId)
      return { ...afterChange(workspace, state.selection), pendingRemoval: null }
    }),
  cancelRemoval: () => set({ pendingRemoval: null }),
  assignMaster: (path) =>
    set((state) => {
      if (state.locked) return { contextMenu: null }
      const location = locate(state.workspace, path)
      if (location?.kind !== 'session') return { contextMenu: null }
      const result = setMaster(state.workspace, location.sessionId, location.role, path)
      return { workspace: result.workspace, contextMenu: null }
    }),
  releaseMaster: (path) =>
    set((state) => {
      if (state.locked) return { contextMenu: null }
      const location = locate(state.workspace, path)
      if (location?.kind !== 'session') return { contextMenu: null }
      return {
        workspace: clearMaster(state.workspace, location.sessionId, location.role),
        contextMenu: null,
      }
    }),
  chooseRepresentative: (path) =>
    set((state) => {
      if (state.locked) return { contextMenu: null }
      const location = locate(state.workspace, path)
      if (location?.kind !== 'session' || location.role !== 'light') return { contextMenu: null }
      return {
        workspace: setRepresentative(state.workspace, location.sessionId, path),
        contextMenu: null,
      }
    }),
  openContextMenu: (contextMenu) => set({ contextMenu }),
  closeContextMenu: () => set({ contextMenu: null }),
  dismissRejections: () => set({ rejections: [] }),
  setDropHover: (dropHover) => set({ dropHover }),
  setDropTarget: (dropTarget) => set({ dropTarget }),
  handleDrop: async (paths, target) => {
    set({ dropHover: false, dropTarget: null })
    if (!target || get().locked) return
    set({ scanning: get().scanning + 1 })
    const files = await expandPaths(paths).finally(() => set({ scanning: get().scanning - 1 }))
    if (files.length > 0) set({ pendingDrop: { paths: files, target } })
  },
  confirmDrop: () => {
    const pending = get().pendingDrop
    set({ pendingDrop: null })
    if (pending) void get().addFiles(pending.paths, pending.target)
  },
  cancelDrop: () => set({ pendingDrop: null }),
}))

const hasSessions = () => useFileListStore.getState().workspace.sessions.length > 0

export const listenForFileDrops = () =>
  listenForDrops({
    onMove: ({ x, y }) => {
      if (!hasSessions() || useFileListStore.getState().locked) return
      const { setDropHover, setDropTarget } = useFileListStore.getState()
      setDropHover(true)
      setDropTarget(dropTargetAt(x, y))
    },
    onLeave: () => {
      const { setDropHover, setDropTarget } = useFileListStore.getState()
      setDropHover(false)
      setDropTarget(null)
    },
    onDrop: (paths, { x, y }) => {
      if (hasSessions()) void useFileListStore.getState().handleDrop(paths, dropTargetAt(x, y))
    },
  })
