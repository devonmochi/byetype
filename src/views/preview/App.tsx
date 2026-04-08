import { useState, useEffect, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { emit, listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'

type ThemeMode = 'light' | 'dark' | 'system'

function useIsDark(): boolean {
  const [isDark, setIsDark] = useState(() =>
    window.matchMedia('(prefers-color-scheme: dark)').matches
  )

  useEffect(() => {
    let themeMode: ThemeMode = 'system'

    const apply = () => {
      if (themeMode === 'system') {
        setIsDark(window.matchMedia('(prefers-color-scheme: dark)').matches)
      } else {
        setIsDark(themeMode === 'dark')
      }
    }

    invoke('get_config').then((config: any) => {
      themeMode = config?.general?.theme ?? 'system'
      apply()
    }).catch(() => {})

    const mq = window.matchMedia('(prefers-color-scheme: dark)')
    const handler = () => apply()
    mq.addEventListener('change', handler)
    return () => mq.removeEventListener('change', handler)
  }, [])

  return isDark
}

const light = {
  bg: '#f5f5f5',
  titlebar: '#e8e8e8',
  textareaBg: '#ffffff',
  textareaColor: '#333333',
  textareaBorder: '#d0d0d0',
  secondaryText: '#666',
  btnBg: '#e0e0e0',
  btnBorder: '#ccc',
  btnText: '#555',
  pinBg: '#d5e8d5',
  pinBorder: '#8aba8a',
  pinStroke: '#4a7a4a',
  unpinStroke: '#999',
  copiedBg: '#c8e6c8',
  copiedText: '#2d7a2d',
}

const dark = {
  bg: '#1a1a1a',
  titlebar: '#252525',
  textareaBg: '#111111',
  textareaColor: '#e0e0e0',
  textareaBorder: '#333333',
  secondaryText: '#888',
  btnBg: '#2a2a2a',
  btnBorder: '#444',
  btnText: '#ccc',
  pinBg: '#2d4a2d',
  pinBorder: '#4a7a4a',
  pinStroke: '#90ee90',
  unpinStroke: '#888',
  copiedBg: '#2d5a2d',
  copiedText: '#90ee90',
}

const PinIcon = ({ pinned, stroke }: { pinned: boolean; stroke: string }) => (
  <svg
    width="12"
    height="12"
    viewBox="0 0 24 24"
    fill="none"
    stroke={stroke}
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
    style={{ transform: pinned ? 'none' : 'rotate(45deg)', transition: 'transform 0.2s' }}
  >
    <line x1="12" y1="17" x2="12" y2="22" />
    <path d="M5 17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1v4.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24z" />
  </svg>
)

export default function App() {
  const isDark = useIsDark()
  const t = isDark ? dark : light
  const [text, setText] = useState('')
  const [copied, setCopied] = useState(false)
  const [pinned, setPinned] = useState(false)
  const textareaRef = useRef<HTMLTextAreaElement>(null)

  useEffect(() => {
    let unlisten: (() => void) | null = null
    listen<string>('preview-text', (event) => {
      setText(event.payload)
    }).then((fn) => {
      unlisten = fn
      emit('preview-ready', {})
    })
    return () => { unlisten?.() }
  }, [])

  useEffect(() => {
    const handleBlur = () => {
      invoke('update_clipboard_text', { text })
    }
    window.addEventListener('blur', handleBlur)
    return () => window.removeEventListener('blur', handleBlur)
  }, [text])

  const handleCopy = async () => {
    await invoke('update_clipboard_text', { text })
    setCopied(true)
    setTimeout(() => setCopied(false), 1500)
  }

  const handlePin = async () => {
    const next = !pinned
    setPinned(next)
    await invoke('set_preview_pinned', { pinned: next })
  }

  const handleClose = () => {
    invoke('close_preview_window')
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100vh', background: t.bg }}>
      {/* 标题栏 */}
      <div
        onMouseDown={() => getCurrentWindow().startDragging()}
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '6px 10px',
          background: t.titlebar,
          cursor: 'grab',
          userSelect: 'none',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          <button
            onClick={handlePin}
            title={pinned ? '取消固定' : '固定窗口'}
            style={{
              width: '22px',
              height: '22px',
              borderRadius: '4px',
              background: pinned ? t.pinBg : t.btnBg,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              cursor: 'pointer',
              border: `1px solid ${pinned ? t.pinBorder : t.btnBorder}`,
              padding: 0,
              transition: 'all 0.2s',
            }}
          >
            <PinIcon pinned={pinned} stroke={pinned ? t.pinStroke : t.unpinStroke} />
          </button>
          <span style={{ color: t.secondaryText, fontSize: '11px' }}>识别结果</span>
        </div>
        <button
          onClick={handleClose}
          title="关闭"
          style={{
            padding: '3px 8px',
            border: `1px solid ${t.btnBorder}`,
            borderRadius: '4px',
            background: t.btnBg,
            color: t.secondaryText,
            fontSize: '11px',
            cursor: 'pointer',
          }}
        >
          ✕
        </button>
      </div>

      {/* 文本编辑区 */}
      <div style={{ flex: 1, padding: '8px' }}>
        <textarea
          ref={textareaRef}
          value={text}
          onChange={(e) => setText(e.target.value)}
          style={{
            width: '100%',
            height: '100%',
            background: t.textareaBg,
            color: t.textareaColor,
            border: `1px solid ${t.textareaBorder}`,
            borderRadius: '6px',
            padding: '10px',
            fontSize: '13px',
            lineHeight: '1.6',
            fontFamily: '-apple-system, BlinkMacSystemFont, monospace',
            resize: 'none',
            outline: 'none',
            boxSizing: 'border-box',
          }}
        />
      </div>

      {/* 底部栏 */}
      <div style={{ display: 'flex', alignItems: 'center', padding: '4px 10px 8px' }}>
        <button
          onClick={handleCopy}
          style={{
            padding: '3px 10px',
            border: `1px solid ${t.btnBorder}`,
            borderRadius: '4px',
            background: copied ? t.copiedBg : t.btnBg,
            color: copied ? t.copiedText : t.btnText,
            fontSize: '11px',
            cursor: 'pointer',
            transition: 'all 0.2s',
          }}
        >
          {copied ? '已复制' : '复制'}
        </button>
      </div>
    </div>
  )
}
