// @vitest-environment jsdom
import { describe, expect, it } from 'vitest'

import { dropTargetOf } from './drop-target'

function mount(html: string): HTMLElement {
  document.body.innerHTML = html
  return document.body
}

describe('dropTargetOf', () => {
  it('finds the role zone for the element and for its children', () => {
    const body = mount(
      '<div data-drop-session="s1" data-drop-role="dark"><span id="inner">x</span></div>',
    )
    const expected = { sessionId: 's1', role: 'dark' }
    expect(dropTargetOf(body.querySelector('#inner'))).toEqual(expected)
    expect(dropTargetOf(body.firstElementChild)).toEqual(expected)
  })

  it('returns null outside any role zone', () => {
    const body = mount('<div id="outside">x</div>')
    expect(dropTargetOf(body.querySelector('#outside'))).toBeNull()
    expect(dropTargetOf(null)).toBeNull()
  })

  it('ignores zones with an unknown role', () => {
    const body = mount('<div data-drop-session="s1" data-drop-role="nope"><i id="i"></i></div>')
    expect(dropTargetOf(body.querySelector('#i'))).toBeNull()
  })
})
