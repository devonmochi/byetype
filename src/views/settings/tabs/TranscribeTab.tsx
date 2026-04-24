import type { AppConfig, ThinkingConfig } from '../../../core/types'
import { getAudioModels, getTextModels, findModel } from '../../../core/models'
import { SettingGroup } from '../components/SettingGroup'
import { SettingRow } from '../components/SettingRow'
import { Toggle } from '../components/Toggle'

interface Props {
  config: AppConfig
  onSave: (config: AppConfig) => void
}

export function TranscribeTab({ config, onSave }: Props) {
  const { transcribe, voiceTemplates } = config

  const audioModels = getAudioModels(config)
  const textModels = getTextModels(config)
  const transcribeModel = findModel(config, transcribe.modelId)
  const voiceTemplatesModel = findModel(config, voiceTemplates.modelId)
  const isTranscribeGemini = transcribeModel?.protocol === 'gemini'
  const isVoiceTemplatesGemini = voiceTemplatesModel?.protocol === 'gemini'
  const isVoiceTemplatesDeepSeek =
    voiceTemplatesModel?.protocol === 'openai-compat' &&
    voiceTemplatesModel?.baseUrl?.includes('api.deepseek.com')

  const updateTranscribe = (changes: Partial<AppConfig['transcribe']>) => {
    onSave({ ...config, transcribe: { ...transcribe, ...changes } })
  }

  const updateTranscribeThinking = (changes: Partial<ThinkingConfig>) => {
    updateTranscribe({ thinking: { ...transcribe.thinking, ...changes } })
  }

  const updateVoiceTemplates = (changes: Partial<AppConfig['voiceTemplates']>) => {
    onSave({ ...config, voiceTemplates: { ...voiceTemplates, ...changes } })
  }

  const updateVoiceTemplatesThinking = (changes: Partial<ThinkingConfig>) => {
    updateVoiceTemplates({ thinking: { ...voiceTemplates.thinking, ...changes } })
  }

  const builtinAudio = audioModels.filter(m => m.builtin)
  const customAudio = audioModels.filter(m => !m.builtin)
  const builtinText = textModels.filter(m => m.builtin)
  const customText = textModels.filter(m => !m.builtin)

  return (
    <div>
      <h2 className="content-title">转写设置</h2>

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

      {/* 区域二：文本优化模型 */}
      <h3 className="section-title">文本优化模型</h3>

      <SettingGroup title="模型">
        <SettingRow label="处理模型" description="转写后的文本优化处理使用此模型">
          <select
            className="select"
            value={voiceTemplates.modelId}
            onChange={e => updateVoiceTemplates({ modelId: e.target.value })}
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
        {isVoiceTemplatesGemini && (
          <>
            <SettingRow label="启用思考" description="让模型在处理前先进行推理">
              <Toggle
                checked={voiceTemplates.thinking.enabled}
                onChange={checked => updateVoiceTemplatesThinking({ enabled: checked })}
              />
            </SettingRow>
            {voiceTemplates.thinking.enabled && (
              <SettingRow label="Thinking Level" description="思考深度级别">
                <select
                  className="select"
                  value={voiceTemplates.thinking.level}
                  onChange={e => updateVoiceTemplatesThinking({ level: e.target.value as ThinkingConfig['level'] })}
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
        {isVoiceTemplatesDeepSeek && (
          <>
            <SettingRow label="启用思考" description="DeepSeek V4 默认开启思考,关闭可显著提速">
              <Toggle
                checked={voiceTemplates.thinking.enabled}
                onChange={checked => updateVoiceTemplatesThinking({ enabled: checked })}
              />
            </SettingRow>
            {voiceTemplates.thinking.enabled && (
              <SettingRow label="Reasoning Effort" description="DeepSeek 思考强度,max 更深更慢">
                <select
                  className="select"
                  value={voiceTemplates.deepseekReasoningEffort ?? 'high'}
                  onChange={e => updateVoiceTemplates({ deepseekReasoningEffort: e.target.value as 'high' | 'max' })}
                  style={{ width: 120 }}
                >
                  <option value="high">high</option>
                  <option value="max">max</option>
                </select>
              </SettingRow>
            )}
          </>
        )}
      </SettingGroup>

    </div>
  )
}
