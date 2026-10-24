import { useState } from 'react'
import { AppConfig } from '../../../core/types'
import { PromptEditor, PromptFileEntry } from '../components/PromptEditor'
import { VoiceLearningSection } from './VoiceLearningTab'

const TRANSCRIBE_PROMPT_FILES: PromptFileEntry[] = [
  { key: 'rules', label: '识别规则', configPath: 'transcribe.prompts.rules', builtinFilename: 'rules.md' },
  { key: 'vocabulary', label: '专有词汇', configPath: 'transcribe.prompts.vocabulary', builtinFilename: 'vocabulary.md' },
]

/** 自动学习作为第三个标签页，与两份提示词并列，不再排在页面下方 */
const LEARNING_KEY = 'learning'

interface Props {
  config: AppConfig
  onSave: (config: AppConfig) => void
}

export function VoicePromptsTab({ config, onSave }: Props) {
  const [activeKey, setActiveKey] = useState('rules')

  const activePrompt = TRANSCRIBE_PROMPT_FILES.find(f => f.key === activeKey)

  return (
    <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minHeight: 0, overflow: 'auto' }}>
      <h2 className="content-title" style={{ flexShrink: 0 }}>语音识别规则</h2>

      <div className="prompt-tabs">
        {TRANSCRIBE_PROMPT_FILES.map(f => (
          <button
            key={f.key}
            className={`prompt-tab${activeKey === f.key ? ' active' : ''}`}
            onClick={() => setActiveKey(f.key)}
          >
            {f.label}
          </button>
        ))}
        <button
          className={`prompt-tab${activeKey === LEARNING_KEY ? ' active' : ''}`}
          onClick={() => setActiveKey(LEARNING_KEY)}
        >
          自动学习
        </button>
      </div>

      {activeKey === LEARNING_KEY ? (
        <VoiceLearningSection config={config} onSave={onSave} />
      ) : (
        <PromptEditor
          key={activeKey}
          config={config}
          onSave={onSave}
          promptFiles={activePrompt ? [activePrompt] : []}
          showTabs={false}
          editorHeight={400}
        />
      )}
    </div>
  )
}
