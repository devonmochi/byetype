import { useState } from 'react'
import type { AppConfig, CustomModelEntry } from '../../../core/types'
import { BUILTIN_MODELS, getAllModels } from '../../../core/models'
import { testModelConnectivity, type ConnectivityResult } from '../../../lib/tauri-api'

interface Props {
  config: AppConfig
  onSave: (config: AppConfig) => void
}

interface TestResults {
  [modelId: string]: { loading: boolean; result?: ConnectivityResult }
}

const EMPTY_FORM: Omit<CustomModelEntry, 'id'> = {
  provider: '', model: '', protocol: 'gemini', baseUrl: '', apiKey: '', supportsAudio: true, supportsText: true, supportsVision: true,
}

export function ModelsTab({ config, onSave }: Props) {
  const [testResults, setTestResults] = useState<TestResults>({})
  const [showForm, setShowForm] = useState(false)
  const [editingId, setEditingId] = useState<string | null>(null)
  const [form, setForm] = useState(EMPTY_FORM)

  const [visibleKeys, setVisibleKeys] = useState<Record<string, boolean>>({})
  const [showCustom, setShowCustom] = useState(false)

  const toggleKeyVisibility = (key: string) => {
    setVisibleKeys(prev => ({ ...prev, [key]: !prev[key] }))
  }

  const EyeIcon = ({ visible }: { visible: boolean }) => (
    visible ? (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
        <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
        <circle cx="12" cy="12" r="3" />
      </svg>
    ) : (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
        <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94" />
        <path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19" />
        <path d="M14.12 14.12a3 3 0 1 1-4.24-4.24" />
        <line x1="1" y1="1" x2="23" y2="23" />
      </svg>
    )
  )

  const updateBuiltinKey = (key: 'gemini' | 'deepseek' | 'dashscope' | 'openrouter', value: string) => {
    onSave({ ...config, models: { ...config.models, builtinApiKeys: { ...config.models.builtinApiKeys, [key]: value } } })
  }

  const testModel = async (modelId: string) => {
    setTestResults(prev => ({ ...prev, [modelId]: { loading: true } }))
    try {
      const result = await testModelConnectivity(modelId)
      setTestResults(prev => ({ ...prev, [modelId]: { loading: false, result } }))
    } catch (e) {
      setTestResults(prev => ({ ...prev, [modelId]: { loading: false, result: { success: false, latencyMs: 0, error: String(e) } } }))
    }
  }

  const testAll = async () => {
    const models = getAllModels(config)
    for (const m of models) { testModel(m.id) }
  }

  const saveCustomModel = () => {
    const id = editingId || crypto.randomUUID()
    const entry: CustomModelEntry = { ...form, id }
    const custom = editingId
      ? config.models.custom.map(m => (m.id === editingId ? entry : m))
      : [...config.models.custom, entry]
    onSave({ ...config, models: { ...config.models, custom } })
    setShowForm(false); setEditingId(null); setForm(EMPTY_FORM)
  }

  const deleteCustomModel = (id: string) => {
    const custom = config.models.custom.filter(m => m.id !== id)
    onSave({ ...config, models: { ...config.models, custom } })
  }

  const startEdit = (entry: CustomModelEntry) => {
    setEditingId(entry.id)
    setForm({ provider: entry.provider, model: entry.model, protocol: entry.protocol, baseUrl: entry.baseUrl, apiKey: entry.apiKey, supportsAudio: entry.supportsAudio, supportsText: entry.supportsText, supportsVision: entry.supportsVision })
    setShowForm(true)
  }

  const cancelForm = () => { setShowForm(false); setEditingId(null); setForm(EMPTY_FORM) }

  const renderTestResult = (modelId: string) => {
    const t = testResults[modelId]
    if (!t) return null
    if (t.loading) return <span className="model-test-result" style={{ color: 'var(--text-tertiary)' }}>...</span>
    if (t.result?.success) return <span className="model-test-result success">{t.result.latencyMs}ms</span>
    return <span className="model-test-result error" title={t.result?.error || ''}>{t.result?.error?.slice(0, 30) || '失败'}</span>
  }

  const geminiKey = config.models.builtinApiKeys.gemini
  const deepseekKey = config.models.builtinApiKeys.deepseek

  const builtinByProvider = BUILTIN_MODELS.reduce<Record<string, { keyField: 'gemini' | 'deepseek' | 'dashscope' | 'openrouter'; placeholder: string; models: typeof BUILTIN_MODELS }>>((acc, m) => {
    if (!acc[m.provider]) {
      const keyField = m.provider === 'OpenRouter' ? 'openrouter' : m.protocol === 'gemini' ? 'gemini' : m.protocol === 'qwen-omni' ? 'dashscope' : 'deepseek'
      const placeholder = m.provider === 'OpenRouter' ? 'sk-or-v1-...' : m.protocol === 'gemini' ? 'AIzaSy...' : 'sk-...'
      acc[m.provider] = { keyField, placeholder, models: [] }
    }
    acc[m.provider].models.push(m)
    return acc
  }, {})

  return (
    <div>
      <div className="models-header">
        <h2 className="content-title" style={{ margin: 0 }}>模型管理</h2>
        <button className="test-all-btn" onClick={testAll}>测试全部连通性</button>
      </div>

      <div className="models-section-title">预置模型</div>
      {Object.entries(builtinByProvider).map(([provider, group]) => {
        const keyValue = group.keyField === 'gemini' ? geminiKey
          : group.keyField === 'dashscope' ? config.models.builtinApiKeys.dashscope
          : group.keyField === 'openrouter' ? config.models.builtinApiKeys.openrouter
          : deepseekKey
        return (
          <div key={provider} className="model-card">
            <div className="model-card-header">
              <span className="model-card-title">{provider}</span>
            </div>
            <div className="model-card-row">
              <label>API Key</label>
              <div className="api-key-wrapper">
                <input className="input" type={visibleKeys[group.keyField] ? 'text' : 'password'} value={keyValue} onChange={e => updateBuiltinKey(group.keyField, e.target.value)} placeholder={group.placeholder} />
                {keyValue && (
                  <button className="api-key-toggle" onClick={() => toggleKeyVisibility(group.keyField)} title={visibleKeys[group.keyField] ? '隐藏密钥' : '显示密钥'}>
                    <EyeIcon visible={visibleKeys[group.keyField]} />
                  </button>
                )}
              </div>
            </div>
            <div className="provider-model-list">
              {group.models.map(m => (
                <div key={m.id} className="provider-model-item">
                  <span className="provider-model-name">{m.model}</span>
                  <span className="model-caps">
                    {m.supportsAudio && <span className="cap-tag cap-audio">音频</span>}
                    {m.supportsVision && <span className="cap-tag cap-vision">图像</span>}
                    {m.supportsText && <span className="cap-tag cap-text">文本</span>}
                  </span>
                  <div className="model-card-actions">
                    {renderTestResult(m.id)}
                    <button className="model-test-btn" onClick={() => testModel(m.id)}>测试</button>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )
      })}

      <div className="models-section-title" style={{ cursor: 'pointer', userSelect: 'none' }} onClick={() => setShowCustom(!showCustom)}>
        自定义模型 {showCustom ? '▼' : '▶'}
      </div>
      {showCustom ? (
        <>
          {config.models.custom.map(entry => (
            <div key={entry.id} className="model-card">
              <div className="model-card-header">
                <span className="model-card-title">{entry.provider} - {entry.model}</span>
                <div className="model-card-actions">
                  {renderTestResult(entry.id)}
                  <button className="model-test-btn" onClick={() => testModel(entry.id)}>测试</button>
                  <button className="model-action-btn" onClick={() => startEdit(entry)}>编辑</button>
                  <button className="model-action-btn danger" onClick={() => deleteCustomModel(entry.id)}>删除</button>
                </div>
              </div>
              <div className="model-card-subtitle">
                {entry.protocol} · {entry.baseUrl}
                <span className="model-caps" style={{ marginLeft: 8 }}>
                  {entry.supportsAudio && <span className="cap-tag cap-audio">音频</span>}
                  {entry.supportsVision && <span className="cap-tag cap-vision">图像</span>}
                  {entry.supportsText && <span className="cap-tag cap-text">文本</span>}
                </span>
              </div>
            </div>
          ))}

      {showForm ? (
        <div className="model-form">
          <div style={{ fontWeight: 600, fontSize: 13, marginBottom: 12, color: 'var(--text-primary)' }}>{editingId ? '编辑模型' : '新建模型'}</div>
          <div className="model-form-row">
            <label>协议类型</label>
            <div style={{ display: 'flex', gap: 12 }}>
              <label style={{ display: 'flex', alignItems: 'center', gap: 4, fontSize: 12, cursor: 'pointer', color: 'var(--text-primary)' }}>
                <input type="radio" checked={form.protocol === 'gemini'} onChange={() => setForm(f => ({ ...f, protocol: 'gemini', baseUrl: '' }))} /> Gemini
              </label>
              <label style={{ display: 'flex', alignItems: 'center', gap: 4, fontSize: 12, cursor: 'pointer', color: 'var(--text-primary)' }}>
                <input type="radio" checked={form.protocol === 'openai-compat' && form.baseUrl !== 'https://openrouter.ai/api/v1'} onChange={() => setForm(f => ({ ...f, protocol: 'openai-compat', baseUrl: '' }))} /> OpenAI 兼容
              </label>
              <label style={{ display: 'flex', alignItems: 'center', gap: 4, fontSize: 12, cursor: 'pointer', color: 'var(--text-primary)' }}>
                <input type="radio" checked={form.protocol === 'openai-compat' && form.baseUrl === 'https://openrouter.ai/api/v1'} onChange={() => setForm(f => ({ ...f, protocol: 'openai-compat', baseUrl: 'https://openrouter.ai/api/v1' }))} /> OpenRouter
              </label>
            </div>
          </div>
          <div className="model-form-row">
            <label>模型能力</label>
            <div style={{ display: 'flex', gap: 12 }}>
              <label style={{ display: 'flex', alignItems: 'center', gap: 4, fontSize: 12, cursor: 'pointer', color: 'var(--text-primary)' }}>
                <input type="checkbox" checked={form.supportsAudio} onChange={e => setForm(f => ({ ...f, supportsAudio: e.target.checked }))} /> 音频转写
              </label>
              <label style={{ display: 'flex', alignItems: 'center', gap: 4, fontSize: 12, cursor: 'pointer', color: 'var(--text-primary)' }}>
                <input type="checkbox" checked={form.supportsVision} onChange={e => setForm(f => ({ ...f, supportsVision: e.target.checked }))} /> 图像识别
              </label>
              <label style={{ display: 'flex', alignItems: 'center', gap: 4, fontSize: 12, cursor: 'pointer', color: 'var(--text-primary)' }}>
                <input type="checkbox" checked={form.supportsText} onChange={e => setForm(f => ({ ...f, supportsText: e.target.checked }))} /> 文本处理
              </label>
            </div>
          </div>
          <div className="model-form-row"><label>Provider</label><input className="input" value={form.provider} onChange={e => setForm(f => ({ ...f, provider: e.target.value }))} placeholder="提供商名称" style={{ flex: 1, maxWidth: 300 }} /></div>
          <div className="model-form-row"><label>Base URL</label><input className="input" value={form.baseUrl} onChange={e => setForm(f => ({ ...f, baseUrl: e.target.value }))} placeholder="https://api.example.com/v1" style={{ flex: 1, maxWidth: 400 }} /></div>
          <div className="model-form-row"><label>Model ID</label><input className="input" value={form.model} onChange={e => setForm(f => ({ ...f, model: e.target.value }))} placeholder="gemini-3-flash-preview" style={{ flex: 1, maxWidth: 300 }} /></div>
          <div className="model-form-row">
            <label>API Key</label>
            <div className="api-key-wrapper" style={{ maxWidth: 400 }}>
              <input className="input" type={visibleKeys['custom-form'] ? 'text' : 'password'} value={form.apiKey} onChange={e => setForm(f => ({ ...f, apiKey: e.target.value }))} />
              {form.apiKey && (
                <button className="api-key-toggle" onClick={() => toggleKeyVisibility('custom-form')} title={visibleKeys['custom-form'] ? '隐藏密钥' : '显示密钥'}>
                  <EyeIcon visible={visibleKeys['custom-form']} />
                </button>
              )}
            </div>
          </div>
          <div className="model-form-actions">
            <button className="model-form-btn" onClick={cancelForm}>取消</button>
            <button className="model-form-btn primary" onClick={saveCustomModel} disabled={!form.provider || !form.baseUrl || !form.model || (!form.supportsAudio && !form.supportsText && !form.supportsVision)}>保存</button>
          </div>
        </div>
      ) : (
        <button className="add-model-btn" onClick={() => { setEditingId(null); setForm(EMPTY_FORM); setShowForm(true) }}>+ 添加自定义模型</button>
      )}
        </>
      ) : (
        <button className="add-model-btn" onClick={() => setShowCustom(true)} style={{ color: 'var(--text-tertiary)', borderColor: 'var(--border-secondary)', fontSize: 12 }}>展开自定义模型</button>
      )}
    </div>
  )
}
