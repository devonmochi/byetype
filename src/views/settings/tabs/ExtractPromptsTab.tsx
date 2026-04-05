import { memo } from 'react'
import { AppConfig } from '../../../core/types'
import { PromptEditor, PromptFileEntry } from '../components/PromptEditor'

const EXTRACT_PROMPT_FILES: PromptFileEntry[] = [
  { key: 'textExtract', label: '图像识别规则', configPath: 'extract.prompt', builtinFilename: 'text-extract.md' },
]

interface Props {
  config: AppConfig
  onSave: (config: AppConfig) => void
}

function ExtractPromptsTabInner({ config, onSave }: Props) {
  return (
    <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minHeight: 0 }}>
      <h2 className="content-title">图像识别提示词</h2>
      <PromptEditor config={config} onSave={onSave} promptFiles={EXTRACT_PROMPT_FILES} showTabs={false} />
    </div>
  )
}

export const ExtractPromptsTab = memo(ExtractPromptsTabInner, (prev, next) => {
  return (
    prev.config.extract.prompt === next.config.extract.prompt &&
    prev.onSave === next.onSave
  )
})
