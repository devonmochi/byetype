import type { AppConfig } from './types'

export interface ModelEntry {
  id: string
  provider: string
  model: string
  protocol: 'gemini' | 'openai-compat' | 'longcat' | 'qwen-omni'
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
    id: 'builtin-deepseek-chat',
    provider: 'DeepSeek',
    model: 'deepseek-chat',
    protocol: 'openai-compat',
    baseUrl: 'https://api.deepseek.com/v1',
    builtin: true,
    supportsAudio: false,
    supportsText: true,
  },
  {
    id: 'builtin-longcat-flash-omni',
    provider: 'LongCat',
    model: 'LongCat-Flash-Omni-2603',
    protocol: 'longcat',
    baseUrl: 'https://api.longcat.chat/openai/v1',
    builtin: true,
    supportsAudio: true,
    supportsText: false,
  },
  {
    id: 'builtin-qwen-omni-plus',
    provider: '阿里云百炼',
    model: 'qwen3.5-omni-plus',
    protocol: 'qwen-omni',
    baseUrl: 'https://dashscope.aliyuncs.com/compatible-mode/v1',
    builtin: true,
    supportsAudio: true,
    supportsText: true,
  },
  {
    id: 'builtin-qwen-omni-flash',
    provider: '阿里云百炼',
    model: 'qwen3.5-omni-flash',
    protocol: 'qwen-omni',
    baseUrl: 'https://dashscope.aliyuncs.com/compatible-mode/v1',
    builtin: true,
    supportsAudio: true,
    supportsText: true,
  },
]

export function getAllModels(config: AppConfig): ModelEntry[] {
  const builtins: ModelEntry[] = BUILTIN_MODELS.map(b => {
    let apiKey = ''
    if (b.protocol === 'gemini') apiKey = config.models.builtinApiKeys.gemini
    else if (b.protocol === 'longcat') apiKey = config.models.builtinApiKeys.longcat
    else if (b.id === 'builtin-deepseek-chat') apiKey = config.models.builtinApiKeys.deepseek
    else if (b.protocol === 'qwen-omni') apiKey = config.models.builtinApiKeys.dashscope
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
