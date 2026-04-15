import { memo, useState } from 'react'
import { AppConfig } from '../../../core/types'
import { PromptEditor, PromptFileEntry } from '../components/PromptEditor'

const TRANSCRIBE_PROMPT_FILES: PromptFileEntry[] = [
  { key: 'agent', label: '角色定义', configPath: 'transcribe.prompts.agent', builtinFilename: 'agent.md' },
  { key: 'rules', label: '转录规则', configPath: 'transcribe.prompts.rules', builtinFilename: 'rules.md' },
  { key: 'vocabulary', label: '专有词汇', configPath: 'transcribe.prompts.vocabulary', builtinFilename: 'vocabulary.md' },
]

const BUILTIN_VOICE_TEMPLATE_IDS = ['voice-optimize', 'voice-translate', 'voice-custom']

function builtinFilenameForTemplate(templateId: string): string {
  switch (templateId) {
    case 'voice-optimize': return 'text-optimize.md'
    case 'voice-translate': return 'voice-translate.md'
    default: return ''
  }
}

interface Props {
  config: AppConfig
  onSave: (config: AppConfig) => void
}

function TemplateNameInput({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  const [localValue, setLocalValue] = useState(value)

  return (
    <input
      className="input"
      value={localValue}
      onChange={e => setLocalValue(e.target.value)}
      onBlur={() => {
        if (localValue !== value) onChange(localValue)
      }}
      style={{
        flex: 1,
        border: 'transparent',
        background: 'transparent',
        fontSize: 13,
        padding: '2px 4px',
        minWidth: 0,
      }}
      onClick={e => e.stopPropagation()}
    />
  )
}

function VoicePromptsTabInner({ config, onSave }: Props) {
  const [expandedTemplates, setExpandedTemplates] = useState<Set<string>>(new Set())

  const toggleExpand = (id: string) => {
    setExpandedTemplates(prev => {
      const next = new Set(prev)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })
  }

  const updateTemplateName = (templateId: string, name: string) => {
    onSave({
      ...config,
      voiceTemplates: {
        ...config.voiceTemplates,
        templates: config.voiceTemplates.templates.map(t =>
          t.id === templateId ? { ...t, name } : t
        ),
      },
    })
  }

  const deleteTemplate = (templateId: string) => {
    onSave({
      ...config,
      voiceTemplates: {
        ...config.voiceTemplates,
        templates: config.voiceTemplates.templates.filter(t => t.id !== templateId),
      },
    })
  }

  const addTemplate = () => {
    const newTemplate = { id: `voice-user-${Date.now()}`, name: '新模板', prompt: '' }
    onSave({
      ...config,
      voiceTemplates: {
        ...config.voiceTemplates,
        templates: [...config.voiceTemplates.templates, newTemplate],
      },
    })
  }

  const templates = config.voiceTemplates.templates ?? []

  return (
    <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minHeight: 0, overflow: 'auto' }}>
      <h2 className="content-title">转写提示词</h2>
      <PromptEditor config={config} onSave={onSave} promptFiles={TRANSCRIBE_PROMPT_FILES} />

      <h2 className="content-title" style={{ marginTop: 24 }}>语音模板列表</h2>
      <div>
        {templates.map(template => {
          const isBuiltin = BUILTIN_VOICE_TEMPLATE_IDS.includes(template.id)
          const isExpanded = expandedTemplates.has(template.id)
          const builtinFilename = builtinFilenameForTemplate(template.id)
          const promptFile: PromptFileEntry = {
            key: template.id,
            label: template.name,
            configPath: `voiceTemplates.templates.${template.id}.prompt`,
            builtinFilename,
          }

          return (
            <div
              key={template.id}
              style={{
                background: 'var(--bg-secondary)',
                border: '1px solid var(--border)',
                borderRadius: 10,
                marginBottom: 10,
                overflow: 'hidden',
              }}
            >
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  padding: '10px 14px',
                  gap: 10,
                  cursor: 'pointer',
                }}
                onClick={() => toggleExpand(template.id)}
              >
                <span style={{ fontSize: 12, color: 'var(--text-secondary)', userSelect: 'none' }}>
                  {isExpanded ? '▼' : '▶'}
                </span>
                <TemplateNameInput
                  value={template.name}
                  onChange={name => updateTemplateName(template.id, name)}
                />
                <span
                  style={{
                    fontSize: 10,
                    color: 'var(--text-secondary)',
                    background: 'var(--bg-tertiary)',
                    padding: '2px 8px',
                    borderRadius: 4,
                    whiteSpace: 'nowrap',
                  }}
                >
                  {isBuiltin ? '内置' : '用户'}
                </span>
                {!isBuiltin && (
                  <button
                    style={{
                      background: 'transparent',
                      border: 'none',
                      color: 'var(--text-secondary)',
                      cursor: 'pointer',
                      fontSize: 14,
                      padding: '2px 6px',
                      borderRadius: 4,
                    }}
                    onClick={e => {
                      e.stopPropagation()
                      deleteTemplate(template.id)
                    }}
                  >
                    ×
                  </button>
                )}
              </div>
              {isExpanded && (
                <div style={{ borderTop: '1px solid var(--border)', padding: '12px 14px' }}>
                  <PromptEditor
                    config={config}
                    onSave={onSave}
                    promptFiles={[promptFile]}
                    showTabs={false}
                  />
                </div>
              )}
            </div>
          )
        })}
        <button
          className="file-picker-btn"
          style={{ width: '100%', marginTop: 8 }}
          onClick={addTemplate}
        >
          + 添加模板
        </button>
      </div>
    </div>
  )
}

export const VoicePromptsTab = memo(VoicePromptsTabInner, (prev, next) => {
  return (
    prev.config.transcribe.prompts.agent === next.config.transcribe.prompts.agent &&
    prev.config.transcribe.prompts.rules === next.config.transcribe.prompts.rules &&
    prev.config.transcribe.prompts.vocabulary === next.config.transcribe.prompts.vocabulary &&
    prev.config.voiceTemplates === next.config.voiceTemplates &&
    prev.onSave === next.onSave
  )
})
