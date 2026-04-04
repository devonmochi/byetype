import type { AppConfig } from '../../../core/types'
import { getTextModels } from '../../../core/models'
import { SettingGroup } from '../components/SettingGroup'
import { SettingRow } from '../components/SettingRow'

interface Props {
  config: AppConfig
  onSave: (config: AppConfig) => void
}

export function ExtractTab({ config, onSave }: Props) {
  const { extract } = config

  const textModels = getTextModels(config)
  const builtinText = textModels.filter(m => m.builtin)
  const customText = textModels.filter(m => !m.builtin)

  const updateExtract = (changes: Partial<AppConfig['extract']>) => {
    onSave({ ...config, extract: { ...extract, ...changes } })
  }

  return (
    <div>
      <h2 className="content-title">识别设置</h2>

      <SettingGroup title="模型">
        <SettingRow label="图像识别模型" description="留空则使用与转写相同的模型">
          <select
            className="select"
            value={extract.modelId || ''}
            onChange={e => updateExtract({ modelId: e.target.value || undefined })}
            style={{ width: 260 }}
          >
            <option value="">与转写相同</option>
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
      </SettingGroup>
    </div>
  )
}
