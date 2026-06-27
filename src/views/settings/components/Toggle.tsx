
interface ToggleProps {
  checked: boolean
  onChange: (checked: boolean) => void
  disabled?: boolean
}

export function Toggle({ checked, onChange, disabled }: ToggleProps) {
  return (
    <button
      className={`toggle${checked ? ' on' : ''}${disabled ? ' disabled' : ''}`}
      onClick={() => !disabled && onChange(!checked)}
      type="button"
      disabled={disabled}
      aria-disabled={disabled}
    >
      <div className="toggle-knob" />
    </button>
  )
}
