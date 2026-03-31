import { useState, useEffect, useRef, useCallback, memo } from 'react'
import { AppConfig } from '../../../core/types'
import { EditorView, basicSetup } from 'codemirror'
import { EditorState, Compartment } from '@codemirror/state'
import { markdown } from '@codemirror/lang-markdown'
import { oneDark } from '@codemirror/theme-one-dark'
import { keymap } from '@codemirror/view'
import {
  isBuiltinPromptPath,
  copyBuiltinPrompt,
  readPromptFile,
  writePromptFile,
  selectFile,
} from '../../../lib/tauri-api'

const PROMPT_FILES = [
  { key: 'agent', label: '角色定义', configPath: 'transcribe.prompts.agent' as const, builtinFilename: 'agent.md' },
  { key: 'rules', label: '转录规则', configPath: 'transcribe.prompts.rules' as const, builtinFilename: 'rules.md' },
  { key: 'vocabulary', label: '专有词汇', configPath: 'transcribe.prompts.vocabulary' as const, builtinFilename: 'vocabulary.md' },
  { key: 'textOptimize', label: '文本优化', configPath: 'optimize.prompt' as const, builtinFilename: 'text-optimize.md' },
]

function getConfigValue(config: AppConfig, configPath: string): string {
  if (configPath === 'optimize.prompt') return config.optimize.prompt
  const key = configPath.split('.').pop() as keyof AppConfig['transcribe']['prompts']
  return config.transcribe.prompts[key]
}

function setConfigValue(config: AppConfig, configPath: string, value: string): AppConfig {
  if (configPath === 'optimize.prompt') {
    return { ...config, optimize: { ...config.optimize, prompt: value } }
  }
  const key = configPath.split('.').pop() as keyof AppConfig['transcribe']['prompts']
  return {
    ...config,
    transcribe: {
      ...config.transcribe,
      prompts: { ...config.transcribe.prompts, [key]: value }
    }
  }
}

interface Props {
  config: AppConfig
  onSave: (config: AppConfig) => void
}

function PromptsTabInner({ config, onSave }: Props) {
  const [activeFile, setActiveFile] = useState(PROMPT_FILES[0].key)
  const [content, setContent] = useState('')
  const [saveStatus, setSaveStatus] = useState<'saved' | 'saving' | 'error' | 'idle'>('idle')
  const [loading, setLoading] = useState(true)
  const [resolvedPath, setResolvedPath] = useState('')

  const editorRef = useRef<HTMLDivElement>(null)
  const viewRef = useRef<EditorView | null>(null)
  const themeCompartment = useRef(new Compartment())
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const contentRef = useRef(content)
  const resolvedPathRef = useRef(resolvedPath)
  const isLoadingRef = useRef(false)

  contentRef.current = content
  resolvedPathRef.current = resolvedPath

  const activePrompt = PROMPT_FILES.find(f => f.key === activeFile)!

  const flushSave = useCallback(async () => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current)
      debounceRef.current = null
      if (resolvedPathRef.current) {
        try {
          await writePromptFile(resolvedPathRef.current, contentRef.current)
        } catch { /* ignore flush errors */ }
      }
    }
  }, [])

  const scheduleSave = useCallback((newContent: string, filePath: string) => {
    if (debounceRef.current) clearTimeout(debounceRef.current)
    debounceRef.current = setTimeout(async () => {
      debounceRef.current = null
      setSaveStatus('saving')
      try {
        await writePromptFile(filePath, newContent)
        setSaveStatus('saved')
        setTimeout(() => setSaveStatus(prev => prev === 'saved' ? 'idle' : prev), 1500)
      } catch {
        setSaveStatus('error')
      }
    }, 500)
  }, [])

  const resolvePath = useCallback(async (prompt: typeof PROMPT_FILES[number]) => {
    const customPath = getConfigValue(config, prompt.configPath)
    if (customPath) {
      const builtin = await isBuiltinPromptPath(customPath)
      if (!builtin) {
        setResolvedPath(customPath)
        return customPath
      }
    }
    const destPath = await copyBuiltinPrompt(prompt.builtinFilename)
    setResolvedPath(destPath)
    const newConfig = setConfigValue(config, prompt.configPath, destPath)
    onSave(newConfig)
    return destPath
  }, [config, onSave])

  const loadFile = useCallback(async (prompt: typeof PROMPT_FILES[number]) => {
    setLoading(true)
    setSaveStatus('idle')
    isLoadingRef.current = true
    try {
      const filePath = await resolvePath(prompt)
      const text = await readPromptFile(filePath)
      setContent(text)
      if (viewRef.current) {
        const state = viewRef.current.state
        viewRef.current.dispatch({
          changes: { from: 0, to: state.doc.length, insert: text }
        })
      }
    } catch (err: unknown) {
      setContent('')
      if (viewRef.current) {
        const state = viewRef.current.state
        viewRef.current.dispatch({
          changes: { from: 0, to: state.doc.length, insert: '' }
        })
      }
      const message = err instanceof Error ? err.message : ''
      if (message.includes('ENOENT') || message.includes('not found')) {
        setSaveStatus('idle')
      } else {
        setSaveStatus('error')
      }
    }
    isLoadingRef.current = false
    setLoading(false)
  }, [resolvePath])

  useEffect(() => {
    if (!editorRef.current || viewRef.current) return

    const isDark = document.documentElement.dataset.theme === 'dark'

    const state = EditorState.create({
      doc: '',
      extensions: [
        basicSetup,
        markdown(),
        themeCompartment.current.of(isDark ? oneDark : []),
        EditorView.lineWrapping,
        EditorView.updateListener.of(update => {
          if (update.docChanged && !isLoadingRef.current) {
            const newContent = update.state.doc.toString()
            setContent(newContent)
            contentRef.current = newContent
            if (resolvedPathRef.current) {
              scheduleSave(newContent, resolvedPathRef.current)
            }
          }
        }),
        keymap.of([]),
      ],
    })

    viewRef.current = new EditorView({
      state,
      parent: editorRef.current,
    })

    return () => {
      viewRef.current?.destroy()
      viewRef.current = null
    }
  }, [])

  useEffect(() => {
    const el = document.documentElement
    const apply = () => {
      if (viewRef.current) {
        const isDark = el.dataset.theme === 'dark'
        viewRef.current.dispatch({
          effects: themeCompartment.current.reconfigure(
            isDark ? oneDark : []
          )
        })
      }
    }
    const observer = new MutationObserver(apply)
    observer.observe(el, { attributes: true, attributeFilter: ['data-theme'] })
    return () => observer.disconnect()
  }, [])

  useEffect(() => {
    flushSave().then(() => loadFile(activePrompt))
  }, [activeFile, config])

  useEffect(() => {
    return () => { flushSave() }
  }, [flushSave])

  const handleTabSwitch = (key: string) => {
    if (key === activeFile) return
    setActiveFile(key)
  }

  const handleBrowse = async () => {
    const filePath = await selectFile()
    if (filePath) {
      const newConfig = setConfigValue(config, activePrompt.configPath, filePath)
      onSave(newConfig)
    }
  }

  const handleResetToBuiltin = async () => {
    if (!window.confirm('确定要重置为内置提示词吗？当前的修改将被覆盖。')) return
    await flushSave()
    const builtinPath = await copyBuiltinPrompt(activePrompt.builtinFilename, true)
    const newConfig = setConfigValue(config, activePrompt.configPath, builtinPath)
    onSave(newConfig)
  }

  return (
    <div>
      <div className="prompt-tabs">
        {PROMPT_FILES.map(f => (
          <button
            key={f.key}
            className={`prompt-tab${activeFile === f.key ? ' active' : ''}`}
            onClick={() => handleTabSwitch(f.key)}
          >
            {f.label}
          </button>
        ))}
      </div>

      <div className="prompt-path-bar">
        <span className="path-text">
          {resolvedPath}
        </span>
        <button className="file-picker-btn" onClick={handleBrowse}>选择文件</button>
        <button className="file-picker-btn" onClick={handleResetToBuiltin}>重置为内置</button>
      </div>

      <div className="prompt-editor-container" ref={editorRef}
        style={{ opacity: loading ? 0.5 : 1, height: 400 }} />

      <div className={`prompt-save-status ${saveStatus}`}>
        {saveStatus === 'saving' && '保存中...'}
        {saveStatus === 'saved' && '✓ 已保存'}
        {saveStatus === 'error' && '保存失败'}
      </div>
    </div>
  )
}

export const PromptsTab = memo(PromptsTabInner, (prev, next) => {
  return (
    prev.config.transcribe.prompts.agent === next.config.transcribe.prompts.agent &&
    prev.config.transcribe.prompts.rules === next.config.transcribe.prompts.rules &&
    prev.config.transcribe.prompts.vocabulary === next.config.transcribe.prompts.vocabulary &&
    prev.config.optimize.prompt === next.config.optimize.prompt &&
    prev.onSave === next.onSave
  )
})
