import type { AppConfig } from '../../../core/types'
import { findModel, getTextModels } from '../../../core/models'
import {
  getVoiceLearningDocument,
  getVoiceLearningPromptPath,
  saveVoiceLearningDocument,
} from '../../../lib/tauri-api'
import { PromptEditor, type PromptFileEntry } from '../components/PromptEditor'
import { SettingGroup } from '../components/SettingGroup'
import { SettingRow } from '../components/SettingRow'
import { Toggle } from '../components/Toggle'

const LEARNING_FILES: PromptFileEntry[] = [
  {
    key: 'voice-learning-prompt',
    label: '学习提示词',
    resolvePath: getVoiceLearningPromptPath,
  },
  {
    key: 'voice-learning-result',
    label: '学习结果',
    loadContent: getVoiceLearningDocument,
    saveContent: saveVoiceLearningDocument,
    refreshEvent: 'voice-learning-updated',
  },
]

interface Props {
  config: AppConfig
  onSave: (config: AppConfig) => void
}

export function VoiceLearningTab({ config, onSave }: Props) {
  const textModels = getTextModels(config)
  const builtinModels = textModels.filter(model => model.builtin)
  const customModels = textModels.filter(model => !model.builtin)
  const selectedModel = findModel(config, config.voiceLearning.modelId)
  const isOpenRouter = selectedModel?.protocol === 'openai-compat'
    && (selectedModel.baseUrl?.includes('openrouter.ai') ?? false)
  const isGemini = selectedModel?.protocol === 'gemini' || isOpenRouter
  const isDeepSeek = selectedModel?.protocol === 'openai-compat'
    && (selectedModel.baseUrl?.includes('api.deepseek.com') ?? false)

  const updateModel = (modelId: string) => {
    onSave({
      ...config,
      voiceLearning: { ...config.voiceLearning, modelId },
    })
  }

  const updateThinking = (changes: Partial<AppConfig['voiceLearning']['thinking']>) => {
    onSave({
      ...config,
      voiceLearning: {
        ...config.voiceLearning,
        thinking: { ...config.voiceLearning.thinking, ...changes },
      },
    })
  }

  const updateDeepSeekEffort = (deepseekReasoningEffort: 'low' | 'high' | 'max') => {
    onSave({
      ...config,
      voiceLearning: { ...config.voiceLearning, deepseekReasoningEffort },
    })
  }

  return (
    <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minHeight: 0, overflow: 'auto' }}>
      <h2 className="content-title" style={{ flexShrink: 0 }}>自动学习</h2>

      <SettingGroup title="模型">
        <SettingRow label="学习模型" description="用于对比原始转写与用户修改文本，并归纳纠错规则">
          <select
            className="select"
            value={config.voiceLearning.modelId}
            onChange={event => updateModel(event.target.value)}
            style={{ width: 260 }}
          >
            <optgroup label="预置模型">
              {builtinModels.map(model => (
                <option key={model.id} value={model.id}>{model.provider} - {model.model}</option>
              ))}
            </optgroup>
            {customModels.length > 0 && (
              <optgroup label="自定义模型">
                {customModels.map(model => (
                  <option key={model.id} value={model.id}>{model.provider} - {model.model}</option>
                ))}
              </optgroup>
            )}
          </select>
        </SettingRow>
        {(isGemini || isDeepSeek) && (
          <SettingRow label="启用思考" description="让模型在归纳纠错规则前先进行推理">
            <Toggle
              checked={config.voiceLearning.thinking.enabled}
              onChange={enabled => updateThinking({ enabled })}
            />
          </SettingRow>
        )}
        {isGemini && config.voiceLearning.thinking.enabled && (
          <SettingRow label="Thinking Level" description="思考深度级别">
            <select
              className="select"
              value={config.voiceLearning.thinking.level}
              onChange={event => updateThinking({ level: event.target.value as AppConfig['voiceLearning']['thinking']['level'] })}
              style={{ width: 120 }}
            >
              <option value="MINIMAL">MINIMAL</option>
              <option value="LOW">LOW</option>
              <option value="MEDIUM">MEDIUM</option>
              <option value="HIGH">HIGH</option>
            </select>
          </SettingRow>
        )}
        {isDeepSeek && config.voiceLearning.thinking.enabled && (
          <SettingRow label="Reasoning Effort" description="DeepSeek思考强度，low更快，max更深">
            <select
              className="select"
              value={config.voiceLearning.deepseekReasoningEffort ?? 'high'}
              onChange={event => updateDeepSeekEffort(event.target.value as 'low' | 'high' | 'max')}
              style={{ width: 120 }}
            >
              <option value="low">low</option>
              <option value="high">high</option>
              <option value="max">max</option>
            </select>
          </SettingRow>
        )}
      </SettingGroup>

      <h3 className="section-title">学习文档</h3>
      <PromptEditor config={config} onSave={onSave} promptFiles={LEARNING_FILES} editorHeight={320} />
    </div>
  )
}
