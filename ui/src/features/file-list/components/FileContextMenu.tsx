import { useTranslation } from 'react-i18next'

import { ContextMenu, type MenuItem } from '../../../shared/ContextMenu'
import { useExportStore } from '../../export/store'
import { locate } from '../model/locate'
import { masterAvailability } from '../model/master'
import { isRepresentative } from '../model/representative'
import { useFileListStore } from '../store'

export function FileContextMenu() {
  const { t } = useTranslation()
  const menu = useFileListStore((state) => state.contextMenu)
  const workspace = useFileListStore((state) => state.workspace)
  const selection = useFileListStore((state) => state.selection)
  const locked = useFileListStore((state) => state.locked)
  const { closeContextMenu, requestRemoval, assignMaster, releaseMaster, chooseRepresentative } =
    useFileListStore.getState()
  if (!menu) return null

  const paths = selection.paths.includes(menu.path) ? selection.paths : [menu.path]
  const location = locate(workspace, menu.path)
  const session =
    location?.kind === 'session'
      ? workspace.sessions.find(({ id }) => id === location.sessionId)
      : undefined
  const role = location?.kind === 'session' ? location.role : null

  const masterItems: MenuItem[] = (() => {
    if (!session || !role || paths.length !== 1) return []
    const slot = session.slots[role]
    if (slot.kind === 'master') {
      return [
        {
          label: t('fileList.contextMenu.clearMaster'),
          disabled: locked,
          onSelect: () => releaseMaster(menu.path),
        },
      ]
    }
    const availability = masterAvailability(session, role, menu.path)
    if (availability === 'light_role') return []
    return [
      {
        label: t('fileList.contextMenu.setMaster'),
        hint: availability === 'other_files' ? t('fileList.contextMenu.masterNeedsAlone') : undefined,
        disabled: locked || availability !== 'ok',
        onSelect: () => assignMaster(menu.path),
      },
    ]
  })()

  const representativeItems: MenuItem[] =
    session && role === 'light' && paths.length === 1
      ? [
          {
            label: t('fileList.contextMenu.setRepresentative'),
            hint: isRepresentative(session, menu.path)
              ? t('fileList.contextMenu.alreadyRepresentative')
              : undefined,
            disabled: locked || isRepresentative(session, menu.path),
            onSelect: () => chooseRepresentative(menu.path),
          },
        ]
      : []

  const resultItems: MenuItem[] =
    location?.kind === 'result' && paths.length === 1
      ? [
          {
            label: t('export.menu'),
            onSelect: () => {
              closeContextMenu()
              useExportStore.getState().open(menu.path)
            },
          },
        ]
      : []

  const items: MenuItem[] = [
    ...resultItems,
    ...representativeItems,
    ...masterItems,
    {
      label: t('fileList.contextMenu.remove', { count: paths.length }),
      danger: true,
      disabled: locked,
      onSelect: () => requestRemoval(paths),
    },
  ]

  return <ContextMenu x={menu.x} y={menu.y} items={items} onClose={closeContextMenu} />
}
