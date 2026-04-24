import { useEffect, useRef, useState } from 'react'

interface EditableLabelProps {
  /** 当前展示的名字（自定义名 ?? 默认名）。 */
  value: string
  /** 默认名，用户清空输入框时回落到这里（通过 onChange(undefined)）。 */
  defaultValue: string
  /** 用户提交修改时触发；undefined 表示「恢复默认」。 */
  onChange: (next: string | undefined) => void
}

export function EditableLabel({ value, defaultValue, onChange }: EditableLabelProps) {
  const [editing, setEditing] = useState(false)
  const [draft, setDraft] = useState(value)
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    if (editing) {
      setDraft(value)
      requestAnimationFrame(() => {
        inputRef.current?.focus()
        const len = inputRef.current?.value.length ?? 0
        inputRef.current?.setSelectionRange(len, len)
      })
    }
  }, [editing, value])

  const commit = () => {
    const trimmed = draft.trim()
    if (trimmed === '' || trimmed === defaultValue) {
      onChange(undefined)
    } else if (trimmed !== value) {
      onChange(trimmed)
    }
    setEditing(false)
  }

  const cancel = () => {
    setDraft(value)
    setEditing(false)
  }

  if (editing) {
    return (
      <input
        ref={inputRef}
        className="setting-row-label setting-row-label-input"
        value={draft}
        maxLength={20}
        onChange={e => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={e => {
          if (e.key === 'Enter') {
            e.preventDefault()
            commit()
          } else if (e.key === 'Escape') {
            e.preventDefault()
            cancel()
          }
        }}
      />
    )
  }

  return (
    <div
      className="setting-row-label setting-row-label-editable"
      title="点击重命名（清空恢复默认）"
      onClick={() => setEditing(true)}
    >
      {value}
    </div>
  )
}
