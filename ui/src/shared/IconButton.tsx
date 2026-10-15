import type { ButtonHTMLAttributes, ReactNode } from 'react'

interface Props extends ButtonHTMLAttributes<HTMLButtonElement> {
  label: string
  children: ReactNode
}

export const IconButton = ({ label, children, className = '', ...rest }: Props) => (
  <button
    type="button"
    title={label}
    aria-label={label}
    className={`inline-flex h-6 w-6 items-center justify-center rounded text-mist transition-colors hover:bg-twilight/70 hover:text-moon disabled:opacity-40 disabled:hover:bg-transparent ${className}`}
    {...rest}
  >
    {children}
  </button>
)
