import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

interface LearningDraft {
  original: string
  corrected: string
  generated: string
  notice: string
  generatedOnce: boolean
}

interface LearningApplyResult {
  added: number
}

type BusyAction = 'regenerate' | 'apply' | null

export default function App() {
  const [original, setOriginal] = useState('')
  const [corrected, setCorrected] = useState('')
  const [generated, setGenerated] = useState('')
  const [notice, setNotice] = useState('正在读取学习结果…')
  const [error, setError] = useState('')
  const [busy, setBusy] = useState<BusyAction>(null)
  const [saved, setSaved] = useState(false)
  const [generatedOnce, setGeneratedOnce] = useState(false)

  const loadDraft = (draft: LearningDraft) => {
    setOriginal(draft.original)
    setCorrected(draft.corrected)
    setGenerated(draft.generated)
    setNotice(draft.notice)
    setGeneratedOnce(draft.generatedOnce)
    setSaved(false)
    setError('')
  }

  useEffect(() => {
    let disposed = false
    const unlistenPromise = listen<LearningDraft>('voice-learning-draft', event => {
      if (!disposed) loadDraft(event.payload)
    })
    invoke<LearningDraft | null>('get_voice_learning_draft')
      .then(draft => {
        if (disposed) return
        if (draft) loadDraft(draft)
        else setNotice('暂无学习草稿，请从托盘菜单重新开始。')
      })
      .catch(reason => {
        if (!disposed) setError(String(reason))
      })
    return () => {
      disposed = true
      unlistenPromise.then(unlisten => unlisten()).catch(() => {})
    }
  }, [])

  const regenerate = async () => {
    setBusy('regenerate')
    setError('')
    setSaved(false)
    try {
      const draft = await invoke<LearningDraft>('regenerate_voice_learning', {
        original,
        corrected,
      })
      loadDraft(draft)
    } catch (reason) {
      setError(String(reason))
    } finally {
      setBusy(null)
    }
  }

  const apply = async () => {
    setBusy('apply')
    setError('')
    try {
      const result = await invoke<LearningApplyResult>('apply_voice_learning_generated', {
        generated,
      })
      setSaved(true)
      setNotice(result.added > 0
        ? `已录入${result.added}条学习内容。`
        : '没有录入新内容，右栏内容已存在。')
    } catch (reason) {
      setError(String(reason))
    } finally {
      setBusy(null)
    }
  }

  const clearDraft = () => {
    setOriginal('')
    setCorrected('')
    setGenerated('')
    setGeneratedOnce(false)
    setSaved(false)
    setError('')
    setNotice('当前三栏已清空，已保存的学习记录不受影响。')
  }

  const close = () => invoke('close_voice_learning_window')

  return (
    <main className="learning-shell">
      <header className="learning-header">
        <div>
          <h1>确认学习内容</h1>
          <p>先核对前两栏，再开始学习。只有右栏会录入系统。</p>
        </div>
        <button className="button button-quiet" onClick={close} disabled={busy !== null}>取消</button>
      </header>

      <section className="editor-grid" aria-busy={busy !== null}>
        <EditorPanel
          label="原始输出"
          helper="最近一次语音转写结果"
          value={original}
          onChange={value => { setOriginal(value); setSaved(false) }}
        />
        <EditorPanel
          label="用户修订／新增要求"
          helper="可粘贴修订文本，也可直接输入要学习的词汇或规则"
          placeholder="例如：新增词汇：ByeType"
          value={corrected}
          onChange={value => { setCorrected(value); setSaved(false) }}
        />
        <EditorPanel
          label="学习结果"
          helper="每行一项，只有这里会录入系统"
          value={generated}
          onChange={value => { setGenerated(value); setSaved(false) }}
          result
        />
        {busy === 'regenerate' && (
          <div className="generating-layer" role="status">
            <strong className="generating-title">Learning…</strong>
            <div className="generating-bar" />
            <span>AI正在分析并生成学习内容…</span>
          </div>
        )}
      </section>

      <footer className="learning-footer">
        <div className={error ? 'status status-error' : saved ? 'status status-success' : 'status'}>
          {error || notice}
        </div>
        <div className="actions">
          <button
            className="button button-clear"
            onClick={clearDraft}
            disabled={busy !== null || (original === '' && corrected === '' && generated === '')}
          >
            清空
          </button>
          <button
            className="button button-secondary"
            onClick={regenerate}
            disabled={busy !== null || corrected.trim() === ''}
          >
            {busy === 'regenerate' ? '正在学习…' : generatedOnce ? '重新生成' : '开始学习'}
          </button>
          <button
            className="button button-primary"
            onClick={apply}
            disabled={busy !== null || generated.trim() === '' || saved}
          >
            {busy === 'apply' ? '正在录入…' : saved ? '已录入' : '确认录入'}
          </button>
        </div>
      </footer>
    </main>
  )
}

interface EditorPanelProps {
  label: string
  helper: string
  value: string
  onChange: (value: string) => void
  result?: boolean
  placeholder?: string
}

function EditorPanel({ label, helper, value, onChange, result = false, placeholder }: EditorPanelProps) {
  return (
    <label className={result ? 'editor-panel result-panel' : 'editor-panel'}>
      <span className="panel-heading">
        <strong>{label}</strong>
        {result && <span className="save-badge">将录入</span>}
      </span>
      <span className="panel-helper">{helper}</span>
      <textarea
        value={value}
        placeholder={placeholder}
        onChange={event => onChange(event.target.value)}
        spellCheck={false}
      />
    </label>
  )
}
