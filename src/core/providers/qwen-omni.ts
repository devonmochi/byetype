import OpenAI from 'openai'
import { proxyAwareFetch } from '../proxy-fetch'
import type { TranscribeProvider } from './types'

export class QwenOmniProvider implements TranscribeProvider {
  name = 'qwen-omni'
  private client: OpenAI

  constructor(apiKey: string) {
    this.client = new OpenAI({
      apiKey,
      baseURL: 'https://dashscope.aliyuncs.com/compatible-mode/v1',
      dangerouslyAllowBrowser: true,
      fetch: proxyAwareFetch
    })
  }

  async transcribe(audioBase64: string, systemPrompt: string, _signal?: AbortSignal): Promise<string> {
    const dataUri = `data:;base64,${audioBase64}`
    const stream = await this.client.chat.completions.create({
      model: 'qwen3-omni-flash',
      messages: [{
        role: 'user',
        content: [
          { type: 'text', text: systemPrompt },
          {
            type: 'input_audio' as never,
            input_audio: { data: dataUri, format: 'wav' }
          } as never
        ]
      }],
      modalities: ['text'] as never,
      stream: true,
      stream_options: { include_usage: true }
    })

    let result = ''
    for await (const chunk of stream) {
      if (chunk.choices?.[0]?.delta?.content) {
        result += chunk.choices[0].delta.content
      }
    }
    return result
  }
}
