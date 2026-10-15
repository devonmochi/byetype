import { useEffect, useState } from 'react'
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

interface Props {
  kind: TemplateKind
  config: AppConfig
  onSave: (config: AppConfig) => void
}

/**
 * 模板提示词编辑：列表选择 + 一个编辑器。
 * 保留改名、新增/删除自定义模板、选择文件、恢复内置的能力。
 */
export function TemplatePromptsPanel({ kind, config, onSave }: Props) {
  const templates = (kind === 'voice' ? config.voiceTemplates.templates : config.extract.templates) ?? []
  const [selectedId, setSelectedId] = useState(templates[0]?.id ?? '')
  const current = templates.find(t => t.id === selectedId) ?? templates[0]
  const [nameDraft, setNameDraft] = useState(current?.name ?? '')

  useEffect(() => {
    setNameDraft(current?.name ?? '')
  }, [current?.id, current?.name])

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
    } catch {
      // 文件创建失败时不添加模板，避免出现保存不了的空白模板
    }
  }

  // 列表为空时仍然保留「添加模板」，否则旧空配置再也加不回模板。
  if (!current) {
    return (
      <div className="template-tools">
        <div className="template-tools-actions">
          <button className="file-picker-btn" onClick={addTemplate}>添加模板</button>
        </div>
      </div>
    )
  }

  const isBuiltin = BUILTIN_IDS[kind].includes(current.id)

  const commitName = () => {
    const name = nameDraft.trim()
    if (!name || name === current.name) {
      setNameDraft(current.name)
      return
    }
    updateTemplates(templates.map(t => (t.id === current.id ? { ...t, name } : t)))
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

  return (
    <>
      <div className="template-tools">
        <select
          className="select template-select"
          value={current.id}
          onChange={e => setSelectedId(e.target.value)}
          aria-label="选择模板"
        >
          {templates.map(t => (
            <option key={t.id} value={t.id}>{t.name}</option>
          ))}
        </select>
        <input
          className="input"
          value={nameDraft}
          onChange={e => setNameDraft(e.target.value)}
          onBlur={commitName}
          onKeyDown={e => {
            if (e.key === 'Enter') e.currentTarget.blur()
          }}
          maxLength={20}
          title="模板名称，改完点击别处生效"
          aria-label="模板名称"
          style={{ width: 180 }}
        />
        <div className="template-tools-actions">
          <button className="file-picker-btn" onClick={addTemplate}>添加模板</button>
          {!isBuiltin && (
            <button className="file-picker-btn" onClick={deleteTemplate}>删除</button>
          )}
        </div>
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
