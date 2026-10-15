import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import type { Role } from '../../../ipc/types'
import { Badge } from '../../../shared/Badge'
import { IconButton } from '../../../shared/IconButton'
import { ChevronIcon, PlusIcon } from '../../../shared/icons'
import { DROP_ROLE_ATTRIBUTE, DROP_SESSION_ATTRIBUTE } from '../drop-target'
import type { ItemWarning } from '../model/warnings'
import { representativeOf } from '../model/representative'
import { type Session, slotItems } from '../model/workspace'
import { useFileListStore } from '../store'
import { FileRow } from './FileRow'

interface Props {
  session: Session
  role: Role
  warnings: Map<string, ItemWarning[]>
}

export function RoleNode({ session, role, warnings }: Props) {
  const { t } = useTranslation()
  const [open, setOpen] = useState(true)
  const pickAndAdd = useFileListStore((state) => state.pickAndAdd)
  const isDropTarget = useFileListStore(
    (state) => state.dropTarget?.sessionId === session.id && state.dropTarget.role === role,
  )
  const locked = useFileListStore((state) => state.locked)
  const slot = session.slots[role]
  const items = slotItems(slot)
  const roleName = t(`roles.${role}`)
  const representativePath = role === 'light' ? representativeOf(session)?.path : undefined

  const zone = { [DROP_SESSION_ATTRIBUTE]: session.id, [DROP_ROLE_ATTRIBUTE]: role }

  return (
    <div
      {...zone}
      className={isDropTarget ? 'rounded bg-ember/10 outline outline-1 outline-ember' : ''}
    >
      <div className="flex h-6 items-center gap-1 pr-1 pl-3 text-mist">
        <button
          type="button"
          className="flex min-w-0 flex-1 items-center gap-1 text-left hover:text-moon"
          onClick={() => setOpen(!open)}
          aria-expanded={open}
        >
          <ChevronIcon open={open} />
          <span className="truncate">{roleName}</span>
          <span className="text-xs">({items.length})</span>
          {slot.kind === 'master' && <Badge tone="ember">{t('fileList.master')}</Badge>}
        </button>
        <IconButton
          label={t('fileList.addToRole', { role: roleName })}
          disabled={locked}
          onClick={() => void pickAndAdd(t('fileList.addToRole', { role: roleName }), {
            sessionId: session.id,
            role,
          })}
        >
          <PlusIcon />
        </IconButton>
      </div>
      {open &&
        items.map((item) => (
          <FileRow
            key={item.path}
            info={item}
            isMaster={slot.kind === 'master'}
            isRepresentative={item.path === representativePath}
            warnings={warnings.get(item.path)}
          />
        ))}
    </div>
  )
}
