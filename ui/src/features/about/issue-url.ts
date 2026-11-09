import { detectPlatform } from '../../ipc/drop-point'
import { LINKS } from './links'

export interface SystemFacts {
  version: string
  system: string
  gpu: string
  language: string
}

const body = ({ version, system, gpu, language }: SystemFacts) => `**What happened**


**Steps to reproduce**
1.

**What did you expect**


**System**
- STARY: ${version}
- System: ${system}
- GPU: ${gpu}
- Language: ${language}
`

/** The page of GitHub that starts a new issue, with the system of the person filled in. */
export function issueUrl(facts: SystemFacts): string {
  const query = new URLSearchParams({ body: body(facts) })
  return `${LINKS.issues}/new?${query.toString()}`
}

const SYSTEM_NAMES = { macos: 'macOS', windows: 'Windows', linux: 'Linux' } as const

/** The operating system, in words. */
export const systemName = () => SYSTEM_NAMES[detectPlatform()]
