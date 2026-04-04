import { memo } from 'react'
import { AppConfig } from '../../../core/types'
import { PromptEditor, PromptFileEntry } from '../components/PromptEditor'

const VOICE_PROMPT_FILES: PromptFileEntry[] = [
  { key: 'agent', label: '角色定义', configPath: 'transcribe.prompts.agent', builtinFilename: 'agent.md' },
  { key: 'rules', label: '转录规则', configPath: 'transcribe.prompts.rules', builtinFilename: 'rules.md' },
  { key: 'vocabulary', label: '专有词汇', configPath: 'transcribe.prompts.vocabulary', builtinFilename: 'vocabulary.md' },
  { key: 'textOptimize', label: '文本优化', configPath: 'optimize.prompt', builtinFilename: 'text-optimize.md' },
]

interface Props {
  config: AppConfig
  onSave: (config: AppConfig) => void
}

function VoicePromptsTabInner({ config, onSave }: Props) {
  return (
    <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minHeight: 0 }}>
      <h2 className="content-title">转写提示词</h2>
      <PromptEditor config={config} onSave={onSave} promptFiles={VOICE_PROMPT_FILES} />
    </div>
  )
}

export const VoicePromptsTab = memo(VoicePromptsTabInner, (prev, next) => {
  return (
    prev.config.transcribe.prompts.agent === next.config.transcribe.prompts.agent &&
    prev.config.transcribe.prompts.rules === next.config.transcribe.prompts.rules &&
    prev.config.transcribe.prompts.vocabulary === next.config.transcribe.prompts.vocabulary &&
    prev.config.optimize.prompt === next.config.optimize.prompt &&
    prev.onSave === next.onSave
  )
})
