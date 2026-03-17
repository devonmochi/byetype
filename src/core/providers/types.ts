export interface TranscribeProvider {
  name: string
  transcribe(
    audioBase64: string,
    systemPrompt: string,
    signal?: AbortSignal
  ): Promise<string>
}

export interface OptimizeProvider {
  name: string
  optimize(
    text: string,
    systemPrompt: string,
    signal?: AbortSignal
  ): Promise<string>
}
