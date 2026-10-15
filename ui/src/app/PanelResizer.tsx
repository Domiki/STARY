import type { PointerEventHandler } from 'react'

interface Props {
  onPointerDown: PointerEventHandler
  onPointerMove: PointerEventHandler
  onPointerUp: PointerEventHandler
  onDoubleClick: () => void
}

export const PanelResizer = (props: Props) => (
  <div
    role="separator"
    aria-orientation="vertical"
    className="w-1 shrink-0 cursor-col-resize bg-twilight/60 transition-colors hover:bg-ember/70"
    {...props}
  />
)
