import React from 'react'
import { AppConfig } from '../../../core/types'
import { SettingGroup } from '../components/SettingGroup'
import { SettingRow } from '../components/SettingRow'
import { Toggle } from '../components/Toggle'


interface Props {
  config: AppConfig
  onSave: (config: AppConfig) => void
}

const MODELS = [
  { value: 'gemini-2.5-flash', label: 'Gemini 2.5 Flash' },
  { value: 'gemini-3-flash-preview', label: 'Gemini 3.0 Flash' },
  { value: 'gemini-3.1-flash-lite-preview', label: 'Gemini 3.1 Flash Lite' },
  { value: 'qwen3-omni-flash', label: 'Qwen3 Omni Flash' },
]

export function TranscribeTab({ config, onSave }: Props) {
  const { transcribe } = config
  const isQwen = transcribe.model === 'qwen3-omni-flash'
  const is25Flash = transcribe.model === 'gemini-2.5-flash'

  const update = (changes: Partial<AppConfig['transcribe']>) => {
    onSave({ ...config, transcribe: { ...transcribe, ...changes } })
  }

  const updateThinking = (changes: Partial<AppConfig['transcribe']['thinking']>) => {
    update({ thinking: { ...transcribe.thinking, ...changes } })
  }

  return (
    <div>
      <h2 className="content-title">转写设置</h2>

      <SettingGroup title="模型">
        <SettingRow label="转写模型">
          <select
            className="select"
            value={transcribe.model}
            onChange={e => update({ model: e.target.value as typeof transcribe.model })}
            style={{ width: 200 }}
          >
            {MODELS.map(m => <option key={m.value} value={m.value}>{m.label}</option>)}
          </select>
        </SettingRow>
      </SettingGroup>

      <SettingGroup title="API 密钥">
        <SettingRow label="Google Gemini API Key">
          <input
            className="input input-wide"
            type="password"
            value={transcribe.geminiApiKey}
            onChange={e => update({ geminiApiKey: e.target.value })}
            placeholder="AIzaSy..."
          />
        </SettingRow>
        <SettingRow label="阿里云 Qwen API Key">
          <input
            className="input input-wide"
            type="password"
            value={transcribe.qwenApiKey}
            onChange={e => update({ qwenApiKey: e.target.value })}
            placeholder="sk-..."
          />
        </SettingRow>
      </SettingGroup>

      <SettingGroup title="思考模式">
        {isQwen ? (
          <div style={{ padding: '12px 16px', color: 'var(--text-tertiary)', fontSize: 13 }}>
            当前模型不支持思考模式
          </div>
        ) : (
          <>
            <SettingRow label="启用思考" description="让模型在转写前先进行推理">
              <Toggle
                checked={transcribe.thinking.enabled}
                onChange={checked => updateThinking({ enabled: checked })}
              />
            </SettingRow>
            {transcribe.thinking.enabled && (
              is25Flash ? (
                <SettingRow label="Thinking Budget" description="思考 token 数量">
                  <input
                    className="input"
                    type="number"
                    value={transcribe.thinking.budget}
                    onChange={e => updateThinking({ budget: Number(e.target.value) })}
                    min={0}
                    style={{ width: 120 }}
                  />
                </SettingRow>
              ) : (
                <SettingRow label="Thinking Level" description="思考深度级别">
                  <select
                    className="select"
                    value={transcribe.thinking.level}
                    onChange={e => updateThinking({ level: e.target.value as typeof transcribe.thinking.level })}
                    style={{ width: 120 }}
                  >
                    <option value="MINIMAL">MINIMAL</option>
                    <option value="LOW">LOW</option>
                    <option value="MEDIUM">MEDIUM</option>
                    <option value="HIGH">HIGH</option>
                  </select>
                </SettingRow>
              )
            )}
          </>
        )}
      </SettingGroup>

    </div>
  )
}
