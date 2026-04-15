import React, { useEffect, useState } from 'react'
import { AppConfig, AudioDevice, ThemeMode } from '../../../core/types'
import {
  getLaunchAtLogin,
  setLaunchAtLogin,
  listInputDevices,
} from '../../../lib/tauri-api'
import { SettingGroup } from '../components/SettingGroup'
import { SettingRow } from '../components/SettingRow'
import { Toggle } from '../components/Toggle'

const IS_MACOS = navigator.platform.toUpperCase().includes('MAC')

function formatShortcutDisplay(combo: string): string {
  if (!IS_MACOS) return combo
  return combo
    .replace(/Command/g, '\u2318')
    .replace(/Shift/g, '\u21E7')
    .replace(/Alt/g, '\u2325')
}

interface Props {
  config: AppConfig
  onSave: (config: AppConfig) => void
}

export function GeneralTab({ config, onSave }: Props) {
  const [recording, setRecording] = useState(false)
  const [recording2, setRecording2] = useState(false)
  const [recordingExtract, setRecordingExtract] = useState(false)
  const [recordingExtract2, setRecordingExtract2] = useState(false)
  const [devices, setDevices] = useState<AudioDevice[]>([])
  const [conflictMsg, setConflictMsg] = useState('')

  // Load device list
  useEffect(() => {
    listInputDevices()
      .then(setDevices)
      .catch(e => console.error('Failed to list input devices:', e))
  }, [])

  const refreshDevices = async () => {
    try {
      const deviceList = await listInputDevices()
      setDevices(deviceList)
      // If current device is gone, switch to system-default
      const currentMic = config.general.microphone
      if (currentMic !== 'system-default' && !deviceList.some(d => d.name === currentMic)) {
        update({ microphone: 'system-default' })
      }
    } catch (e) {
      console.error('Failed to refresh devices:', e)
    }
  }

  useEffect(() => {
    getLaunchAtLogin().then(enabled => {
      if (enabled !== config.general.launchAtLogin) {
        onSave({ ...config, general: { ...config.general, launchAtLogin: enabled } })
      }
    }).catch(e => console.error('Failed to get launch at login:', e))
  }, [])

  const update = (changes: Partial<AppConfig['general']>) => {
    onSave({ ...config, general: { ...config.general, ...changes } })
  }

  const updateAdvanced = (changes: Partial<AppConfig['advanced']>) => {
    onSave({ ...config, advanced: { ...config.advanced, ...changes } })
  }

  function createKeyHandler(
    setRec: (v: boolean) => void,
    onCapture: (combo: string) => void,
    others: { key: string; label: string }[],
  ) {
    return (e: React.KeyboardEvent) => {
      e.preventDefault()
      if (e.key === 'Escape') {
        setRec(false)
        return
      }
      if (e.key === 'Tab') return
      if (['Control', 'Alt', 'Shift', 'Meta'].includes(e.key)) return

      const key = e.key === ' ' ? 'Space' : e.key
      const parts: string[] = []
      if (e.ctrlKey) parts.push('Ctrl')
      if (e.altKey) parts.push('Alt')
      if (e.shiftKey) parts.push('Shift')
      if (e.metaKey) parts.push(IS_MACOS ? 'Command' : 'Win')
      parts.push(key)
      const combo = parts.join('+')

      const conflict = others.find(o => o.key === combo)
      if (conflict) {
        setConflictMsg(`\u4E0E${conflict.label}\u51B2\u7A81`)
        setTimeout(() => setConflictMsg(''), 3000)
        setRec(false)
        return
      }

      onCapture(combo)
      setRec(false)
    }
  }

  const handleKeyDown = createKeyHandler(
    setRecording,
    (combo) => update({ shortcut: combo }),
    [
      { key: config.general.shortcut2, label: '转录模式 II' },
      { key: config.general.extractShortcut, label: '图像模式 I' },
      { key: config.general.extractShortcut2, label: '图像模式 II' },
    ],
  )

  const handleKeyDown2 = createKeyHandler(
    setRecording2,
    (combo) => update({ shortcut2: combo }),
    [
      { key: config.general.shortcut, label: '转录模式 I' },
      { key: config.general.extractShortcut, label: '图像模式 I' },
      { key: config.general.extractShortcut2, label: '图像模式 II' },
    ],
  )

  const handleExtractKeyDown = createKeyHandler(
    setRecordingExtract,
    (combo) => update({ extractShortcut: combo }),
    [
      { key: config.general.shortcut, label: '转录模式 I' },
      { key: config.general.shortcut2, label: '转录模式 II' },
      { key: config.general.extractShortcut2, label: '图像模式 II' },
    ],
  )

  const handleExtractKeyDown2 = createKeyHandler(
    setRecordingExtract2,
    (combo) => update({ extractShortcut2: combo }),
    [
      { key: config.general.shortcut, label: '转录模式 I' },
      { key: config.general.shortcut2, label: '转录模式 II' },
      { key: config.general.extractShortcut, label: '图像模式 I' },
    ],
  )

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

      <SettingGroup title="语音转录">
        <SettingRow label="转录模式 I" description="语音转录并使用模板处理">
          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <select
              className="select"
              value={config.general.shortcutTemplate}
              onChange={e => update({ shortcutTemplate: e.target.value })}
              style={{ minWidth: 100 }}
            >
              <option value="">无</option>
              {config.voiceTemplates.templates.map(t => (
                <option key={t.id} value={t.id}>{t.name}</option>
              ))}
            </select>
            <input
              className={`kbd${recording ? ' recording' : ''}`}
              value={formatShortcutDisplay(config.general.shortcut)}
              onKeyDown={recording ? handleKeyDown : undefined}
              onFocus={() => setRecording(true)}
              onBlur={() => setRecording(false)}
              readOnly
              style={{ width: 120, textAlign: 'center', cursor: 'pointer' }}
            />
          </div>
        </SettingRow>
        <SettingRow label="转录模式 II" description="语音转录并使用模板处理">
          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <select
              className="select"
              value={config.general.shortcut2Template}
              onChange={e => update({ shortcut2Template: e.target.value })}
              style={{ minWidth: 100 }}
            >
              <option value="">无</option>
              {config.voiceTemplates.templates.map(t => (
                <option key={t.id} value={t.id}>{t.name}</option>
              ))}
            </select>
            <input
              className={`kbd${recording2 ? ' recording' : ''}`}
              value={formatShortcutDisplay(config.general.shortcut2)}
              onKeyDown={recording2 ? handleKeyDown2 : undefined}
              onFocus={() => setRecording2(true)}
              onBlur={() => setRecording2(false)}
              readOnly
              style={{ width: 120, textAlign: 'center', cursor: 'pointer' }}
            />
          </div>
        </SettingRow>
      </SettingGroup>

      <SettingGroup title="图像识别">
        <SettingRow label="图像模式 I" description="截屏并使用模板识别处理">
          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <select
              className="select"
              value={config.general.extractShortcutTemplate}
              onChange={e => update({ extractShortcutTemplate: e.target.value })}
              style={{ minWidth: 100 }}
            >
              {config.extract.templates.map(t => (
                <option key={t.id} value={t.id}>{t.name}</option>
              ))}
            </select>
            <input
              className={`kbd${recordingExtract ? ' recording' : ''}`}
              value={formatShortcutDisplay(config.general.extractShortcut)}
              onKeyDown={recordingExtract ? handleExtractKeyDown : undefined}
              onFocus={() => setRecordingExtract(true)}
              onBlur={() => setRecordingExtract(false)}
              readOnly
              style={{ width: 120, textAlign: 'center', cursor: 'pointer' }}
            />
          </div>
        </SettingRow>
        <SettingRow label="图像模式 II" description="截屏并使用模板识别处理">
          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <select
              className="select"
              value={config.general.extractShortcut2Template}
              onChange={e => update({ extractShortcut2Template: e.target.value })}
              style={{ minWidth: 100 }}
            >
              {config.extract.templates.map(t => (
                <option key={t.id} value={t.id}>{t.name}</option>
              ))}
            </select>
            <input
              className={`kbd${recordingExtract2 ? ' recording' : ''}`}
              value={formatShortcutDisplay(config.general.extractShortcut2)}
              onKeyDown={recordingExtract2 ? handleExtractKeyDown2 : undefined}
              onFocus={() => setRecordingExtract2(true)}
              onBlur={() => setRecordingExtract2(false)}
              readOnly
              style={{ width: 120, textAlign: 'center', cursor: 'pointer' }}
            />
          </div>
        </SettingRow>
      </SettingGroup>

      {conflictMsg && (
        <div style={{ color: '#ff3b30', fontSize: 12, padding: '4px 16px' }}>
          {conflictMsg}
        </div>
      )}

      <SettingGroup title="其他">
        <SettingRow label="最大录音时长" description="超时自动停止并处理，单位为秒">
          <input
            type="number"
            className="input"
            value={config.general.maxRecordingSeconds}
            min={10}
            max={600}
            step={10}
            onChange={e => {
              const v = parseInt(e.target.value, 10)
              if (!isNaN(v) && v >= 10) update({ maxRecordingSeconds: v })
            }}
            style={{ width: 80, textAlign: 'center' }}
          />
        </SettingRow>
        <SettingRow label="开机自启" description="登录后自动启动 ByeType">
          <Toggle
            checked={config.general.launchAtLogin}
            onChange={async checked => {
              try {
                await setLaunchAtLogin(checked)
                update({ launchAtLogin: checked })
              } catch (e) {
                console.error('Failed to set launch at login:', e)
              }
            }}
          />
        </SettingRow>
      </SettingGroup>

      <SettingGroup title="麦克风">
        <SettingRow label="输入设备" description="选择用于语音输入的麦克风">
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <select
              className="select"
              value={config.general.microphone}
              onChange={e => update({ microphone: e.target.value })}
              style={{ maxWidth: 200 }}
            >
              {devices.map(d => (
                <option key={d.name} value={d.name}>
                  {d.name === 'system-default'
                    ? '系统默认'
                    : `${d.name}${d.isDefault ? ' (默认)' : ''}`}
                </option>
              ))}
            </select>
            <button
              className="file-picker-btn"
              onClick={refreshDevices}
              title="刷新设备列表"
            >
              刷新
            </button>
          </div>
        </SettingRow>
      </SettingGroup>

      <SettingGroup title="网络与性能">
        <SettingRow label="转写超时时间" description="单位：秒">
          <input
            className="input"
            type="number"
            value={config.advanced.transcribeTimeout}
            onChange={e => updateAdvanced({ transcribeTimeout: Number(e.target.value) })}
            min={1}
            style={{ width: 100 }}
          />
        </SettingRow>
        <SettingRow label="文本优化超时时间" description="单位：秒">
          <input
            className="input"
            type="number"
            value={config.advanced.optimizeTimeout}
            onChange={e => updateAdvanced({ optimizeTimeout: Number(e.target.value) })}
            min={1}
            style={{ width: 100 }}
          />
        </SettingRow>
        <SettingRow label="最大重试次数">
          <input
            className="input"
            type="number"
            value={config.advanced.maxRetries}
            onChange={e => updateAdvanced({ maxRetries: Number(e.target.value) })}
            min={0}
            style={{ width: 100 }}
          />
        </SettingRow>
        <SettingRow label="最大并行任务数">
          <input
            className="input"
            type="number"
            value={config.advanced.maxParallel}
            onChange={e => updateAdvanced({ maxParallel: Number(e.target.value) })}
            min={1}
            style={{ width: 100 }}
          />
        </SettingRow>
        <SettingRow label="HTTP 代理" description="用于 Gemini 等需要代理的服务">
          <Toggle
            checked={config.advanced.proxyEnabled}
            onChange={checked => updateAdvanced({ proxyEnabled: checked })}
          />
        </SettingRow>
        {config.advanced.proxyEnabled && (
          <SettingRow label="代理地址">
            <input
              className="input input-wide"
              value={config.advanced.proxyUrl}
              onChange={e => updateAdvanced({ proxyUrl: e.target.value })}
              placeholder="http://127.0.0.1:10809"
            />
          </SettingRow>
        )}
      </SettingGroup>
    </div>
  )
}
