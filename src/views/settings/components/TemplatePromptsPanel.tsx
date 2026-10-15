import { useEffect, useRef, useState } from 'react'
import { AppConfig, TemplateEntry } from '../../../core/types'
import { PromptEditor } from './PromptEditor'
import { createUserPromptFile } from '../../../lib/tauri-api'

type TemplateKind = 'voice' | 'image'

const BUILTIN_IDS: Record<TemplateKind, string[]> = {
  voice: ['voice-optimize', 'voice-translate', 'voice-custom'],
  image: ['image-extract', 'image-translate', 'image-custom'],
}

const BUILTIN_FILENAMES: Record<TemplateKind, Record<string, string>> = {
  voice: { 'voice-optimize': 'text-optimize.md', 'voice-translate': 'voice-translate.md' },
  image: { 'image-extract': 'text-extract.md', 'image-translate': 'image-translate.md' },
}

const CONFIG_PREFIX: Record<TemplateKind, string> = {
  voice: 'voiceTemplates',
  image: 'extract',
}

function PencilIcon() {
  return (
    <svg width="13" height="13" viewBox="0 0 16 16" fill="none" aria-hidden="true">
      <path
        d="M11.5 2.5a1.6 1.6 0 0 1 2.3 2.3L5.6 13 2.5 13.5 3 10.4z"
        stroke="currentColor"
        strokeWidth="1.3"
        strokeLinejoin="round"
      />
    </svg>
  )
}

function TrashIcon() {
  return (
    <svg width="13" height="13" viewBox="0 0 16 16" fill="none" aria-hidden="true">
      <path
        d="M2.5 4.2h11M6.3 4.2V2.6h3.4v1.6M4 4.2l.7 9.2h6.6l.7-9.2M6.6 6.6v4.4M9.4 6.6v4.4"
        stroke="currentColor"
        strokeWidth="1.3"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  )
}

interface Props {
  kind: TemplateKind
  config: AppConfig
  onSave: (config: AppConfig) => void
}

/**
 * 模板提示词编辑：名字下拉选模板，名字旁铅笔原地改名，编辑器改提示词。
 * 保留新增/删除自定义模板、选择文件、恢复内置的能力。
 */
export function TemplatePromptsPanel({ kind, config, onSave }: Props) {
  const templates = (kind === 'voice' ? config.voiceTemplates.templates : config.extract.templates) ?? []
  const [selectedId, setSelectedId] = useState(templates[0]?.id ?? '')
  const current = templates.find(t => t.id === selectedId) ?? templates[0]
  const [menuOpen, setMenuOpen] = useState(false)
  const [editingName, setEditingName] = useState(false)
  const [nameDraft, setNameDraft] = useState(current?.name ?? '')
  const nameInputRef = useRef<HTMLInputElement>(null)
  const toolsRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    setNameDraft(current?.name ?? '')
    setMenuOpen(false)
    setEditingName(false)
  }, [current?.id])

  // 进入改名状态时聚焦并全选，直接打字即可覆盖原名
  useEffect(() => {
    if (!editingName) return
    nameInputRef.current?.focus()
    nameInputRef.current?.select()
  }, [editingName])

  // 点空白处收起下拉
  useEffect(() => {
    if (!menuOpen) return
    const onDocumentClick = (event: MouseEvent) => {
      if (!toolsRef.current?.contains(event.target as Node)) setMenuOpen(false)
    }
    document.addEventListener('mousedown', onDocumentClick)
    return () => document.removeEventListener('mousedown', onDocumentClick)
  }, [menuOpen])

  const updateTemplates = (next: TemplateEntry[]) => {
    onSave(
      kind === 'voice'
        ? { ...config, voiceTemplates: { ...config.voiceTemplates, templates: next } }
        : { ...config, extract: { ...config.extract, templates: next } }
    )
  }

  const addTemplate = async () => {
    const id = `${kind}-user-${Date.now()}`
    try {
      // 先建空白提示词文件，编辑器才有真实文件可读写
      const promptPath = await createUserPromptFile(id)
      updateTemplates([...templates, { id, name: '新模板', prompt: promptPath }])
      setSelectedId(id)
      setMenuOpen(false)
      // 新模板建好就进入改名状态，名字直接可用
      setEditingName(true)
    } catch {
      // 文件创建失败时不添加模板，避免出现保存不了的空白模板
    }
  }

  // 删除自定义模板时，把指向它的快捷键绑定改到保留的模板上，
  // 避免界面显示一个模板、后端却按已删除的模板执行。
  const deleteTemplate = () => {
    const remaining = templates.filter(t => t.id !== current.id)
    const fallback = remaining[0]?.id ?? ''
    const general = { ...config.general }
    if (kind === 'voice') {
      if (general.shortcutTemplate === current.id) general.shortcutTemplate = fallback
      if (general.shortcut2Template === current.id) general.shortcut2Template = fallback
      onSave({ ...config, general, voiceTemplates: { ...config.voiceTemplates, templates: remaining } })
    } else {
      if (general.extractShortcutTemplate === current.id) general.extractShortcutTemplate = fallback
      if (general.extractShortcut2Template === current.id) general.extractShortcut2Template = fallback
      onSave({ ...config, general, extract: { ...config.extract, templates: remaining } })
    }
    setSelectedId(remaining[0]?.id ?? '')
  }

  const commitName = () => {
    if (!current) return
    const name = nameDraft.trim()
    setEditingName(false)
    if (!name || name === current.name) {
      setNameDraft(current.name)
      return
    }
    updateTemplates(templates.map(t => (t.id === current.id ? { ...t, name } : t)))
  }

  // 列表为空时仍然保留「添加模板」，否则旧空配置再也加不回模板。
  if (!current) {
    return (
      <div className="template-tools">
        <button className="template-add-empty" onClick={addTemplate}>添加模板</button>
      </div>
    )
  }

  const isBuiltin = BUILTIN_IDS[kind].includes(current.id)

  return (
    <>
      <div className="template-tools" ref={toolsRef}>
        {editingName ? (
          <input
            ref={nameInputRef}
            className="template-name-input"
            value={nameDraft}
            onChange={e => setNameDraft(e.target.value)}
            onBlur={commitName}
            onKeyDown={e => {
              if (e.key === 'Enter') e.currentTarget.blur()
              if (e.key === 'Escape') {
                setNameDraft(current.name)
                setEditingName(false)
              }
            }}
            maxLength={20}
            aria-label="模板名称"
          />
        ) : (
          <button
            className="template-name-btn"
            onClick={() => setMenuOpen(open => !open)}
            aria-haspopup="listbox"
            aria-expanded={menuOpen}
          >
            <span className="template-name-text">{current.name}</span>
            <span className="caret">▼</span>
          </button>
        )}
        {!editingName && (
          <>
            <button
              className="template-icon-btn"
              title="重命名"
              aria-label="重命名模板"
              onClick={() => setEditingName(true)}
            >
              <PencilIcon />
            </button>
            {!isBuiltin && (
              <button
                className="template-icon-btn template-icon-danger"
                title="删除"
                aria-label="删除模板"
                onClick={deleteTemplate}
              >
                <TrashIcon />
              </button>
            )}
          </>
        )}

        {menuOpen && (
          <div className="template-menu" role="listbox" aria-label="选择模板">
            {templates.map(t => (
              <button
                key={t.id}
                className={`template-menu-item${t.id === current.id ? ' selected' : ''}`}
                role="option"
                aria-selected={t.id === current.id}
                onClick={() => {
                  setSelectedId(t.id)
                  setMenuOpen(false)
                }}
              >
                {t.name}
                {t.id === current.id && <span className="check">✓</span>}
              </button>
            ))}
            <div className="template-menu-sep" />
            <button className="template-menu-item template-menu-add" onClick={addTemplate}>
              ＋ 添加模板
            </button>
          </div>
        )}
      </div>

      <PromptEditor
        config={config}
        onSave={onSave}
        promptFiles={[{
          key: current.id,
          label: current.name,
          configPath: `${CONFIG_PREFIX[kind]}.templates.${current.id}.prompt`,
          builtinFilename: BUILTIN_FILENAMES[kind][current.id] ?? '',
        }]}
        showTabs={false}
        editorHeight={360}
      />
    </>
  )
}
