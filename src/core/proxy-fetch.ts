import { getConfig } from '../lib/tauri-api'
import { proxyRequest } from '../lib/tauri-api'

export async function proxyAwareFetch(
  input: RequestInfo | URL,
  init?: RequestInit
): Promise<Response> {
  const config = await getConfig()
  const proxyUrl = config.advanced.proxyUrl

  if (!proxyUrl) {
    return fetch(input, init)
  }

  const url = typeof input === 'string' ? input : input instanceof URL ? input.toString() : input.url
  const method = init?.method || 'GET'
  const headers: Record<string, string> = {}
  if (init?.headers) {
    const h = new Headers(init.headers)
    h.forEach((v, k) => { headers[k] = v })
  }
  const body = init?.body ? (typeof init.body === 'string' ? init.body : undefined) : undefined

  const resp = await proxyRequest({ url, method, headers, body })

  return new Response(resp.body, {
    status: resp.status,
    headers: resp.headers,
  })
}
