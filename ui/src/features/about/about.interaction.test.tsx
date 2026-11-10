// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'

import { useAppMenu } from '../../app/useAppMenu'
import { initI18n } from '../../i18n'
import { checkUpdate, listenForMenu, openExternal, setMenuText } from '../../ipc/commands'
import type { MenuAction } from '../../ipc/commands'
import { useSettingsStore } from '../settings/store'
import { useUpdateStore } from '../update/store'
import { AboutDialog } from './components/AboutDialog'
import { issueUrl } from './issue-url'
import { useAboutStore } from './store'

vi.mock('../../ipc/commands', () => ({
  appVersion: vi.fn(async () => '0.1.0'),
  isTauri: vi.fn(() => true),
  checkUpdate: vi.fn(async () => null),
  openExternal: vi.fn(async () => {}),
  gpuInfo: vi.fn(async () => ({ name: 'Test GPU', backend: 'Metal', shares_memory: true })),
  setMenuText: vi.fn(async () => {}),
  listenForMenu: vi.fn(async (handler: (action: MenuAction) => void) => {
    choose = handler
    return () => {}
  }),
}))

let choose: (action: MenuAction) => void = () => {}

function MenuBar() {
  useAppMenu()
  return null
}

beforeAll(async () => {
  await initI18n('en')
})

beforeEach(() => {
  vi.mocked(openExternal).mockClear()
  vi.mocked(checkUpdate).mockClear()
  useAboutStore.setState({ open: false })
  useUpdateStore.setState({ enabled: true, outcome: 'none', release: null, dialogOpen: false })
  useSettingsStore.setState({ gpu: undefined })
})

afterEach(cleanup)

describe('the address that reports an issue', () => {
  it('starts an issue on GitHub with the version and the system filled in', () => {
    const url = new URL(
      issueUrl({ version: '0.1.0', system: 'macOS', gpu: 'Test GPU (Metal)', language: 'ko' }),
    )
    expect(url.origin + url.pathname).toBe('https://github.com/Domiki/STARY/issues/new')
    const body = url.searchParams.get('body') ?? ''
    expect(body).toContain('- STARY: 0.1.0')
    expect(body).toContain('- System: macOS')
    expect(body).toContain('- GPU: Test GPU (Metal)')
    expect(body).toContain('**Steps to reproduce**')
  })
})

describe('the menu bar of the window', () => {
  it('gets its words in the language of the screen', async () => {
    render(<MenuBar />)
    await act(async () => {})
    expect(setMenuText).toHaveBeenLastCalledWith({
      help: 'Help',
      about: 'About STARY',
      report_issue: 'Report an issue',
      documentation: 'Documentation',
    })
  })

  it('opens the About window when About is chosen', async () => {
    render(<MenuBar />)
    await act(async () => {})
    act(() => choose('about'))
    expect(useAboutStore.getState().open).toBe(true)
  })

  it('opens the manual in the browser when Documentation is chosen', async () => {
    render(<MenuBar />)
    await act(async () => {})
    act(() => choose('documentation'))
    expect(openExternal).toHaveBeenCalledWith('https://stary.readthedocs.io/')
  })

  it('opens a new issue on GitHub with the system filled in when Report an issue is chosen', async () => {
    render(<MenuBar />)
    await act(async () => {})
    act(() => choose('report_issue'))
    await waitFor(() => expect(openExternal).toHaveBeenCalledTimes(1))
    const url = new URL(vi.mocked(openExternal).mock.calls[0]![0])
    expect(url.pathname).toBe('/Domiki/STARY/issues/new')
    const body = url.searchParams.get('body') ?? ''
    expect(body).toContain('- STARY: 0.1.0')
    expect(body).toContain('Test GPU (Metal)')
  })

  it('stops listening when it goes away', async () => {
    const { unmount } = render(<MenuBar />)
    await act(async () => {})
    expect(listenForMenu).toHaveBeenCalled()
    unmount()
  })
})

describe('the About window', () => {
  const open = async () => {
    useAboutStore.setState({ open: true })
    render(<AboutDialog />)
    await act(async () => {})
  }

  it('shows the version, the developer and the license', async () => {
    await open()
    expect(screen.getByText('Version 0.1.0 (pre-release)')).toBeTruthy()
    expect(screen.getByText('Domiki')).toBeTruthy()
    expect(screen.getByText('ado505050@gmail.com')).toBeTruthy()
    expect(screen.getByLabelText('License').textContent).toContain('Permission is hereby granted')
    expect(screen.getByText(/MIT License — Copyright \(c\) 2026 Domiki/)).toBeTruthy()
  })

  it('opens the links in the browser', async () => {
    await open()
    fireEvent.click(screen.getByRole('button', { name: 'Website' }))
    expect(openExternal).toHaveBeenCalledWith('https://domiki.github.io/STARY/')
    fireEvent.click(screen.getByRole('button', { name: 'github.com/Domiki' }))
    expect(openExternal).toHaveBeenCalledWith('https://github.com/Domiki')
    fireEvent.click(screen.getByRole('button', { name: 'Buy me a coffee' }))
    expect(openExternal).toHaveBeenCalledWith('https://buymeacoffee.com/domiki')
  })

  it('shows no result of an earlier look until it is asked', async () => {
    useUpdateStore.setState({ outcome: 'failed' })
    await open()
    expect(screen.queryByText('Could not look for updates.')).toBeNull()
  })

  it('says when there is no newer version', async () => {
    await open()
    fireEvent.click(screen.getByRole('button', { name: 'Check for updates' }))
    await waitFor(() => expect(screen.getByText('You have the newest version.')).toBeTruthy())
    expect(checkUpdate).toHaveBeenCalledWith('0.1.0')
  })

  it('closes with its button', async () => {
    await open()
    fireEvent.click(screen.getAllByRole('button', { name: 'Close' }).at(-1)!)
    expect(useAboutStore.getState().open).toBe(false)
  })
})
