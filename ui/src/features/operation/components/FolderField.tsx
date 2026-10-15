import { useTranslation } from 'react-i18next'

import { pickFolder } from '../../../ipc/commands'
import type { OutputKind } from '../output-folder'
import { useOperationStore } from '../store'

interface Props {
  kind: OutputKind
  label: string
  defaultFolder: string | null
}

export function FolderField({ kind, label, defaultFolder }: Props) {
  const { t } = useTranslation()
  const chosen = useOperationStore((state) => state.folders[kind])
  const setFolder = useOperationStore((state) => state.setFolder)
  const folder = chosen ?? defaultFolder

  const change = async () => {
    const picked = await pickFolder(t('operation.pickFolder'))
    if (picked) setFolder(kind, picked)
  }

  return (
    <div className="space-y-1">
      <div className="flex items-center justify-between text-xs text-mist">
        <span>{label}</span>
        {chosen === null && folder !== null && <span>{t('operation.defaultFolder')}</span>}
      </div>
      <div className="flex items-center gap-1">
        <span
          className="min-w-0 flex-1 truncate rounded border border-twilight bg-black/20 px-2 py-1 text-xs select-text"
          title={folder ?? undefined}
        >
          {folder ?? '—'}
        </span>
        <button
          type="button"
          onClick={() => void change()}
          className="shrink-0 rounded border border-twilight px-2 py-1 text-xs hover:bg-twilight/50"
        >
          {t('operation.changeFolder')}
        </button>
      </div>
    </div>
  )
}
