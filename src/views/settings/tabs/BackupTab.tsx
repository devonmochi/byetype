import { useState, useEffect, useCallback } from 'react'
import { SettingGroup } from '../components/SettingGroup'
import { SettingRow } from '../components/SettingRow'
import type { AppConfig, BackupEntry } from '../../../core/types'
import {
  testS3Connection,
  backupToS3,
  listS3Backups,
  restoreFromS3,
  backupToLocal,
  restoreFromLocal,
} from '../../../lib/tauri-api'

interface Props {
  config: AppConfig
  onSave: (config: AppConfig) => void
}

export function BackupTab({ config, onSave }: Props) {
  const [testing, setTesting] = useState(false)
  const [testResult, setTestResult] = useState<{ ok: boolean; msg: string } | null>(null)
  const [s3Backing, setS3Backing] = useState(false)
  const [s3Restoring, setS3Restoring] = useState(false)
  const [localBacking, setLocalBacking] = useState(false)
  const [localRestoring, setLocalRestoring] = useState(false)
  const [backups, setBackups] = useState<BackupEntry[]>([])
  const [loadingBackups, setLoadingBackups] = useState(false)
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null)

  const s3 = config.backup.s3

  const updateS3 = (changes: Partial<typeof s3>) => {
    onSave({ ...config, backup: { ...config.backup, s3: { ...s3, ...changes } } })
  }

  const showMessage = (type: 'success' | 'error', text: string) => {
    setMessage({ type, text })
    setTimeout(() => setMessage(null), 5000)
  }

  const handleTestConnection = useCallback(async () => {
    setTesting(true)
    setTestResult(null)
    try {
      await testS3Connection()
      setTestResult({ ok: true, msg: '连接成功' })
    } catch (e: any) {
      setTestResult({ ok: false, msg: String(e) })
    } finally {
      setTesting(false)
    }
  }, [])

  const handleBackupToS3 = useCallback(async () => {
    setS3Backing(true)
    try {
      const key = await backupToS3()
      showMessage('success', `备份成功：${key}`)
      // 刷新列表
      await refreshBackups()
    } catch (e: any) {
      showMessage('error', `备份失败：${e}`)
    } finally {
      setS3Backing(false)
    }
  }, [])

  const handleRestoreFromS3 = useCallback(async (key: string) => {
    if (!confirm(`确认从 ${key} 恢复？当前配置将被覆盖，恢复后需要重启应用。`)) return
    setS3Restoring(true)
    try {
      await restoreFromS3(key)
      showMessage('success', '恢复成功，请重启应用使配置生效')
    } catch (e: any) {
      showMessage('error', `恢复失败：${e}`)
    } finally {
      setS3Restoring(false)
    }
  }, [])

  const handleBackupToLocal = useCallback(async () => {
    setLocalBacking(true)
    try {
      const path = await backupToLocal()
      showMessage('success', `备份已保存：${path}`)
    } catch (e: any) {
      if (String(e).includes('未选择')) return
      showMessage('error', `备份失败：${e}`)
    } finally {
      setLocalBacking(false)
    }
  }, [])

  const handleRestoreFromLocal = useCallback(async () => {
    if (!confirm('确认从本地文件恢复？当前配置将被覆盖，恢复后需要重启应用。')) return
    setLocalRestoring(true)
    try {
      await restoreFromLocal()
      showMessage('success', '恢复成功，请重启应用使配置生效')
    } catch (e: any) {
      if (String(e).includes('未选择')) return
      showMessage('error', `恢复失败：${e}`)
    } finally {
      setLocalRestoring(false)
    }
  }, [])

  const refreshBackups = useCallback(async () => {
    setLoadingBackups(true)
    try {
      const list = await listS3Backups()
      setBackups(list)
    } catch (e: any) {
      setBackups([])
    } finally {
      setLoadingBackups(false)
    }
  }, [])

  useEffect(() => {
    if (s3.bucket) {
      refreshBackups()
    }
  }, [s3.bucket])

  const formatSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`
  }

  const formatDate = (s: string) => {
    try {
      return new Date(s).toLocaleString('zh-CN')
    } catch {
      return s
    }
  }

  return (
    <div>
      <h2>备份与恢复</h2>

      {message && (
        <div style={{
          padding: '8px 12px',
          marginBottom: 12,
          borderRadius: 6,
          background: message.type === 'success' ? '#d4edda' : '#f8d7da',
          color: message.type === 'success' ? '#155724' : '#721c24',
          fontSize: 13,
        }}>
          {message.text}
        </div>
      )}

      <SettingGroup title="S3 兼容存储配置">
        <SettingRow label="Endpoint" description="留空则使用 AWS S3 标准地址">
          <input
            type="text"
            value={s3.endpoint}
            onChange={e => updateS3({ endpoint: e.target.value })}
            placeholder="https://s3.amazonaws.com"
            style={{ width: 240 }}
          />
        </SettingRow>
        <SettingRow label="Region">
          <input
            type="text"
            value={s3.region}
            onChange={e => updateS3({ region: e.target.value })}
            placeholder="us-east-1"
            style={{ width: 240 }}
          />
        </SettingRow>
        <SettingRow label="Bucket">
          <input
            type="text"
            value={s3.bucket}
            onChange={e => updateS3({ bucket: e.target.value })}
            placeholder="my-backup-bucket"
            style={{ width: 240 }}
          />
        </SettingRow>
        <SettingRow label="Access Key">
          <input
            type="text"
            value={s3.accessKey}
            onChange={e => updateS3({ accessKey: e.target.value })}
            placeholder="AKIAXXXXX"
            style={{ width: 240 }}
          />
        </SettingRow>
        <SettingRow label="Secret Key">
          <input
            type="password"
            value={s3.secretKey}
            onChange={e => updateS3({ secretKey: e.target.value })}
            placeholder="******"
            style={{ width: 240 }}
          />
        </SettingRow>
        <SettingRow label="路径前缀" description="S3 对象 key 的前缀">
          <input
            type="text"
            value={s3.prefix}
            onChange={e => updateS3({ prefix: e.target.value })}
            placeholder="byetype/backups"
            style={{ width: 240 }}
          />
        </SettingRow>
        <SettingRow label="连接测试">
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <button onClick={handleTestConnection} disabled={testing} style={{ padding: '4px 12px' }}>
              {testing ? '测试中...' : '测试连接'}
            </button>
            {testResult && (
              <span style={{
                fontSize: 12,
                color: testResult.ok ? '#155724' : '#721c24',
              }}>
                {testResult.ok ? '✓' : '✗'} {testResult.msg}
              </span>
            )}
          </div>
        </SettingRow>
      </SettingGroup>

      <SettingGroup title="S3 备份">
        <SettingRow label="立即备份到 S3" description="将配置和提示词打包上传到 S3">
          <button onClick={handleBackupToS3} disabled={s3Backing || !s3.bucket} style={{ padding: '4px 12px' }}>
            {s3Backing ? '备份中...' : '备份到 S3'}
          </button>
        </SettingRow>
        <SettingRow label="从 S3 恢复" description="选择一个备份恢复">
          <button onClick={refreshBackups} disabled={loadingBackups || !s3.bucket} style={{ padding: '4px 12px' }}>
            {loadingBackups ? '加载中...' : '刷新列表'}
          </button>
        </SettingRow>
        {backups.length > 0 && (
          <div style={{ marginTop: 8 }}>
            <table style={{ width: '100%', fontSize: 12, borderCollapse: 'collapse' }}>
              <thead>
                <tr style={{ borderBottom: '1px solid #e0e0e0' }}>
                  <th style={{ textAlign: 'left', padding: '4px 8px' }}>备份文件</th>
                  <th style={{ textAlign: 'left', padding: '4px 8px' }}>大小</th>
                  <th style={{ textAlign: 'left', padding: '4px 8px' }}>时间</th>
                  <th style={{ padding: '4px 8px' }}>操作</th>
                </tr>
              </thead>
              <tbody>
                {backups.map(b => (
                  <tr key={b.key} style={{ borderBottom: '1px solid #f0f0f0' }}>
                    <td style={{ padding: '4px 8px' }}>{b.key.split('/').pop()}</td>
                    <td style={{ padding: '4px 8px' }}>{formatSize(b.size)}</td>
                    <td style={{ padding: '4px 8px' }}>{formatDate(b.lastModified)}</td>
                    <td style={{ padding: '4px 8px' }}>
                      <button
                        onClick={() => handleRestoreFromS3(b.key)}
                        disabled={s3Restoring}
                        style={{ padding: '2px 8px', fontSize: 12 }}
                      >
                        恢复
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </SettingGroup>

      <SettingGroup title="本地备份">
        <SettingRow label="备份到本地" description="选择保存位置，导出 zip 备份文件">
          <button onClick={handleBackupToLocal} disabled={localBacking} style={{ padding: '4px 12px' }}>
            {localBacking ? '备份中...' : '备份到本地'}
          </button>
        </SettingRow>
        <SettingRow label="从本地恢复" description="选择 zip 备份文件恢复配置">
          <button onClick={handleRestoreFromLocal} disabled={localRestoring} style={{ padding: '4px 12px' }}>
            {localRestoring ? '恢复中...' : '从本地恢复'}
          </button>
        </SettingRow>
      </SettingGroup>
    </div>
  )
}
