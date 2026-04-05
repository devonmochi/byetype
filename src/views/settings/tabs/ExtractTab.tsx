import type { AppConfig } from '../../../core/types'
import { getVisionModels } from '../../../core/models'
import { SettingGroup } from '../components/SettingGroup'
import { SettingRow } from '../components/SettingRow'

interface Props {
  config: AppConfig
  onSave: (config: AppConfig) => void
}

export function ExtractTab({ config, onSave }: Props) {
  const { extract } = config

  const visionModels = getVisionModels(config)
  const builtinVision = visionModels.filter(m => m.builtin)
  const customVision = visionModels.filter(m => !m.builtin)

  const updateExtract = (changes: Partial<AppConfig['extract']>) => {
    onSave({ ...config, extract: { ...extract, ...changes } })
  }

  return (
    <div>
      <h2 className="content-title">图像识别设置</h2>

      <SettingGroup title="模型">
        <SettingRow label="图像识别模型">
          <select
            className="select"
            value={extract.modelId || ''}
            onChange={e => updateExtract({ modelId: e.target.value || undefined })}
            style={{ width: 260 }}
          >
            <optgroup label="预置模型">
              {builtinVision.map(m => <option key={m.id} value={m.id}>{m.provider} - {m.model}</option>)}
            </optgroup>
            {customVision.length > 0 && (
              <optgroup label="自定义模型">
                {customVision.map(m => <option key={m.id} value={m.id}>{m.provider} - {m.model}</option>)}
              </optgroup>
            )}
          </select>
        </SettingRow>
      </SettingGroup>
    </div>
  )
}
