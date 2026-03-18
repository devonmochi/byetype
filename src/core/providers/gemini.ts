import { GoogleGenAI } from '@google/genai'
import { proxyAwareFetch } from '../proxy-fetch'
import type { TranscribeProvider, OptimizeProvider } from './types'

interface GeminiConfig {
  apiKey: string
  modelId: string
  thinkingEnabled: boolean
  thinkingBudget?: number
  thinkingLevel?: 'MINIMAL' | 'LOW' | 'MEDIUM' | 'HIGH'
}

export class GeminiProvider implements TranscribeProvider, OptimizeProvider {
  name = 'gemini'
  private client: GoogleGenAI
  private config: GeminiConfig

  constructor(config: GeminiConfig) {
    this.config = config
    this.client = new GoogleGenAI({ apiKey: config.apiKey, httpOptions: { fetch: proxyAwareFetch } as any })
  }

  private buildThinkingConfig(): Record<string, unknown> {
    if (!this.config.thinkingEnabled) {
      if (this.config.modelId === 'gemini-2.5-flash') {
        return { thinkingConfig: { thinkingBudget: 0 } }
      }
      return {}
    }
    if (this.config.modelId === 'gemini-2.5-flash') {
      return { thinkingConfig: { thinkingBudget: this.config.thinkingBudget || 1024 } }
    }
    return { thinkingConfig: { thinkingLevel: this.config.thinkingLevel || 'LOW' } }
  }

  async transcribe(audioBase64: string, systemPrompt: string, _signal?: AbortSignal): Promise<string> {
    const thinkingConfig = this.buildThinkingConfig()
    const response = await this.client.models.generateContent({
      model: this.config.modelId,
      contents: [
        { text: systemPrompt },
        { inlineData: { data: audioBase64, mimeType: 'audio/wav' } }
      ],
      config: thinkingConfig
    })
    return response.text || ''
  }

  async optimize(text: string, systemPrompt: string, _signal?: AbortSignal): Promise<string> {
    const thinkingConfig = this.buildThinkingConfig()
    const response = await this.client.models.generateContent({
      model: this.config.modelId,
      contents: [
        { text: `<voice-input>\n${text}\n</voice-input>\n\n${systemPrompt}` }
      ],
      config: thinkingConfig
    })
    return response.text || ''
  }
}
