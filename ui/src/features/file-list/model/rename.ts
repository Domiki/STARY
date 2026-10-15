import { type Workspace, updateSession } from './workspace'

export const MAX_SESSION_NAME_LENGTH = 40

export const normalizeSessionName = (raw: string): string | null => {
  const trimmed = raw.trim().slice(0, MAX_SESSION_NAME_LENGTH).trim()
  return trimmed.length > 0 ? trimmed : null
}

export const renameSession = (workspace: Workspace, sessionId: string, raw: string): Workspace =>
  updateSession(workspace, sessionId, (session) => ({
    ...session,
    name: normalizeSessionName(raw),
  }))
