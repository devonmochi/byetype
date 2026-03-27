import type { AppConfig, ThinkingConfig } from '../../../core/types'
import { getAudioModels, getTextModels, findModel } from '../../../core/models'
import { SettingGroup } from '../components/SettingGroup'
import { SettingRow } from '../components/SettingRow'
import { Toggle } from '../components/Toggle'
import { PromptsTab } from './PromptsTab'

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

  const builtinAudio = audioModels.filter(m => m.builtin)
  const customAudio = audioModels.filter(m => !m.builtin)
  const builtinText = textModels.filter(m => m.builtin)
  const customText = textModels.filter(m => !m.builtin)

  return (
    <div>
      <h2 className="content-title">语音转写</h2>

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
      </SettingGroup>

      {isTranscribeGemini && (
        <SettingGroup title="思考模式">
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
        </SettingGroup>
      )}

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
          <SettingGroup title="优化模型">
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
          </SettingGroup>

          {isOptimizeGemini && (
            <SettingGroup title="优化思考模式">
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
