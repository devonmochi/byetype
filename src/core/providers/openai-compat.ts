import OpenAI from 'openai'
import type { OptimizeProvider } from './types'

export class OpenAICompatProvider implements OptimizeProvider {
  name: string
  private client: OpenAI
  private model: string

  constructor(providerName: string, baseUrl: string, model: string, apiKey: string) {
    this.name = providerName
    this.model = model
    this.client = new OpenAI({
      apiKey,
      baseURL: baseUrl,
      dangerouslyAllowBrowser: true
    })
  }

  async optimize(text: string, systemPrompt: string, _signal?: AbortSignal): Promise<string> {
    const response = await this.client.chat.completions.create({
      model: this.model,
      messages: [
        { role: 'user', content: `<voice-input>\n${text}\n</voice-input>\n\n${systemPrompt}` }
      ]
    })
    return response.choices[0]?.message?.content || text
  }
}
