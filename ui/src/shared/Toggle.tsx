interface Props {
  label: string
  checked: boolean
  disabled?: boolean
  onChange?: (checked: boolean) => void
}

export const Toggle = ({ label, checked, disabled, onChange }: Props) => (
  <label
    className={`flex items-center justify-between gap-3 py-1 ${disabled ? 'opacity-45' : 'cursor-pointer'}`}
  >
    <span>{label}</span>
    <input
      type="checkbox"
      role="switch"
      className="peer sr-only"
      checked={checked}
      disabled={disabled}
      onChange={(event) => onChange?.(event.target.checked)}
    />
    <span
      aria-hidden
      className="relative h-4 w-8 rounded-full bg-twilight transition-colors peer-checked:bg-ember/80 peer-focus-visible:outline-2 peer-focus-visible:outline-ember"
    >
      <span
        className={`absolute top-0.5 h-3 w-3 rounded-full bg-moon transition-all ${checked ? 'left-[18px]' : 'left-0.5'}`}
      />
    </span>
  </label>
)
