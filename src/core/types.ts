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

export interface CustomModelEntry {
  id: string
  provider: string
  model: string
  protocol: 'gemini' | 'openai-compat' | 'longcat'
  baseUrl: string
  apiKey: string
  supportsAudio: boolean
  supportsText: boolean
}

export interface BuiltinApiKeys {
  gemini: string
  deepseek: string
  longcat: string
}

export interface ModelsConfig {
  builtinApiKeys: BuiltinApiKeys
  custom: CustomModelEntry[]
}

export interface TranscribeConfig {
  modelId: string
  thinking: ThinkingConfig
  prompts: { agent: string; rules: string; vocabulary: string }
}

export interface OptimizeConfig {
  enabled: boolean
  modelId: string
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
  models: ModelsConfig
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

export interface UpdateInfo {
  version: string
  body: string | null
}

export type UpdatePhase = 'idle' | 'checking' | 'available' | 'downloading' | 'downloaded' | 'error'

export interface UpdateState {
  phase: UpdatePhase
  info: UpdateInfo | null
  progress: number
  error: string | null
  dismissed: boolean
  checkedOnce: boolean
}
