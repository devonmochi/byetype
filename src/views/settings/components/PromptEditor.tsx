import { useState, useEffect, useRef, useCallback } from 'react'
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
  onEvent,
} from '../../../lib/tauri-api'
import { ask } from '@tauri-apps/plugin-dialog'

export interface PromptFileEntry {
  key: string
  label: string
  configPath?: string
  builtinFilename?: string
  resolvePath?: () => Promise<string>
  loadContent?: () => Promise<{ path: string; content: string }>
  saveContent?: (content: string, baseContent: string) => Promise<{ path: string; content: string }>
  refreshEvent?: string
}

function getConfigValue(config: AppConfig, configPath: string): string {
  if (configPath.startsWith('voiceTemplates.templates.')) {
    const templateId = configPath.split('.')[2]
    const template = config.voiceTemplates.templates.find(t => t.id === templateId)
    return template?.prompt ?? ''
  }
  if (configPath.startsWith('extract.templates.')) {
    const templateId = configPath.split('.')[2]
    const template = config.extract.templates.find(t => t.id === templateId)
    return template?.prompt ?? ''
  }
  if (configPath === 'extract.prompt') return config.extract.prompt
  const key = configPath.split('.').pop() as keyof AppConfig['transcribe']['prompts']
  return config.transcribe.prompts[key]
}

function setConfigValue(config: AppConfig, configPath: string, value: string): AppConfig {
  if (configPath.startsWith('voiceTemplates.templates.')) {
    const templateId = configPath.split('.')[2]
    return {
      ...config,
      voiceTemplates: {
        ...config.voiceTemplates,
        templates: config.voiceTemplates.templates.map(t =>
          t.id === templateId ? { ...t, prompt: value } : t
        ),
      },
    }
  }
  if (configPath.startsWith('extract.templates.')) {
    const templateId = configPath.split('.')[2]
    return {
      ...config,
      extract: {
        ...config.extract,
        templates: config.extract.templates.map(t =>
          t.id === templateId ? { ...t, prompt: value } : t
        ),
      },
    }
  }
  if (configPath === 'extract.prompt') {
    return { ...config, extract: { ...config.extract, prompt: value } }
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
  promptFiles: PromptFileEntry[]
  showTabs?: boolean
}

export function PromptEditor({ config, onSave, promptFiles, showTabs = true }: Props) {
  const [activeFile, setActiveFile] = useState(promptFiles[0]?.key ?? '')
  const [content, setContent] = useState('')
  const [saveStatus, setSaveStatus] = useState<'saved' | 'saving' | 'error' | 'idle'>('idle')
  const [loading, setLoading] = useState(true)
  const [resolvedPath, setResolvedPath] = useState('')

  const editorRef = useRef<HTMLDivElement>(null)
  const viewRef = useRef<EditorView | null>(null)
  const themeCompartment = useRef(new Compartment())
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const pendingPromptRef = useRef<PromptFileEntry | null>(null)
  const savePromiseRef = useRef<Promise<void> | null>(null)
  const persistedContentRef = useRef('')
  const loadRequestRef = useRef(0)
  const contentRef = useRef(content)
  const resolvedPathRef = useRef(resolvedPath)
  const isLoadingRef = useRef(false)
  const configRef = useRef(config)
  configRef.current = config

  contentRef.current = content
  resolvedPathRef.current = resolvedPath

  const activePrompt = promptFiles.find(f => f.key === activeFile) ?? promptFiles[0]
  const activePromptKey = activePrompt?.key ?? ''
  const activePromptRef = useRef(activePrompt)
  activePromptRef.current = activePrompt

  useEffect(() => {
    if (!promptFiles.some(f => f.key === activeFile) && promptFiles.length > 0) {
      setActiveFile(promptFiles[0].key)
    }
  }, [promptFiles, activeFile])

  const saveFile = useCallback(async (prompt: PromptFileEntry, filePath: string, newContent: string) => {
    const previous = savePromiseRef.current
    const save = (previous ? previous.catch(() => undefined) : Promise.resolve()).then(async () => {
      const result = prompt.saveContent
        ? await prompt.saveContent(newContent, persistedContentRef.current)
        : (await writePromptFile(filePath, newContent), { path: filePath, content: newContent })
      persistedContentRef.current = result.content
      if (contentRef.current === newContent && result.content !== newContent && viewRef.current) {
        isLoadingRef.current = true
        const state = viewRef.current.state
        viewRef.current.dispatch({
          changes: { from: 0, to: state.doc.length, insert: result.content }
        })
        setContent(result.content)
        contentRef.current = result.content
        isLoadingRef.current = false
      }
    })
    savePromiseRef.current = save
    try {
      await save
    } finally {
      if (savePromiseRef.current === save) savePromiseRef.current = null
    }
  }, [])

  const flushSave = useCallback(async () => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current)
      debounceRef.current = null
      if (resolvedPathRef.current) {
        try {
          const prompt = pendingPromptRef.current
          if (prompt) await saveFile(prompt, resolvedPathRef.current, contentRef.current)
        } catch { /* ignore flush errors */ }
      }
    }
    if (savePromiseRef.current) await savePromiseRef.current
  }, [saveFile])

  const scheduleSave = useCallback((prompt: PromptFileEntry, newContent: string, filePath: string) => {
    if (debounceRef.current) clearTimeout(debounceRef.current)
    pendingPromptRef.current = prompt
    debounceRef.current = setTimeout(async () => {
      debounceRef.current = null
      setSaveStatus('saving')
      try {
        await saveFile(prompt, filePath, newContent)
        setSaveStatus('saved')
        setTimeout(() => setSaveStatus(prev => prev === 'saved' ? 'idle' : prev), 1500)
      } catch {
        setSaveStatus('error')
      }
    }, 500)
  }, [saveFile])

  const resolvePath = useCallback(async (prompt: PromptFileEntry) => {
    if (prompt.resolvePath) {
      const filePath = await prompt.resolvePath()
      setResolvedPath(filePath)
      return filePath
    }

    if (!prompt.configPath || !prompt.builtinFilename) {
      throw new Error('提示词文件配置不完整')
    }
    const currentConfig = configRef.current
    const customPath = getConfigValue(currentConfig, prompt.configPath)
    if (customPath) {
      const builtin = await isBuiltinPromptPath(customPath)
      if (!builtin) {
        setResolvedPath(customPath)
        return customPath
      }
    }
    const destPath = await copyBuiltinPrompt(prompt.builtinFilename)
    setResolvedPath(destPath)
    const newConfig = setConfigValue(currentConfig, prompt.configPath, destPath)
    onSave(newConfig)
    return destPath
  }, [onSave])

  const loadFile = useCallback(async (prompt: PromptFileEntry) => {
    const requestId = ++loadRequestRef.current
    setLoading(true)
    setSaveStatus('idle')
    isLoadingRef.current = true
    try {
      const loaded = prompt.loadContent
        ? await prompt.loadContent()
        : await resolvePath(prompt).then(async path => ({ path, content: await readPromptFile(path) }))
      if (requestId !== loadRequestRef.current) return
      const filePath = loaded.path
      const text = loaded.content
      setResolvedPath(filePath)
      persistedContentRef.current = text
      setContent(text)
      if (viewRef.current) {
        const state = viewRef.current.state
        viewRef.current.dispatch({
          changes: { from: 0, to: state.doc.length, insert: text }
        })
      }
    } catch (err: unknown) {
      if (requestId !== loadRequestRef.current) return
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
    if (requestId === loadRequestRef.current) {
      isLoadingRef.current = false
      setLoading(false)
    }
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
              const prompt = activePromptRef.current
              if (prompt) scheduleSave(prompt, newContent, resolvedPathRef.current)
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
  }, [scheduleSave])

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
    const prompt = promptFiles.find(f => f.key === activePromptKey) ?? promptFiles[0]
    if (!prompt) return
    flushSave().then(() => loadFile(prompt))
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activePromptKey])

  useEffect(() => {
    const prompt = promptFiles.find(f => f.key === activePromptKey) ?? promptFiles[0]
    if (!prompt?.refreshEvent) return

    let unlisten: (() => void) | null = null
    let cancelled = false
    onEvent(prompt.refreshEvent, async () => {
      await flushSave()
      if (cancelled) return
      await loadFile(prompt)
    }).then(fn => {
      if (cancelled) fn()
      else unlisten = fn
    })

    return () => {
      cancelled = true
      unlisten?.()
    }
  }, [activePromptKey, flushSave, loadFile, promptFiles])

  useEffect(() => {
    return () => { flushSave() }
  }, [flushSave])

  const handleTabSwitch = (key: string) => {
    if (key === activeFile) return
    setActiveFile(key)
  }

  const handleBrowse = async () => {
    if (!activePrompt?.configPath) return
    const filePath = await selectFile()
    if (filePath) {
      await flushSave()
      const newConfig = setConfigValue(configRef.current, activePrompt.configPath, filePath)
      onSave(newConfig)
      await loadFile(activePrompt)
    }
  }

  const handleResetToBuiltin = async () => {
    if (!activePrompt?.configPath || !activePrompt.builtinFilename) return
    const yes = await ask('确定要重置为内置提示词吗？当前的修改将被覆盖。', { title: 'ByeType', kind: 'warning' })
    if (!yes) return
    await flushSave()
    const builtinPath = await copyBuiltinPrompt(activePrompt.builtinFilename, true)
    const newConfig = setConfigValue(configRef.current, activePrompt.configPath, builtinPath)
    onSave(newConfig)
    await loadFile(activePrompt)
  }

  if (promptFiles.length === 0) return null

  return (
    <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minHeight: 0 }}>
      {showTabs && promptFiles.length > 1 && (
        <div className="prompt-tabs">
          {promptFiles.map(f => (
            <button
              key={f.key}
              className={`prompt-tab${activeFile === f.key ? ' active' : ''}`}
              onClick={() => handleTabSwitch(f.key)}
            >
              {f.label}
            </button>
          ))}
        </div>
      )}

      <div className="prompt-path-bar">
        <span className="path-text">
          {resolvedPath}
        </span>
        {!activePrompt?.resolvePath && !activePrompt?.loadContent && (
          <>
            <button className="file-picker-btn" onClick={handleBrowse}>选择文件</button>
            <button className="file-picker-btn" onClick={handleResetToBuiltin}>重置为内置</button>
          </>
        )}
      </div>

      <div className="prompt-editor-container" ref={editorRef}
        style={{ opacity: loading ? 0.5 : 1, flex: 1, minHeight: 150 }} />

      <div className={`prompt-save-status ${saveStatus}`}>
        {saveStatus === 'saving' && '保存中...'}
        {saveStatus === 'saved' && '✓ 已保存'}
        {saveStatus === 'error' && '保存失败'}
      </div>
    </div>
  )
}
