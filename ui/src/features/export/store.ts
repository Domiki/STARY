import { create } from 'zustand'

import { pickSavePath, saveResultAs } from '../../ipc/commands'
import type { CommandError } from '../../ipc/types'
import { fileNameOf } from '../../shared/format'
import {
  type ExportOptions,
  defaultOptions,
  extensionOf,
  requestFor,
  settled,
  suggestedName,
} from './model/options'

/** The folder a path is in, with the separator it uses. */
export const folderOf = (path: string): string => path.slice(0, path.length - fileNameOf(path).length)

const samePath = (a: string, b: string) => a === b

interface ExportState {
  /** The result being saved as something else, when the dialog is open. */
  source: string | null
  options: ExportOptions
  busy: boolean
  /** What went wrong the last time, for the dialog. */
  failure: string | null
  /** The file made last, for the status bar. */
  saved: string | null
  open: (source: string) => void
  close: () => void
  change: (change: Partial<ExportOptions>) => void
  /** Asks where to save, then makes the file. `title` is the name of the dialog that asks. */
  save: (title: string, sameFile: string) => Promise<void>
}

export const useExportStore = create<ExportState>((set, get) => ({
  source: null,
  options: defaultOptions(''),
  busy: false,
  failure: null,
  saved: null,
  open: (source) =>
    set({ source, options: settled(defaultOptions(fileNameOf(source))), failure: null, busy: false }),
  close: () => {
    if (!get().busy) set({ source: null, failure: null })
  },
  change: (change) => set(({ options }) => ({ options: settled({ ...options, ...change }) })),
  save: async (title, sameFile) => {
    const { source, options } = get()
    if (!source || get().busy) return
    const name = suggestedName(fileNameOf(source), options)
    const target = await pickSavePath(title, folderOf(source) + name, extensionOf(options.container))
    if (!target) return
    if (samePath(target, source)) return set({ failure: sameFile })
    set({ busy: true, failure: null })
    try {
      await saveResultAs(requestFor(source, target, options))
      set({ busy: false, source: null, saved: target })
    } catch (error) {
      const { message } = error as Partial<CommandError>
      set({ busy: false, failure: message ?? String(error) })
    }
  },
}))
