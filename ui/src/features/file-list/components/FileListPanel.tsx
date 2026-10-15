import { useTranslation } from 'react-i18next'

import { IconButton } from '../../../shared/IconButton'
import { SessionPlusIcon } from '../../../shared/icons'
import { useFileListStore } from '../store'
import { InspectorPanel } from '../../inspector/components/InspectorPanel'
import { usePanelHeight } from '../../../shared/usePanelHeight'
import { RejectionList } from './RejectionList'
import { ResultSection } from './ResultSection'
import { SessionNode } from './SessionNode'

export function FileListPanel() {
  const { t } = useTranslation()
  const sessions = useFileListStore((state) => state.workspace.sessions)
  const addSession = useFileListStore((state) => state.addSession)
  const locked = useFileListStore((state) => state.locked)
  const inspector = usePanelHeight({ initial: 380, min: 120, max: 640 })

  const onKeyDown = (event: React.KeyboardEvent) => {
    const { selection, requestRemoval, selectEverything, clearSelection } = useFileListStore.getState()
    if (event.key === 'Delete' || event.key === 'Backspace') {
      event.preventDefault()
      requestRemoval(selection.paths)
    } else if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'a') {
      event.preventDefault()
      selectEverything()
    } else if (event.key === 'Escape') clearSelection()
  }

  return (
    <div className="flex h-full min-h-0 flex-col bg-twilight/10">
      <header className="flex items-center justify-between border-b border-twilight px-3 py-2">
        <h2>{t('fileList.title')}</h2>
        <IconButton label={t('fileList.addSession')} disabled={locked} onClick={addSession}>
          <SessionPlusIcon />
        </IconButton>
      </header>
      <RejectionList />
      <div
        role="listbox"
        aria-multiselectable
        tabIndex={0}
        className="min-h-0 flex-1 overflow-auto outline-none"
        onKeyDown={onKeyDown}
      >
        {sessions.length === 0 && (
          <p className="px-4 py-6 text-center text-xs text-mist">{t('fileList.empty')}</p>
        )}
        {sessions.map((session, index) => (
          <SessionNode key={session.id} session={session} index={index} />
        ))}
        <ResultSection />
      </div>
      <div
        role="separator"
        aria-orientation="horizontal"
        className="h-1 shrink-0 cursor-row-resize bg-twilight/60 transition-colors hover:bg-ember/70"
        {...inspector.handleProps}
      />
      <div className="flex shrink-0 flex-col" style={{ height: inspector.height }}>
        <InspectorPanel />
      </div>
    </div>
  )
}
