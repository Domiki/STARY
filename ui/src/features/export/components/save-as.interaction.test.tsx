// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'

import { initI18n } from '../../../i18n'
import { pickSavePath, saveResultAs } from '../../../ipc/commands'
import { useExportStore } from '../store'
import { SaveAsDialog } from './SaveAsDialog'

vi.mock('../../../ipc/commands', () => ({
  pickSavePath: vi.fn(async () => '/out/file.tif'),
  saveResultAs: vi.fn(async () => {}),
}))

beforeAll(async () => {
  await initI18n('en')
})

beforeEach(() => {
  vi.mocked(pickSavePath).mockClear()
  vi.mocked(saveResultAs).mockClear()
  useExportStore.setState({ source: null, busy: false, failure: null, saved: null })
})

afterEach(cleanup)

const open = (path = '/sky/M16_stack_37f.tif') => act(() => useExportStore.getState().open(path))
const radio = (name: string) => screen.getByRole('radio', { name }) as HTMLButtonElement

describe('the save-as dialog', () => {
  it('shows nothing until a result is opened', () => {
    render(<SaveAsDialog />)
    expect(screen.queryByRole('dialog')).toBeNull()
    open()
    expect(screen.getByRole('dialog', { name: 'Save as' })).toBeTruthy()
    expect(screen.getByText('M16_stack_37f.tif')).toBeTruthy()
  })

  it('offers the bits of the format and says when the file holds less', () => {
    render(<SaveAsDialog />)
    open()
    expect(screen.getByText(/holds less than the result/)).toBeTruthy()
    fireEvent.click(radio('32-bit float'))
    expect(screen.queryByText(/holds less than the result/)).toBeNull()
    expect(radio('Gamma').disabled).toBe(true)
    fireEvent.click(radio('JPEG'))
    expect(screen.queryByRole('radio', { name: '16-bit' })).toBeNull()
    expect(radio('8-bit').getAttribute('aria-checked')).toBe('true')
    expect(screen.getByRole('slider', { name: 'Quality' })).toBeTruthy()
  })

  it('keeps the colors and the linear values for FITS', () => {
    render(<SaveAsDialog />)
    open()
    fireEvent.click(radio('FITS'))
    expect(radio('sRGB').disabled).toBe(true)
    expect(radio('Linear').getAttribute('aria-checked')).toBe('true')
    expect(radio('As the result').getAttribute('aria-checked')).toBe('true')
  })

  it('saves with the choices and closes', async () => {
    render(<SaveAsDialog />)
    open()
    fireEvent.click(radio('Display P3'))
    fireEvent.click(radio('Gamma'))
    await act(async () => void fireEvent.click(screen.getByRole('button', { name: 'Save…' })))
    expect(pickSavePath).toHaveBeenCalledTimes(1)
    expect(saveResultAs).toHaveBeenCalledWith(
      expect.objectContaining({ space: 'display_p3', tone: 'display', depth: 'sixteen', target: '/out/file.tif' }),
    )
    expect(screen.queryByRole('dialog')).toBeNull()
  })

  it('shows what went wrong and stays', async () => {
    vi.mocked(saveResultAs).mockRejectedValueOnce({ code: 'x', message: 'no room' })
    render(<SaveAsDialog />)
    open()
    await act(async () => void fireEvent.click(screen.getByRole('button', { name: 'Save…' })))
    expect(screen.getByRole('alert').textContent).toContain('no room')
    expect(screen.getByRole('dialog')).toBeTruthy()
  })

  it('closes with Cancel and with Escape', () => {
    render(<SaveAsDialog />)
    open()
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(screen.queryByRole('dialog')).toBeNull()
    open()
    fireEvent.keyDown(window, { key: 'Escape' })
    expect(screen.queryByRole('dialog')).toBeNull()
  })
})
