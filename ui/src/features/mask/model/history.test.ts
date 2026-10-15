import { describe, expect, it } from 'vitest'

import { NO_HISTORY, type History, record, redo, undo } from './history'

describe('history', () => {
  it('goes back and forward through what was recorded', () => {
    let history: History<number> = NO_HISTORY
    history = record(history, 1)
    history = record(history, 2)
    const back = undo(history, 3)!
    expect(back.present).toBe(2)
    const further = undo(back.history, back.present)!
    expect(further.present).toBe(1)
    expect(undo(further.history, further.present)).toBeNull()
    const forward = redo(further.history, further.present)!
    expect(forward.present).toBe(2)
    expect(redo(redo(forward.history, forward.present)!.history, 3)).toBeNull()
  })

  it('forgets the line forward when something new is recorded', () => {
    let history: History<number> = record(NO_HISTORY, 1)
    const back = undo(history, 2)!
    history = record(back.history, back.present)
    expect(history.future).toEqual([])
  })

  it('keeps a limited number of steps', () => {
    let history: History<number> = NO_HISTORY
    for (let step = 0; step < 100; step++) history = record(history, step)
    expect(history.past.length).toBeLessThan(100)
    expect(history.past.at(-1)).toBe(99)
  })
})
