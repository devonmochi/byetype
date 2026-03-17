export interface RetryOptions {
  timeoutMs: number
  maxRetries: number
  onRetry?: (attempt: number) => void
  onTimeout?: () => void
}

export async function withRetry<T>(
  fn: (signal: AbortSignal) => Promise<T>,
  options: RetryOptions
): Promise<T> {
  const { timeoutMs, maxRetries, onRetry, onTimeout } = options

  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    const controller = new AbortController()
    const timer = setTimeout(() => controller.abort(), timeoutMs)

    try {
      const result = await fn(controller.signal)
      clearTimeout(timer)
      return result
    } catch (err: unknown) {
      clearTimeout(timer)
      if (controller.signal.aborted) {
        onTimeout?.()
      }
      if (attempt < maxRetries) {
        onRetry?.(attempt + 1)
        continue
      }
      throw err
    }
  }
  throw new Error('All retries exhausted')
}
