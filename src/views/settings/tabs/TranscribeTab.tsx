import { AppConfig, ThinkingConfig } from '../../../core/types'
import { SettingGroup } from '../components/SettingGroup'
import { SettingRow } from '../components/SettingRow'
import { Toggle } from '../components/Toggle'
import { PromptsTab } from './PromptsTab'

interface Props {
  config: AppConfig
  onSave: (config: AppConfig) => void
}

const MODELS = [
  { value: 'gemini-3-flash-preview', label: 'Gemini 3.0 Flash' },
  { value: 'gemini-3.1-flash-lite-preview', label: 'Gemini 3.1 Flash Lite' },
  { value: 'qwen3-omni-flash', label: 'Qwen3 Omni Flash' },
]

export function TranscribeTab({ config, onSave }: Props) {
  const { transcribe, optimize } = config
  const isQwen = transcribe.model === 'qwen3-omni-flash'

  const updateTranscribe = (changes: Partial<AppConfig['transcribe']>) => {
    onSave({ ...config, transcribe: { ...transcribe, ...changes } })
  }

  const updateTranscribeThinking = (changes: Partial<AppConfig['transcribe']['thinking']>) => {
    updateTranscribe({ thinking: { ...transcribe.thinking, ...changes } })
  }

  const updateOptimize = (changes: Partial<AppConfig['optimize']>) => {
    onSave({ ...config, optimize: { ...optimize, ...changes } })
  }

  const updateOpenAI = (changes: Partial<AppConfig['optimize']['openaiCompat']>) => {
    updateOptimize({ openaiCompat: { ...optimize.openaiCompat, ...changes } })
  }

  const updateOptimizeThinking = (changes: Partial<ThinkingConfig>) => {
    updateOptimize({ thinking: { ...optimize.thinking, ...changes } })
  }

  return (
    <div>
      <h2 className="content-title">语音转写</h2>

      {/* 区域一：转写模型 */}
      <SettingGroup title="模型">
        <SettingRow label="转写模型">
          <select
            className="select"
            value={transcribe.model}
            onChange={e => updateTranscribe({ model: e.target.value as typeof transcribe.model })}
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
            onChange={e => updateTranscribe({ geminiApiKey: e.target.value })}
            placeholder="AIzaSy..."
          />
        </SettingRow>
        <SettingRow label="阿里云 Qwen API Key">
          <input
            className="input input-wide"
            type="password"
            value={transcribe.qwenApiKey}
            onChange={e => updateTranscribe({ qwenApiKey: e.target.value })}
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
                onChange={checked => updateTranscribeThinking({ enabled: checked })}
              />
            </SettingRow>
            {transcribe.thinking.enabled && (
              <SettingRow label="Thinking Level" description="思考深度级别">
                <select
                  className="select"
                  value={transcribe.thinking.level}
                  onChange={e => updateTranscribeThinking({ level: e.target.value as typeof transcribe.thinking.level })}
                  style={{ width: 120 }}
                >
                  <option value="MINIMAL">MINIMAL</option>
                  <option value="LOW">LOW</option>
                  <option value="MEDIUM">MEDIUM</option>
                  <option value="HIGH">HIGH</option>
                </select>
              </SettingRow>
            )}
          </>
        )}
      </SettingGroup>

      {/* 区域二：文本优化 */}
      <h3 className="section-title">文本优化</h3>

      <SettingGroup>
        <SettingRow label="启用文本优化" description="转写后自动优化文本格式和表达">
          <Toggle
            checked={optimize.enabled}
            onChange={checked => updateOptimize({ enabled: checked })}
          />
        </SettingRow>
      </SettingGroup>

      {optimize.enabled && (
        <>
          <SettingGroup title="模型类型">
            <SettingRow label="优化引擎">
              <div style={{ display: 'flex', gap: 16 }}>
                <label style={{ display: 'flex', alignItems: 'center', gap: 4, fontSize: 13, color: 'var(--text-primary)', cursor: 'pointer' }}>
                  <input type="radio" value="openai-compat" checked={optimize.type === 'openai-compat'} onChange={() => updateOptimize({ type: 'openai-compat' })} />
                  OpenAI 兼容
                </label>
                <label style={{ display: 'flex', alignItems: 'center', gap: 4, fontSize: 13, color: 'var(--text-primary)', cursor: 'pointer' }}>
                  <input type="radio" value="gemini" checked={optimize.type === 'gemini'} onChange={() => updateOptimize({ type: 'gemini' })} />
                  Gemini
                </label>
              </div>
            </SettingRow>
          </SettingGroup>

          {optimize.type === 'openai-compat' ? (
            <SettingGroup title="OpenAI 兼容配置">
              <SettingRow label="Provider 名称">
                <input className="input" value={optimize.openaiCompat.providerName} onChange={e => updateOpenAI({ providerName: e.target.value })} placeholder="DeepSeek" style={{ width: 200 }} />
              </SettingRow>
              <SettingRow label="Base URL">
                <input className="input input-wide" value={optimize.openaiCompat.baseUrl} onChange={e => updateOpenAI({ baseUrl: e.target.value })} placeholder="https://api.deepseek.com/v1" />
              </SettingRow>
              <SettingRow label="Model">
                <input className="input" value={optimize.openaiCompat.model} onChange={e => updateOpenAI({ model: e.target.value })} placeholder="deepseek-chat" style={{ width: 200 }} />
              </SettingRow>
              <SettingRow label="API Key">
                <input className="input input-wide" type="password" value={optimize.openaiCompat.apiKey} onChange={e => updateOpenAI({ apiKey: e.target.value })} />
              </SettingRow>
            </SettingGroup>
          ) : (
            <SettingGroup title="Gemini 配置">
              <SettingRow label="Gemini 模型" description="复用转写设置中的 API Key">
                <select className="select" value={optimize.geminiModel} onChange={e => updateOptimize({ geminiModel: e.target.value })} style={{ width: 200 }}>
                  <option value="gemini-3-flash-preview">Gemini 3.0 Flash</option>
                  <option value="gemini-3.1-flash-lite-preview">Gemini 3.1 Flash Lite</option>
                </select>
              </SettingRow>
              <SettingRow label="启用思考" description="让模型在优化前先进行推理">
                <Toggle
                  checked={optimize.thinking.enabled}
                  onChange={checked => updateOptimizeThinking({ enabled: checked })}
                />
              </SettingRow>
              {optimize.thinking.enabled && (
                <SettingRow label="Thinking Level" description="思考深度级别">
                  <select
                    className="select"
                    value={optimize.thinking.level}
                    onChange={e => updateOptimizeThinking({ level: e.target.value as ThinkingConfig['level'] })}
                    style={{ width: 120 }}
                  >
                    <option value="MINIMAL">MINIMAL</option>
                    <option value="LOW">LOW</option>
                    <option value="MEDIUM">MEDIUM</option>
                    <option value="HIGH">HIGH</option>
                  </select>
                </SettingRow>
              )}
            </SettingGroup>
          )}
        </>
      )}

      {/* 区域三：提示词 */}
      <h3 className="section-title">提示词</h3>
      <PromptsTab config={config} onSave={onSave} />
    </div>
  )
}
