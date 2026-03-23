export type ThemeMode = 'light' | 'dark' | 'system'

export interface AudioDevice {
  name: string
  isDefault: boolean
}

export interface GeneralConfig {
  shortcut: string
  launchAtLogin: boolean
  theme: ThemeMode
  maxRecordingSeconds: number
  microphone: string
}

export interface ThinkingConfig {
  enabled: boolean
  budget: number
  level: 'MINIMAL' | 'LOW' | 'MEDIUM' | 'HIGH'
}

export interface TranscribeConfig {
  model: 'gemini-3-flash-preview' | 'gemini-3.1-flash-lite-preview' | 'qwen3-omni-flash'
  geminiApiKey: string
  qwenApiKey: string
  thinking: ThinkingConfig
  prompts: {
    agent: string
    rules: string
    vocabulary: string
  }
}

export interface OpenAICompatConfig {
  providerName: string
  baseUrl: string
  model: string
  apiKey: string
}

export interface OptimizeConfig {
  enabled: boolean
  type: 'openai-compat' | 'gemini'
  openaiCompat: OpenAICompatConfig
  geminiModel: string
  thinking: ThinkingConfig
  prompt: string
}

export interface AdvancedConfig {
  transcribeTimeout: number
  optimizeTimeout: number
  maxRetries: number
  maxParallel: number
  proxyUrl: string
}

export interface AppConfig {
  general: GeneralConfig
  transcribe: TranscribeConfig
  optimize: OptimizeConfig
  advanced: AdvancedConfig
}

export type TaskStatus = 'recording' | 'transcribing' | 'optimizing' | 'retrying' | 'completed' | 'failed' | 'cancelled'

export interface HistoryRecord {
  id: number
  createdAt: string
  audioPath: string | null
  transcribeText: string | null
  optimizeText: string | null
  status: 'completed' | 'failed' | 'cancelled'
  errorMessage?: string
}

export interface RetryStatusUpdate {
  recordId: number
  stage: 'transcribing' | 'optimizing' | 'retrying' | 'cancelled'
}
