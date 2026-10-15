// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'

import { initI18n } from '../i18n'
import { listenForCloseRequest, quitApp } from '../ipc/commands'
import { CloseGuard } from './CloseGuard'

let request: () => void = () => {}

vi.mock('../ipc/commands', () => ({
  listenForCloseRequest: vi.fn(async (handler: () => void) => {
    request = handler
    return () => {}
  }),
  quitApp: vi.fn(async () => {}),
}))

beforeAll(async () => {
  await initI18n('en')
})

beforeEach(() => {
  vi.mocked(quitApp).mockClear()
  vi.mocked(listenForCloseRequest).mockClear()
})

afterEach(cleanup)

describe('closing the program while a job runs', () => {
  it('asks first, and closes only when told to', async () => {
    render(<CloseGuard />)
    await act(async () => {})
    expect(screen.queryByRole('alertdialog')).toBeNull()
    act(() => request())
    expect(screen.getByText('A job is running')).toBeTruthy()
    expect(quitApp).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: 'Close' }))
    expect(quitApp).toHaveBeenCalledTimes(1)
  })

  it('keeps running when told to stay', async () => {
    render(<CloseGuard />)
    await act(async () => {})
    act(() => request())
    fireEvent.click(screen.getByRole('button', { name: 'Keep running' }))
    expect(screen.queryByRole('alertdialog')).toBeNull()
    expect(quitApp).not.toHaveBeenCalled()
  })
})
