import { useState, useEffect, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { emit, listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'

const PinIcon = ({ pinned }: { pinned: boolean }) => (
  <svg
    width="12"
    height="12"
    viewBox="0 0 24 24"
    fill="none"
    stroke={pinned ? '#90ee90' : '#888'}
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
    <div style={{ display: 'flex', flexDirection: 'column', height: '100vh' }}>
      {/* 标题栏 */}
      <div
        onMouseDown={() => getCurrentWindow().startDragging()}
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '6px 10px',
          background: '#252525',
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
              background: pinned ? '#2d4a2d' : '#333',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              cursor: 'pointer',
              border: `1px solid ${pinned ? '#4a7a4a' : '#444'}`,
              padding: 0,
              transition: 'all 0.2s',
            }}
          >
            <PinIcon pinned={pinned} />
          </button>
          <span style={{ color: '#888', fontSize: '11px' }}>识别结果</span>
        </div>
        <button
          onClick={handleClose}
          title="关闭"
          style={{
            padding: '3px 8px',
            border: '1px solid #444',
            borderRadius: '4px',
            background: '#2a2a2a',
            color: '#888',
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
            background: '#111',
            color: '#e0e0e0',
            border: '1px solid #333',
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
            border: '1px solid #444',
            borderRadius: '4px',
            background: copied ? '#2d5a2d' : '#2a2a2a',
            color: copied ? '#90ee90' : '#ccc',
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
