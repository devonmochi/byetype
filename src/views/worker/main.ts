import { TaskManager } from '../../core/task-manager'
import { HistoryManager } from '../../core/history'
import { listen } from '@tauri-apps/api/event'
import { getConfig } from '../../lib/tauri-api'

let taskManager: TaskManager | null = null
let currentTaskId: number | null = null

async function init() {
  const { emit } = await import('@tauri-apps/api/event')

  const historyManager = new HistoryManager((records) => {
    // Broadcast history changes to all windows (settings listens)
    emit('history-updated', records).catch(console.error)
  })
  await historyManager.init()
  taskManager = new TaskManager(historyManager)

  listen('recording-started', async () => {
    if (!taskManager) return
    const config = await getConfig()
    if (taskManager.canCreateTask(config)) {
      currentTaskId = taskManager.createTask()
      console.log(`[Worker] Task created: #${currentTaskId}`)
    }
  })

  listen<{ audio: string }>('recording-complete', (event) => {
    if (!taskManager || currentTaskId === null) return
    const taskId = currentTaskId
    currentTaskId = null
    console.log(`[Worker] Processing recording for task #${taskId}`)
    taskManager.processRecording(taskId, event.payload.audio)
  })

  listen<{ message: string }>('recording-error', (event) => {
    console.error(`[Worker] Recording error:`, event.payload.message)
    currentTaskId = null
  })

  // Listen for retry requests from settings window (via Rust command bridge)
  listen<{ recordId: number }>('retry-request', async (event) => {
    if (!taskManager) return
    const { recordId } = event.payload
    console.log(`[Worker] Retry request for record #${recordId}`)
    const result = await taskManager.retryFromHistory(recordId)
    if (!result.accepted) {
      console.warn(`[Worker] Retry rejected: ${result.reason}`)
    }
  })

  // Keepalive: prevent macOS from suspending WKWebView JS after idle
  setInterval(() => {
    // Minimal work to keep the JS event loop alive
    void 0
  }, 5000)

  console.log('[Worker] Pipeline ready')
}

init().catch(console.error)
