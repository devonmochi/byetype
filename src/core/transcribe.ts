import type { TranscribeProvider } from './providers/types'
import { GeminiProvider } from './providers/gemini'
import { QwenOmniProvider } from './providers/qwen-omni'
import { withRetry } from './retry'
import { buildTranscribePrompt } from './prompt-loader'
import type { AppConfig } from './types'

export function createTranscribeProvider(config: AppConfig): TranscribeProvider {
  const { model, geminiApiKey, qwenApiKey, thinking } = config.transcribe
  if (model === 'qwen3-omni-flash') {
    return new QwenOmniProvider(qwenApiKey)
  }
  return new GeminiProvider({
    apiKey: geminiApiKey,
    modelId: model,
    thinkingEnabled: thinking.enabled,
    thinkingBudget: thinking.budget,
    thinkingLevel: thinking.level
  })
}

export async function transcribeAudio(
  audioBase64: string,
  config: AppConfig,
  promptPaths: { agent: string; vocabulary: string; rules: string },
  callbacks?: { onRetry?: (attempt: number) => void; onTimeout?: () => void }
): Promise<string> {
  const provider = createTranscribeProvider(config)
  const systemPrompt = await buildTranscribePrompt(
    promptPaths.agent,
    promptPaths.vocabulary,
    promptPaths.rules
  )

  return withRetry(
    (signal) => provider.transcribe(audioBase64, systemPrompt, signal),
    {
      timeoutMs: config.advanced.transcribeTimeout * 1000,
      maxRetries: config.advanced.maxRetries,
      onRetry: callbacks?.onRetry,
      onTimeout: callbacks?.onTimeout
    }
  )
}
