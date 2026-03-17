import React from 'react'
import { AppConfig } from '../../../core/types'
import { SettingGroup } from '../components/SettingGroup'
import { SettingRow } from '../components/SettingRow'

interface Props {
  config: AppConfig
  onSave: (config: AppConfig) => void
}

export function AdvancedTab({ config, onSave }: Props) {
  const { advanced } = config

  const update = (changes: Partial<AppConfig['advanced']>) => {
    onSave({ ...config, advanced: { ...advanced, ...changes } })
  }

  return (
    <div>
      <h2 className="content-title">高级设置</h2>

      <SettingGroup title="超时和重试">
        <SettingRow label="转写超时时间" description="单位：秒">
          <input
            className="input"
            type="number"
            value={advanced.transcribeTimeout}
            onChange={e => update({ transcribeTimeout: Number(e.target.value) })}
            min={1}
            style={{ width: 100 }}
          />
        </SettingRow>
        <SettingRow label="文本优化超时时间" description="单位：秒">
          <input
            className="input"
            type="number"
            value={advanced.optimizeTimeout}
            onChange={e => update({ optimizeTimeout: Number(e.target.value) })}
            min={1}
            style={{ width: 100 }}
          />
        </SettingRow>
        <SettingRow label="最大重试次数">
          <input
            className="input"
            type="number"
            value={advanced.maxRetries}
            onChange={e => update({ maxRetries: Number(e.target.value) })}
            min={0}
            style={{ width: 100 }}
          />
        </SettingRow>
      </SettingGroup>

      <SettingGroup title="并行和网络">
        <SettingRow label="最大并行任务数">
          <input
            className="input"
            type="number"
            value={advanced.maxParallel}
            onChange={e => update({ maxParallel: Number(e.target.value) })}
            min={1}
            style={{ width: 100 }}
          />
        </SettingRow>
        <SettingRow label="HTTP 代理地址" description="用于 Gemini 等需要代理的服务，留空不使用">
          <input
            className="input input-wide"
            value={advanced.proxyUrl}
            onChange={e => update({ proxyUrl: e.target.value })}
            placeholder="http://127.0.0.1:10809"
          />
        </SettingRow>
      </SettingGroup>
    </div>
  )
}
