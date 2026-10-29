import React, { useEffect, useRef, useState } from 'react'
import { AppConfig, AudioDevice, LocalApiStatus, ThemeMode } from '../../../core/types'
import {
  getLaunchAtLogin,
  getLocalApiStatus,
  onEvent,
  setLaunchAtLogin,
  listInputDevices,
} from '../../../lib/tauri-api'
import { SettingGroup } from '../components/SettingGroup'
import { SettingRow } from '../components/SettingRow'
import { Toggle } from '../components/Toggle'

/** 语音入口固定名称，不再允许自定义 */
const VOICE_PLAIN_LABEL = '语音输入'
const VOICE_LABEL_1 = '语音输入 1'
const VOICE_LABEL_2 = '语音输入 2'
/** 图像入口固定名称 */
const IMAGE_LABEL_1 = '图片识别1'
const IMAGE_LABEL_2 = '图片识别2'

const IS_MACOS = navigator.platform.toUpperCase().includes('MAC')

function formatShortcutDisplay(combo: string): string {
  if (!IS_MACOS) return combo === 'AltRight' ? '右 Alt' : combo
  return combo
    .replace(/Command/g, '\u2318')
    .replace(/Shift/g, '\u21E7')
    .replace(/Alt/g, '\u2325')
}

interface ShortcutInputProps {
  label: string
  value: string
  recording: boolean
  onRecordingChange: (recording: boolean) => void
  onKeyDown: (e: React.KeyboardEvent<HTMLInputElement>) => void
  onKeyUp: (e: React.KeyboardEvent<HTMLInputElement>) => void
}

function ShortcutInput({ label, value, recording, onRecordingChange, onKeyDown, onKeyUp }: ShortcutInputProps) {
  return (
    <input
      className={`kbd${recording ? ' recording' : ''}`}
      value={formatShortcutDisplay(value)}
      onKeyDown={recording ? onKeyDown : undefined}
      onKeyUp={recording ? onKeyUp : undefined}
      onFocus={() => onRecordingChange(true)}
      onBlur={() => onRecordingChange(false)}
      readOnly
      placeholder="未设置"
      aria-label={`修改${label}快捷键`}
      style={{ width: 120, textAlign: 'center', cursor: 'pointer' }}
    />
  )
}

interface TemplateSelectProps {
  value: string
  templates: { id: string; name: string }[]
  onChange: (templateId: string) => void
}

function TemplateSelect({ value, templates, onChange }: TemplateSelectProps) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
      <span style={{ fontSize: 12, color: 'var(--text-secondary)', whiteSpace: 'nowrap' }}>输出风格</span>
      <select
        className="select"
        value={value}
        onChange={e => onChange(e.target.value)}
        style={{ minWidth: 100 }}
      >
        {templates.map(t => (
          <option key={t.id} value={t.id}>{t.name}</option>
        ))}
      </select>
    </div>
  )
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
  const [localApiStatus, setLocalApiStatus] = useState<LocalApiStatus>({
    running: false,
    port: null,
    error: null,
  })
  const [curlCopied, setCurlCopied] = useState(false)

  // Load device list
  useEffect(() => {
    listInputDevices()
      .then(setDevices)
      .catch(e => console.error('Failed to list input devices:', e))
  }, [])

  useEffect(() => {
    let disposed = false
    let unsubscribe: (() => void) | undefined
    getLocalApiStatus()
      .then(status => { if (!disposed) setLocalApiStatus(status) })
      .catch(error => console.error('Failed to get local API status:', error))
    onEvent<LocalApiStatus>('local-api-status', status => {
      if (!disposed) setLocalApiStatus(status)
    }).then(fn => {
      if (disposed) fn()
      else unsubscribe = fn
    })
    return () => {
      disposed = true
      unsubscribe?.()
    }
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

  const curlCommand = `curl -fsS -X POST --data-binary @recording.m4a -H 'Content-Type: audio/mp4' 'http://127.0.0.1:${config.localApi.port}/transcribe'`

  const strengtheningEnabled = config.general.textStrengtheningEnabled !== false
  const voiceTemplates = config.voiceTemplates.templates ?? []

  // 文本优化关闭时第二个语音快捷键不注册，也不该再挡住其他快捷键，
  // 与 Rust 侧 register 的冲突检测保持一致。
  const secondVoiceConflict = (): { key: string; label: string }[] =>
    strengtheningEnabled && config.general.shortcut2
      ? [{ key: config.general.shortcut2, label: VOICE_LABEL_2 }]
      : []

  const rightAltField = useRef<string | null>(null)

  function createKeyHandler(
    field: string,
    setRec: (v: boolean) => void,
    onCapture: (combo: string) => void,
    others: { key: string; label: string }[],
  ) {
    const commit = (combo: string, input: HTMLInputElement) => {
      const conflict = combo && others.find(o => o.key === combo)
      if (conflict) {
        setConflictMsg(`\u4E0E${conflict.label}\u51B2\u7A81`)
        setTimeout(() => setConflictMsg(''), 3000)
        setRec(false)
        input.blur()
        return
      }

      onCapture(combo)
      setRec(false)
      input.blur()
    }

    const onKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
      e.preventDefault()
      if (e.key === 'Escape') {
        rightAltField.current = null
        setRec(false)
        e.currentTarget.blur()
        return
      }
      if (e.key === 'Tab') return
      if (e.key === 'Backspace') {
        rightAltField.current = null
        commit('', e.currentTarget)
        return
      }
      if (!IS_MACOS && e.code === 'AltRight') {
        rightAltField.current = field
        return
      }
      if (['Control', 'Alt', 'Shift', 'Meta', 'AltGraph'].includes(e.key)) return

      rightAltField.current = null
      const key = e.key === ' ' ? 'Space' : e.key
      const parts: string[] = []
      if (e.ctrlKey) parts.push('Ctrl')
      if (e.altKey) parts.push('Alt')
      if (e.shiftKey) parts.push('Shift')
      if (e.metaKey) parts.push(IS_MACOS ? 'Command' : 'Win')
      parts.push(key)
      commit(parts.join('+'), e.currentTarget)
    }

    const onKeyUp = (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (IS_MACOS || e.code !== 'AltRight' || rightAltField.current !== field) return
      e.preventDefault()
      rightAltField.current = null
      commit('AltRight', e.currentTarget)
    }

    return { onKeyDown, onKeyUp }
  }

  const { onKeyDown: handleKeyDown, onKeyUp: handleKeyUp } = createKeyHandler(
    'shortcut',
    setRecording,
    (combo) => update({ shortcut: combo }),
    [
      ...secondVoiceConflict(),
      { key: config.general.extractShortcut, label: IMAGE_LABEL_1 },
      { key: config.general.extractShortcut2, label: IMAGE_LABEL_2 },
    ],
  )

  const { onKeyDown: handleKeyDown2, onKeyUp: handleKeyUp2 } = createKeyHandler(
    'shortcut2',
    setRecording2,
    (combo) => update({ shortcut2: combo }),
    [
      { key: config.general.shortcut, label: VOICE_LABEL_1 },
      { key: config.general.extractShortcut, label: IMAGE_LABEL_1 },
      { key: config.general.extractShortcut2, label: IMAGE_LABEL_2 },
    ],
  )

  const { onKeyDown: handleExtractKeyDown, onKeyUp: handleExtractKeyUp } = createKeyHandler(
    'extractShortcut',
    setRecordingExtract,
    (combo) => update({ extractShortcut: combo }),
    [
      { key: config.general.shortcut, label: VOICE_LABEL_1 },
      ...secondVoiceConflict(),
      { key: config.general.extractShortcut2, label: IMAGE_LABEL_2 },
    ],
  )

  const { onKeyDown: handleExtractKeyDown2, onKeyUp: handleExtractKeyUp2 } = createKeyHandler(
    'extractShortcut2',
    setRecordingExtract2,
    (combo) => update({ extractShortcut2: combo }),
    [
      { key: config.general.shortcut, label: VOICE_LABEL_1 },
      ...secondVoiceConflict(),
      { key: config.general.extractShortcut, label: IMAGE_LABEL_1 },
    ],
  )

  // 开关只切换 enabled，不改任何模板选择与快捷键；
  // 旧配置里「输出风格选无」存的是空模板，打开时补一个可用默认项。
  const setStrengthening = (enabled: boolean) => {
    if (!enabled) {
      update({ textStrengtheningEnabled: false })
      return
    }
    update({
      textStrengtheningEnabled: true,
      shortcutTemplate: config.general.shortcutTemplate || voiceTemplates[0]?.id || '',
      shortcut2Template:
        config.general.shortcut2Template || voiceTemplates[1]?.id || voiceTemplates[0]?.id || '',
    })
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

      <SettingGroup title="语音输入">
        {!strengtheningEnabled && (
          <SettingRow label={VOICE_PLAIN_LABEL}>
            <ShortcutInput
              label={VOICE_PLAIN_LABEL}
              value={config.general.shortcut}
              recording={recording}
              onRecordingChange={setRecording}
              onKeyDown={handleKeyDown}
              onKeyUp={handleKeyUp}
            />
          </SettingRow>
        )}
        <SettingRow
          label={
            <span className="mode-label">
              文本优化
              <span className="strengthening-formula">（识别文本＋规则＝最终文本）</span>
            </span>
          }
        >
          <Toggle checked={strengtheningEnabled} onChange={setStrengthening} />
        </SettingRow>
        {strengtheningEnabled && (
          <div className="strengthened-voice-inputs">
            <SettingRow label={VOICE_LABEL_1}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                <TemplateSelect
                  value={config.general.shortcutTemplate}
                  templates={voiceTemplates}
                  onChange={templateId => update({ shortcutTemplate: templateId })}
                />
                <ShortcutInput
                  label={VOICE_LABEL_1}
                  value={config.general.shortcut}
                  recording={recording}
                  onRecordingChange={setRecording}
                  onKeyDown={handleKeyDown}
                  onKeyUp={handleKeyUp}
                />
              </div>
            </SettingRow>
            <SettingRow label={VOICE_LABEL_2}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                <TemplateSelect
                  value={config.general.shortcut2Template}
                  templates={voiceTemplates}
                  onChange={templateId => update({ shortcut2Template: templateId })}
                />
                <ShortcutInput
                  label={VOICE_LABEL_2}
                  value={config.general.shortcut2}
                  recording={recording2}
                  onRecordingChange={setRecording2}
                  onKeyDown={handleKeyDown2}
                  onKeyUp={handleKeyUp2}
                />
              </div>
            </SettingRow>
          </div>
        )}
        <SettingRow label="按住说话模式" description="开启后按住快捷键期间录音，松开立即识别">
          <Toggle
            checked={!!config.general.pttMode}
            onChange={checked => update({ pttMode: checked })}
          />
        </SettingRow>
        <SettingRow
          label="覆盖剪贴板"
          description="开启后识别结果会写入剪贴板，覆盖你之前复制的内容；关闭后识别完成会自动还原原剪贴板"
        >
          <Toggle
            checked={config.general.overwriteClipboard !== false}
            onChange={checked => update({ overwriteClipboard: checked })}
          />
        </SettingRow>
      </SettingGroup>

      <SettingGroup title="图像识别">
        <SettingRow label={IMAGE_LABEL_1}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <TemplateSelect
              value={config.general.extractShortcutTemplate}
              templates={config.extract.templates}
              onChange={templateId => update({ extractShortcutTemplate: templateId })}
            />
            <ShortcutInput
              label={IMAGE_LABEL_1}
              value={config.general.extractShortcut}
              recording={recordingExtract}
              onRecordingChange={setRecordingExtract}
              onKeyDown={handleExtractKeyDown}
              onKeyUp={handleExtractKeyUp}
            />
          </div>
        </SettingRow>
        <SettingRow label={IMAGE_LABEL_2}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <TemplateSelect
              value={config.general.extractShortcut2Template}
              templates={config.extract.templates}
              onChange={templateId => update({ extractShortcut2Template: templateId })}
            />
            <ShortcutInput
              label={IMAGE_LABEL_2}
              value={config.general.extractShortcut2}
              recording={recordingExtract2}
              onRecordingChange={setRecordingExtract2}
              onKeyDown={handleExtractKeyDown2}
              onKeyUp={handleExtractKeyUp2}
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
              if (!isNaN(v) && v >= 10 && v <= 600) update({ maxRecordingSeconds: v })
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
        <SettingRow
          label="本机转写接口"
          description="允许本机程序通过HTTP提交音频，结果以纯文本返回"
        >
          <Toggle
            checked={config.localApi.enabled}
            onChange={enabled => onSave({ ...config, localApi: { ...config.localApi, enabled } })}
          />
        </SettingRow>
        <SettingRow label="本机接口端口" description="仅监听127.0.0.1，不接受局域网访问">
          <input
            className="input"
            type="number"
            value={config.localApi.port}
            min={1024}
            max={65535}
            onChange={event => {
              const port = Number(event.target.value)
              if (Number.isInteger(port) && port >= 1024 && port <= 65535) {
                onSave({ ...config, localApi: { ...config.localApi, port } })
              }
            }}
            style={{ width: 100 }}
          />
        </SettingRow>
        <SettingRow label="接口状态">
          <span style={{
            color: localApiStatus.error
              ? '#ff3b30'
              : localApiStatus.running
                ? '#34c759'
                : 'var(--text-secondary)',
            fontSize: 12,
            maxWidth: 360,
            display: 'inline-block',
            textAlign: 'right',
          }}>
            {localApiStatus.error
              ? localApiStatus.error
              : localApiStatus.running
                ? `运行中：127.0.0.1:${localApiStatus.port}`
                : '未开启'}
          </span>
        </SettingRow>
        <SettingRow label="curl示例" description="文件名和Content-Type按实际音频格式修改">
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, maxWidth: 440 }}>
            <code style={{
              fontSize: 11,
              color: 'var(--text-secondary)',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
              maxWidth: 340,
            }} title={curlCommand}>{curlCommand}</code>
            <button
              className="file-picker-btn"
              onClick={async () => {
                await navigator.clipboard.writeText(curlCommand)
                setCurlCopied(true)
                setTimeout(() => setCurlCopied(false), 1500)
              }}
            >
              {curlCopied ? '已复制' : '复制'}
            </button>
          </div>
        </SettingRow>
        <SettingRow label="转写超时时间" description="单位：秒">
          <input
            className="input"
            type="number"
            value={config.advanced.transcribeTimeout}
            onChange={e => {
              const v = Number(e.target.value)
              if (e.target.value === '' || !Number.isFinite(v) || v < 1) return
              updateAdvanced({ transcribeTimeout: v })
            }}
            min={1}
            style={{ width: 100 }}
          />
        </SettingRow>
        <SettingRow label="文本优化超时时间" description="单位：秒">
          <input
            className="input"
            type="number"
            value={config.advanced.optimizeTimeout}
            onChange={e => {
              const v = Number(e.target.value)
              if (e.target.value === '' || !Number.isFinite(v) || v < 1) return
              updateAdvanced({ optimizeTimeout: v })
            }}
            min={1}
            style={{ width: 100 }}
          />
        </SettingRow>
        <SettingRow label="最大重试次数">
          <input
            className="input"
            type="number"
            value={config.advanced.maxRetries}
            onChange={e => {
              const v = Number(e.target.value)
              if (e.target.value === '' || !Number.isFinite(v) || v < 0) return
              updateAdvanced({ maxRetries: v })
            }}
            min={0}
            style={{ width: 100 }}
          />
        </SettingRow>
        <SettingRow label="最大并行任务数">
          <input
            className="input"
            type="number"
            value={config.advanced.maxParallel}
            onChange={e => {
              const v = Number(e.target.value)
              if (e.target.value === '' || !Number.isFinite(v) || v < 1) return
              updateAdvanced({ maxParallel: v })
            }}
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
