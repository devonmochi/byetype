
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
    >
      <div className="toggle-knob" />
    </button>
  )
}
