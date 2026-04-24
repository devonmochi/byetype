import type { AppConfig } from './types'

export interface ModelEntry {
  id: string
  provider: string
  model: string
  protocol: 'gemini' | 'openai-compat' | 'qwen-omni' | 'mimo' | 'longcat'
  baseUrl: string
  apiKey: string
  builtin: boolean
  supportsAudio: boolean
  supportsText: boolean
  supportsVision: boolean
}

export const BUILTIN_MODELS: Omit<ModelEntry, 'apiKey'>[] = [
  {
    id: 'builtin-qwen-omni-plus',
    provider: '阿里云百炼',
    model: 'qwen3.5-omni-plus',
    protocol: 'qwen-omni',
    baseUrl: 'https://dashscope.aliyuncs.com/compatible-mode/v1',
    builtin: true,
    supportsAudio: true,
    supportsText: true,
    supportsVision: true,
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
    supportsVision: true,
  },
  {
    id: 'builtin-gemini-3-flash',
    provider: 'Google Gemini',
    model: 'gemini-3-flash-preview',
    protocol: 'gemini',
    baseUrl: 'https://generativelanguage.googleapis.com',
    builtin: true,
    supportsAudio: true,
    supportsText: true,
    supportsVision: true,
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
    supportsVision: true,
  },
  {
    id: 'builtin-longcat-omni',
    provider: 'LongCat',
    model: 'LongCat-Flash-Omni-2603',
    protocol: 'longcat',
    baseUrl: 'https://api.longcat.chat/openai/v1',
    builtin: true,
    supportsAudio: true,
    supportsText: true,
    supportsVision: true,
  },
  {
    id: 'builtin-mimo-v2.5',
    provider: 'XiaoMi',
    model: 'mimo-v2.5',
    protocol: 'mimo',
    baseUrl: 'https://api.xiaomimimo.com/v1',
    builtin: true,
    supportsAudio: true,
    supportsText: true,
    supportsVision: true,
  },
  {
    id: 'builtin-or-gemini-3-flash',
    provider: 'OpenRouter',
    model: 'google/gemini-3-flash-preview',
    protocol: 'openai-compat',
    baseUrl: 'https://openrouter.ai/api/v1',
    builtin: true,
    supportsAudio: true,
    supportsText: true,
    supportsVision: true,
  },
  {
    id: 'builtin-or-gemini-3.1-flash-lite',
    provider: 'OpenRouter',
    model: 'google/gemini-3.1-flash-lite-preview',
    protocol: 'openai-compat',
    baseUrl: 'https://openrouter.ai/api/v1',
    builtin: true,
    supportsAudio: true,
    supportsText: true,
    supportsVision: true,
  },
  {
    id: 'builtin-deepseek-v4-flash',
    provider: 'DeepSeek',
    model: 'deepseek-v4-flash',
    protocol: 'openai-compat',
    baseUrl: 'https://api.deepseek.com',
    builtin: true,
    supportsAudio: false,
    supportsText: true,
    supportsVision: false,
  },
  {
    id: 'builtin-deepseek-v4-pro',
    provider: 'DeepSeek',
    model: 'deepseek-v4-pro',
    protocol: 'openai-compat',
    baseUrl: 'https://api.deepseek.com',
    builtin: true,
    supportsAudio: false,
    supportsText: true,
    supportsVision: false,
  },
]

export function getAllModels(config: AppConfig): ModelEntry[] {
  const builtins: ModelEntry[] = BUILTIN_MODELS.map(b => {
    let apiKey = ''
    if (b.id.startsWith('builtin-or-')) apiKey = config.models.builtinApiKeys.openrouter
    else if (b.protocol === 'gemini') apiKey = config.models.builtinApiKeys.gemini
    else if (b.id.startsWith('builtin-deepseek-')) apiKey = config.models.builtinApiKeys.deepseek
    else if (b.protocol === 'qwen-omni') apiKey = config.models.builtinApiKeys.dashscope
    else if (b.protocol === 'mimo') apiKey = config.models.builtinApiKeys.mimo
    else if (b.protocol === 'longcat') apiKey = config.models.builtinApiKeys.longcat
    return { ...b, apiKey }
  })
  const customs: ModelEntry[] = config.models.custom.map(c => ({
    ...c,
    builtin: false,
    supportsVision: c.supportsVision ?? true,
  }))
  return [...builtins, ...customs]
}

export function getAudioModels(config: AppConfig): ModelEntry[] {
  return getAllModels(config).filter(m => m.supportsAudio)
}

export function getTextModels(config: AppConfig): ModelEntry[] {
  return getAllModels(config).filter(m => m.supportsText)
}

export function getVisionModels(config: AppConfig): ModelEntry[] {
  return getAllModels(config).filter(m => m.supportsVision)
}

export function findModel(config: AppConfig, modelId: string): ModelEntry | undefined {
  return getAllModels(config).find(m => m.id === modelId)
}
