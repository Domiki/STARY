import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { ChevronIcon } from '../../../shared/icons'
import { useFileListStore } from '../store'
import { FileRow } from './FileRow'

export function ResultSection() {
  const { t } = useTranslation()
  const [open, setOpen] = useState(true)
  const results = useFileListStore((state) => state.workspace.results)

  return (
    <div className="pt-1">
      <button
        type="button"
        className="flex w-full items-center gap-1 px-1 py-1.5 text-left"
        aria-expanded={open}
        onClick={() => setOpen(!open)}
      >
        <ChevronIcon open={open} />
        <span>{t('fileList.result')}</span>
        <span className="text-xs text-mist">({results.length})</span>
      </button>
      {open && results.length === 0 && (
        <p className="px-6 py-1 text-xs text-mist">{t('fileList.resultEmpty')}</p>
      )}
      {open &&
        results.map((item) => (
          <FileRow key={item.path} info={item} />
        ))}
    </div>
  )
}
