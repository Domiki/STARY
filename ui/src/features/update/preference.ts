const STORAGE_KEY = 'stary.checkUpdates'

/** Whether the program looks for a newer release when it starts; on unless the person turned it off. */
export function loadCheckUpdates(): boolean {
  try {
    return localStorage.getItem(STORAGE_KEY) !== 'off'
  } catch {
    return true
  }
}

export function saveCheckUpdates(enabled: boolean): void {
  try {
    localStorage.setItem(STORAGE_KEY, enabled ? 'on' : 'off')
  } catch {
    // storage can be unavailable; the choice then lasts for this run only
  }
}
