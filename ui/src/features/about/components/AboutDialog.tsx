import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'

import appIcon from '../../../assets/app-icon.png'
import wordmark from '../../../assets/logo-wordmark.png'
import { appVersion, isTauri, openExternal } from '../../../ipc/commands'
import { Modal } from '../../../shared/Modal'
import { type Outcome, useUpdateStore } from '../../update/store'
import { DEVELOPER, LICENSE, LINKS } from '../links'
import { useAboutStore } from '../store'

function Link({ href, children }: { href: string; children: string }) {
  return (
    <button
      type="button"
      onClick={() => void openExternal(href)}
      className="text-left text-ember hover:underline"
    >
      {children}
    </button>
  )
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="space-y-1.5 border-t border-twilight/60 pt-3">
      <h3 className="text-xs tracking-wide text-mist">{title}</h3>
      {children}
    </section>
  )
}

const OUTCOME_TEXT: Partial<Record<Outcome, string>> = {
  checking: 'about.checking',
  latest: 'about.upToDate',
  failed: 'about.checkFailed',
}

export function AboutDialog() {
  const { t } = useTranslation()
  const { open, setOpen } = useAboutStore()
  const { outcome, checkNow } = useUpdateStore()
  const [version, setVersion] = useState('')
  const [asked, setAsked] = useState(false)

  useEffect(() => {
    if (!open) return
    setAsked(false)
    if (isTauri()) void appVersion().then(setVersion)
  }, [open])

  if (!open) return null
  const status = asked ? OUTCOME_TEXT[outcome] : undefined

  return (
    <Modal title={t('about.title')} closeLabel={t('about.close')} onClose={() => setOpen(false)}>
      <div className="flex items-center gap-4">
        <img src={appIcon} alt="" className="h-14 w-14" draggable={false} />
        <div className="space-y-1">
          <img src={wordmark} alt={t('app.name')} className="h-6 w-auto" draggable={false} />
          <p className="text-xs text-mist">{t('about.version', { version })}</p>
        </div>
      </div>
      <p>{t('about.description')}</p>

      <Section title={t('about.developer')}>
        <p>{DEVELOPER.name}</p>
        <div className="flex flex-col gap-0.5">
          <Link href={DEVELOPER.github}>{DEVELOPER.github.replace('https://', '')}</Link>
          <Link href={`mailto:${DEVELOPER.email}`}>{DEVELOPER.email}</Link>
        </div>
      </Section>

      <Section title={t('about.links')}>
        <div className="flex flex-col gap-0.5">
          <Link href={LINKS.website}>{t('about.website')}</Link>
          <Link href={LINKS.documentation}>{t('about.documentation')}</Link>
          <Link href={LINKS.repository}>{t('about.repository')}</Link>
          <Link href={LINKS.support}>{t('about.support')}</Link>
        </div>
      </Section>

      <Section title={t('about.license')}>
        <p className="text-xs text-mist">
          {LICENSE.name} — {LICENSE.copyright}
        </p>
        <pre
          tabIndex={0}
          aria-label={t('about.license')}
          className="max-h-32 overflow-auto whitespace-pre-wrap rounded border border-twilight bg-black/20 p-2 font-[inherit] text-xs leading-relaxed text-mist select-text"
        >
          {LICENSE.text}
        </pre>
        <p className="text-xs text-mist">{t('about.thirdParty')}</p>
      </Section>

      <div className="flex items-center justify-between gap-3">
        <div className="flex min-w-0 items-center gap-3">
          <button
            type="button"
            disabled={outcome === 'checking'}
            onClick={() => {
              setAsked(true)
              void checkNow()
            }}
            className="rounded border border-twilight px-3 py-1 hover:bg-twilight/50 disabled:opacity-45"
          >
            {t('about.checkUpdates')}
          </button>
          {status && <span className="truncate text-xs text-mist">{t(status)}</span>}
        </div>
        <button
          type="button"
          autoFocus
          onClick={() => setOpen(false)}
          className="rounded border border-ember/70 bg-ember/15 px-3 py-1 text-ember hover:bg-ember/25"
        >
          {t('about.close')}
        </button>
      </div>
    </Modal>
  )
}
