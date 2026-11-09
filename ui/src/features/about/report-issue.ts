import { appVersion, openExternal } from '../../ipc/commands'
import { i18n } from '../../i18n'
import { useSettingsStore } from '../settings/store'
import { issueUrl, systemName } from './issue-url'

/** Opens a new issue on GitHub, with the version and the system filled in. */
export async function reportIssue(): Promise<void> {
  await useSettingsStore.getState().detectGpu()
  const gpu = useSettingsStore.getState().gpu
  const url = issueUrl({
    version: await appVersion().catch(() => 'unknown'),
    system: systemName(),
    gpu: gpu ? `${gpu.name} (${gpu.backend})` : 'none',
    language: i18n.language,
  })
  await openExternal(url)
}
