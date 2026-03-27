import type { AppConfig } from './types'

export interface ModelEntry {
  id: string
  provider: string
  model: string
  protocol: 'gemini' | 'openai-compat'
  baseUrl: string
  apiKey: string
  builtin: boolean
  supportsAudio: boolean
  supportsText: boolean
}

export const BUILTIN_MODELS: Omit<ModelEntry, 'apiKey'>[] = [
  {
    id: 'builtin-gemini-3-flash',
    provider: 'Google Gemini',
    model: 'gemini-3-flash-preview',
    protocol: 'gemini',
    baseUrl: 'https://generativelanguage.googleapis.com',
    builtin: true,
    supportsAudio: true,
    supportsText: true,
  },
  {
    id: 'builtin-gemini-3.1-flash-lite',
    provider: 'Google Gemini',
    model: 'gemini-3.1-flash-lite-preview',
    protocol: 'gemini',
    baseUrl: 'https://generativelanguage.googleapis.com',
    builtin: true,
    supportsAudio: true,
    supportsText: true,
  },
  {
    id: 'builtin-qwen3-omni-flash',
    provider: '\u963f\u91cc\u4e91\u767e\u70bc',
    model: 'qwen3-omni-flash',
    protocol: 'openai-compat',
    baseUrl: 'https://dashscope.aliyuncs.com/compatible-mode/v1',
    builtin: true,
    supportsAudio: true,
    supportsText: false,
  },
]

export function getAllModels(config: AppConfig): ModelEntry[] {
  const builtins: ModelEntry[] = BUILTIN_MODELS.map(b => {
    let apiKey = ''
    if (b.protocol === 'gemini') apiKey = config.models.builtinApiKeys.gemini
    else if (b.id === 'builtin-qwen3-omni-flash') apiKey = config.models.builtinApiKeys.qwen
    return { ...b, apiKey }
  })
  const customs: ModelEntry[] = config.models.custom.map(c => ({ ...c, builtin: false }))
  return [...builtins, ...customs]
}

export function getAudioModels(config: AppConfig): ModelEntry[] {
  return getAllModels(config).filter(m => m.supportsAudio)
}

export function getTextModels(config: AppConfig): ModelEntry[] {
  return getAllModels(config).filter(m => m.supportsText)
}

export function findModel(config: AppConfig, modelId: string): ModelEntry | undefined {
  return getAllModels(config).find(m => m.id === modelId)
}
