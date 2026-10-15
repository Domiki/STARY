import type { ReactNode } from 'react'

type Tone = 'ember' | 'mist' | 'caution'

const TONES: Record<Tone, string> = {
  ember: 'border-ember/70 text-ember',
  mist: 'border-mist/50 text-mist',
  caution: 'border-caution/60 text-caution',
}

export const Badge = ({ tone = 'mist', children }: { tone?: Tone; children: ReactNode }) => (
  <span
    className={`inline-flex items-center rounded border px-1 text-[10px] leading-4 tracking-wide ${TONES[tone]}`}
  >
    {children}
  </span>
)
