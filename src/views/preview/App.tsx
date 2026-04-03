import { useState, useEffect, useRef } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { emit, listen } from '@tauri-apps/api/event'

export default function App() {
  const [text, setText] = useState('')
  const [copied, setCopied] = useState(false)
  const textareaRef = useRef<HTMLTextAreaElement>(null)

  useEffect(() => {
    const unlisten = listen<string>('preview-text', (event) => {
      setText(event.payload)
    })
    emit('preview-ready', {})
    return () => { unlisten.then(fn => fn()) }
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

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100vh', padding: '8px' }}>
      <div style={{ display: 'flex', justifyContent: 'flex-start', padding: '4px 0 8px 0', gap: '8px', ...({ WebkitAppRegion: 'drag' } as any) }}>
        <button
          onClick={handleCopy}
          style={{
            ...({ WebkitAppRegion: 'no-drag' } as any),
            padding: '4px 12px',
            border: '1px solid #444',
            borderRadius: '4px',
            background: copied ? '#2d5a2d' : '#2a2a2a',
            color: copied ? '#90ee90' : '#ccc',
            cursor: 'pointer',
            fontSize: '12px',
            transition: 'all 0.2s',
          }}
        >
          {copied ? '已复制' : '复制'}
        </button>
      </div>
      <textarea
        ref={textareaRef}
        value={text}
        onChange={(e) => setText(e.target.value)}
        style={{
          flex: 1,
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
        }}
      />
    </div>
  )
}
