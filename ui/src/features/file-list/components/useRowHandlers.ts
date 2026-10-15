import type { MouseEvent } from 'react'

import { useFileListStore } from '../store'

export function useRowHandlers(path: string) {
  const selected = useFileListStore((state) => state.selection.paths.includes(path))
  const clickItem = useFileListStore((state) => state.clickItem)
  const openContextMenu = useFileListStore((state) => state.openContextMenu)
  const requestRemoval = useFileListStore((state) => state.requestRemoval)

  return {
    selected,
    onClick: (event: MouseEvent) =>
      clickItem(path, { toggle: event.metaKey || event.ctrlKey, range: event.shiftKey }),
    onContextMenu: (event: MouseEvent) => {
      event.preventDefault()
      if (!useFileListStore.getState().selection.paths.includes(path)) {
        clickItem(path, { toggle: false, range: false })
      }
      openContextMenu({ x: event.clientX, y: event.clientY, path })
    },
    onRemove: () => requestRemoval([path]),
  }
}
