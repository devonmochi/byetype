import { useState, useEffect, useCallback, useRef } from 'react'
import { GeneralTab } from './tabs/GeneralTab'
import { TranscribeTab } from './tabs/TranscribeTab'
import { ModelsTab } from './tabs/ModelsTab'
import { HistoryTab } from './tabs/HistoryTab'
import { AboutTab } from './tabs/AboutTab'
import { PromptsTab } from './tabs/PromptsTab'
import type { AppConfig, UpdateState, UpdateInfo } from '../../core/types'
import { getVersion } from '@tauri-apps/api/app'
import { getConfig, saveConfig, onEvent, checkUpdate } from '../../lib/tauri-api'
import './theme.css'

const TABS = [
  { id: 'general', label: '通用设置' },
  { id: 'models', label: '模型管理' },
  { id: 'transcribe', label: '转写设置' },
  { id: 'prompts', label: '提示词' },
  { id: 'history', label: '历史记录' },
  { id: 'about', label: '关于' },
]

const INITIAL_UPDATE_STATE: UpdateState = {
  phase: 'idle',
  info: null,
  progress: 0,
  error: null,
  dismissed: false,
  checkedOnce: false,
}

export function App() {
  const [activeTab, setActiveTab] = useState('general')
  const [config, setConfig] = useState<AppConfig | null>(null)
  const [saved, setSaved] = useState(false)
  const [updateState, setUpdateState] = useState<UpdateState>(INITIAL_UPDATE_STATE)
  const [appVersion, setAppVersion] = useState('')
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  const handleUpdateState = useCallback((partial: Partial<UpdateState>) => {
    setUpdateState(prev => ({ ...prev, ...partial }))
  }, [])

  useEffect(() => {
    getConfig().then((c) => setConfig(c))
    getVersion().then((v) => setAppVersion(v))

    let unsubUpdateAvailable: (() => void) | null = null
    onEvent<UpdateInfo>('update-available', (payload) => {
      setUpdateState(prev => ({
        ...prev,
        phase: 'available',
        info: payload,
        dismissed: false,
      }))
    }).then(unsub => { unsubUpdateAvailable = unsub })

    let unsubProgress: (() => void) | null = null
    onEvent<{ percent: number }>('update-progress', (payload) => {
      setUpdateState(prev => ({ ...prev, progress: payload.percent }))
    }).then(unsub => { unsubProgress = unsub })

    let unsubComplete: (() => void) | null = null
    onEvent<Record<string, never>>('update-complete', () => {
      setUpdateState(prev => ({ ...prev, phase: 'downloaded' }))
    }).then(unsub => { unsubComplete = unsub })

    let unsubError: (() => void) | null = null
    onEvent<{ message: string }>('update-error', (payload) => {
      setUpdateState(prev => ({ ...prev, phase: 'error', error: payload.message }))
    }).then(unsub => { unsubError = unsub })

    let unsubNavigate: (() => void) | null = null
    onEvent<{ tab: string }>('navigate-to-tab', (payload) => {
      setActiveTab(payload.tab)
      if (payload.tab === 'about') {
        setUpdateState(prev => {
          if (prev.phase === 'idle') {
            checkUpdate().then(result => {
              if (result) {
                setUpdateState(p => ({ ...p, phase: 'available', info: result, dismissed: false, checkedOnce: true }))
              }
            }).catch(() => {})
            return { ...prev, phase: 'checking' }
          }
          return prev
        })
      }
    }).then(unsub => { unsubNavigate = unsub })

    return () => {
      unsubUpdateAvailable?.()
      unsubProgress?.()
      unsubComplete?.()
      unsubError?.()
      unsubNavigate?.()
    }
  }, [])

  useEffect(() => {
    const theme = config?.general.theme
    if (!theme) return

    if (theme === 'light' || theme === 'dark') {
      document.documentElement.dataset.theme = theme
      return
    }

    // theme === 'system': follow OS preference
    const mq = window.matchMedia('(prefers-color-scheme: dark)')
    document.documentElement.dataset.theme = mq.matches ? 'dark' : 'light'

    const handler = (e: MediaQueryListEvent) => {
      document.documentElement.dataset.theme = e.matches ? 'dark' : 'light'
    }
    mq.addEventListener('change', handler)
    return () => mq.removeEventListener('change', handler)
  }, [config?.general.theme])

  const handleSave = useCallback((newConfig: AppConfig) => {
    setConfig(newConfig)
    if (debounceRef.current) clearTimeout(debounceRef.current)
    debounceRef.current = setTimeout(async () => {
      await saveConfig(newConfig)
      setSaved(true)
      setTimeout(() => setSaved(false), 1500)
    }, 300)
  }, [])

  const showDot = updateState.phase === 'available' && !updateState.dismissed

  if (!config) return <div style={{ padding: 20, color: 'var(--text-primary)' }}>Loading...</div>

  return (
    <div style={{ fontFamily: '-apple-system, BlinkMacSystemFont, sans-serif', height: '100vh', display: 'flex' }}>
      <div className="sidebar">
        {TABS.map(tab => (
          <button
            key={tab.id}
            className={`sidebar-item${activeTab === tab.id ? ' active' : ''}`}
            onClick={() => setActiveTab(tab.id)}
            style={{ position: 'relative' }}
          >
            {tab.label}
            {tab.id === 'about' && showDot && <span className="sidebar-dot" />}
          </button>
        ))}
      </div>
      <div style={{ flex: 1, padding: 24, overflow: 'auto', position: 'relative' }}>
        <span className={`saved-toast${saved ? ' visible' : ''}`}
          style={{ position: 'absolute', top: 24, right: 24 }}>
          ✓ 已保存
        </span>
        {activeTab === 'history' && <HistoryTab />}
        {activeTab === 'general' && <GeneralTab config={config} onSave={handleSave} />}
        {activeTab === 'transcribe' && <TranscribeTab config={config} onSave={handleSave} />}
        {activeTab === 'models' && <ModelsTab config={config} onSave={handleSave} />}
        {activeTab === 'prompts' && <PromptsTab config={config} onSave={handleSave} />}
        {activeTab === 'about' && (
          <AboutTab
            updateState={updateState}
            onUpdateState={handleUpdateState}
            appVersion={appVersion}
          />
        )}
      </div>
    </div>
  )
}
