import type { AppConfig, ThinkingConfig } from '../../../core/types'
import { getTextModels, findModel, supportsMinimalThinking } from '../../../core/models'
import { SettingGroup } from '../components/SettingGroup'
import { SettingRow } from '../components/SettingRow'
import { Toggle } from '../components/Toggle'
import { TemplatePromptsPanel } from '../components/TemplatePromptsPanel'

interface Props {
  config: AppConfig
  onSave: (config: AppConfig) => void
}

/** 文本优化：处理模型、思考设置和各输出风格的规则提示词都放在这一页 */
export function TextOptimizeTab({ config, onSave }: Props) {
  const { voiceTemplates } = config
  const textModels = getTextModels(config)
  const voiceTemplatesModel = findModel(config, voiceTemplates.modelId)
  const isOpenRouter = (m: ReturnType<typeof findModel>) =>
    m?.protocol === 'openai-compat' && (m?.baseUrl?.includes('openrouter.ai') ?? false)
  const isVoiceTemplatesGemini = voiceTemplatesModel?.protocol === 'gemini' || isOpenRouter(voiceTemplatesModel)
  // Gemini 3.7 系列不支持 MINIMAL 思考档位，隐藏该选项并按 LOW 显示
  const voiceTemplatesSupportsMinimal = supportsMinimalThinking(voiceTemplatesModel?.model)
  const isVoiceTemplatesDeepSeek =
    voiceTemplatesModel?.protocol === 'openai-compat' &&
    voiceTemplatesModel?.baseUrl?.includes('api.deepseek.com')

  const updateVoiceTemplates = (changes: Partial<AppConfig['voiceTemplates']>) => {
    onSave({ ...config, voiceTemplates: { ...voiceTemplates, ...changes } })
  }

  const updateVoiceTemplatesThinking = (changes: Partial<ThinkingConfig>) => {
    updateVoiceTemplates({ thinking: { ...voiceTemplates.thinking, ...changes } })
  }

  const builtinText = textModels.filter(m => m.builtin)
  const customText = textModels.filter(m => !m.builtin)

  return (
    <div>
      <h2 className="content-title">文本优化</h2>

      <SettingGroup title="模型">
        <SettingRow label="处理模型" description="按输出风格处理识别文本的模型">
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
                  value={voiceTemplatesSupportsMinimal || voiceTemplates.thinking.level !== 'MINIMAL' ? voiceTemplates.thinking.level : 'LOW'}
                  onChange={e => updateVoiceTemplatesThinking({ level: e.target.value as ThinkingConfig['level'] })}
                  style={{ width: 120 }}
                >
                  {voiceTemplatesSupportsMinimal && <option value="MINIMAL">MINIMAL</option>}
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
            <SettingRow label="启用思考" description="DeepSeek V4.1 Flash 默认开启思考,关闭可显著提速">
              <Toggle
                checked={voiceTemplates.thinking.enabled}
                onChange={checked => updateVoiceTemplatesThinking({ enabled: checked })}
              />
            </SettingRow>
            {voiceTemplates.thinking.enabled && (
              <SettingRow label="Reasoning Effort" description="DeepSeek思考强度，low更快，max更深">
                <select
                  className="select"
                  value={voiceTemplates.deepseekReasoningEffort ?? 'high'}
                  onChange={e => updateVoiceTemplates({ deepseekReasoningEffort: e.target.value as 'low' | 'high' | 'max' })}
                  style={{ width: 120 }}
                >
                  <option value="low">low</option>
                  <option value="high">high</option>
                  <option value="max">max</option>
                </select>
              </SettingRow>
            )}
          </>
        )}
      </SettingGroup>

      <div className="setting-group-title">规则</div>
      <TemplatePromptsPanel kind="voice" config={config} onSave={onSave} />
    </div>
  )
}
