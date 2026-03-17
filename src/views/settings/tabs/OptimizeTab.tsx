import { AppConfig, ThinkingConfig } from '../../../core/types'
import { SettingGroup } from '../components/SettingGroup'
import { SettingRow } from '../components/SettingRow'
import { Toggle } from '../components/Toggle'


interface Props {
  config: AppConfig
  onSave: (config: AppConfig) => void
}

export function OptimizeTab({ config, onSave }: Props) {
  const { optimize } = config

  const update = (changes: Partial<AppConfig['optimize']>) => {
    onSave({ ...config, optimize: { ...optimize, ...changes } })
  }

  const updateOpenAI = (changes: Partial<AppConfig['optimize']['openaiCompat']>) => {
    update({ openaiCompat: { ...optimize.openaiCompat, ...changes } })
  }

  const updateThinking = (changes: Partial<ThinkingConfig>) => {
    update({ thinking: { ...optimize.thinking, ...changes } })
  }

  const is25Flash = optimize.geminiModel === 'gemini-2.5-flash'

  return (
    <div>
      <h2 className="content-title">文本优化</h2>

      <SettingGroup>
        <SettingRow label="启用文本优化" description="转写后自动优化文本格式和表达">
          <Toggle
            checked={optimize.enabled}
            onChange={checked => update({ enabled: checked })}
          />
        </SettingRow>
      </SettingGroup>

      {optimize.enabled && (
        <>
          <SettingGroup title="模型类型">
            <SettingRow label="优化引擎">
              <div style={{ display: 'flex', gap: 16 }}>
                <label style={{ display: 'flex', alignItems: 'center', gap: 4, fontSize: 13, color: 'var(--text-primary)', cursor: 'pointer' }}>
                  <input type="radio" value="openai-compat" checked={optimize.type === 'openai-compat'} onChange={() => update({ type: 'openai-compat' })} />
                  OpenAI 兼容
                </label>
                <label style={{ display: 'flex', alignItems: 'center', gap: 4, fontSize: 13, color: 'var(--text-primary)', cursor: 'pointer' }}>
                  <input type="radio" value="gemini" checked={optimize.type === 'gemini'} onChange={() => update({ type: 'gemini' })} />
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
                <select className="select" value={optimize.geminiModel} onChange={e => update({ geminiModel: e.target.value })} style={{ width: 200 }}>
                  <option value="gemini-2.5-flash">Gemini 2.5 Flash</option>
                  <option value="gemini-3-flash-preview">Gemini 3.0 Flash</option>
                  <option value="gemini-3.1-flash-lite-preview">Gemini 3.1 Flash Lite</option>
                </select>
              </SettingRow>
              <SettingRow label="启用思考" description="让模型在优化前先进行推理">
                <Toggle
                  checked={optimize.thinking.enabled}
                  onChange={checked => updateThinking({ enabled: checked })}
                />
              </SettingRow>
              {optimize.thinking.enabled && (
                is25Flash ? (
                  <SettingRow label="Thinking Budget" description="思考 token 数量">
                    <input
                      className="input"
                      type="number"
                      value={optimize.thinking.budget}
                      onChange={e => updateThinking({ budget: Number(e.target.value) })}
                      min={0}
                      style={{ width: 120 }}
                    />
                  </SettingRow>
                ) : (
                  <SettingRow label="Thinking Level" description="思考深度级别">
                    <select
                      className="select"
                      value={optimize.thinking.level}
                      onChange={e => updateThinking({ level: e.target.value as ThinkingConfig['level'] })}
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
            </SettingGroup>
          )}

        </>
      )}
    </div>
  )
}
