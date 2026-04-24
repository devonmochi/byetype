export type ThemeMode = 'light' | 'dark' | 'system'

export interface AudioDevice {
  name: string
  isDefault: boolean
}

export interface GeneralConfig {
  shortcut: string
  shortcut2: string
  launchAtLogin: boolean
  theme: ThemeMode
  maxRecordingSeconds: number
  microphone: string
  extractShortcut: string
  extractShortcut2: string
  shortcutTemplate: string
  shortcut2Template: string
  extractShortcutTemplate: string
  extractShortcut2Template: string
  shortcutLabel?: string
  shortcut2Label?: string
  extractShortcutLabel?: string
  extractShortcut2Label?: string
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
  protocol: 'gemini' | 'openai-compat' | 'qwen-omni' | 'mimo' | 'longcat'
  baseUrl: string
  apiKey: string
  supportsAudio: boolean
  supportsText: boolean
  supportsVision: boolean
}

export interface BuiltinApiKeys {
  gemini: string
  deepseek: string
  dashscope: string
  openrouter: string
  mimo: string
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

export interface TemplateEntry {
  id: string
  name: string
  prompt: string
}

export interface VoiceTemplatesConfig {
  modelId: string
  thinking: ThinkingConfig
  templates: TemplateEntry[]
}

export interface ExtractConfig {
  modelId?: string
  thinking?: ThinkingConfig
  prompt: string
  templates: TemplateEntry[]
}

export interface AdvancedConfig {
  transcribeTimeout: number
  optimizeTimeout: number
  maxRetries: number
  maxParallel: number
  proxyEnabled: boolean
  proxyUrl: string
}

export interface AppConfig {
  general: GeneralConfig
  models: ModelsConfig
  transcribe: TranscribeConfig
  voiceTemplates: VoiceTemplatesConfig
  extract: ExtractConfig
  advanced: AdvancedConfig
}

export type TaskStatus = 'recording' | 'transcribing' | 'optimizing' | 'retrying' | 'extracting' | 'completed' | 'failed' | 'cancelled'

export interface HistoryRecord {
  id: number
  createdAt: string
  audioPath: string | null
  transcribeText: string | null
  optimizeText: string | null
  status: 'completed' | 'failed' | 'cancelled'
  errorMessage?: string
  recordType?: 'voice' | 'extract'
  screenshotPath?: string | null
  extractText?: string | null
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
