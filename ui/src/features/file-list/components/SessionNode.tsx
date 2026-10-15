import { useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { ROLES } from '../../../ipc/types'
import { IconButton } from '../../../shared/IconButton'
import { ChevronIcon, CloseIcon } from '../../../shared/icons'
import { MAX_SESSION_NAME_LENGTH } from '../model/rename'
import { warningsOf } from '../model/warnings'
import type { Session } from '../model/workspace'
import { sessionDisplayName } from '../session-name'
import { useFileListStore } from '../store'
import { RoleNode } from './RoleNode'

interface Props {
  session: Session
  index: number
}

interface NameEditorProps {
  initial: string
  onCommit: (name: string) => void
  onCancel: () => void
}

function NameEditor({ initial, onCommit, onCancel }: NameEditorProps) {
  const [value, setValue] = useState(initial)
  const finished = useRef(false)
  const finish = (action: () => void) => {
    if (finished.current) return
    finished.current = true
    action()
  }
  return (
    <input
      autoFocus
      value={value}
      maxLength={MAX_SESSION_NAME_LENGTH}
      onFocus={(event) => event.currentTarget.select()}
      onChange={(event) => setValue(event.target.value)}
      onBlur={() => finish(() => onCommit(value))}
      onKeyDown={(event) => {
        if (event.key === 'Enter') finish(() => onCommit(value))
        else if (event.key === 'Escape') finish(onCancel)
        event.stopPropagation()
      }}
      className="min-w-0 flex-1 rounded border border-ember/70 bg-midnight px-1 text-moon select-text"
    />
  )
}

export function SessionNode({ session, index }: Props) {
  const { t } = useTranslation()
  const [open, setOpen] = useState(true)
  const [editing, setEditing] = useState(false)
  const locked = useFileListStore((state) => state.locked)
  const active = useFileListStore((state) => state.workspace.activeSessionId === session.id)
  const setActiveSession = useFileListStore((state) => state.setActiveSession)
  const renameSession = useFileListStore((state) => state.renameSession)
  const requestSessionRemoval = useFileListStore((state) => state.requestSessionRemoval)
  const warnings = warningsOf(session)
  const name = sessionDisplayName(session, index, t)

  return (
    <div className="border-b border-twilight/60 pb-1">
      <div className="flex h-7 items-center gap-1 px-1">
        <button
          type="button"
          aria-expanded={open}
          aria-label={name}
          className="text-mist hover:text-moon"
          onClick={() => setOpen(!open)}
        >
          <ChevronIcon open={open} />
        </button>
        {editing ? (
          <NameEditor
            initial={name}
            onCancel={() => setEditing(false)}
            onCommit={(next) => {
              renameSession(session.id, next)
              setEditing(false)
            }}
          />
        ) : (
          <span
            className={`min-w-0 flex-1 cursor-default truncate ${active ? 'text-moon' : 'text-mist'}`}
            title={t('fileList.renameHint')}
            onClick={() => setActiveSession(session.id)}
            onDoubleClick={() => setEditing(true)}
          >
            {name}
          </span>
        )}
        {active && !editing && <span aria-hidden className="h-1.5 w-1.5 rounded-full bg-ember" />}
        <IconButton label={t('fileList.removeSession')} disabled={locked} onClick={() => requestSessionRemoval(session.id)}>
          <CloseIcon />
        </IconButton>
      </div>
      {open &&
        ROLES.map((role) => (
          <RoleNode key={role} session={session} role={role} warnings={warnings} />
        ))}
    </div>
  )
}
