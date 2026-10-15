import { AppConfig } from '../../../core/types'
import { TemplatePromptsPanel } from '../components/TemplatePromptsPanel'

interface Props {
  config: AppConfig
  onSave: (config: AppConfig) => void
}

export function ExtractPromptsTab({ config, onSave }: Props) {
  return (
    <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minHeight: 0, overflow: 'auto' }}>
      <h2 className="content-title" style={{ flexShrink: 0 }}>图像识别提示词</h2>
      <TemplatePromptsPanel kind="image" config={config} onSave={onSave} />
    </div>
  )
}
