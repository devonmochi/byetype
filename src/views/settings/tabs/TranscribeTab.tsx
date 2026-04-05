import type { AppConfig, ThinkingConfig } from '../../../core/types'
import { getAudioModels, getTextModels, findModel } from '../../../core/models'
import { SettingGroup } from '../components/SettingGroup'
import { SettingRow } from '../components/SettingRow'
import { Toggle } from '../components/Toggle'

interface PresetConfig {
  id: string
  title: string
  desc: string
  tag: string
  transcribeModelId: string
  optimizeModelId: string
}

const PRESETS: PresetConfig[] = [
  {
    id: 'domestic-best',
    title: '⚡ 效果最好',
    desc: 'Qwen 3.5 Omni Plus 全能处理',
    tag: '无需代理',
    transcribeModelId: 'builtin-qwen-omni-plus',
    optimizeModelId: 'builtin-qwen-omni-plus',
  },
  {
    id: 'domestic-lite',
    title: '🚀 极速轻量',
    desc: 'Qwen 3.5 Omni Flash 极速处理',
    tag: '无需代理',
    transcribeModelId: 'builtin-qwen-omni-flash',
    optimizeModelId: 'builtin-qwen-omni-flash',
  },
  {
    id: 'best',
    title: '⚡ 效果最好',
    desc: 'Gemini 3 Flash 全能处理',
    tag: '需代理',
    transcribeModelId: 'builtin-gemini-3-flash',
    optimizeModelId: 'builtin-gemini-3-flash',
  },
  {
    id: 'lite',
    title: '🚀 极速轻量',
    desc: 'Gemini 3.1 Flash Lite 极速处理',
    tag: '需代理',
    transcribeModelId: 'builtin-gemini-3.1-flash-lite',
    optimizeModelId: 'builtin-gemini-3.1-flash-lite',
  },
  {
    id: 'openrouter',
    title: '🌐 OpenRouter',
    desc: 'Gemini 3 Flash via OpenRouter',
    tag: '需代理',
    transcribeModelId: 'builtin-or-gemini-3-flash',
    optimizeModelId: 'builtin-or-qwen3.6-plus-free',
  },
]

interface Props {
  config: AppConfig
  onSave: (config: AppConfig) => void
}

export function TranscribeTab({ config, onSave }: Props) {
  const { transcribe, optimize } = config

  const audioModels = getAudioModels(config)
  const textModels = getTextModels(config)
  const transcribeModel = findModel(config, transcribe.modelId)
  const optimizeModel = findModel(config, optimize.modelId)
  const isTranscribeGemini = transcribeModel?.protocol === 'gemini'
  const isOptimizeGemini = optimizeModel?.protocol === 'gemini'

  const updateTranscribe = (changes: Partial<AppConfig['transcribe']>) => {
    onSave({ ...config, transcribe: { ...transcribe, ...changes } })
  }

  const updateTranscribeThinking = (changes: Partial<ThinkingConfig>) => {
    updateTranscribe({ thinking: { ...transcribe.thinking, ...changes } })
  }

  const updateOptimize = (changes: Partial<AppConfig['optimize']>) => {
    onSave({ ...config, optimize: { ...optimize, ...changes } })
  }

  const updateOptimizeThinking = (changes: Partial<ThinkingConfig>) => {
    updateOptimize({ thinking: { ...optimize.thinking, ...changes } })
  }

  const activePreset = PRESETS.find(
    p =>
      transcribe.modelId === p.transcribeModelId &&
      optimize.modelId === p.optimizeModelId &&
      optimize.enabled
  )

  const applyPreset = (preset: PresetConfig) => {
    onSave({
      ...config,
      transcribe: { ...transcribe, modelId: preset.transcribeModelId },
      optimize: { ...optimize, enabled: true, modelId: preset.optimizeModelId },
    })
  }

  const builtinAudio = audioModels.filter(m => m.builtin)
  const customAudio = audioModels.filter(m => !m.builtin)
  const builtinText = textModels.filter(m => m.builtin)
  const customText = textModels.filter(m => !m.builtin)

  return (
    <div>
      <h2 className="content-title">转写设置</h2>

      <div className="preset-section">
        <div className="preset-section-title">快速预设</div>
        <div className="preset-cards">
          {PRESETS.map(preset => (
            <div
              key={preset.id}
              className={`preset-card${activePreset?.id === preset.id ? ' active' : ''}`}
              onClick={() => applyPreset(preset)}
            >
              {activePreset?.id === preset.id && (
                <span className="preset-card-badge">✓ 当前</span>
              )}
              <div className="preset-card-title">{preset.title}</div>
              <div className="preset-card-desc">{preset.desc}</div>
              <div className="preset-card-tag">{preset.tag}</div>
            </div>
          ))}
        </div>
      </div>

      {/* 区域一：转写模型 */}
      <SettingGroup title="模型">
        <SettingRow label="转写模型">
          <select
            className="select"
            value={transcribe.modelId}
            onChange={e => updateTranscribe({ modelId: e.target.value })}
            style={{ width: 260 }}
          >
            <optgroup label="预置模型">
              {builtinAudio.map(m => <option key={m.id} value={m.id}>{m.provider} - {m.model}</option>)}
            </optgroup>
            {customAudio.length > 0 && (
              <optgroup label="自定义模型">
                {customAudio.map(m => <option key={m.id} value={m.id}>{m.provider} - {m.model}</option>)}
              </optgroup>
            )}
          </select>
        </SettingRow>
        {isTranscribeGemini && (
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
                  onChange={e => updateTranscribeThinking({ level: e.target.value as ThinkingConfig['level'] })}
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
        <SettingGroup title="模型">
          <SettingRow label="优化模型">
            <select
              className="select"
              value={optimize.modelId}
              onChange={e => updateOptimize({ modelId: e.target.value })}
              style={{ width: 260 }}
            >
              <optgroup label="预置模型">
                {builtinText.map(m => <option key={m.id} value={m.id}>{m.provider} - {m.model}</option>)}
              </optgroup>
              {customText.length > 0 && (
                <optgroup label="自定义模型">
                  {customText.map(m => <option key={m.id} value={m.id}>{m.provider} - {m.model}</option>)}
                </optgroup>
              )}
            </select>
          </SettingRow>
          {isOptimizeGemini && (
            <>
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
            </>
          )}
        </SettingGroup>
      )}

    </div>
  )
}
