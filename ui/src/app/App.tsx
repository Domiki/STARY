import { AboutDialog } from '../features/about/components/AboutDialog'
import { UpdateDialog } from '../features/update/components/UpdateDialog'
import { SaveAsDialog } from '../features/export/components/SaveAsDialog'
import { DropConfirm } from '../features/file-list/components/DropConfirm'
import { FileContextMenu } from '../features/file-list/components/FileContextMenu'
import { FileListPanel } from '../features/file-list/components/FileListPanel'
import { RemovalConfirm } from '../features/file-list/components/RemovalConfirm'
import { MaskDialogs } from '../features/mask/components/MaskDialogs'
import { OperationPanel } from '../features/operation/components/OperationPanel'
import { SettingsDialog } from '../features/settings/components/SettingsDialog'
import { ViewerPanel } from '../features/viewer/components/ViewerPanel'
import { CloseGuard } from './CloseGuard'
import { PanelResizer } from './PanelResizer'
import { StatusBar } from './StatusBar'
import { TopBar } from './TopBar'
import { useJobEvents } from './useJobEvents'
import { useAppMenu } from './useAppMenu'
import { useLaunchContext } from './useLaunchContext'
import { useGpuDetection } from './useGpuDetection'
import { useUpdateCheck } from './useUpdateCheck'
import { usePanelWidth } from './usePanelWidth'

export function App() {
  useLaunchContext()
  useUpdateCheck()
  useAppMenu()
  useGpuDetection()
  useJobEvents()
  const left = usePanelWidth({ storageKey: 'stary.panel.left', initial: 300, min: 220, max: 520, edge: 'left' })
  const right = usePanelWidth({ storageKey: 'stary.panel.right', initial: 300, min: 240, max: 520, edge: 'right' })

  return (
    <div className="flex h-full flex-col">
      <TopBar />
      <div className="flex min-h-0 flex-1">
        <aside className="shrink-0 overflow-hidden" style={{ width: left.width }}>
          <FileListPanel />
        </aside>
        <PanelResizer {...left.handleProps} onDoubleClick={left.toggleCollapsed} />
        <main className="min-w-0 flex-1">
          <ViewerPanel />
        </main>
        <PanelResizer {...right.handleProps} onDoubleClick={right.toggleCollapsed} />
        <aside className="shrink-0 overflow-hidden" style={{ width: right.width }}>
          <OperationPanel />
        </aside>
      </div>
      <StatusBar />
      <SettingsDialog />
      <FileContextMenu />
      <RemovalConfirm />
      <DropConfirm />
      <MaskDialogs />
      <SaveAsDialog />
      <AboutDialog />
      <UpdateDialog />
      <CloseGuard />
    </div>
  )
}
