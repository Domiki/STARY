import { useTranslation } from 'react-i18next'

import { IconButton } from '../../../shared/IconButton'
import { CloseIcon, WarningIcon } from '../../../shared/icons'
import { fileNameOf } from '../../../shared/format'
import { useFileListStore } from '../store'

export function RejectionList() {
  const { t } = useTranslation()
  const rejections = useFileListStore((state) => state.rejections)
  const dismiss = useFileListStore((state) => state.dismissRejections)
  if (rejections.length === 0) return null

  return (
    <div className="max-h-40 overflow-auto border-b border-twilight bg-danger/10 px-2 py-1.5 text-xs">
      <div className="mb-1 flex items-center justify-between text-danger">
        <WarningIcon />
        <IconButton label={t('fileList.dismiss')} onClick={dismiss}>
          <CloseIcon />
        </IconButton>
      </div>
      <ul className="space-y-1">
        {rejections.map((rejection) => (
          <li key={`${rejection.path}-${rejection.reason}`}>
            {t(`rejections.${rejection.reason}`, {
              name: fileNameOf(rejection.path),
              role: rejection.role ? t(`roles.${rejection.role}`) : '',
              expected: rejection.expected ? t(`roles.${rejection.expected}`) : '',
            })}
          </li>
        ))}
      </ul>
    </div>
  )
}
