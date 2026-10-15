import type { TFunction } from 'i18next'

import type { Session } from './model/workspace'

export const sessionDisplayName = (session: Session | undefined, index: number, t: TFunction) =>
  session?.name ?? t('fileList.session', { n: index + 1 })
