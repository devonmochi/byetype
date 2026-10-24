import type { AppConfig, ThinkingConfig } from '../../../core/types'
import { getAudioModels, findModel, supportsMinimalThinking } from '../../../core/models'
import { SettingGroup } from '../components/SettingGroup'
import { SettingRow } from '../components/SettingRow'
import { Toggle } from '../components/Toggle'

interface Props {
  config: AppConfig
  onSave: (config: AppConfig) => void
}

export function TranscribeTab({ config, onSave }: Props) {
  const { transcribe } = config
  const strengtheningEnabled = config.general.textStrengtheningEnabled !== false

  const audioModels = getAudioModels(config)
  const transcribeModel = findModel(config, transcribe.modelId)
  const isOpenRouter = (m: ReturnType<typeof findModel>) =>
    m?.protocol === 'openai-compat' && (m?.baseUrl?.includes('openrouter.ai') ?? false)
  const isTranscribeGemini = transcribeModel?.protocol === 'gemini' || isOpenRouter(transcribeModel)
  const isTranscribeQwen = transcribeModel?.protocol === 'qwen-omni'
  // Gemini 3.7 系列不支持 MINIMAL 思考档位，隐藏该选项并按 LOW 显示
  const transcribeSupportsMinimal = supportsMinimalThinking(transcribeModel?.model)

  const updateTranscribe = (changes: Partial<AppConfig['transcribe']>) => {
    onSave({ ...config, transcribe: { ...transcribe, ...changes } })
  }

  const updateTranscribeThinking = (changes: Partial<ThinkingConfig>) => {
    updateTranscribe({ thinking: { ...transcribe.thinking, ...changes } })
  }

  const builtinAudio = audioModels.filter(m => m.builtin)
  const customAudio = audioModels.filter(m => !m.builtin)

  return (
    <div>
      <h2 className="content-title">语音识别设置</h2>

      <SettingGroup title="语音识别模型">
        <SettingRow label="语音识别模型">
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
        {(isTranscribeGemini || isTranscribeQwen) && (
          <>
            <SettingRow label="启用思考" description="让模型在转写前先进行推理">
              <Toggle
                checked={transcribe.thinking.enabled}
                onChange={checked => updateTranscribeThinking({ enabled: checked })}
              />
            </SettingRow>
            {isTranscribeGemini && transcribe.thinking.enabled && (
              <SettingRow label="Thinking Level" description="思考深度级别">
                <select
                  className="select"
                  value={transcribeSupportsMinimal || transcribe.thinking.level !== 'MINIMAL' ? transcribe.thinking.level : 'LOW'}
                  onChange={e => updateTranscribeThinking({ level: e.target.value as ThinkingConfig['level'] })}
                  style={{ width: 120 }}
                >
                  {transcribeSupportsMinimal && <option value="MINIMAL">MINIMAL</option>}
                  <option value="LOW">LOW</option>
                  <option value="MEDIUM">MEDIUM</option>
                  <option value="HIGH">HIGH</option>
                </select>
              </SettingRow>
            )}
          </>
        )}
        <SettingRow
          label={
            <span style={strengtheningEnabled ? { color: 'var(--text-secondary)' } : undefined}>
              规则增强
              <span className="rule-direction">将文本优化规则注入语音规则</span>
            </span>
          }
          description={
            strengtheningEnabled
              ? '文本优化会把同一条规则跑第二遍，需先关闭文本优化才能注入'
              : '识别阶段直接按所选风格的规则输出，一轮完成'
          }
        >
          <Toggle
            checked={(transcribe.injectTextRules ?? false) && !strengtheningEnabled}
            disabled={strengtheningEnabled}
            onChange={checked => updateTranscribe({ injectTextRules: checked })}
          />
        </SettingRow>
      </SettingGroup>
    </div>
  )
}
