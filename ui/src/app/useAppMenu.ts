import { useEffect } from 'react'
import { useTranslation } from 'react-i18next'

import { useAboutStore } from '../features/about/store'
import { reportIssue } from '../features/about/report-issue'
import { LINKS } from '../features/about/links'
import { isTauri, listenForMenu, openExternal, setMenuText } from '../ipc/commands'

/** Joins the menu bar of the window to the screen: its words follow the language, its entries open windows. */
export function useAppMenu() {
  const { t, i18n } = useTranslation()

  useEffect(() => {
    if (!isTauri()) return
    void setMenuText({
      help: t('help.title'),
      about: t('help.about'),
      report_issue: t('help.reportIssue'),
      documentation: t('help.documentation'),
    }).catch(() => {})
  }, [t, i18n.language])

  useEffect(() => {
    if (!isTauri()) return
    let unlisten: (() => void) | undefined
    let cancelled = false
    void listenForMenu((action) => {
      if (action === 'about') useAboutStore.getState().setOpen(true)
      else if (action === 'documentation') void openExternal(LINKS.documentation)
      else void reportIssue()
    }).then((stop) => {
      if (cancelled) stop()
      else unlisten = stop
    })
    return () => {
      cancelled = true
      unlisten?.()
    }
  }, [])
}
