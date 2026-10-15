// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'

import { initI18n } from '../../../i18n'
import type { Role } from '../../../ipc/types'
import { placeInfo } from '../model/placement'
import { info } from '../model/test-fixtures'
import { type Workspace, emptyWorkspace } from '../model/workspace'
import { listenForDrops } from '../../../ipc/commands'
import { useExportStore } from '../../export/store'
import { useJobStore } from '../../operation/job-store'
import { listenForFileDrops, useFileListStore } from '../store'
import { FileContextMenu } from './FileContextMenu'
import { FileListPanel } from './FileListPanel'
import { DropConfirm } from './DropConfirm'
import { RemovalConfirm } from './RemovalConfirm'

vi.mock('../../../ipc/commands', () => ({
  probeFiles: vi.fn(async (paths: string[]) =>
    paths.map((path) => ({ status: 'ok', info: info({ path }) })),
  ),
  expandPaths: vi.fn(async (paths: string[]) => paths.flatMap((path) =>
    path.endsWith('/folder')
      ? [`${path}/a.fits`, `${path}/b.fits`]
      : path.endsWith('/empty-folder')
        ? []
        : [path],
  )),
  pickFiles: vi.fn(),
  listenForDrops: vi.fn(),
}))

function seed(entries: [Role, string][]): Workspace {
  let workspace = placeInfo(emptyWorkspace(), info({ path: '/l/1.fits' })).workspace
  const sessionId = workspace.sessions[0]!.id
  workspace = placeInfo(workspace, info({ path: '/l/2.fits' })).workspace
  workspace = placeInfo(workspace, info({ path: '/l/3.fits' })).workspace
  workspace = placeInfo(workspace, info({ path: '/l/4.fits' })).workspace
  for (const [role, path] of entries) {
    workspace = placeInfo(workspace, info({ path }), { sessionId, role }).workspace
  }
  return workspace
}

const renderPanel = () =>
  render(
    <>
      <FileListPanel />
      <FileContextMenu />
      <RemovalConfirm />
      <DropConfirm />
    </>,
  )

const rowAt = (path: string) => screen.getByTitle(path).closest('[role="option"]') as HTMLElement
const row = (name: string) => screen.getByText(name).closest('[role="option"]') as HTMLElement
const names = () => screen.queryAllByRole('option').map((element) => element.textContent)

beforeAll(async () => {
  await initI18n('en')
})

beforeEach(() => {
  useFileListStore.setState({
    workspace: seed([]),
    selection: { paths: [], primary: null, anchor: null },
    pendingRemoval: null,
    contextMenu: null,
    pendingDrop: null,
    dropTarget: null,
  })
})

afterEach(cleanup)

describe('file list interactions', () => {
  it('has no add-files button in the header', () => {
    renderPanel()
    expect(screen.queryByLabelText('Add files')).toBeNull()
    expect(screen.getByLabelText('Add session')).toBeTruthy()
  })

  it('marks the lights a job left out, with the reason as a tooltip', () => {
    renderPanel()
    expect(screen.queryByText('Left out')).toBeNull()
    act(() => useJobStore.setState({ leftOut: { '/l/2.fits': 'blurry' } }))
    const badge = screen.getByText('Left out')
    expect(within(row('2.fits')).getByText('Left out')).toBe(badge)
    expect(badge.parentElement?.getAttribute('title')).toBe('Blurrier than the other frames.')
    act(() => useJobStore.setState({ leftOut: {} }))
  })

  it('selects a range with shift and asks before removing with Delete', () => {
    renderPanel()
    fireEvent.click(row('1.fits'))
    fireEvent.click(row('3.fits'), { shiftKey: true })
    expect(useFileListStore.getState().selection.paths).toEqual(['/l/1.fits', '/l/2.fits', '/l/3.fits'])

    fireEvent.keyDown(screen.getByRole('listbox'), { key: 'Delete' })
    const dialog = screen.getByRole('alertdialog')
    expect(within(dialog).getByText(/Delete 3 selected file/)).toBeTruthy()
    expect(within(dialog).getByText(/cannot be undone/i)).toBeTruthy()
    expect(names()).toHaveLength(4)

    fireEvent.click(within(dialog).getByText('Delete'))
    expect(names()).toEqual(['4.fits'])
  })

  it('keeps everything when the confirmation is cancelled', () => {
    renderPanel()
    fireEvent.click(row('2.fits'))
    fireEvent.keyDown(screen.getByRole('listbox'), { key: 'Delete' })
    fireEvent.click(within(screen.getByRole('alertdialog')).getByText('Cancel'))
    expect(names()).toHaveLength(4)
    expect(screen.queryByRole('alertdialog')).toBeNull()
  })

  it('toggles items with the command key', () => {
    renderPanel()
    fireEvent.click(row('1.fits'))
    fireEvent.click(row('3.fits'), { metaKey: true })
    expect(useFileListStore.getState().selection.paths).toEqual(['/l/1.fits', '/l/3.fits'])
  })

  it('confirms before deleting a session', () => {
    renderPanel()
    fireEvent.click(screen.getByLabelText('Remove session'))
    expect(within(screen.getByRole('alertdialog')).getByText(/4 file\(s\) in this session/)).toBeTruthy()
    fireEvent.click(within(screen.getByRole('alertdialog')).getByText('Delete'))
    expect(useFileListStore.getState().workspace.sessions).toHaveLength(0)
  })

  it('offers master only for a file that is alone in its role', () => {
    useFileListStore.setState({ workspace: seed([['dark', '/d/1.fits'], ['dark', '/d/2.fits']]) })
    renderPanel()
    fireEvent.contextMenu(screen.getByTitle('/d/1.fits'))
    const disabled = screen.getByRole('menuitem', { name: /Set as master/ })
    expect((disabled as HTMLButtonElement).disabled).toBe(true)
    expect(screen.getByText(/Remove the other files/)).toBeTruthy()
    fireEvent.click(screen.getByRole('menuitem', { name: /Remove 1 file/ }))
    fireEvent.click(within(screen.getByRole('alertdialog')).getByText('Delete'))

    fireEvent.contextMenu(screen.getByTitle('/d/2.fits'))
    fireEvent.click(screen.getByRole('menuitem', { name: /Set as master/ }))
    expect(useFileListStore.getState().workspace.sessions[0]!.slots.dark.kind).toBe('master')
    expect(screen.getAllByText('MASTER').length).toBeGreaterThan(0)
  })

  it('can release a master and has no master option for Light', () => {
    renderPanel()
    fireEvent.contextMenu(screen.getByTitle('/l/1.fits'))
    expect(screen.queryByRole('menuitem', { name: /master/i })).toBeNull()
    expect(screen.getByRole('menuitem', { name: /Remove 1 file/ })).toBeTruthy()
  })
})

describe('while a job is running', () => {
  it('keeps the list as it is, but still lets files be looked at', () => {
    renderPanel()
    act(() => useJobStore.setState({ phase: 'running', kind: 'trails' }))
    const before = useFileListStore.getState().workspace
    const { requestRemoval, addSession, chooseRepresentative } = useFileListStore.getState()
    requestRemoval(['/l/1.fits'])
    addSession()
    chooseRepresentative('/l/2.fits')
    expect(useFileListStore.getState().pendingRemoval).toBeNull()
    expect(useFileListStore.getState().workspace).toBe(before)
    expect((screen.getByLabelText('Add session') as HTMLButtonElement).disabled).toBe(true)
    fireEvent.click(row('2.fits'))
    expect(useFileListStore.getState().selection.primary).toBe('/l/2.fits')
    act(() => useJobStore.setState({ phase: 'idle', kind: null }))
    expect((screen.getByLabelText('Add session') as HTMLButtonElement).disabled).toBe(false)
  })
})

describe('adding many files', () => {
  it('reports how far the files have been looked at, then nothing', async () => {
    const seen: (number | null)[] = []
    const stop = useFileListStore.subscribe((state) => seen.push(state.importProgress?.done ?? null))
    await useFileListStore.getState().addFiles(Array.from({ length: 50 }, (_, i) => `/many/${i}.fits`))
    stop()
    expect(seen).toContain(24)
    expect(seen).toContain(48)
    expect(seen.at(-1)).toBeNull()
    expect(useFileListStore.getState().importProgress).toBeNull()
  })
})

describe('adding files at the same time', () => {
  it('adds the progress of both and clears it only when both are done', async () => {
    const seen: ({ done: number; total: number } | null)[] = []
    const stop = useFileListStore.subscribe((state) => seen.push(state.importProgress))
    const files = (name: string, count: number) =>
      Array.from({ length: count }, (_, i) => `/${name}/${i}.fits`)
    await Promise.all([
      useFileListStore.getState().addFiles(files('a', 30)),
      useFileListStore.getState().addFiles(files('b', 60)),
    ])
    stop()
    const totals = seen.flatMap((progress) => (progress ? [progress.total] : []))
    expect(totals.every((total) => total === 30 || total === 90)).toBe(true)
    expect(seen.filter((progress) => progress === null)).toHaveLength(1)
    expect(seen.at(-1)).toBeNull()
    const dones = seen.flatMap((progress) => (progress ? [progress.done] : []))
    expect(dones).toEqual([...dones].sort((a, b) => a - b))
  })
})

describe('searching dropped folders', () => {
  it('counts the drops being searched, and none afterwards', async () => {
    const seen: number[] = []
    const stop = useFileListStore.subscribe((state) => seen.push(state.scanning))
    const target = { sessionId: useFileListStore.getState().workspace.sessions[0]!.id, role: 'light' as const }
    await useFileListStore.getState().handleDrop(['/x/folder'], target)
    stop()
    expect(seen).toContain(1)
    expect(useFileListStore.getState().scanning).toBe(0)
    useFileListStore.getState().cancelDrop()
  })
})

describe('dropping files onto a role', () => {
  const darkTarget = () => ({
    sessionId: useFileListStore.getState().workspace.sessions[0]!.id,
    role: 'dark' as const,
  })

  it('asks for confirmation with the file count, session and role, then adds', async () => {
    renderPanel()
    await act(() => useFileListStore.getState().handleDrop(['/x/folder'], darkTarget()))
    const dialog = screen.getByRole('alertdialog')
    expect(within(dialog).getByText(/2 file\(s\) will be added to Dark in Session 1/)).toBeTruthy()

    fireEvent.click(within(dialog).getByText('Add'))
    await waitFor(() =>
      expect(useFileListStore.getState().workspace.sessions[0]!.slots.dark.kind).toBe('frames'),
    )
    expect(screen.queryByRole('alertdialog')).toBeNull()
    expect(screen.getByTitle('/x/folder/a.fits')).toBeTruthy()
  })

  it('adds nothing when the confirmation is cancelled', async () => {
    renderPanel()
    await act(() => useFileListStore.getState().handleDrop(['/x/one.fits'], darkTarget()))
    fireEvent.click(within(screen.getByRole('alertdialog')).getByText('Cancel'))
    expect(useFileListStore.getState().workspace.sessions[0]!.slots.dark.kind).toBe('empty')
  })

  it('silently ignores a drop that is not on a role list', async () => {
    renderPanel()
    await act(() => useFileListStore.getState().handleDrop(['/x/one.fits'], null))
    expect(screen.queryByRole('alertdialog')).toBeNull()
    expect(useFileListStore.getState().pendingDrop).toBeNull()
    expect(screen.queryByText(/role list/i)).toBeNull()
  })

  it('silently ignores a folder without supported files', async () => {
    renderPanel()
    await act(() => useFileListStore.getState().handleDrop(['/x/empty-folder'], darkTarget()))
    expect(screen.queryByRole('alertdialog')).toBeNull()
  })

  it('ignores dragging entirely while there is no session', () => {
    useFileListStore.setState({ workspace: emptyWorkspace() })
    listenForFileDrops()
    const handlers = vi.mocked(listenForDrops).mock.calls.at(-1)![0]
    handlers.onMove({ x: 10, y: 10 })
    expect(useFileListStore.getState().dropHover).toBe(false)
    handlers.onDrop(['/x/one.fits'], { x: 10, y: 10 })
    expect(useFileListStore.getState().pendingDrop).toBeNull()
  })

  it('marks the role list under the cursor', () => {
    renderPanel()
    act(() => useFileListStore.getState().setDropTarget(darkTarget()))
    const zone = document.querySelector('[data-drop-role="dark"]') as HTMLElement
    expect(zone.className).toMatch(/outline/)
  })
})

describe('sessions', () => {
  it('asks to add a session first when there are none', () => {
    useFileListStore.setState({ workspace: emptyWorkspace() })
    renderPanel()
    expect(screen.getByText('Add a session first.')).toBeTruthy()
  })

  it('renames a session by double-clicking its name', () => {
    renderPanel()
    fireEvent.doubleClick(screen.getByText('Session 1'))
    const input = screen.getByDisplayValue('Session 1')
    fireEvent.change(input, { target: { value: '  Eagle Nebula ' } })
    fireEvent.keyDown(input, { key: 'Enter' })
    expect(screen.getByText('Eagle Nebula')).toBeTruthy()
    expect(screen.queryByDisplayValue('Eagle Nebula')).toBeNull()
  })

  it('keeps the old name when editing is cancelled with Escape', () => {
    renderPanel()
    fireEvent.doubleClick(screen.getByText('Session 1'))
    const input = screen.getByDisplayValue('Session 1')
    fireEvent.change(input, { target: { value: 'Something else' } })
    fireEvent.keyDown(input, { key: 'Escape' })
    fireEvent.blur(input)
    expect(screen.getByText('Session 1')).toBeTruthy()
  })

  it('goes back to the default name when the name is emptied', () => {
    renderPanel()
    fireEvent.doubleClick(screen.getByText('Session 1'))
    const input = screen.getByDisplayValue('Session 1')
    fireEvent.change(input, { target: { value: '   ' } })
    fireEvent.blur(input)
    expect(screen.getByText('Session 1')).toBeTruthy()
  })

  it('uses the new name in the delete confirmation', () => {
    renderPanel()
    fireEvent.doubleClick(screen.getByText('Session 1'))
    const input = screen.getByDisplayValue('Session 1')
    fireEvent.change(input, { target: { value: 'M16' } })
    fireEvent.keyDown(input, { key: 'Enter' })
    fireEvent.click(screen.getByLabelText('Remove session'))
    expect(within(screen.getByRole('alertdialog')).getByText('Delete M16?')).toBeTruthy()
  })
})

describe('representative image', () => {
  it('marks the first light as the representative image', () => {
    renderPanel()
    const marked = screen.getAllByRole('img', { name: 'Representative image' })
    expect(marked).toHaveLength(1)
    expect(rowAt('/l/1.fits').contains(marked[0]!)).toBe(true)
  })

  it('changes the representative image from the context menu of a light', () => {
    renderPanel()
    fireEvent.contextMenu(screen.getByTitle('/l/3.fits'))
    fireEvent.click(screen.getByRole('menuitem', { name: /Set as representative image/ }))
    const marked = screen.getAllByRole('img', { name: 'Representative image' })
    expect(marked).toHaveLength(1)
    expect(rowAt('/l/3.fits').contains(marked[0]!)).toBe(true)
    expect(rowAt('/l/1.fits').querySelector('[aria-label="Representative image"]')).toBeNull()
    expect(screen.queryByRole('menu')).toBeNull()
  })

  it('does not offer the representative image for the one that already is', () => {
    renderPanel()
    fireEvent.contextMenu(screen.getByTitle('/l/1.fits'))
    const item = screen.getByRole('menuitem', { name: /Set as representative image/ })
    expect((item as HTMLButtonElement).disabled).toBe(true)
    expect(screen.getByText('This is already the representative image')).toBeTruthy()
  })

  it('offers it only for lights and only for one file at a time', () => {
    useFileListStore.setState({ workspace: seed([['dark', '/d/1.fits']]) })
    renderPanel()
    fireEvent.contextMenu(screen.getByTitle('/d/1.fits'))
    expect(screen.queryByRole('menuitem', { name: /representative/i })).toBeNull()
    fireEvent.keyDown(window, { key: 'Escape' })

    fireEvent.click(rowAt('/l/1.fits'))
    fireEvent.click(rowAt('/l/2.fits'), { metaKey: true })
    fireEvent.contextMenu(screen.getByTitle('/l/1.fits'))
    expect(screen.queryByRole('menuitem', { name: /representative/i })).toBeNull()
  })

  it('hands the mark to the next light when the representative is deleted', () => {
    renderPanel()
    fireEvent.click(rowAt('/l/1.fits'))
    fireEvent.keyDown(screen.getByRole('listbox'), { key: 'Delete' })
    fireEvent.click(within(screen.getByRole('alertdialog')).getByText('Delete'))
    const marked = screen.getAllByRole('img', { name: 'Representative image' })
    expect(rowAt('/l/2.fits').contains(marked[0]!)).toBe(true)
  })

  it('offers to save a result as something else, and only a result', () => {
    const workspace = { ...seed([]), results: [info({ path: '/out/M16_stack_37f.tif' })] }
    useFileListStore.setState({ workspace })
    renderPanel()
    fireEvent.contextMenu(screen.getByTitle('/out/M16_stack_37f.tif'))
    fireEvent.click(screen.getByRole('menuitem', { name: 'Save as…' }))
    expect(useExportStore.getState().source).toBe('/out/M16_stack_37f.tif')
    expect(useFileListStore.getState().contextMenu).toBeNull()

    useFileListStore.setState({ workspace: seed([]) })
    cleanup()
    renderPanel()
    fireEvent.contextMenu(screen.getByTitle('/l/1.fits'))
    expect(screen.queryByRole('menuitem', { name: 'Save as…' })).toBeNull()
  })
})
