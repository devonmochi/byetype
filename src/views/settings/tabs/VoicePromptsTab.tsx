import { memo } from 'react'
import { AppConfig } from '../../../core/types'
import { PromptEditor, PromptFileEntry } from '../components/PromptEditor'

const TRANSCRIBE_PROMPT_FILES: PromptFileEntry[] = [
  { key: 'agent', label: '角色定义', configPath: 'transcribe.prompts.agent', builtinFilename: 'agent.md' },
  { key: 'rules', label: '识别规则', configPath: 'transcribe.prompts.rules', builtinFilename: 'rules.md' },
  { key: 'vocabulary', label: '专有词汇', configPath: 'transcribe.prompts.vocabulary', builtinFilename: 'vocabulary.md' },
]

interface Props {
  config: AppConfig
  onSave: (config: AppConfig) => void
}

function VoicePromptsTabInner({ config, onSave }: Props) {
  return (
    <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minHeight: 0, overflow: 'auto' }}>
      <h2 className="content-title" style={{ flexShrink: 0 }}>语音识别规则</h2>
      <PromptEditor
        config={config}
        onSave={onSave}
        promptFiles={TRANSCRIBE_PROMPT_FILES}
        editorHeight={400}
      />
    </div>
  )
}

export const VoicePromptsTab = memo(VoicePromptsTabInner, (prev, next) => {
  return (
    prev.config.transcribe.prompts.agent === next.config.transcribe.prompts.agent &&
    prev.config.transcribe.prompts.rules === next.config.transcribe.prompts.rules &&
    prev.config.transcribe.prompts.vocabulary === next.config.transcribe.prompts.vocabulary &&
    prev.onSave === next.onSave
  )
})
