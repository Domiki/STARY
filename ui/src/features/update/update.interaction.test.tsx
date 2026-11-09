// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'

import { initI18n } from '../../i18n'
import { appVersion, checkUpdate, openExternal } from '../../ipc/commands'
import type { NewerRelease } from '../../ipc/types'
import { UpdateDialog } from './components/UpdateDialog'
import { useUpdateStore } from './store'

vi.mock('../../ipc/commands', () => ({
  appVersion: vi.fn(async () => '0.1.0'),
  checkUpdate: vi.fn(),
  openExternal: vi.fn(async () => {}),
}))

const release: NewerRelease = {
  version: '0.2.0',
  name: 'STARY 0.2.0',
  notes: '## New\n- faster stacking\n- fixes',
  url: 'https://github.com/Domiki/STARY/releases/tag/v0.2.0',
  published: '2026-12-01T09:00:00Z',
  prerelease: false,
}

beforeAll(async () => {
  await initI18n('en')
})

beforeEach(() => {
  vi.mocked(checkUpdate).mockReset()
  vi.mocked(openExternal).mockClear()
  useUpdateStore.setState({
    enabled: true,
    version: null,
    outcome: 'none',
    release: null,
    dialogOpen: false,
  })
})

afterEach(cleanup)

describe('looking for a newer release', () => {
  it('opens the window about a newer release when the program starts', async () => {
    vi.mocked(checkUpdate).mockResolvedValue(release)
    await act(() => useUpdateStore.getState().checkAtStart())
    expect(checkUpdate).toHaveBeenCalledWith('0.1.0')
    expect(useUpdateStore.getState().dialogOpen).toBe(true)
  })

  it('stays quiet when there is nothing new', async () => {
    vi.mocked(checkUpdate).mockResolvedValue(null)
    await act(() => useUpdateStore.getState().checkAtStart())
    expect(useUpdateStore.getState().dialogOpen).toBe(false)
    expect(useUpdateStore.getState().outcome).toBe('latest')
  })

  it('stays quiet when GitHub cannot be reached', async () => {
    vi.mocked(checkUpdate).mockRejectedValue({ code: 'update_check_failed', message: 'offline' })
    await act(() => useUpdateStore.getState().checkAtStart())
    expect(useUpdateStore.getState().dialogOpen).toBe(false)
    expect(useUpdateStore.getState().outcome).toBe('failed')
  })

  it('does not look at all when the person turned it off', async () => {
    useUpdateStore.setState({ enabled: false })
    await act(() => useUpdateStore.getState().checkAtStart())
    expect(checkUpdate).not.toHaveBeenCalled()
    expect(appVersion).not.toHaveBeenCalled()
  })

  it('looks on request even when the start check is off', async () => {
    useUpdateStore.setState({ enabled: false })
    vi.mocked(checkUpdate).mockResolvedValue(release)
    await act(() => useUpdateStore.getState().checkNow())
    expect(useUpdateStore.getState().dialogOpen).toBe(true)
  })
})

describe('the window about a newer release', () => {
  const open = () => {
    useUpdateStore.setState({ release, version: '0.1.0', dialogOpen: true })
    render(<UpdateDialog />)
  }

  it('shows both versions and the release notes', () => {
    open()
    expect(screen.getByText('STARY 0.2.0 is available. You have 0.1.0.')).toBeTruthy()
    const notes = screen.getByLabelText('Release notes')
    expect(notes.textContent).toContain('faster stacking')
    expect(screen.getByText('2026-12-01')).toBeTruthy()
  })

  it('closing keeps the current version and goes nowhere', () => {
    open()
    fireEvent.click(screen.getAllByRole('button', { name: 'Close' }).at(-1)!)
    expect(useUpdateStore.getState().dialogOpen).toBe(false)
    expect(openExternal).not.toHaveBeenCalled()
  })

  it('the download button opens the release and closes the window', () => {
    open()
    fireEvent.click(screen.getByRole('button', { name: 'Go to the download' }))
    expect(openExternal).toHaveBeenCalledWith(release.url)
    expect(useUpdateStore.getState().dialogOpen).toBe(false)
  })

  it('says so when the release has no notes', () => {
    useUpdateStore.setState({ release: { ...release, notes: '  ' }, version: '0.1.0', dialogOpen: true })
    render(<UpdateDialog />)
    expect(screen.getByText('There are no release notes.')).toBeTruthy()
  })
})
