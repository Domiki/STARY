import { beforeEach, describe, expect, it, vi } from 'vitest'

import { pickSavePath, saveResultAs } from '../../ipc/commands'
import { folderOf, useExportStore } from './store'

vi.mock('../../ipc/commands', () => ({
  pickSavePath: vi.fn(),
  saveResultAs: vi.fn(async () => {}),
}))

const SOURCE = '/sky/out/M16_stack_37f.tif'

beforeEach(() => {
  vi.mocked(pickSavePath).mockReset().mockResolvedValue('/elsewhere/M16.png')
  vi.mocked(saveResultAs).mockReset().mockResolvedValue(undefined)
  useExportStore.setState({ source: null, busy: false, failure: null, saved: null })
})

describe('the folder of a path', () => {
  it('keeps the separator it finds', () => {
    expect(folderOf('/a/b/c.tif')).toBe('/a/b/')
    expect(folderOf('C:\\a\\b.tif')).toBe('C:\\a\\')
    expect(folderOf('c.tif')).toBe('')
  })
})

describe('saving a result as something else', () => {
  it('starts from sensible choices for the result', () => {
    useExportStore.getState().open(SOURCE)
    expect(useExportStore.getState()).toMatchObject({
      source: SOURCE,
      options: { container: 'tiff', depth: 'sixteen', space: 'srgb', tone: 'stretch' },
    })
    useExportStore.getState().open('/sky/out/a_lighten_5f.tif')
    expect(useExportStore.getState().options.tone).toBe('display')
  })

  it('keeps the choices possible when one changes', () => {
    useExportStore.getState().open(SOURCE)
    useExportStore.getState().change({ container: 'jpeg' })
    expect(useExportStore.getState().options).toMatchObject({ container: 'jpeg', depth: 'eight' })
    useExportStore.getState().change({ container: 'fits' })
    expect(useExportStore.getState().options).toMatchObject({ space: null, tone: 'linear' })
  })

  it('asks where, suggesting a name next to the result, and then makes the file', async () => {
    useExportStore.getState().open(SOURCE)
    await useExportStore.getState().save('Save as', 'same')
    expect(pickSavePath).toHaveBeenCalledWith('Save as', '/sky/out/M16_stack_37f_srgb16.tif', 'tif')
    expect(saveResultAs).toHaveBeenCalledWith({
      source: SOURCE,
      target: '/elsewhere/M16.png',
      container: 'tiff',
      depth: 'sixteen',
      space: 'srgb',
      tone: 'stretch',
      quality: 92,
    })
    expect(useExportStore.getState()).toMatchObject({ source: null, saved: '/elsewhere/M16.png', busy: false })
  })

  it('stays open and makes nothing when the user gives up the file panel', async () => {
    vi.mocked(pickSavePath).mockResolvedValue(null)
    useExportStore.getState().open(SOURCE)
    await useExportStore.getState().save('Save as', 'same')
    expect(saveResultAs).not.toHaveBeenCalled()
    expect(useExportStore.getState().source).toBe(SOURCE)
  })

  it('does not write over the result itself', async () => {
    vi.mocked(pickSavePath).mockResolvedValue(SOURCE)
    useExportStore.getState().open(SOURCE)
    await useExportStore.getState().save('Save as', 'Choose another file')
    expect(saveResultAs).not.toHaveBeenCalled()
    expect(useExportStore.getState().failure).toBe('Choose another file')
  })

  it('reports why a file could not be made, and stays open', async () => {
    vi.mocked(saveResultAs).mockRejectedValue({ code: 'write_failed', message: 'disk full' })
    useExportStore.getState().open(SOURCE)
    await useExportStore.getState().save('Save as', 'same')
    expect(useExportStore.getState()).toMatchObject({ source: SOURCE, failure: 'disk full', busy: false })
  })

  it('cannot be closed while it is saving', async () => {
    let finish: () => void = () => {}
    vi.mocked(saveResultAs).mockReturnValue(new Promise<void>((resolve) => (finish = resolve)))
    useExportStore.getState().open(SOURCE)
    const saving = useExportStore.getState().save('Save as', 'same')
    await vi.waitFor(() => expect(useExportStore.getState().busy).toBe(true))
    useExportStore.getState().close()
    expect(useExportStore.getState().source).toBe(SOURCE)
    finish()
    await saving
    expect(useExportStore.getState().source).toBeNull()
  })
})
