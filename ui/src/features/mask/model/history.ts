/** What can be undone and redone, with the present kept apart by the caller. */
export interface History<T> {
  past: T[]
  future: T[]
}

export const NO_HISTORY: History<never> = { past: [], future: [] }

const LIMIT = 30

/** Remembers `present` before it is changed; a change after an undo ends the redo line. */
export function record<T>(history: History<T>, present: T): History<T> {
  return { past: [...history.past, present].slice(-LIMIT), future: [] }
}

export function undo<T>(history: History<T>, present: T): { history: History<T>; present: T } | null {
  const previous = history.past[history.past.length - 1]
  if (previous === undefined) return null
  return {
    present: previous,
    history: { past: history.past.slice(0, -1), future: [present, ...history.future] },
  }
}

export function redo<T>(history: History<T>, present: T): { history: History<T>; present: T } | null {
  const next = history.future[0]
  if (next === undefined) return null
  return {
    present: next,
    history: { past: [...history.past, present].slice(-LIMIT), future: history.future.slice(1) },
  }
}
