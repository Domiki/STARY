import { useEffect } from 'react'

import { listenForFileDrops, useFileListStore } from '../features/file-list/store'
import { useSettingsStore } from '../features/settings/store'
import { isTauri, launchContext } from '../ipc/commands'
import { type LaunchGroup, ROLES, type Role } from '../ipc/types'

const roleOf = (name: string | null): Role | null => ROLES.find((role) => role === name) ?? null

async function addLaunchGroups(groups: LaunchGroup[]) {
  const { addFiles, addFilesToRole } = useFileListStore.getState()
  for (const { role, paths } of groups) {
    const target = roleOf(role)
    await (target ? addFilesToRole(paths, target) : addFiles(paths))
  }
}

let launchHandled = false

export function useLaunchContext() {
  useEffect(() => {
    if (!isTauri()) return
    let unlisten: (() => void) | undefined
    if (!launchHandled) {
      launchHandled = true
      void launchContext().then(({ groups, language }) => {
        if (language) useSettingsStore.getState().overrideLanguage(language)
        void addLaunchGroups(groups)
      })
    }
    void listenForFileDrops().then((stop) => (unlisten = stop))
    return () => unlisten?.()
  }, [])
}
