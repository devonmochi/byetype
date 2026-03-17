import React, { useState } from 'react'
import { AppConfig, ThemeMode } from '../../../core/types'
import { SettingGroup } from '../components/SettingGroup'
import { SettingRow } from '../components/SettingRow'
import { Toggle } from '../components/Toggle'

interface Props {
  config: AppConfig
  onSave: (config: AppConfig) => void
}

export function GeneralTab({ config, onSave }: Props) {
  const [recording, setRecording] = useState(false)

  const update = (changes: Partial<AppConfig['general']>) => {
    onSave({ ...config, general: { ...config.general, ...changes } })
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (!recording) return
    e.preventDefault()
    const key = e.key === ' ' ? 'Space' : e.key
    const parts: string[] = []
    if (e.ctrlKey) parts.push('Ctrl')
    if (e.altKey) parts.push('Alt')
    if (e.shiftKey) parts.push('Shift')
    if (e.metaKey) parts.push('Command')
    if (!['Control', 'Alt', 'Shift', 'Meta'].includes(e.key)) parts.push(key)
    const combo = parts.join('+')
    update({ shortcut: combo })
    setRecording(false)
  }

  const themes: { value: ThemeMode; label: string; style: React.CSSProperties }[] = [
    { value: 'light', label: '浅色', style: { background: '#ffffff', border: '1px solid #d2d2d7' } },
    { value: 'dark', label: '深色', style: { background: '#1c1c1e' } },
    { value: 'system', label: '自动', style: { background: 'linear-gradient(to right, #ffffff 50%, #1c1c1e 50%)' } },
  ]

  return (
    <div>
      <h2 className="content-title">通用设置</h2>

      <SettingGroup title="外观">
        <div style={{ padding: '12px 16px' }}>
          <div className="appearance-options">
            {themes.map(t => (
              <button
                key={t.value}
                className={`appearance-option${config.general.theme === t.value ? ' active' : ''}`}
                onClick={() => update({ theme: t.value })}
              >
                <div className="appearance-preview" style={t.style} />
                <div className="appearance-label">{t.label}</div>
              </button>
            ))}
          </div>
        </div>
      </SettingGroup>

      <SettingGroup title="通用">
        <SettingRow label="录音快捷键" description={recording ? '请按下快捷键...' : '点击后按下新快捷键'}>
          <input
            className={`kbd${recording ? ' recording' : ''}`}
            value={config.general.shortcut}
            onKeyDown={handleKeyDown}
            onFocus={() => setRecording(true)}
            onBlur={() => setRecording(false)}
            readOnly
            style={{ width: 120, textAlign: 'center', cursor: 'pointer' }}
          />
        </SettingRow>
        <SettingRow label="开机自启" description="登录后自动启动 ByeType">
          <Toggle
            checked={config.general.launchAtLogin}
            onChange={checked => update({ launchAtLogin: checked })}
          />
        </SettingRow>
      </SettingGroup>
    </div>
  )
}
