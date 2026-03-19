import { useState, useEffect, useCallback, useRef } from 'react'
import { GeneralTab } from './tabs/GeneralTab'
import { TranscribeTab } from './tabs/TranscribeTab'
import { HistoryTab } from './tabs/HistoryTab'
import type { AppConfig } from '../../core/types'
import { getConfig, saveConfig, getTheme, onThemeChange } from '../../lib/tauri-api'
import './theme.css'

const TABS = [
  { id: 'history', label: '历史记录' },
  { id: 'transcribe', label: '语音转写' },
  { id: 'general', label: '通用设置' },
]

export function App() {
  const [activeTab, setActiveTab] = useState('history')
  const [config, setConfig] = useState<AppConfig | null>(null)
  const [saved, setSaved] = useState(false)
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => {
    getConfig().then((c) => setConfig(c))

    getTheme().then((theme) => {
      document.documentElement.dataset.theme = theme
    })

    const unsubTheme = onThemeChange((theme) => {
      document.documentElement.dataset.theme = theme
    })

    return () => {
      unsubTheme()
    }
  }, [])

  const handleSave = useCallback((newConfig: AppConfig) => {
    setConfig(newConfig)
    if (debounceRef.current) clearTimeout(debounceRef.current)
    debounceRef.current = setTimeout(async () => {
      await saveConfig(newConfig)
      setSaved(true)
      setTimeout(() => setSaved(false), 1500)
    }, 300)
  }, [])

  if (!config) return <div style={{ padding: 20, color: 'var(--text-primary)' }}>Loading...</div>

  return (
    <div style={{ fontFamily: '-apple-system, BlinkMacSystemFont, sans-serif', height: '100vh', display: 'flex' }}>
      <div className="sidebar">
        {TABS.map(tab => (
          <button
            key={tab.id}
            className={`sidebar-item${activeTab === tab.id ? ' active' : ''}`}
            onClick={() => setActiveTab(tab.id)}
          >
            {tab.label}
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
      </div>
    </div>
  )
}
