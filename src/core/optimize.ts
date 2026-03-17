import type { OptimizeProvider } from './providers/types'
import { GeminiProvider } from './providers/gemini'
import { OpenAICompatProvider } from './providers/openai-compat'
import { withRetry } from './retry'
import { loadPromptAsDocument } from './prompt-loader'
import type { AppConfig } from './types'

export function createOptimizeProvider(config: AppConfig): OptimizeProvider {
  const { type, openaiCompat, geminiModel } = config.optimize
  if (type === 'gemini') {
    const { thinking } = config.optimize
    return new GeminiProvider({
      apiKey: config.transcribe.geminiApiKey,
      modelId: geminiModel,
      thinkingEnabled: thinking.enabled,
      thinkingBudget: thinking.budget,
      thinkingLevel: thinking.level
    })
  }
  return new OpenAICompatProvider(
    openaiCompat.providerName,
    openaiCompat.baseUrl,
    openaiCompat.model,
    openaiCompat.apiKey
  )
}

export async function optimizeText(
  text: string,
  config: AppConfig,
  promptPath: string,
  callbacks?: { onRetry?: (attempt: number) => void; onTimeout?: () => void }
): Promise<string> {
  if (!config.optimize.enabled) return text

  const provider = createOptimizeProvider(config)
  const systemPrompt = await loadPromptAsDocument(promptPath)
  if (!systemPrompt) return text

  return withRetry(
    (signal) => provider.optimize(text, systemPrompt, signal),
    {
      timeoutMs: config.advanced.optimizeTimeout * 1000,
      maxRetries: config.advanced.maxRetries,
      onRetry: callbacks?.onRetry,
      onTimeout: callbacks?.onTimeout
    }
  )
}
