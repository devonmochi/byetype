import { readTextFile } from '@tauri-apps/plugin-fs'

async function loadPrompt(filePath: string): Promise<string> {
  if (!filePath) return ''
  try {
    return await readTextFile(filePath)
  } catch {
    return ''
  }
}

function wrapDocument(name: string, content: string): string {
  if (!content) return ''
  return `<document name="${name}">\n${content.trim()}\n</document>`
}

export async function loadPromptAsDocument(filePath: string): Promise<string> {
  const content = await loadPrompt(filePath)
  if (!content) return ''
  const name = filePath.split('/').pop() || filePath
  return wrapDocument(name, content)
}

export function resolvePromptPath(customPath: string, builtinPath: string): string {
  return customPath || builtinPath
}

export async function buildTranscribePrompt(
  agentPath: string,
  vocabularyPath: string,
  rulesPath: string
): Promise<string> {
  const [agent, vocabulary, rules] = await Promise.all([
    loadPrompt(agentPath),
    loadPrompt(vocabularyPath),
    loadPrompt(rulesPath)
  ])

  return [
    wrapDocument('agent.md', agent),
    wrapDocument('vocabulary.md', vocabulary),
    wrapDocument('rules.md', rules)
  ].filter(Boolean).join('\n\n')
}
