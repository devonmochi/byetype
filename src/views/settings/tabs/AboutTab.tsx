import { SettingGroup } from '../components/SettingGroup'
import { SettingRow } from '../components/SettingRow'
import type { UpdateState } from '../../../core/types'
import {
  checkUpdate,
  downloadUpdate,
  installAndRestart,
} from '../../../lib/tauri-api'

interface Props {
  updateState: UpdateState
  onUpdateState: (state: Partial<UpdateState>) => void
  appVersion: string
}

export function AboutTab({ updateState, onUpdateState, appVersion }: Props) {
  const { phase, info, progress, error, dismissed } = updateState

  const handleCheck = async () => {
    onUpdateState({ phase: 'checking', error: null })
    try {
      const result = await checkUpdate()
      if (result) {
        onUpdateState({ phase: 'available', info: result, dismissed: false, checkedOnce: true })
      } else {
        onUpdateState({ phase: 'idle', info: null, checkedOnce: true })
      }
    } catch (e) {
      onUpdateState({ phase: 'error', error: String(e) })
    }
  }

  const handleDownload = async () => {
    onUpdateState({ phase: 'downloading', progress: 0, dismissed: true })
    try {
      await downloadUpdate()
    } catch (e) {
      onUpdateState({ phase: 'error', error: String(e) })
    }
  }

  const handleInstall = async () => {
    try {
      await installAndRestart()
    } catch (e) {
      onUpdateState({ phase: 'error', error: String(e) })
    }
  }

  const handleDismiss = () => {
    onUpdateState({ dismissed: true })
  }

  return (
    <div>
      <h2 className="content-title">关于</h2>

      <div className="about-header">
        <div className="about-app-icon">B</div>
        <div className="about-app-name">ByeType</div>
        <div className="about-app-version">版本 {appVersion}</div>
      </div>

      {(phase === 'idle' || phase === 'checking') && (
        <div className="about-check-area">
          <button
            className="update-btn update-btn-primary"
            onClick={handleCheck}
            disabled={phase === 'checking'}
          >
            {phase === 'checking' ? '检查中...' : '检查更新'}
          </button>
        </div>
      )}

      {phase === 'idle' && info === null && updateState.checkedOnce && (
        <div className="about-status success">已是最新版本</div>
      )}

      {phase === 'available' && info && dismissed && (
        <div className="update-collapsed">
          <span className="update-collapsed-text">v{info.version} 可用</span>
          <button className="update-btn update-btn-primary" onClick={handleDownload}>
            立即更新
          </button>
        </div>
      )}

      {phase === 'available' && info && !dismissed && (
        <div className="update-card">
          <div className="update-card-header">
            <span className="update-card-version">v{info.version}</span>
            <span className="update-badge">新版本</span>
          </div>
          {info.body && (
            <ul className="update-changelog">
              {info.body.split('\n').filter(line => line.trim()).map((line, i) => (
                <li key={i}>{line.replace(/^[-*]\s*/, '')}</li>
              ))}
            </ul>
          )}
          <div className="update-actions">
            <button className="update-btn update-btn-primary" onClick={handleDownload}>
              立即更新
            </button>
            <button className="update-btn update-btn-secondary" onClick={handleDismiss}>
              稍后
            </button>
          </div>
        </div>
      )}

      {phase === 'downloading' && (
        <div className="update-card">
          <div className="update-card-header">
            <span className="update-card-version">正在下载 v{info?.version}</span>
          </div>
          <div className="update-progress-area">
            <div className="update-progress-bar">
              <div className="update-progress-fill" style={{ width: `${progress}%` }} />
            </div>
            <div className="update-progress-text">{Math.round(progress)}%</div>
          </div>
        </div>
      )}

      {phase === 'downloaded' && (
        <div className="update-card">
          <div className="update-card-header">
            <span className="update-card-version">v{info?.version} 已准备就绪</span>
          </div>
          <div className="update-actions">
            <button className="update-btn update-btn-primary" onClick={handleInstall}>
              重启并安装
            </button>
          </div>
        </div>
      )}

      {phase === 'error' && (
        <div>
          <div className="update-error">{error}</div>
          <div className="about-check-area">
            <button className="update-btn update-btn-primary" onClick={handleCheck}>
              重试
            </button>
          </div>
        </div>
      )}

      <SettingGroup>
        <SettingRow label="当前版本">
          <span style={{ fontSize: 13, color: 'var(--text-secondary)' }}>{appVersion}</span>
        </SettingRow>
      </SettingGroup>
    </div>
  )
}
