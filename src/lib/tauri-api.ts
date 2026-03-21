import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs'
import type { AppConfig } from '../core/types'

// Config commands
export async function getConfig(): Promise<AppConfig> {
  return invoke<AppConfig>('get_config')
}

export async function saveConfig(config: AppConfig): Promise<boolean> {
  return invoke<boolean>('save_config', { config })
}

// Prompt commands
export async function getPromptsDir(): Promise<string> {
  return invoke<string>('get_prompts_dir')
}

export async function getBuiltinPromptPath(filename: string): Promise<string> {
  return invoke<string>('get_builtin_prompt_path', { filename })
}

export async function copyBuiltinPrompt(filename: string, force: boolean = false): Promise<string> {
  return invoke<string>('copy_builtin_prompt', { filename, force })
}

export async function isBuiltinPromptPath(path: string): Promise<boolean> {
  return invoke<boolean>('is_builtin_prompt_path', { path })
}

// File operations
export async function selectFile(): Promise<string | null> {
  const result = await openDialog({
    filters: [{ name: 'Markdown', extensions: ['md'] }],
    multiple: false,
  })
  return result as string | null
}

export async function openFile(path: string): Promise<void> {
  await invoke('open_file', { path })
}

export async function readPromptFile(path: string): Promise<string> {
  return readTextFile(path)
}

export async function writePromptFile(path: string, content: string): Promise<void> {
  return writeTextFile(path, content)
}

// Theme
export async function getTheme(): Promise<string> {
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
}

export function onThemeChange(callback: (theme: string) => void): () => void {
  const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
  const handler = (e: MediaQueryListEvent) => {
    callback(e.matches ? 'dark' : 'light')
  }
  mediaQuery.addEventListener('change', handler)
  return () => mediaQuery.removeEventListener('change', handler)
}

// Event listeners
export async function onEvent<T>(event: string, callback: (payload: T) => void): Promise<UnlistenFn> {
  return listen<T>(event, (e) => callback(e.payload))
}

// Recording state
export async function getRecordingState(): Promise<boolean> {
  return invoke<boolean>('get_recording_state')
}

// Launch at login
export async function setLaunchAtLogin(enabled: boolean): Promise<void> {
  await invoke('set_launch_at_login', { enabled })
}

export async function getLaunchAtLogin(): Promise<boolean> {
  return invoke<boolean>('get_launch_at_login')
}


