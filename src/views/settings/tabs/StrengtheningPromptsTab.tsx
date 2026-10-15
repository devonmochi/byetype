import { AppConfig } from '../../../core/types'
import { TemplatePromptsPanel } from '../components/TemplatePromptsPanel'

interface Props {
  config: AppConfig
  onSave: (config: AppConfig) => void
}

export function StrengtheningPromptsTab({ config, onSave }: Props) {
  return (
    <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minHeight: 0, overflow: 'auto' }}>
      <h2 className="content-title optimization-heading" style={{ flexShrink: 0 }}>文本强化规则</h2>
      <p className="rules-intro">按输出风格处理识别文本，例如分段、翻译。</p>
      <TemplatePromptsPanel kind="voice" config={config} onSave={onSave} />
    </div>
  )
}
